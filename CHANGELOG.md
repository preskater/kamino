# Changelog

All notable changes to this project are documented here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and the project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html) once it reaches 1.0.

## [Unreleased]

### Added

- Cargo workspace with two axum services: `agents` (`:8081`) and `projects` (`:8082`).
- Full CRUD REST APIs for every table in `AGENTS.md` and `PROJECTS.md`.
- Shared crates split by responsibility: `error`, `config`, `telemetry`, `db`, `query`, `http`.
- Pagination, search, filtering and whitelisted sorting on every collection endpoint.
- Consistent JSON error envelope with Postgres error mapping (`409`, `422`, `400`, `404`).
- OpenAPI 3 documents generated with utoipa and served through Swagger UI at `/docs`.
- Append-only project activity log written in the same transaction as project and task changes.
- Postgres-per-service migrations applied automatically on startup.
- Docker Compose stack, multi-stage Dockerfile, Makefile and repository hooks.
- CI: format, clippy, integration tests, dependency audit and Docker builds.
- Open-source contribution docs: contributing guide, code of conduct, security policy,
  issue and pull-request templates, Dependabot.
