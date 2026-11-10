# SkillBridge OSS

[Bahasa Indonesia](README.md) · English

SkillBridge OSS is an early, self-hosted workspace for hands-on vocational classes. Teachers share practice instructions and enroll students. Students can open help requests, and each student and the class teacher can reply in a private thread. The interface refreshes every five seconds.

| Scope | Status |
| --- | --- |
| Accounts, class instructions, enrollment, help requests and replies | Implemented for local development and evaluation |
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

## Project references

- [Architecture, roles and data boundaries](docs/architecture.md)
- [Veyon configuration and license boundary](docs/veyon.md)
- [SkillPassport proposal](docs/skillpassport.md) and [roadmap](docs/roadmap.md)
- [Contributing](CONTRIBUTING.md) and [security reporting](SECURITY.md)

SkillBridge OSS is distributed under the [MIT License](LICENSE). Veyon is installed independently and has its own [license terms](https://github.com/veyon/veyon/blob/main/COPYING).
