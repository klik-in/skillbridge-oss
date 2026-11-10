use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::Deserialize;
use sqlx::{PgPool, Postgres, Transaction};
use uuid::Uuid;

use crate::{
    App,
    auth::Session,
    classroom,
    error::{ApiError, bounded},
    model::{HelpRequest, HelpStatus, Message, Role},
};

pub(crate) async fn list(
    State(app): State<App>,
    session: Session,
    Path(class_id): Path<Uuid>,
) -> Result<Json<Vec<HelpRequest>>, ApiError> {
    classroom::accessible(&app.pool, class_id, &session).await?;
    let student_id = match session.account.role {
        Role::Instructor => None,
        Role::Student => Some(session.account.id),
    };
    let tickets = sqlx::query_as(
        "SELECT h.id, h.subject, h.status, h.created_at, a.name AS student_name, h.seat_id,
                s.label AS seat_label, h.claimed_at,
                ($3 AND h.status = 'open' AND h.claimed_at IS NULL AND
                 coalesce(h.consent_until > now(), false)) AS remote_available
         FROM help_requests h JOIN accounts a ON a.id = h.student_id
         LEFT JOIN lab_seats s ON s.id = h.seat_id
         WHERE h.class_id = $1 AND ($2::uuid IS NULL OR h.student_id = $2)
         ORDER BY h.status, h.created_at DESC, h.id DESC LIMIT 100",
    )
    .bind(class_id)
    .bind(student_id)
    .bind(app.remote_enabled)
    .fetch_all(&app.pool)
    .await?;
    Ok(Json(tickets))
}

#[derive(Deserialize)]
pub(crate) struct NewRequest {
    subject: String,
    #[serde(default)]
    consent: bool,
}

pub(crate) async fn create(
    State(app): State<App>,
    session: Session,
    Path(class_id): Path<Uuid>,
    Json(input): Json<NewRequest>,
) -> Result<(StatusCode, Json<Created>), ApiError> {
    session.student()?;
    classroom::accessible(&app.pool, class_id, &session).await?;
    let student = session.account.id;
    let subject = bounded(&input.subject, 200)?;
    let seat = if input.consent {
        if !app.remote_enabled {
            return Err(ApiError::Conflict("Remote assistance is disabled"));
        }
        Some(
            sqlx::query_scalar::<_, Uuid>(
                "SELECT id FROM lab_seats WHERE class_id = $1 AND student_id = $2",
            )
            .bind(class_id)
            .bind(student)
            .fetch_optional(&app.pool)
            .await?
            .ok_or(ApiError::Conflict(
                "No lab seat is assigned to this student",
            ))?,
        )
    } else {
        None
    };
    let id = sqlx::query_scalar(
        "INSERT INTO help_requests (class_id, student_id, subject, seat_id, consent_until)
         VALUES ($1, $2, $3, $4, CASE WHEN $4::uuid IS NOT NULL THEN now() + interval '10 minutes' END)
         RETURNING id",
    ).bind(class_id).bind(student).bind(subject).bind(seat).fetch_one(&app.pool).await?;
    Ok((StatusCode::CREATED, Json(Created { id })))
}

#[derive(serde::Serialize)]
pub(crate) struct Created {
    id: Uuid,
}

async fn lock_ticket(
    tx: &mut Transaction<'_, Postgres>,
    id: Uuid,
    session: &Session,
) -> Result<HelpStatus, ApiError> {
    sqlx::query_scalar(
        "SELECT h.status FROM help_requests h JOIN classes c ON c.id = h.class_id
         WHERE h.id = $1 AND (c.instructor_id = $2 OR h.student_id = $2) FOR UPDATE OF h",
    )
    .bind(id)
    .bind(session.account.id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ApiError::NotFound)
}

pub(crate) async fn close(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<Uuid>,
) -> Result<StatusCode, ApiError> {
    let mut tx = pool.begin().await?;
    lock_ticket(&mut tx, id, &session).await?;
    sqlx::query("UPDATE help_requests SET status = 'closed' WHERE id = $1")
        .bind(id)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}

pub(crate) async fn messages(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<Uuid>,
) -> Result<Json<Vec<Message>>, ApiError> {
    let accessible = sqlx::query_scalar::<_, bool>(
        "SELECT EXISTS (SELECT 1 FROM help_requests h JOIN classes c ON c.id = h.class_id
         WHERE h.id = $1 AND (c.instructor_id = $2 OR h.student_id = $2))",
    )
    .bind(id)
    .bind(session.account.id)
    .fetch_one(&pool)
    .await?;
    if !accessible {
        return Err(ApiError::NotFound);
    }
    let mut messages: Vec<Message> = sqlx::query_as(
        "SELECT m.id, a.name AS author_name, m.body, m.created_at
         FROM messages m JOIN accounts a ON a.id = m.author_id
         WHERE m.help_id = $1 ORDER BY m.id DESC LIMIT 200",
    )
    .bind(id)
    .fetch_all(&pool)
    .await?;
    messages.reverse();
    Ok(Json(messages))
}

#[derive(Deserialize)]
pub(crate) struct NewMessage {
    body: String,
}

pub(crate) async fn reply(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<NewMessage>,
) -> Result<StatusCode, ApiError> {
    let body = bounded(&input.body, 2000)?;
    let mut tx = pool.begin().await?;
    match lock_ticket(&mut tx, id, &session).await? {
        HelpStatus::Closed => return Err(ApiError::Conflict("This help request is closed")),
        HelpStatus::Open => {}
    }
    sqlx::query("INSERT INTO messages (help_id, author_id, body) VALUES ($1, $2, $3)")
        .bind(id)
        .bind(session.account.id)
        .bind(body)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(StatusCode::CREATED)
}

#[derive(Deserialize, serde::Serialize)]
pub(crate) struct RemoteClaim {
    pub seat_id: Uuid,
}

pub(crate) async fn claim(
    State(app): State<App>,
    session: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<RemoteClaim>,
) -> Result<StatusCode, ApiError> {
    session.instructor()?;
    if !app.remote_enabled {
        return Err(ApiError::NotFound);
    }
    let mut tx = app.pool.begin().await?;
    lock_ticket(&mut tx, id, &session).await?;
    let updated = sqlx::query(
        "UPDATE help_requests SET claimed_at = clock_timestamp(), claimed_by = $1
         WHERE id = $2 AND seat_id = $3 AND status = 'open' AND claimed_at IS NULL
           AND consent_until > clock_timestamp()",
    )
    .bind(session.account.id)
    .bind(id)
    .bind(input.seat_id)
    .execute(&mut *tx)
    .await?;
    if updated.rows_affected() == 0 {
        return Err(ApiError::Conflict(
            "Remote consent is unavailable for this ticket and seat",
        ));
    }
    tx.commit().await?;
    Ok(StatusCode::NO_CONTENT)
}
