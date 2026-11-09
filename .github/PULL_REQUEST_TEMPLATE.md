## Summary

<!-- What does this change and why? Link the issue it closes. -->

Closes #

## Type of change

- [ ] Bug fix
- [ ] New feature / endpoint
- [ ] Refactor
- [ ] Documentation
- [ ] Infrastructure / CI

## How was it tested?

<!-- Commands, new or updated tests, manual curl requests. -->

```bash
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
DATABASE_URL=postgres://kamino:kamino@localhost:5432/postgres cargo test --workspace
```

## Checklist

- [ ] Tests cover the change (integration test added or updated when endpoints change)
- [ ] `cargo fmt` and `cargo clippy -D warnings` pass locally
- [ ] Migrations are append-only (no applied migration was edited)
- [ ] `AGENTS.md` / `PROJECTS.md` diagrams updated if tables or relations changed
- [ ] OpenAPI annotations and the crate's `openapi.rs` updated when routes or DTOs change
- [ ] No secrets, `.env` files or credentials committed
- [ ] Breaking changes are called out above and in `CHANGELOG.md`
