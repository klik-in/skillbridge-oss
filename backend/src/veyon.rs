use std::{
    collections::HashMap,
    net::Ipv4Addr,
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use anyhow::{Context, Result, bail, ensure};
use reqwest::{Client, StatusCode, redirect::Policy};
use serde::Deserialize;
use url::Url;
use uuid::Uuid;

use crate::{auth::Credentials, error::email, help::RemoteClaim};

#[derive(clap::Args)]
pub(crate) struct Args {
    /// Trusted workstation-local TOML configuration.
    #[arg(long)]
    config: PathBuf,
    /// Instructor email.
    #[arg(long)]
    email: String,
    /// Help request ID shown in the classroom.
    #[arg(long)]
    ticket: Uuid,
    /// Seat ID shown on the consented help request.
    #[arg(long)]
    seat: Uuid,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Config {
    origin: Url,
    veyon_cli: PathBuf,
    seats: HashMap<Uuid, Ipv4Addr>,
}

#[derive(Deserialize)]
struct Token {
    token: String,
}

impl Config {
    fn target(&self, seat: Uuid) -> Result<Ipv4Addr> {
        let host = *self
            .seats
            .get(&seat)
            .context("seat is absent from the local allowlist")?;
        ensure!(
            host.is_private() && !matches!(host.octets()[3], 0 | 255),
            "seat must be a unicast RFC 1918 IPv4 address"
        );
        Ok(host)
    }

    fn validate(&self) -> Result<()> {
        let origin = &self.origin;
        let loopback = origin.host_str() == Some("127.0.0.1");
        ensure!(
            (origin.scheme() == "https" || (loopback && origin.scheme() == "http"))
                && origin.host().is_some()
                && origin.username().is_empty()
                && origin.password().is_none()
                && origin.path() == "/"
                && origin.query().is_none()
                && origin.fragment().is_none(),
            "origin must be an HTTPS origin or local development HTTP at 127.0.0.1"
        );
        ensure!(
            self.veyon_cli.is_absolute() && self.veyon_cli.is_file(),
            "configure an absolute Veyon CLI executable"
        );
        let filename = self.veyon_cli.file_name().and_then(|name| name.to_str());
        ensure!(
            matches!(filename, Some("veyon-cli" | "veyon-cli.exe")),
            "configure the veyon-cli executable"
        );
        Ok(())
    }
}

pub(crate) async fn run(args: &Args) -> Result<()> {
    let config: Config =
        toml::from_str(&std::fs::read_to_string(&args.config).context("read local Veyon config")?)
            .context("parse local Veyon config")?;
    config.validate()?;
    let target = config.target(args.seat)?;
    let address = email(&args.email)?;
    println!("Ticket {} on seat {} ({target})", args.ticket, args.seat);
    let answer =
        rpassword::prompt_password("Verify the seat with the student. Type LAUNCH to continue: ")?;
    ensure!(answer == "LAUNCH", "launch cancelled");
    let input = Credentials {
        email: address,
        password: rpassword::prompt_password("Instructor password: ")?,
    };
    let client = Client::builder()
        .redirect(Policy::none())
        .no_proxy()
        .timeout(Duration::from_secs(8))
        .build()?;
    let endpoint = config.origin.join("/api/session")?;
    let response = client.post(endpoint.clone()).json(&input).send().await?;
    ensure!(
        response.status() == StatusCode::OK,
        "sign-in rejected (HTTP {})",
        response.status()
    );
    let token = response.json::<Token>().await?.token;

    let claim = config
        .origin
        .join(&format!("/api/help/{}/remote", args.ticket))?;
    let launch = async {
        let response = client
            .post(claim)
            .bearer_auth(&token)
            .json(&RemoteClaim { seat_id: args.seat })
            .send()
            .await?;
        if response.status() != StatusCode::NO_CONTENT {
            bail!(
                "consent claim rejected (HTTP {}); Veyon was not launched",
                response.status()
            );
        }
        local_command(&config.veyon_cli, target)
            .spawn()
            .context("launch Veyon (consent has been consumed)")
    }
    .await;
    // The temporary API token is never passed to Veyon or persisted by the helper.
    match client.delete(endpoint).bearer_auth(token).send().await {
        Ok(response) if response.status() == StatusCode::NO_CONTENT => {}
        Ok(response) => {
            tracing::warn!(status = %response.status(), "could not revoke temporary session")
        }
        Err(error) => tracing::warn!(%error, "could not revoke temporary session"),
    }
    let status = launch?.wait().context("wait for Veyon to close")?;
    ensure!(
        status.success(),
        "Veyon exited with {status}; consent has been consumed"
    );
    Ok(())
}

fn local_command(executable: &Path, target: Ipv4Addr) -> Command {
    let mut command = Command::new(executable);
    command
        .args(["remoteaccess", "control"])
        .arg(target.to_string());
    command
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_a_public_target_even_when_it_is_in_the_allowlist() -> Result<()> {
        let seat = Uuid::new_v4();
        let config = Config {
            origin: Url::parse("https://class.example.test")?,
            veyon_cli: PathBuf::new(),
            seats: HashMap::from([(seat, Ipv4Addr::new(8, 8, 8, 8))]),
        };

        assert!(config.target(seat).is_err());
        Ok(())
    }

    #[test]
    fn builds_only_the_fixed_veyon_control_command_for_the_local_address() {
        let executable = Path::new("/opt/veyon-cli");
        let command = local_command(executable, Ipv4Addr::new(192, 168, 10, 22));

        assert_eq!(command.get_program(), executable.as_os_str());
        assert_eq!(
            command.get_args().collect::<Vec<_>>(),
            ["remoteaccess", "control", "192.168.10.22"]
        );
    }
}
