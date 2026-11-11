use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
use sqlx::PgPool;
use uuid::Uuid;

use super::{OTHER_TEACHER, PEER, STUDENT, TEACHER, data, fixture, request};

fn task() -> Value {
    json!({
        "title": "Uji jaringan", "instructions": "Bangun jaringan dan jelaskan hasil uji.",
        "rubric": [{"label": "Konfigurasi", "max_score": 60}, {"label": "Pengujian", "max_score": 40}],
        "due_at": null
    })
}

fn submission() -> Value {
    json!({"body": "Konfigurasi dan hasil pengujian.", "artifact_url": "https://example.test/evidence"})
}

fn review(outcome: &str) -> Value {
    json!({"scores": [50, 35], "outcome": outcome, "feedback": "Jelaskan langkah pengujian."})
}

fn id(body: &Value) -> anyhow::Result<Uuid> {
    Ok(body["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("missing ID"))?
        .parse()?)
}

async fn create_task(app: &axum::Router, class: Uuid) -> anyhow::Result<Uuid> {
    let response = request(
        app,
        Method::POST,
        &format!("/api/classes/{class}/assignments"),
        Some(TEACHER),
        task(),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::CREATED);
    id(&data(response).await?)
}

async fn submit(app: &axum::Router, task: Uuid, token: &str) -> anyhow::Result<Uuid> {
    let response = request(
        app,
        Method::POST,
        &format!("/api/assignments/{task}/submissions"),
        Some(token),
        submission(),
    )
    .await?;
    assert_eq!(response.status(), StatusCode::CREATED);
    id(&data(response).await?)
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn scopes_assignment_creation_and_details_to_the_class(pool: PgPool) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let collection = format!("/api/classes/{}/assignments", setup.class_id);
    let assignment = create_task(&setup.app, setup.class_id).await?;
    assert_eq!(
        request(&setup.app, Method::POST, &collection, Some(STUDENT), task())
            .await?
            .status(),
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &collection,
            Some(OTHER_TEACHER),
            task()
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    for endpoint in [collection, format!("/api/assignments/{assignment}")] {
        assert_eq!(
            request(
                &setup.app,
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
            request(
                &setup.app,
                Method::GET,
                &endpoint,
                Some(STUDENT),
                Value::Null
            )
            .await?
            .status(),
            StatusCode::OK
        );
    }
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &format!("/api/assignments/{assignment}/submissions"),
            Some(TEACHER),
            submission()
        )
        .await?
        .status(),
        StatusCode::FORBIDDEN
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn blocks_an_unenrolled_student_from_reading_or_submitting_to_a_task(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    sqlx::query("DELETE FROM enrollments WHERE student_id = (SELECT id FROM accounts WHERE email = 'peer@example.test')")
        .execute(&pool).await?;

    assert_eq!(
        request(
            &setup.app,
            Method::GET,
            &format!("/api/assignments/{assignment}"),
            Some(PEER),
            Value::Null
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &format!("/api/assignments/{assignment}/submissions"),
            Some(PEER),
            submission()
        )
        .await?
        .status(),
        StatusCode::NOT_FOUND
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn hides_peer_submissions_and_blocks_peer_assessment(pool: PgPool) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    let own = submit(&setup.app, assignment, STUDENT).await?;
    let peer = submit(&setup.app, assignment, PEER).await?;
    let endpoint = format!("/api/assignments/{assignment}");
    let detail = data(
        request(
            &setup.app,
            Method::GET,
            &endpoint,
            Some(STUDENT),
            Value::Null,
        )
        .await?,
    )
    .await?;
    assert_eq!(detail["submissions"].as_array().map(Vec::len), Some(1));
    assert_eq!(detail["submissions"][0]["id"], json!(own));
    let teacher = data(
        request(
            &setup.app,
            Method::GET,
            &endpoint,
            Some(TEACHER),
            Value::Null,
        )
        .await?,
    )
    .await?;
    assert_eq!(teacher["submissions"].as_array().map(Vec::len), Some(2));
    for (token, expected) in [
        (STUDENT, StatusCode::FORBIDDEN),
        (OTHER_TEACHER, StatusCode::NOT_FOUND),
    ] {
        assert_eq!(
            request(
                &setup.app,
                Method::POST,
                &format!("/api/submissions/{peer}/review"),
                Some(token),
                review("accepted")
            )
            .await?
            .status(),
            expected
        );
    }
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn retains_attempts_and_reviews_through_revision_and_acceptance(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    let first = submit(&setup.app, assignment, STUDENT).await?;
    let endpoint = format!("/api/assignments/{assignment}/submissions");
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &endpoint,
            Some(STUDENT),
            submission()
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &format!("/api/submissions/{first}/review"),
            Some(TEACHER),
            review("revision_requested")
        )
        .await?
        .status(),
        StatusCode::CREATED
    );
    let second = submit(&setup.app, assignment, STUDENT).await?;
    assert_ne!(first, second);
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &format!("/api/submissions/{first}/review"),
            Some(TEACHER),
            review("accepted")
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &format!("/api/submissions/{second}/review"),
            Some(TEACHER),
            review("accepted")
        )
        .await?
        .status(),
        StatusCode::CREATED
    );
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &endpoint,
            Some(STUDENT),
            submission()
        )
        .await?
        .status(),
        StatusCode::CONFLICT
    );
    let detail = data(
        request(
            &setup.app,
            Method::GET,
            &format!("/api/assignments/{assignment}"),
            Some(STUDENT),
            Value::Null,
        )
        .await?,
    )
    .await?;
    assert_eq!(detail["submissions"][0]["attempt"], 2);
    assert_eq!(detail["submissions"][0]["review"]["outcome"], "accepted");
    assert_eq!(
        detail["submissions"][1]["review"]["outcome"],
        "revision_requested"
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn accepts_late_submissions_with_an_explicit_late_flag(pool: PgPool) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    sqlx::query(
        "UPDATE assignments SET due_at = clock_timestamp() - interval '1 second' WHERE id = $1",
    )
    .bind(assignment)
    .execute(&pool)
    .await?;
    submit(&setup.app, assignment, STUDENT).await?;
    let detail = data(
        request(
            &setup.app,
            Method::GET,
            &format!("/api/assignments/{assignment}"),
            Some(STUDENT),
            Value::Null,
        )
        .await?,
    )
    .await?;
    assert_eq!(detail["submissions"][0]["late"], true);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn validates_rubric_bounds_and_score_shape(pool: PgPool) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let collection = format!("/api/classes/{}/assignments", setup.class_id);
    for rubric in [
        json!([]),
        json!([{"label": "Invalid", "max_score": 0}]),
        json!([{"label": "Same", "max_score": 40}, {"label": "Same ", "max_score": 40}]),
        json!([{"label": "A", "max_score": 60}, {"label": "B", "max_score": 60}]),
    ] {
        let mut input = task();
        input["rubric"] = rubric;
        assert_eq!(
            request(&setup.app, Method::POST, &collection, Some(TEACHER), input)
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let assignment = create_task(&setup.app, setup.class_id).await?;
    let submitted = submit(&setup.app, assignment, STUDENT).await?;
    for scores in [
        json!([60]),
        json!([-1, 20]),
        json!([61, 20]),
        json!([60, 40, 0]),
    ] {
        let mut input = review("accepted");
        input["scores"] = scores;
        assert_eq!(
            request(
                &setup.app,
                Method::POST,
                &format!("/api/submissions/{submitted}/review"),
                Some(TEACHER),
                input
            )
            .await?
            .status(),
            StatusCode::BAD_REQUEST
        );
    }
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn refuses_unsafe_result_links_and_empty_answers(pool: PgPool) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    let endpoint = format!("/api/assignments/{assignment}/submissions");
    for link in [
        "javascript:alert(1)",
        "http://example.test",
        "https://user:password@example.test",
        "file:///etc/passwd",
    ] {
        let mut input = submission();
        input["artifact_url"] = json!(link);
        assert_eq!(
            request(&setup.app, Method::POST, &endpoint, Some(STUDENT), input)
                .await?
                .status(),
            StatusCode::BAD_REQUEST
        );
    }
    let mut input = submission();
    input["body"] = json!("  ");
    assert_eq!(
        request(&setup.app, Method::POST, &endpoint, Some(STUDENT), input)
            .await?
            .status(),
        StatusCode::BAD_REQUEST
    );
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn serializes_simultaneous_submissions_without_overwriting_student_work(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    let endpoint = format!("/api/assignments/{assignment}/submissions");
    let (first, second) = tokio::try_join!(
        request(
            &setup.app,
            Method::POST,
            &endpoint,
            Some(STUDENT),
            submission()
        ),
        request(
            &setup.app,
            Method::POST,
            &endpoint,
            Some(STUDENT),
            submission()
        ),
    )?;
    let mut outcomes = [first.status(), second.status()];
    outcomes.sort();
    assert_eq!(outcomes, [StatusCode::CREATED, StatusCode::CONFLICT]);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn serializes_competing_reviews_so_the_first_assessment_is_preserved(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    let submitted = submit(&setup.app, assignment, STUDENT).await?;
    let endpoint = format!("/api/submissions/{submitted}/review");
    let (first, second) = tokio::try_join!(
        request(
            &setup.app,
            Method::POST,
            &endpoint,
            Some(TEACHER),
            review("accepted")
        ),
        request(
            &setup.app,
            Method::POST,
            &endpoint,
            Some(TEACHER),
            review("revision_requested")
        ),
    )?;
    let mut outcomes = [first.status(), second.status()];
    outcomes.sort();
    assert_eq!(outcomes, [StatusCode::CREATED, StatusCode::CONFLICT]);
    Ok(())
}

#[sqlx::test]
#[ignore = "requires an isolated PostgreSQL test server"]
async fn accepts_supported_unicode_answer_length_through_the_http_body_limit(
    pool: PgPool,
) -> anyhow::Result<()> {
    let setup = fixture(&pool, false).await?;
    let assignment = create_task(&setup.app, setup.class_id).await?;
    let body = "🛠".repeat(6000);
    let input = json!({"body": body, "artifact_url": null});
    assert_eq!(
        request(
            &setup.app,
            Method::POST,
            &format!("/api/assignments/{assignment}/submissions"),
            Some(STUDENT),
            input
        )
        .await?
        .status(),
        StatusCode::CREATED
    );
    Ok(())
}
