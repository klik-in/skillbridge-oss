use axum::{
    Router,
    body::{Body, to_bytes},
    http::{Method, Request, StatusCode},
    response::Response,
};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use tower::ServiceExt;
use uuid::Uuid;

use crate::{App, auth, model::Role, router};

#[path = "account_tests.rs"]
mod account_tests;

#[path = "practice_tests.rs"]
mod practice_tests;

const TEACHER: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const STUDENT: &str = "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb";
const PEER: &str = "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc";
const OTHER_TEACHER: &str = "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd";
const PASSWORD: &str = "fictional test password";

struct Fixture {
    app: Router,
    class_id: Uuid,
    ticket_id: Uuid,
    seat_id: Uuid,
}

async fn fixture(pool: &PgPool, remote_enabled: bool) -> anyhow::Result<Fixture> {
    let password = auth::hash_password(PASSWORD)?;
    let mut ids = Vec::new();
    for (name, role, token) in [
        ("teacher", Role::Instructor, TEACHER),
        ("student", Role::Student, STUDENT),
        ("peer", Role::Student, PEER),
        ("other", Role::Instructor, OTHER_TEACHER),
    ] {
        let id = sqlx::query_scalar::<_, Uuid>(
            "INSERT INTO accounts (email, name, role, password_hash) VALUES ($1, $2, $3, $4) RETURNING id",
        ).bind(format!("{name}@example.test")).bind(name).bind(role).bind(&password).fetch_one(pool).await?;
        sqlx::query("INSERT INTO sessions (digest, account_id) VALUES ($1, $2)")
            .bind(Sha256::digest(token.as_bytes()).to_vec())
            .bind(id)
            .execute(pool)
            .await?;
        ids.push(id);
    }
    let [teacher, student, peer, _] = ids.as_slice() else {
        anyhow::bail!("expected four accounts");
    };
    let class_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO classes (instructor_id, title, brief) VALUES ($1, 'Practice', 'Build a lab.') RETURNING id",
    ).bind(teacher).fetch_one(pool).await?;
    sqlx::query("INSERT INTO enrollments (class_id, student_id) VALUES ($1, $2), ($1, $3)")
        .bind(class_id)
        .bind(student)
        .bind(peer)
        .execute(pool)
        .await?;
    let seat_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO lab_seats (class_id, student_id, label) VALUES ($1, $2, 'Lab 01') RETURNING id",
    ).bind(class_id).bind(student).fetch_one(pool).await?;
    let ticket_id = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO help_requests (class_id, student_id, subject, seat_id, consent_until)
         VALUES ($1, $2, 'Student needs help', $3, now() + interval '10 minutes') RETURNING id",
    )
    .bind(class_id)
    .bind(student)
    .bind(seat_id)
    .fetch_one(pool)
    .await?;
    Ok(Fixture {
        app: router(App {
            pool: pool.clone(),
            login: auth::LoginGuard::new()?.into(),
            remote_enabled,
        }),
        class_id,
        ticket_id,
        seat_id,
    })
}

async fn request(
    app: &Router,
    method: Method,
    path: &str,
    token: Option<&str>,
    body: Value,
) -> anyhow::Result<Response> {
    let mut builder = Request::builder()
        .method(method)
        .uri(path)
        .header("content-type", "application/json");
    if let Some(token) = token {
        builder = builder.header("authorization", format!("Bearer {token}"));
    }
    Ok(app
        .clone()
        .oneshot(builder.body(Body::from(body.to_string()))?)
        .await?)
}

async fn data(response: Response) -> anyhow::Result<Value> {
    Ok(serde_json::from_slice(
        &to_bytes(response.into_body(), 1024 * 1024).await?,
    )?)
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn rejects_login_when_the_account_is_disabled(pool: PgPool) -> anyhow::Result<()> {
    let Fixture { app, .. } = fixture(&pool, false).await?;
    sqlx::query("UPDATE accounts SET disabled = true WHERE email = 'student@example.test'")
        .execute(&pool)
        .await?;

    let response = request(
        &app,
        Method::POST,
        "/api/session",
        None,
        json!({"email": "student@example.test", "password": PASSWORD}),
    )
    .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn rejects_existing_sessions_when_the_account_is_disabled(
    pool: PgPool,
) -> anyhow::Result<()> {
    let Fixture { app, .. } = fixture(&pool, false).await?;
    sqlx::query("UPDATE accounts SET disabled = true WHERE email = 'student@example.test'")
        .execute(&pool)
        .await?;

    let response = request(
        &app,
        Method::GET,
        "/api/classes",
        Some(STUDENT),
        Value::Null,
    )
    .await?;

    assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn denies_invalid_password_and_revokes_the_login_token(pool: PgPool) -> anyhow::Result<()> {
    let Fixture { app, .. } = fixture(&pool, false).await?;
    let rejected = request(
        &app,
        Method::POST,
        "/api/session",
        None,
        json!({"email": "teacher@example.test", "password": "invalid"}),
    )
    .await?;
    assert_eq!(rejected.status(), StatusCode::UNAUTHORIZED);

    let accepted = request(
        &app,
        Method::POST,
        "/api/session",
        None,
        json!({"email": "Teacher@example.test", "password": PASSWORD}),
    )
    .await?;
    assert_eq!(accepted.status(), StatusCode::OK);
    let payload = data(accepted).await?;
    let token = payload["token"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing token"))?;
    assert_eq!(
        request(&app, Method::GET, "/api/classes", Some(token), Value::Null)
            .await?
            .status(),
        StatusCode::OK
    );

    assert_eq!(
        request(
            &app,
            Method::DELETE,
            "/api/session",
            Some(token),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        request(&app, Method::GET, "/api/classes", Some(token), Value::Null)
            .await?
            .status(),
        StatusCode::UNAUTHORIZED
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn scopes_classes_and_tickets_by_role_and_ownership(pool: PgPool) -> anyhow::Result<()> {
    let Fixture {
        app,
        class_id,
        ticket_id,
        ..
    } = fixture(&pool, false).await?;
    let endpoint = format!("/api/classes/{class_id}/help");
    let messages = format!("/api/help/{ticket_id}/messages");
    let denied = request(
        &app,
        Method::POST,
        "/api/classes",
        Some(STUDENT),
        json!({"title": "Forged", "brief": "Student cannot create classes"}),
    )
    .await?;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        request(
            &app,
            Method::GET,
            &endpoint,
            Some(OTHER_TEACHER),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        data(request(&app, Method::GET, &endpoint, Some(PEER), Value::Null).await?).await?,
        json!([])
    );
    assert_eq!(
        request(&app, Method::GET, &messages, Some(PEER), Value::Null)
            .await?
            .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &app,
            Method::POST,
            &messages,
            Some(PEER),
            json!({"body": "intrusion"})
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn blocks_new_messages_after_the_student_closes_the_ticket(
    pool: PgPool,
) -> anyhow::Result<()> {
    let Fixture { app, ticket_id, .. } = fixture(&pool, false).await?;
    let endpoint = format!("/api/help/{ticket_id}");
    assert_eq!(
        request(
            &app,
            Method::POST,
            &format!("{endpoint}/close"),
            Some(STUDENT),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::NO_CONTENT
    );
    let reply = request(
        &app,
        Method::POST,
        &format!("{endpoint}/messages"),
        Some(TEACHER),
        json!({"body": "Too late"}),
    )
    .await?;
    assert_eq!(reply.status(), StatusCode::CONFLICT);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn consumes_remote_consent_once_even_during_simultaneous_claims(
    pool: PgPool,
) -> anyhow::Result<()> {
    let Fixture {
        app,
        ticket_id,
        seat_id,
        ..
    } = fixture(&pool, true).await?;
    let endpoint = format!("/api/help/{ticket_id}/remote");
    let body = json!({"seat_id": seat_id});
    let (first, second) = tokio::try_join!(
        request(&app, Method::POST, &endpoint, Some(TEACHER), body.clone()),
        request(&app, Method::POST, &endpoint, Some(TEACHER), body),
    )?;
    let mut outcomes = [first.status(), second.status()];
    outcomes.sort();
    assert_eq!(outcomes, [StatusCode::NO_CONTENT, StatusCode::CONFLICT]);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn rejects_remote_claims_without_current_consent_for_the_assigned_seat(
    pool: PgPool,
) -> anyhow::Result<()> {
    let Fixture {
        app,
        ticket_id,
        seat_id,
        ..
    } = fixture(&pool, true).await?;
    let endpoint = format!("/api/help/{ticket_id}/remote");
    let body = json!({"seat_id": seat_id});
    assert_eq!(
        request(
            &app,
            Method::POST,
            &endpoint,
            Some(OTHER_TEACHER),
            body.clone()
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(&app, Method::POST, &endpoint, Some(STUDENT), body.clone())
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &app,
            Method::POST,
            &endpoint,
            Some(TEACHER),
            json!({"seat_id": Uuid::new_v4()})
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );

    sqlx::query(
        "UPDATE help_requests SET consent_until = now() - interval '1 second' WHERE id = $1",
    )
    .bind(ticket_id)
    .execute(&pool)
    .await?;
    assert_eq!(
        request(&app, Method::POST, &endpoint, Some(TEACHER), body)
            .await?
            .status(),
        StatusCode::CONFLICT
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn student_can_offer_only_their_assigned_seat_and_withdraw_before_claim(
    pool: PgPool,
) -> anyhow::Result<()> {
    let Fixture {
        app,
        class_id,
        seat_id,
        ..
    } = fixture(&pool, true).await?;
    let collection = format!("/api/classes/{class_id}/help");
    let input =
        json!({"subject": "Help with my own seat", "consent": true, "seat_id": Uuid::new_v4()});
    let no_seat = request(&app, Method::POST, &collection, Some(PEER), input.clone()).await?;
    assert_eq!(no_seat.status(), StatusCode::CONFLICT);
    let created = request(&app, Method::POST, &collection, Some(STUDENT), input).await?;
    assert_eq!(created.status(), StatusCode::CREATED);
    let ticket = data(created).await?;
    let id = ticket["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing ticket ID"))?;
    let tickets =
        data(request(&app, Method::GET, &collection, Some(TEACHER), Value::Null).await?).await?;
    let offered = tickets
        .as_array()
        .and_then(|items| items.iter().find(|item| item["id"] == id))
        .ok_or_else(|| anyhow::anyhow!("missing new ticket"))?;
    assert_eq!(offered["seat_id"], json!(seat_id));
    assert_eq!(offered["remote_available"], true);

    let closed = request(
        &app,
        Method::POST,
        &format!("/api/help/{id}/close"),
        Some(STUDENT),
        Value::Null,
    )
    .await?;
    assert_eq!(closed.status(), StatusCode::NO_CONTENT);
    let rejected = request(
        &app,
        Method::POST,
        &format!("/api/help/{id}/remote"),
        Some(TEACHER),
        json!({"seat_id": seat_id}),
    )
    .await?;
    assert_eq!(rejected.status(), StatusCode::CONFLICT);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn fails_closed_when_remote_assistance_is_disabled(pool: PgPool) -> anyhow::Result<()> {
    let Fixture {
        app,
        ticket_id,
        seat_id,
        class_id,
        ..
    } = fixture(&pool, false).await?;
    let endpoint = format!("/api/help/{ticket_id}/remote");
    assert_eq!(
        request(
            &app,
            Method::POST,
            &endpoint,
            Some(TEACHER),
            json!({"seat_id": seat_id})
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    let create = format!("/api/classes/{class_id}/help");
    assert_eq!(
        request(
            &app,
            Method::POST,
            &create,
            Some(STUDENT),
            json!({"subject": "Help", "consent": true})
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );
    Ok(())
}
