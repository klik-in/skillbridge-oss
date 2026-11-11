# Architecture

```text
Student / teacher browser
       │ same-origin JSON and static files
       ▼
      Nginx ──────► Rust / Axum ──────► PostgreSQL 17
      :8080         private :3000       private :5432
                        ▲
                        │ one-time consent claim over HTTPS*
              Teacher's local adapter
                        │ executes separately installed veyon-cli
                        ▼
              Assigned computer on the lab LAN

* Loopback HTTP is accepted by the adapter for local development only.
```

The Rust backend owns authorization and persistence. Nginx serves the React and TypeScript bundle and proxies `/api/`. The browser keeps the opaque bearer token in memory. PostgreSQL stores its SHA-256 digest with an eight-hour expiry. Logout removes the session. Argon2id protects passwords and the server bounds concurrent password work and sign-in attempts. This first release targets a single API instance. Request bodies are bounded, SQL inputs are bound as parameters and the API does not enable cross-origin access.

| Operation | Who has access |
| --- | --- |
| Provision accounts, assign physical lab seats | Operator with local CLI and database access |
| Reset passwords, disable or enable accounts | Operator with local CLI and database access |
| Create classes, enroll existing students | Instructor who owns the class |
| Read class instructions | Owning instructor and enrolled students |
| Open a help request | Enrolled student, for their own account |
| Read or reply in a help thread | Requesting student and owning instructor |
| Close a help request | Requesting student or owning instructor |
| Consume a remote-control opt-in | Owning instructor, before expiry, for the bound ticket and seat only |
| Publish a practice task and rubric | Owning instructor |
| Read tasks and submit work | Enrolled student, submitting for their own account |
| Read submissions and assessments | Submitting student and owning instructor |
| Assess a submission | Owning instructor, for the latest unreviewed attempt |

The backend checks object ownership, including for direct calls with a guessed UUID. A PostgreSQL row lock serializes replies, closing and remote consent claims. Closed threads remain readable to both participants. The UI shows up to 100 requests per class, open first and newest within each state, and the most recent 200 replies per thread. The database retains earlier records. The UI refreshes active data every five seconds.

Account changes advance an authentication version, revoke all of the account's sessions and expire that student's unused remote consent in a single transaction. Login verifies the password outside a database transaction, then locks and checks the current account version before issuing a session. This serializes session issuance with operator changes. Disabled accounts are rejected at sign-in and at session lookup. Requests already authorized before a change can finish. Classroom and message history stays associated with the account.

Migration `0002_account_lifecycle.sql` adds active status and version defaults for existing accounts. Existing sessions continue to work for active accounts until expiry, logout or an operator change. The operator CLI supplies recovery after the operator verifies the account holder's identity. Email-based self-service recovery remains future work.

## API at a glance

All paths are under `/api`. Send JSON for payloads and `Authorization: Bearer <token>` for protected routes.

| Method and path | Purpose |
| --- | --- |
| GET /health, /ready | Liveness and database readiness |
| POST /session | Sign in with email and password |
| DELETE /session | Revoke current token |
| GET, POST /classes | List visible classes or create one |
| GET /classes/{id} | Read class and optional assigned seat |
| POST /classes/{id}/enrollments | Enroll a student by email |
| GET, POST /classes/{id}/help | List visible tickets or open one |
| GET, POST /help/{id}/messages | Read the thread or post a reply |
| POST /help/{id}/close | Close the thread |
| POST /help/{id}/remote | Consume unexpired consent for the supplied seat ID |
| GET, POST /classes/{id}/assignments | List tasks or publish a task with its fixed rubric |
| GET /assignments/{id} | Read the task and authorized submission history |
| POST /assignments/{id}/submissions | Submit an initial answer or a requested revision |
| POST /submissions/{id}/review | Score the latest unreviewed attempt and provide feedback |

## Practice records and assessment

Migration `0003_practice_assignments.sql` adds assignments, immutable submission attempts and one assessment per attempt. Rubrics have 1–5 distinct criterion labels, each worth 1–100 points, with a total of at most 100. Each assessment supplies one integer score per criterion, within that criterion's bounds, and requires written feedback. The teacher explicitly accepts the work or requests revision. Acceptance records that classroom decision.

The API locks the student's enrollment row while accepting submissions or reviews. A new attempt requires a revision request on the previous attempt. A review targets a specific submission ID and is rejected if that submission has already been reviewed or superseded. Competing writes return a conflict and preserve the stored version. Earlier attempts and assessments remain readable.

The task list shows the newest 100 tasks per class. Task detail shows the newest 200 submissions visible to the current account. Larger histories need pagination in a later iteration. Deadlines are optional UTC instants displayed in the viewer's local time. The server records submission time and labels late attempts without blocking them.

Written answers are limited to 6000 characters, instructions to 4000, feedback to 2000, and result links to 2048. The API and Nginx accept request bodies up to 64 KiB to support these limits with Unicode text. Result links require HTTPS without embedded credentials. The server stores the URL and never fetches its content. The browser displays the destination and uses noopener and noreferrer when opening it. File uploads and credential issuance remain future work.

## Before a school pilot

Use an HTTPS deployment, an edge sign-in rate limit and a least-privilege PostgreSQL runtime role with a separate migration operator. Establish an identity-verification procedure for operator-assisted account recovery and disabling, enrollment removal, backups with a restore test, data retention and deletion, and operator access controls. Review remote audit retention and incident handling. Validate keyboard and assistive-technology workflows with the people who will use the lab.

This foundation stores names, email addresses, enrollment, messages and remote consent metadata. Pilot with synthetic data while deciding which information the school needs, who may access it, when it is deleted and how students can report a concern. The app has no multi-school tenant boundary yet.
