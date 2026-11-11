mod accounts;
mod auth;
mod classroom;
mod error;
mod help;
mod model;
mod operator;
mod veyon;

#[cfg(test)]
mod api_tests;

use std::{env, net::SocketAddr, sync::Arc};

use anyhow::{Context, Result};
use axum::{
    Router,
    extract::Request,
    extract::{DefaultBodyLimit, FromRef, State},
    http::{HeaderValue, StatusCode, header::CACHE_CONTROL},
    middleware::{Next, from_fn},
    response::Response,
    routing::{get, post},
};
use clap::Parser;
use sqlx::{PgPool, postgres::PgPoolOptions};

#[derive(Clone, FromRef)]
pub(crate) struct App {
    pool: PgPool,
    login: Arc<auth::LoginGuard>,
    remote_enabled: bool,
}

#[derive(Parser)]
#[command(version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<operator::Command>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let cli = Cli::parse();
    if let Some(operator::Command::Veyon(args)) = &cli.command {
        return veyon::run(args).await;
    }
    let pool = PgPoolOptions::new()
        .max_connections(12)
        .connect(&env::var("DATABASE_URL").context("set DATABASE_URL")?)
        .await
        .context("connect to PostgreSQL")?;
    sqlx::migrate!()
        .run(&pool)
        .await
        .context("migrate PostgreSQL")?;

    match cli.command {
        Some(command) => operator::run(command, &pool).await,
        None => serve(pool).await,
    }
}

async fn serve(pool: PgPool) -> Result<()> {
    let remote_enabled = env::var("REMOTE_ASSISTANCE")
        .map_or(Ok(false), |value| value.parse::<bool>())
        .context("REMOTE_ASSISTANCE must be true or false")?;
    let app = App {
        pool,
        login: Arc::new(auth::LoginGuard::new()?),
        remote_enabled,
    };
    let addr: SocketAddr = env::var("BIND_ADDR")
        .unwrap_or_else(|_| "127.0.0.1:3000".into())
        .parse()
        .context("invalid BIND_ADDR")?;
    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!(address = %listener.local_addr()?, remote_enabled, "SkillBridge API listening");
    axum::serve(listener, router(app))
        .with_graceful_shutdown(async {
            if let Err(error) = tokio::signal::ctrl_c().await {
                tracing::error!(%error, "could not wait for Ctrl-C");
            }
        })
        .await?;
    Ok(())
}

fn router(app: App) -> Router {
    Router::new()
        .nest(
            "/api",
            Router::new()
                .route("/health", get(|| async { StatusCode::NO_CONTENT }))
                .route("/ready", get(ready))
                .route("/session", post(auth::login).delete(auth::logout))
                .route("/classes", get(classroom::list).post(classroom::create))
                .route("/classes/{id}", get(classroom::detail))
                .route("/classes/{id}/enrollments", post(classroom::enroll))
                .route("/classes/{id}/help", get(help::list).post(help::create))
                .route("/help/{id}/messages", get(help::messages).post(help::reply))
                .route("/help/{id}/close", post(help::close))
                .route("/help/{id}/remote", post(help::claim))
                .layer(DefaultBodyLimit::max(16 * 1024))
                .layer(from_fn(no_cache)),
        )
        .with_state(app)
}

async fn ready(State(pool): State<PgPool>) -> StatusCode {
    match sqlx::query("SELECT 1").execute(&pool).await {
        Ok(_) => StatusCode::NO_CONTENT,
        Err(_) => StatusCode::SERVICE_UNAVAILABLE,
    }
}

async fn no_cache(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));
    response
}
