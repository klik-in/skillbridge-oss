use std::{
    collections::VecDeque,
    sync::Arc,
    time::{Duration, Instant},
};

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::{FromRequestParts, State},
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tokio::sync::{Mutex, Semaphore};
use uuid::Uuid;

use crate::{
    App,
    error::{ApiError, email},
    model::{Account, Role},
};

pub(crate) struct LoginGuard {
    attempts: Mutex<VecDeque<Instant>>,
    workers: Arc<Semaphore>,
    dummy_hash: String,
}

impl LoginGuard {
    pub(crate) fn new() -> anyhow::Result<Self> {
        Ok(Self {
            attempts: Mutex::new(VecDeque::new()),
            workers: Arc::new(Semaphore::new(2)),
            dummy_hash: hash_password(&random_secret())?,
        })
    }

    async fn record_attempt(&self) -> Result<(), ApiError> {
        let mut attempts = self.attempts.lock().await;
        let now = Instant::now();
        while attempts
            .front()
            .is_some_and(|time| now.duration_since(*time) >= Duration::from_secs(60))
        {
            attempts.pop_front();
        }
        if attempts.len() >= 40 {
            return Err(ApiError::Throttled);
        }
        attempts.push_back(now);
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Credentials {
    pub email: String,
    pub password: String,
}

#[derive(Serialize)]
pub(crate) struct LoginResponse {
    token: String,
    account: Account,
}

#[derive(sqlx::FromRow)]
struct LoginAccount {
    #[sqlx(flatten)]
    account: Account,
    password_hash: String,
}

pub(crate) struct Session {
    pub account: Account,
    digest: Vec<u8>,
}

impl Session {
    pub(crate) const fn instructor(&self) -> Result<(), ApiError> {
        match self.account.role {
            Role::Instructor => Ok(()),
            Role::Student => Err(ApiError::Forbidden),
        }
    }

    pub(crate) const fn student(&self) -> Result<(), ApiError> {
        match self.account.role {
            Role::Student => Ok(()),
            Role::Instructor => Err(ApiError::Forbidden),
        }
    }
}

impl FromRequestParts<App> for Session {
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, app: &App) -> Result<Self, Self::Rejection> {
        let token = parts
            .headers
            .get(AUTHORIZATION)
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "))
            .filter(|value| value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
            .ok_or(ApiError::Unauthorized)?;
        let digest = digest(token);
        let account = sqlx::query_as::<_, Account>(
            "SELECT a.id, a.name, a.role FROM accounts a JOIN sessions s ON s.account_id = a.id
             WHERE s.digest = $1 AND s.expires_at > now()",
        )
        .bind(&digest)
        .fetch_optional(&app.pool)
        .await?
        .ok_or(ApiError::Unauthorized)?;
        Ok(Self { account, digest })
    }
}

pub(crate) fn random_secret() -> String {
    format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple())
}

fn digest(token: &str) -> Vec<u8> {
    Sha256::digest(token.as_bytes()).to_vec()
}

pub(crate) fn hash_password(password: &str) -> anyhow::Result<String> {
    let salt = SaltString::encode_b64(Uuid::new_v4().as_bytes())
        .map_err(|error| anyhow::anyhow!("generate password salt: {error}"))?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|error| anyhow::anyhow!("hash password: {error}"))
}

pub(crate) async fn login(
    State(app): State<App>,
    Json(input): Json<Credentials>,
) -> Result<Json<LoginResponse>, ApiError> {
    let email = email(&input.email).map_err(|_| ApiError::Unauthorized)?;
    if input.password.is_empty() || input.password.len() > 256 {
        return Err(ApiError::Unauthorized);
    }
    app.login.record_attempt().await?;
    let slot = app
        .login
        .workers
        .clone()
        .try_acquire_owned()
        .map_err(|_| ApiError::Throttled)?;
    let entry = sqlx::query_as::<_, LoginAccount>(
        "SELECT id, name, role, password_hash FROM accounts WHERE email = $1",
    )
    .bind(email)
    .fetch_optional(&app.pool)
    .await?;
    let guard = app.login.clone();
    let account = tokio::task::spawn_blocking(move || {
        let _slot = slot;
        let hash = entry
            .as_ref()
            .map_or(guard.dummy_hash.as_str(), |a| a.password_hash.as_str());
        let parsed = PasswordHash::new(hash).map_err(|_| ApiError::Internal)?;
        if Argon2::default()
            .verify_password(input.password.as_bytes(), &parsed)
            .is_err()
        {
            return Err(ApiError::Unauthorized);
        }
        entry
            .map(|record| record.account)
            .ok_or(ApiError::Unauthorized)
    })
    .await
    .map_err(|_| ApiError::Internal)??;

    let token = random_secret();
    sqlx::query("DELETE FROM sessions WHERE expires_at <= now()")
        .execute(&app.pool)
        .await?;
    sqlx::query("INSERT INTO sessions (digest, account_id) VALUES ($1, $2)")
        .bind(digest(&token))
        .bind(account.id)
        .execute(&app.pool)
        .await?;
    Ok(Json(LoginResponse { token, account }))
}

pub(crate) async fn logout(
    State(pool): State<PgPool>,
    session: Session,
) -> Result<StatusCode, ApiError> {
    sqlx::query("DELETE FROM sessions WHERE digest = $1")
        .bind(session.digest)
        .execute(&pool)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
