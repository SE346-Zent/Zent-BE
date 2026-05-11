# Zent Backend - Makefile

# Variables
CARGO = cargo
DATABASE_URL ?= $(shell grep DATABASE_URL .env | cut -d '=' -f2)

# Default target
.PHONY: all
all: help

.PHONY: help
help:
	@echo "Available commands:"
	@echo "  make build            - Build the project"
	@echo "  make run              - Run the application"
	@echo "  make test             - Run tests"
	@echo "  make check            - Check the code"
	@echo "  make lint             - Run clippy for linting"
	@echo "  make fmt              - Format the code"
	@echo "  make clean            - Clean build artifacts"
	@echo ""
	@echo "Database Migrations:"
	@echo "  make migrate-up       - Run pending migrations"
	@echo "  make migrate-down     - Rollback the last migration"
	@echo "  make migrate-status   - Check migration status"
	@echo "  make migrate-fresh    - Drop all tables and re-run all migrations"
	@echo ""
	@echo "Seeding:"
	@echo "  make seed             - Seed the database with default data"
	@echo "  make seed-interactive - Run the seeder in interactive mode"
	@echo ""
	@echo "Docker:"
	@echo "  make docker-up        - Start services with docker-compose"
	@echo "  make docker-down      - Stop services with docker-compose"

# Build and Run
.PHONY: build
build:
	$(CARGO) build

.PHONY: run
run:
	$(CARGO) run

.PHONY: test
test:
	$(CARGO) test

.PHONY: check
check:
	$(CARGO) check

.PHONY: lint
lint:
	$(CARGO) clippy -- -D warnings

.PHONY: fmt
fmt:
	$(CARGO) fmt

.PHONY: clean
clean:
	$(CARGO) clean

# Database Migrations (using migration sub-project)
.PHONY: migrate-up
migrate-up:
	$(CARGO) run -p migration -- up

.PHONY: migrate-down
migrate-down:
	$(CARGO) run -p migration -- down

.PHONY: migrate-status
migrate-status:
	$(CARGO) run -p migration -- status

.PHONY: migrate-fresh
migrate-fresh:
	$(CARGO) run -p migration -- fresh

# Seeding (using seeder sub-project)
.PHONY: seed
seed:
	@if [ -z "$(DATABASE_URL)" ]; then \
		echo "Error: DATABASE_URL is not set. Please provide it in .env or as an environment variable."; \
		exit 1; \
	fi
	$(CARGO) run -p seeder -- --db-url "$(DATABASE_URL)" --num-users 10 --work-orders 20 --products 10 --warranties 5

.PHONY: seed-interactive
seed-interactive:
	$(CARGO) run -p seeder -- --interactive

# Docker
.PHONY: docker-up
docker-up:
	docker-compose up -d

.PHONY: docker-down
docker-down:
	docker-compose down
