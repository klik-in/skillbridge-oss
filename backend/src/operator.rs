use anyhow::{Context, Result, bail};
use clap::Subcommand;
use sqlx::PgPool;
use uuid::Uuid;

use crate::{
    auth,
    error::{bounded, email},
    model::Role,
    veyon,
};

#[derive(Subcommand)]
pub(crate) enum Command {
    /// Add an account with an interactively entered password.
    Account {
        role: Role,
        email: String,
        name: String,
    },
    /// Add a lab seat to an existing class enrollment.
    AssignSeat {
        class_id: Uuid,
        student_email: String,
        label: String,
    },
    /// Create fictional accounts and a practice class in a development database.
    Demo,
    /// Consume a student's one-time consent and open Veyon locally.
    Veyon(veyon::Args),
}

pub(crate) async fn run(command: Command, pool: &PgPool) -> Result<()> {
    match command {
        Command::Account {
            role,
            email: address,
            name,
        } => {
            let address = email(&address)?;
            let name = bounded(&name, 100)?;
            let password = rpassword::prompt_password("New password (at least 12 characters): ")?;
            if password.chars().count() < 12 || password.len() > 256 {
                bail!("password must have at least 12 characters and at most 256 bytes");
            }
            if password != rpassword::prompt_password("Confirm password: ")? {
                bail!("passwords do not match");
            }
            let id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO accounts (email, name, role, password_hash) VALUES ($1, $2, $3, $4) RETURNING id",
            ).bind(address).bind(name).bind(role).bind(auth::hash_password(&password)?)
                .fetch_one(pool).await.context("create account (email must be unique)")?;
            println!("Account ID: {id}");
        }
        Command::AssignSeat {
            class_id,
            student_email,
            label,
        } => {
            let id = sqlx::query_scalar::<_, Uuid>(
                "INSERT INTO lab_seats (class_id, student_id, label)
                 SELECT e.class_id, e.student_id, $3
                 FROM enrollments e JOIN accounts a ON a.id = e.student_id
                 WHERE e.class_id = $1 AND a.email = $2
                 RETURNING id",
            )
            .bind(class_id)
            .bind(email(&student_email)?)
            .bind(bounded(&label, 80)?)
            .fetch_optional(pool)
            .await
            .context("assign seat (student and label must be unique in class)")?
            .context("student must already be enrolled in this class")?;
            println!("Seat ID: {id}");
        }
        Command::Demo => demo(pool).await?,
        Command::Veyon(_) => bail!("Veyon must run on the teacher workstation"),
    }
    Ok(())
}

async fn demo(pool: &PgPool) -> Result<()> {
    let teacher_password = auth::random_secret();
    let student_password = auth::random_secret();
    let teacher_hash = auth::hash_password(&teacher_password)?;
    let student_hash = auth::hash_password(&student_password)?;
    let mut tx = pool.begin().await?;
    let teacher = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO accounts (email, name, role, password_hash)
         VALUES ('guru@example.test', 'Guru Demo', 'instructor', $1) RETURNING id",
    )
    .bind(teacher_hash)
    .fetch_one(&mut *tx)
    .await
    .context("demo already exists or could not be created")?;
    let student = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO accounts (email, name, role, password_hash)
         VALUES ('siswa@example.test', 'Siswa Demo', 'student', $1) RETURNING id",
    )
    .bind(student_hash)
    .fetch_one(&mut *tx)
    .await?;
    let class = sqlx::query_scalar::<_, Uuid>(
        "INSERT INTO classes (instructor_id, title, brief)
         VALUES ($1, 'Praktik jaringan komputer', 'Rancang dan uji sambungan jaringan untuk dua komputer di lab.')
         RETURNING id",
    ).bind(teacher).fetch_one(&mut *tx).await?;
    sqlx::query("INSERT INTO enrollments (class_id, student_id) VALUES ($1, $2)")
        .bind(class)
        .bind(student)
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    println!("guru@example.test {teacher_password}\nsiswa@example.test {student_password}");
    println!("Class ID: {class}");
    Ok(())
}
