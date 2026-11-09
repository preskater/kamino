# Contributing to Kamino

Thanks for taking the time to contribute. This project is an early-stage workspace with
two REST APIs (agents and projects), so feedback on API shape is as valuable as code.

By participating you agree to the [Code of Conduct](CODE_OF_CONDUCT.md). For security
issues, follow [SECURITY.md](SECURITY.md) instead of opening a public issue.

## Prerequisites

- Rust `stable` (the repo pins it in `rust-toolchain.toml`, currently 1.94+ features)
- Docker, for Postgres
- `make` (optional, but every command below is also plain cargo)

## Setup

```bash
cp .env.example .env        # Postgres on localhost:5432
make up                     # docker compose up -d postgres
make run-agents             # :8081
make run-projects           # :8082
```

Swagger UI is at `/docs` on each service, the raw documents at `/api-docs/openapi.json`.
Both services apply their embedded migrations on startup, so a fresh database is all you need.

Enable the repository hooks once per clone:

```bash
git config core.hooksPath .githooks
```

## Repository layout

| Path | Responsibility |
| --- | --- |
| `crates/error` | error type, JSON envelope, database error mapping |
| `crates/config` | environment loading and typed lookups |
| `crates/telemetry` | tracing subscriber setup |
| `crates/db` | connection pool, migrations, readiness |
| `crates/query` | pagination models and the runtime-checked SQL layer |
| `crates/http` | extractors, middleware, server lifecycle |
| `crates/agents` | agents API (`AGENTS.md` domain) |
| `crates/projects` | projects API (`PROJECTS.md` domain) |

The domain models are documented in [AGENTS.md](AGENTS.md) and [PROJECTS.md](PROJECTS.md);
keep those diagrams in sync when you add tables or relations.

## Workflow

1. Open an issue first for anything larger than a bug fix or typo, especially API changes.
2. Branch from `main`: `feat/...`, `fix/...`, `docs/...`, `chore/...`.
3. Keep commits focused and write imperative messages (`add task dependency endpoints`,
   not `added`/`adds`). Conventional Commits prefixes are welcome but not required.
4. Open a pull request and fill in the template. `main` requires the `fmt`, `clippy` and
   `test` checks.

## Quality gate

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=postgres://kamino:kamino@localhost:5432/postgres cargo test --workspace
```

`cargo check` and `clippy` do not need a database. Integration tests use `#[sqlx::test]`,
which creates and drops a throwaway database on the server behind `DATABASE_URL`.

## Making changes

### API endpoints

- Reuse the generic query layer in `crates/query`; a new resource needs a `Resource` column
  map, a `ListSpec` (filters, search, sort), request/response DTOs and route handlers.
- Response types derive `utoipa::ToSchema` and every handler is annotated with
  `#[utoipa::path]`; register both in the crate's `openapi.rs`.
- Add an integration test covering create → read → list → patch → delete and the relevant
  error cases (404, 409, 400/422).

### Migrations

- Migrations are append-only. Never edit a migration that has been applied or merged.
- Add a new numbered file under `crates/<service>/migrations/` and make it idempotent for
  fresh databases only (sqlx tracks applied versions).
- Each service owns its database. Cross-service references are plain UUID columns; do not
  add foreign keys across `kamino_agents` and `kamino_projects`.
- New indexes belong in the same migration as the table they support.

### Dependencies

- Add versions to `[workspace.dependencies]` and reference them with `dep.workspace = true`.
- Prefer keeping each crate's dependency list to what it actually uses.
- Dependabot opens weekly grouped updates.

## Reporting

- Bugs and feature requests: use the issue templates.
- Questions and open-ended ideas: GitHub Discussions.
- Security: [private advisory](SECURITY.md), never a public issue.

## License

Contributions are accepted under the MIT license (see [LICENSE](LICENSE)).
