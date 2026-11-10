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
| Create classes, enroll existing students | Instructor who owns the class |
| Read class instructions | Owning instructor and enrolled students |
| Open a help request | Enrolled student, for their own account |
| Read or reply in a help thread | Requesting student and owning instructor |
| Close a help request | Requesting student or owning instructor |
| Consume a remote-control opt-in | Owning instructor, before expiry, for the bound ticket and seat only |

The backend checks object ownership, including for direct calls with a guessed UUID. A PostgreSQL row lock serializes replies, closing and remote consent claims. Closed threads remain readable to both participants. The UI shows up to 100 requests per class, open first and newest within each state, and the most recent 200 replies per thread. The database retains earlier records. The UI refreshes active data every five seconds.

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

## Before a school pilot

Use an HTTPS deployment, an edge sign-in rate limit and a least-privilege PostgreSQL runtime role with a separate migration operator. Establish account recovery, deprovisioning, enrollment removal, backups with a restore test, data retention and deletion, and operator access controls. Review remote audit retention and incident handling. Validate keyboard and assistive-technology workflows with the people who will use the lab.

This foundation stores names, email addresses, enrollment, messages and remote consent metadata. Pilot with synthetic data while deciding which information the school needs, who may access it, when it is deleted and how students can report a concern. The app has no multi-school tenant boundary yet.
