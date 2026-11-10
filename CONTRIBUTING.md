# Contributing

Use a GitHub issue for ordinary bugs and proposals. For vulnerabilities, follow [SECURITY.md](SECURITY.md). Use synthetic classroom data in examples, logs, screenshots and tests. By submitting a contribution, you agree to license it under the repository's [MIT License](LICENSE).

Keep changes focused and describe the user-visible behavior, test steps and any implications for roles, stored data or lab access. Add a regression test when changing an access-control decision or a previously failing workflow. Keep applied migrations immutable and add new numbered migrations.

## Native development

Install Rust 1.99, Bun 1.3.14 and PostgreSQL 17. Create a development database and an independent test database. Set `DATABASE_URL` to the development connection string and run these in separate terminals from the repo root.

```sh
cargo run --locked --manifest-path backend/Cargo.toml
cd frontend
bun install --frozen-lockfile
bun run dev
```

Vite proxies API calls to `127.0.0.1:3000`. Run the backend CLI with the same database URL and append `-- demo` to seed synthetic data. The backend applies migrations at startup.

## Checks

```sh
cd backend
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
# Point DATABASE_URL at an isolated test server. SQLx creates a database per test.
cargo test --locked -- --ignored
```

The PostgreSQL test user needs permission to create and drop the per-test databases. Keep this credential separate from real installations.

```sh
cd frontend
bun install --frozen-lockfile
bun run build
bunx playwright install chromium
bun run test:e2e
```

For the browser test, start the stack and run the demo command first. Set `SKILLBRIDGE_TEACHER_PASSWORD` and `SKILLBRIDGE_STUDENT_PASSWORD` to the generated demo passwords in your shell. Set `SKILLBRIDGE_TEST_URL` if serving at an origin other than `http://127.0.0.1:8080`. The test creates and closes a synthetic ticket. CI exercises the Docker Compose stack and runs the PostgreSQL authorization tests independently.
