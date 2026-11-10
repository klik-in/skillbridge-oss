use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Deserialize, Serialize, sqlx::Type, clap::ValueEnum)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "account_role", rename_all = "snake_case")]
pub(crate) enum Role {
    Instructor,
    Student,
}

#[derive(Clone, Debug, Serialize, FromRow)]
pub(crate) struct Account {
    pub id: Uuid,
    pub name: String,
    pub role: Role,
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct Class {
    pub id: Uuid,
    pub title: String,
    pub brief: String,
    pub instructor_name: String,
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct Seat {
    pub id: Uuid,
    pub label: String,
}

#[derive(Debug, Serialize, sqlx::Type)]
#[serde(rename_all = "snake_case")]
#[sqlx(type_name = "help_status", rename_all = "snake_case")]
pub(crate) enum HelpStatus {
    Open,
    Closed,
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct HelpRequest {
    pub id: Uuid,
    pub subject: String,
    pub student_name: String,
    pub status: HelpStatus,
    pub created_at: DateTime<Utc>,
    pub seat_id: Option<Uuid>,
    pub seat_label: Option<String>,
    pub remote_available: bool,
    pub claimed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, FromRow)]
pub(crate) struct Message {
    pub id: i64,
    pub author_name: String,
    pub body: String,
    pub created_at: DateTime<Utc>,
}
