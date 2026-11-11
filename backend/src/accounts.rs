use anyhow::{Context, Result, ensure};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{auth, error::email};

pub(crate) struct NewPassword(String);

impl NewPassword {
    pub(crate) fn parse(value: String) -> Result<Self> {
        ensure!(
            value.chars().count() >= 12 && value.len() <= 256,
            "password must have at least 12 characters and at most 256 bytes"
        );
        Ok(Self(value))
    }

    pub(crate) fn hash(&self) -> Result<String> {
        auth::hash_password(&self.0)
    }
}

pub(crate) enum Change<'a> {
    Password(&'a NewPassword),
    Disabled(bool),
}

pub(crate) async fn update(pool: &PgPool, address: &str, change: Change<'_>) -> Result<Uuid> {
    let address = email(address)?;
    let (password_hash, disabled) = match change {
        Change::Password(password) => (Some(password.hash()?), None),
        Change::Disabled(disabled) => (None, Some(disabled)),
    };
    let mut tx = pool.begin().await?;
    let id = sqlx::query_scalar::<_, Uuid>(
        "UPDATE accounts SET password_hash = coalesce($2, password_hash),
            disabled = coalesce($3, disabled), auth_version = auth_version + 1
         WHERE email = $1 RETURNING id",
    )
    .bind(address)
    .bind(password_hash)
    .bind(disabled)
    .fetch_optional(&mut *tx)
    .await?
    .context("account not found")?;
    sqlx::query("DELETE FROM sessions WHERE account_id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "UPDATE help_requests SET consent_until = clock_timestamp()
         WHERE student_id = $1 AND claimed_at IS NULL AND consent_until > clock_timestamp()",
    )
    .bind(id)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(id)
}

#[cfg(test)]
mod tests {
    use super::NewPassword;

    #[test]
    fn rejects_short_or_oversized_passwords() {
        assert!(NewPassword::parse("a".repeat(11)).is_err());
        assert!(NewPassword::parse("a".repeat(257)).is_err());
        assert!(NewPassword::parse("界".repeat(11)).is_err());
        assert!(NewPassword::parse("界".repeat(86)).is_err());
    }

    #[test]
    fn accepts_passwords_at_the_supported_character_and_byte_limits() {
        assert!(NewPassword::parse("a".repeat(12)).is_ok());
        assert!(NewPassword::parse("a".repeat(256)).is_ok());
        assert!(NewPassword::parse("界".repeat(12)).is_ok());
        assert!(NewPassword::parse("界".repeat(85)).is_ok());
    }
}
