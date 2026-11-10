use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    App,
    auth::Session,
    error::{ApiError, bounded, email},
    model::{Class, Role, Seat},
};

#[derive(Serialize)]
pub(crate) struct Classroom {
    #[serde(flatten)]
    class: Class,
    seat: Option<Seat>,
}

pub(crate) async fn list(
    State(pool): State<PgPool>,
    session: Session,
) -> Result<Json<Vec<Class>>, ApiError> {
    let actor = session.account.id;
    let classes = sqlx::query_as(
        "SELECT c.id, c.title, c.brief, a.name AS instructor_name
         FROM classes c JOIN accounts a ON a.id = c.instructor_id
         WHERE c.instructor_id = $1 OR EXISTS (
             SELECT 1 FROM enrollments e WHERE e.class_id = c.id AND e.student_id = $1
         ) ORDER BY c.created_at DESC, c.id DESC",
    )
    .bind(actor)
    .fetch_all(&pool)
    .await?;
    Ok(Json(classes))
}

pub(crate) async fn accessible(
    pool: &PgPool,
    id: Uuid,
    session: &Session,
) -> Result<Class, ApiError> {
    let actor = session.account.id;
    sqlx::query_as(
        "SELECT c.id, c.title, c.brief, a.name AS instructor_name
         FROM classes c JOIN accounts a ON a.id = c.instructor_id
         WHERE c.id = $1 AND (c.instructor_id = $2 OR EXISTS (
             SELECT 1 FROM enrollments e WHERE e.class_id = c.id AND e.student_id = $2
         ))",
    )
    .bind(id)
    .bind(actor)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}

pub(crate) async fn detail(
    State(app): State<App>,
    session: Session,
    Path(id): Path<Uuid>,
) -> Result<Json<Classroom>, ApiError> {
    let class = accessible(&app.pool, id, &session).await?;
    let seat = if app.remote_enabled {
        sqlx::query_as("SELECT id, label FROM lab_seats WHERE class_id = $1 AND student_id = $2")
            .bind(id)
            .bind(session.account.id)
            .fetch_optional(&app.pool)
            .await?
    } else {
        None
    };
    Ok(Json(Classroom { class, seat }))
}

#[derive(Deserialize)]
pub(crate) struct NewClass {
    title: String,
    brief: String,
}

pub(crate) async fn create(
    State(pool): State<PgPool>,
    session: Session,
    Json(input): Json<NewClass>,
) -> Result<(StatusCode, Json<Class>), ApiError> {
    session.instructor()?;
    let class = sqlx::query_as(
        "INSERT INTO classes (instructor_id, title, brief) VALUES ($1, $2, $3)
         RETURNING id, title, brief, $4::text AS instructor_name",
    )
    .bind(session.account.id)
    .bind(bounded(&input.title, 120)?)
    .bind(bounded(&input.brief, 2000)?)
    .bind(&session.account.name)
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(class)))
}

#[derive(Deserialize)]
pub(crate) struct Enrollment {
    email: String,
}

pub(crate) async fn enroll(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<Enrollment>,
) -> Result<StatusCode, ApiError> {
    session.instructor()?;
    accessible(&pool, id, &session).await?;
    let student =
        sqlx::query_scalar::<_, Uuid>("SELECT id FROM accounts WHERE email = $1 AND role = $2")
            .bind(email(&input.email)?)
            .bind(Role::Student)
            .fetch_optional(&pool)
            .await?
            .ok_or(ApiError::NotFound)?;
    sqlx::query(
        "INSERT INTO enrollments (class_id, student_id) VALUES ($1, $2) ON CONFLICT DO NOTHING",
    )
    .bind(id)
    .bind(student)
    .execute(&pool)
    .await?;
    Ok(StatusCode::NO_CONTENT)
}
