SHELL := /bin/bash
DATABASE_URL ?= postgres://kamino:kamino@localhost:5432/postgres

.PHONY: help up down logs fmt lint check test test-agents test-projects run-agents run-projects psql-agents psql-projects docker-build clean

help:
	@echo "up            start postgres (docker compose)"
	@echo "down          stop everything and remove volumes"
	@echo "fmt           format the workspace"
	@echo "lint          clippy with warnings denied"
	@echo "check         cargo check --workspace --all-targets"
	@echo "test          run all tests (needs postgres)"
	@echo "run-agents    run the agents API on :8081"
	@echo "run-projects  run the projects API on :8082"
	@echo "docker-build  build both images"

up:
	docker compose up -d postgres
	@until docker compose exec -T postgres pg_isready -U kamino -d postgres >/dev/null 2>&1; do sleep 1; done
	@echo "postgres ready on localhost:5432"

down:
	docker compose down -v

logs:
	docker compose logs -f

fmt:
	cargo fmt --all

lint:
	cargo clippy --workspace --all-targets -- -D warnings

check:
	cargo check --workspace --all-targets

test:
	DATABASE_URL=$(DATABASE_URL) cargo test --workspace

test-agents:
	DATABASE_URL=$(DATABASE_URL) cargo test -p kamino-agents

test-projects:
	DATABASE_URL=$(DATABASE_URL) cargo test -p kamino-projects

run-agents:
	cargo run -p kamino-agents

run-projects:
	cargo run -p kamino-projects

psql-agents:
	docker compose exec postgres psql -U kamino -d kamino_agents

psql-projects:
	docker compose exec postgres psql -U kamino -d kamino_projects

docker-build:
	docker compose build

clean:
	cargo clean
