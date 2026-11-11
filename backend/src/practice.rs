use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{FromRow, PgPool, Postgres, Transaction, types::Json as DbJson};
use uuid::Uuid;

use crate::{
    auth::Session,
    classroom,
    error::{ApiError, bounded},
    model::Role,
};

#[derive(Clone, Deserialize, Serialize)]
pub(crate) struct Criterion {
    label: String,
    max_score: i32,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Assignment {
    id: Uuid,
    class_id: Uuid,
    title: String,
    instructions: String,
    rubric: DbJson<Vec<Criterion>>,
    due_at: Option<DateTime<Utc>>,
    created_at: DateTime<Utc>,
}

#[derive(Deserialize)]
pub(crate) struct NewAssignment {
    title: String,
    instructions: String,
    rubric: Vec<Criterion>,
    due_at: Option<DateTime<Utc>>,
}

fn rubric(input: &[Criterion]) -> Result<Vec<Criterion>, ApiError> {
    if input.is_empty() || input.len() > 5 {
        return Err(ApiError::Invalid(
            "A rubric needs between one and five criteria",
        ));
    }
    let mut total: i32 = 0;
    let mut result: Vec<Criterion> = Vec::with_capacity(input.len());
    for criterion in input {
        let label = bounded(&criterion.label, 120)?.to_owned();
        if !(1..=100).contains(&criterion.max_score)
            || result.iter().any(|item| item.label == label)
        {
            return Err(ApiError::Invalid(
                "Each rubric criterion needs a unique label and 1–100 points",
            ));
        }
        total = total
            .checked_add(criterion.max_score)
            .ok_or(ApiError::Invalid("Invalid rubric total"))?;
        result.push(Criterion {
            label,
            max_score: criterion.max_score,
        });
    }
    if total > 100 {
        return Err(ApiError::Invalid(
            "The rubric total must be at most 100 points",
        ));
    }
    Ok(result)
}

pub(crate) async fn list(
    State(pool): State<PgPool>,
    session: Session,
    Path(class_id): Path<Uuid>,
) -> Result<Json<Vec<Assignment>>, ApiError> {
    classroom::accessible(&pool, class_id, &session).await?;
    let assignments = sqlx::query_as(
        "SELECT id, class_id, title, instructions, rubric, due_at, created_at
         FROM assignments WHERE class_id = $1 ORDER BY created_at DESC, id DESC LIMIT 100",
    )
    .bind(class_id)
    .fetch_all(&pool)
    .await?;
    Ok(Json(assignments))
}

pub(crate) async fn create(
    State(pool): State<PgPool>,
    session: Session,
    Path(class_id): Path<Uuid>,
    Json(input): Json<NewAssignment>,
) -> Result<(StatusCode, Json<Assignment>), ApiError> {
    session.instructor()?;
    classroom::accessible(&pool, class_id, &session).await?;
    let assignment = sqlx::query_as(
        "INSERT INTO assignments (class_id, title, instructions, rubric, due_at)
         VALUES ($1, $2, $3, $4, $5)
         RETURNING id, class_id, title, instructions, rubric, due_at, created_at",
    )
    .bind(class_id)
    .bind(bounded(&input.title, 120)?)
    .bind(bounded(&input.instructions, 4000)?)
    .bind(DbJson(rubric(&input.rubric)?))
    .bind(input.due_at)
    .fetch_one(&pool)
    .await?;
    Ok((StatusCode::CREATED, Json(assignment)))
}

async fn accessible(pool: &PgPool, id: Uuid, session: &Session) -> Result<Assignment, ApiError> {
    sqlx::query_as(
        "SELECT a.id, a.class_id, a.title, a.instructions, a.rubric, a.due_at, a.created_at
         FROM assignments a JOIN classes c ON c.id = a.class_id
         WHERE a.id = $1 AND (c.instructor_id = $2 OR EXISTS (
           SELECT 1 FROM enrollments e WHERE e.class_id = c.id AND e.student_id = $2))",
    )
    .bind(id)
    .bind(session.account.id)
    .fetch_optional(pool)
    .await?
    .ok_or(ApiError::NotFound)
}

#[derive(Clone, Copy, Deserialize, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "assessment_outcome", rename_all = "snake_case")]
pub(crate) enum Outcome {
    RevisionRequested,
    Accepted,
}

#[derive(Deserialize, Serialize)]
pub(crate) struct Review {
    scores: Vec<i32>,
    outcome: Outcome,
    feedback: String,
    reviewer_name: String,
    reviewed_at: DateTime<Utc>,
}

#[derive(Serialize, FromRow)]
pub(crate) struct Submission {
    id: Uuid,
    student_id: Uuid,
    student_name: String,
    attempt: i32,
    body: String,
    artifact_url: Option<String>,
    submitted_at: DateTime<Utc>,
    late: bool,
    review: Option<DbJson<Review>>,
}

#[derive(Serialize)]
pub(crate) struct Detail {
    assignment: Assignment,
    submissions: Vec<Submission>,
}

pub(crate) async fn detail(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<Uuid>,
) -> Result<Json<Detail>, ApiError> {
    let assignment = accessible(&pool, id, &session).await?;
    let student = match session.account.role {
        Role::Instructor => None,
        Role::Student => Some(session.account.id),
    };
    let submissions = sqlx::query_as(
        "SELECT s.id, s.student_id, a.name AS student_name, s.attempt, s.body, s.artifact_url,
                s.submitted_at, coalesce(s.submitted_at > $3, false) AS late,
                CASE WHEN r.submission_id IS NOT NULL THEN jsonb_build_object(
                    'scores', r.scores, 'outcome', r.outcome, 'feedback', r.feedback,
                    'reviewer_name', reviewer.name, 'reviewed_at', r.reviewed_at) END AS review
         FROM submissions s JOIN accounts a ON a.id = s.student_id
         LEFT JOIN submission_reviews r ON r.submission_id = s.id
         LEFT JOIN accounts reviewer ON reviewer.id = r.reviewed_by
         WHERE s.assignment_id = $1 AND ($2::uuid IS NULL OR s.student_id = $2)
         ORDER BY s.submitted_at DESC, s.attempt DESC, s.id DESC LIMIT 200",
    )
    .bind(id)
    .bind(student)
    .bind(assignment.due_at)
    .fetch_all(&pool)
    .await?;
    Ok(Json(Detail {
        assignment,
        submissions,
    }))
}

#[derive(Deserialize)]
pub(crate) struct NewSubmission {
    body: String,
    artifact_url: Option<String>,
}

fn artifact(raw: Option<&str>) -> Result<Option<String>, ApiError> {
    let Some(raw) = raw.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };
    let raw = bounded(raw, 2048)?;
    let url =
        url::Url::parse(raw).map_err(|_| ApiError::Invalid("Use a valid HTTPS result link"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(ApiError::Invalid(
            "Use an HTTPS result link without embedded credentials",
        ));
    }
    Ok(Some(url.into()))
}

async fn lock_enrollment(
    tx: &mut Transaction<'_, Postgres>,
    class_id: Uuid,
    student_id: Uuid,
) -> Result<(), ApiError> {
    sqlx::query_scalar::<_, Uuid>(
        "SELECT student_id FROM enrollments WHERE class_id = $1 AND student_id = $2 FOR UPDATE",
    )
    .bind(class_id)
    .bind(student_id)
    .fetch_optional(&mut **tx)
    .await?
    .ok_or(ApiError::NotFound)?;
    Ok(())
}

#[derive(FromRow)]
struct Latest {
    id: Uuid,
    attempt: i32,
    outcome: Option<Outcome>,
}

async fn latest(
    tx: &mut Transaction<'_, Postgres>,
    assignment_id: Uuid,
    student_id: Uuid,
) -> Result<Option<Latest>, ApiError> {
    Ok(sqlx::query_as(
        "SELECT s.id, s.attempt, r.outcome FROM submissions s
         LEFT JOIN submission_reviews r ON r.submission_id = s.id
         WHERE s.assignment_id = $1 AND s.student_id = $2 ORDER BY s.attempt DESC LIMIT 1",
    )
    .bind(assignment_id)
    .bind(student_id)
    .fetch_optional(&mut **tx)
    .await?)
}

#[derive(Serialize)]
pub(crate) struct Created {
    id: Uuid,
}

pub(crate) async fn submit(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<NewSubmission>,
) -> Result<(StatusCode, Json<Created>), ApiError> {
    session.student()?;
    let assignment = accessible(&pool, id, &session).await?;
    let body = bounded(&input.body, 6000)?;
    let artifact = artifact(input.artifact_url.as_deref())?;
    let mut tx = pool.begin().await?;
    lock_enrollment(&mut tx, assignment.class_id, session.account.id).await?;
    let attempt = match latest(&mut tx, id, session.account.id).await? {
        None => 1,
        Some(previous) => match previous.outcome {
            Some(Outcome::RevisionRequested) => previous
                .attempt
                .checked_add(1)
                .ok_or(ApiError::Conflict("Too many revisions"))?,
            None | Some(Outcome::Accepted) => {
                return Err(ApiError::Conflict(
                    "A new submission needs a revision request from the teacher",
                ));
            }
        },
    };
    let id = sqlx::query_scalar(
        "INSERT INTO submissions (assignment_id, class_id, student_id, attempt, body, artifact_url)
         VALUES ($1, $2, $3, $4, $5, $6) RETURNING id",
    )
    .bind(id)
    .bind(assignment.class_id)
    .bind(session.account.id)
    .bind(attempt)
    .bind(body)
    .bind(artifact)
    .fetch_one(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok((StatusCode::CREATED, Json(Created { id })))
}

#[derive(Deserialize)]
pub(crate) struct NewReview {
    scores: Vec<i32>,
    outcome: Outcome,
    feedback: String,
}

pub(crate) async fn assess(
    State(pool): State<PgPool>,
    session: Session,
    Path(id): Path<Uuid>,
    Json(input): Json<NewReview>,
) -> Result<StatusCode, ApiError> {
    session.instructor()?;
    let (assignment_id, student_id) = sqlx::query_as::<_, (Uuid, Uuid)>(
        "SELECT s.assignment_id, s.student_id FROM submissions s JOIN classes c ON c.id = s.class_id
         WHERE s.id = $1 AND c.instructor_id = $2",
    ).bind(id).bind(session.account.id).fetch_optional(&pool).await?.ok_or(ApiError::NotFound)?;
    let assignment = accessible(&pool, assignment_id, &session).await?;
    let feedback = bounded(&input.feedback, 2000)?;
    if input.scores.len() != assignment.rubric.len()
        || input
            .scores
            .iter()
            .zip(assignment.rubric.iter())
            .any(|(score, criterion)| !(0..=criterion.max_score).contains(score))
    {
        return Err(ApiError::Invalid(
            "Provide one valid score for every rubric criterion",
        ));
    }
    let mut tx = pool.begin().await?;
    lock_enrollment(&mut tx, assignment.class_id, student_id).await?;
    let previous = latest(&mut tx, assignment_id, student_id)
        .await?
        .ok_or(ApiError::NotFound)?;
    if previous.id != id || previous.outcome.is_some() {
        return Err(ApiError::Conflict(
            "Only the latest unreviewed submission can be assessed",
        ));
    }
    sqlx::query(
        "INSERT INTO submission_reviews (submission_id, reviewed_by, scores, outcome, feedback)
         VALUES ($1, $2, $3, $4, $5)",
    )
    .bind(id)
    .bind(session.account.id)
    .bind(DbJson(input.scores))
    .bind(input.outcome)
    .bind(feedback)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(StatusCode::CREATED)
}
