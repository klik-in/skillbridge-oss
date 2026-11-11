use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;

use super::{PASSWORD, PEER, STUDENT, TEACHER, data, fixture, request};
use crate::{
    accounts::{self, Change, NewPassword},
    operator::{self, Command},
};

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn password_reset_revokes_all_sessions_and_accepts_only_the_new_password(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, true).await?;
    sqlx::query(
        "INSERT INTO sessions (digest, account_id)
         SELECT decode(repeat('ee', 32), 'hex'), id FROM accounts WHERE email = 'student@example.test'",
    ).execute(&pool).await?;
    let password = NewPassword::parse("new fictional password".to_owned())?;

    accounts::update(&pool, "Student@example.test", Change::Password(&password)).await?;

    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM sessions s JOIN accounts a ON a.id = s.account_id
         WHERE a.email = 'student@example.test'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 0);
    assert_eq!(
        request(
            &setup.app,
            Method::GET,
            "/api/classes",
            Some(STUDENT),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &setup.app,
            Method::GET,
            "/api/classes",
            Some(TEACHER),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            "/api/session",
            None,
            json!({"email": "student@example.test", "password": PASSWORD})
        )
        .await?
        .status(),
        StatusCode::UNAUTHORIZED
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            "/api/session",
            None,
            json!({"email": "student@example.test", "password": "new fictional password"})
        )
        .await?
        .status(),
        StatusCode::OK
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn disabling_an_account_preserves_its_classroom_records_and_cancels_pending_remote_consent(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, true).await?;

    operator::run(
        Command::DisableAccount {
            email: "student@example.test".to_owned(),
        },
        &pool,
    )
    .await?;

    let endpoint = format!("/api/help/{}/remote", setup.ticket_id);
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &endpoint,
            Some(TEACHER),
            json!({"seat_id": setup.seat_id})
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );
    let tickets = data(
        request(
            &setup.app,
            Method::GET,
            &format!("/api/classes/{}/help", setup.class_id),
            Some(TEACHER),
            Value::Null,
        )
        .await?,
    )
    .await?;
    assert_eq!(tickets[0]["id"], json!(setup.ticket_id));
    assert_eq!(tickets[0]["remote_available"], false);
    assert_eq!(
        request(
            &setup.app,
            Method::GET,
            "/api/classes",
            Some(STUDENT),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let count = sqlx::query_scalar::<_, i64>(
        "SELECT count(*) FROM sessions s JOIN accounts a ON a.id = s.account_id
         WHERE a.email = 'student@example.test'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(count, 0);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn reenabling_requires_fresh_sign_in_and_keeps_previous_consent_expired(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, true).await?;
    operator::run(
        Command::DisableAccount {
            email: "student@example.test".to_owned(),
        },
        &pool,
    )
    .await?;

    operator::run(
        Command::EnableAccount {
            email: "Student@example.test".to_owned(),
        },
        &pool,
    )
    .await?;

    assert_eq!(
        request(
            &setup.app,
            Method::GET,
            "/api/classes",
            Some(STUDENT),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::UNAUTHORIZED
    );
    let response = request(
        &setup.app,
        Method::POST,
        "/api/session",
        None,
        json!({"email": "student@example.test", "password": PASSWORD}),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::OK);
    let payload = data(response).await?;
    let token = payload["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing token"))?;
    let classes = data(
        request(
            &setup.app,
            Method::GET,
            "/api/classes",
            Some(token),
            Value::Null,
        )
        .await?,
    )
    .await?;
    assert_eq!(classes[0]["id"], json!(setup.class_id));
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &format!("/api/help/{}/remote", setup.ticket_id),
            Some(TEACHER),
            json!({"seat_id": setup.seat_id})
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn password_reset_keeps_a_disabled_account_disabled(pool: PgPool) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    operator::run(
        Command::DisableAccount {
            email: "student@example.test".to_owned(),
        },
        &pool,
    )
    .await?;
    let password = NewPassword::parse("new fictional password".to_owned())?;

    accounts::update(&pool, "student@example.test", Change::Password(&password)).await?;

    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            "/api/session",
            None,
            json!({"email": "student@example.test", "password": "new fictional password"})
        )
        .await?
        .status(),
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn changing_an_unknown_account_leaves_other_accounts_signed_in(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;

    let result = operator::run(
        Command::DisableAccount {
            email: "missing@example.test".to_owned(),
        },
        &pool,
    )
    .await;

    assert!(result.is_err());
    for token in [STUDENT, TEACHER, PEER] {
        assert_eq!(
            request(
                &setup.app,
                Method::GET,
                "/api/classes",
                Some(token),
                Value::Null
            )
            .await?
            .status(),
            StatusCode::OK
        );
    }
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn rolls_back_password_replacement_when_session_revocation_fails(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    sqlx::raw_sql(
        "CREATE FUNCTION reject_revocation() RETURNS trigger LANGUAGE plpgsql AS $$
             BEGIN RAISE EXCEPTION 'test session deletion failure'; END $$;
         CREATE TRIGGER reject_revocation BEFORE DELETE ON sessions
             FOR EACH ROW EXECUTE FUNCTION reject_revocation();",
    )
    .execute(&pool)
    .await?;
    let password = NewPassword::parse("new fictional password".to_owned())?;

    let result = accounts::update(&pool, "student@example.test", Change::Password(&password)).await;

    assert!(result.is_err());
    assert_eq!(
        request(
            &setup.app,
            Method::GET,
            "/api/classes",
            Some(STUDENT),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::OK
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            "/api/session",
            None,
            json!({"email": "student@example.test", "password": PASSWORD})
        )
        .await?
        .status(),
        StatusCode::OK
    );
    let version = sqlx::query_scalar::<_, i64>(
        "SELECT auth_version FROM accounts WHERE email = 'student@example.test'",
    )
    .fetch_one(&pool)
    .await?;
    assert_eq!(version, 0);
    Ok(())
}
