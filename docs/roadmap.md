# Roadmap

## Implemented foundation

- Self-hosted class instructions and teacher-managed enrollment.
- Account provisioning, sign-in, role and ownership checks, private help requests and two-way text replies.
- Operator-assisted password reset, account disabling and reenabling, with session revocation and cancellation of unused student remote consent.
- Practice tasks with fixed competency rubrics, text and HTTPS-link submissions, teacher scores and feedback, requested revisions, and attempt history.
- Opt-in Veyon consent flow and local adapter, with a lab validation checklist.
- PostgreSQL migrations, Compose startup, GitHub Actions workflow, and automated API and browser tests.

## Next evaluation milestones

- Validate the Veyon workflow with two supervised lab workstations and verify denial, expiration and student-side confirmation.
- Run a synthetic classroom pilot, then review deployment, accessibility, privacy and operations with a school before enrolling real students.
- Add self-service account recovery, roster changes, operator-facing audit review, and pagination for older tasks, submissions, help requests and replies.
- Define evidence retention, competency frameworks across tasks, rubric versioning for edited tasks, delegated assessor permissions and the institutional issuer model.

## SkillPassport milestone

Extend the classroom evidence and assessment records into a learner-controlled competency portfolio. Prototype signing, learner export and revocation with synthetic records. Validate the chosen credential format with independent tools before making compatibility or recognition claims. See the [design proposal](skillpassport.md).
