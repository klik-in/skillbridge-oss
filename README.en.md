# SkillBridge OSS

[Bahasa Indonesia](README.md) · English

SkillBridge OSS is an early, self-hosted workspace for hands-on vocational classes. Teachers publish instructions and tasks, enroll students, and assess work against competency rubrics. Students submit results, revise work after feedback, and exchange replies with their class teacher in private help threads. The interface refreshes every five seconds.

| Scope | Status |
| --- | --- |
| Accounts, class instructions, enrollment, help requests and replies | Implemented for local development and evaluation |
| Practice tasks, submissions, rubrics, feedback and revisions | Implemented with a history of each submission attempt |
| Veyon | Optional teacher-workstation adapter for a separately installed Veyon. Off by default. See the [lab guide](docs/veyon.md). |
| SkillPassport, assessed portfolios and digital credentials | [Design proposal](docs/skillpassport.md) |

This repository does not provide evidence of school adoption, active users, certification or accreditation.

## Run locally

Requires Docker Engine 28+ with Docker Compose v2. Copy `.env.example` to `.env` and set `POSTGRES_PASSWORD` to a unique, URL-safe development secret using only letters, numbers, underscores or hyphens.

```sh
docker compose up --build --wait
docker compose run --rm api demo
```

Open [http://127.0.0.1:8080](http://127.0.0.1:8080). The one-time demo command creates a fictional class and prints randomly generated passwords for `guru@example.test` and `siswa@example.test`. Sign in as the student, submit a help request, and sign in as the teacher in a separate window or browser profile to reply. The student can reply in the same thread. Either participant can close it.

Create individual accounts using the operator CLI, which prompts for a password. Teachers then create classes and enroll provisioned students by email.

```sh
docker compose run --rm api account instructor teacher@example.test "Teacher"
docker compose run --rm api account student student@example.test "Student"
docker compose down
```

`docker compose down` preserves the PostgreSQL volume. The only published port is the local web UI. For native development and tests, see [CONTRIBUTING](CONTRIBUTING.md).

## Practice workflow

The teacher opens a class and selects **Buat tugas praktik**. Enter a title, instructions, an optional deadline, and 1–5 rubric criteria totaling at most 100 points. The rubric is fixed when the task is published.

Students open the task, submit a written answer or explanation, and may include an HTTPS result link. The teacher scores each criterion, writes feedback, and selects **Perlu revisi** (revision requested) or **Diterima** (accepted). A revision request permits another submission attempt. Earlier attempts and assessments remain stored. Late submissions are marked and remain eligible for review.

Students see their own submissions and assessments. The class owner can assess students in that class. Users open result links in their browser.

## Account management

An operator with server access can reset a password or change an account's status. Verify the account holder's identity before resetting their password. The CLI prompts twice for the new password with input hidden.

```sh
docker compose run --rm api reset-password student@example.test
docker compose run --rm api disable-account student@example.test
docker compose run --rm api enable-account student@example.test
```

Each command revokes all sessions for the account. Disabling blocks sign-in while retaining classroom records and conversations. Reenabling requires a fresh sign-in. A password reset preserves the account's active or disabled status. Every change also cancels any unused Veyon consent belonging to that student. End an already opened Veyon session in the Veyon window.

## Project references

- [Architecture, roles and data boundaries](docs/architecture.md)
- [Veyon configuration and license boundary](docs/veyon.md)
- [SkillPassport proposal](docs/skillpassport.md) and [roadmap](docs/roadmap.md)
- [Contributing](CONTRIBUTING.md) and [security reporting](SECURITY.md)

SkillBridge OSS is distributed under the [MIT License](LICENSE). Veyon is installed independently and has its own [license terms](https://github.com/veyon/veyon/blob/main/COPYING).
