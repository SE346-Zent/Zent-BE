# Zent - Backend Service

A robust, enterprise-grade REST API backend for Zent, a Licensed Field Technician Service Management System. Built with Rust and designed for high reliability, this service orchestrates mobile field operations including work order management, real-time notifications, geolocation verification, and complex service documentation.

## Project Information

| Attribute | Value |
|-----------|-------|
| **Course** | SE346.Q22 - Mobile Application Development |
| **Framework** | [Axum](https://github.com/tokio-rs/axum) (Rust) |
| **Main Database** | MySQL 8.0 (via [SeaORM](https://www.sea-ql.org/SeaORM/)) |
| **NoSQL Database** | MongoDB (Notifications & Audit Logs) |
| **Cache** | Valkey (Redis-compatible) |
| **Message Broker** | RabbitMQ (AMQP) |
| **Observability** | OpenTelemetry + Grafana Alloy |
| **API Documentation** | OpenAPI 3.1 / Scalar |

## Architecture

Zent-BE utilizes a modern, distributed architecture to ensure high availability and data integrity:

- **Asynchronous Processing:** Long-running tasks like email delivery, FCM push notifications, and work order assignments are offloaded to background consumers via RabbitMQ.
- **Outbox Pattern:** Ensures reliable delivery of notifications and cross-system messages even during transient failures.
- **Observability Stack:** Comprehensive monitoring using OpenTelemetry (OTLP) exported via Grafana Alloy, covering logs, metrics, and distributed tracing.
- **Modular Design:** Strict separation of concerns across `handlers`, `services`, `entities`, and `infrastructure` layers.
- **Cron Scheduling:** Automated maintenance tasks (cleanup, relaying, metrics collection) managed by an internal scheduler.

## Features

### Core Capabilities

- **Authentication & RBAC** - JWT-based auth with strict role-based access control (Admin, Technician, Customer).
- **Work Order Lifecycle** - From creation and auto-assignment to multi-stage completion and refusal handling.
- **Reliable Notifications** - Multi-channel (Email, Push, In-app) notifications backed by an outbox pattern and AMQP.
- **Inventory & Parts Management** - QR-based part tracking, product registration, and technician inventory oversight.
- **Service Documentation** - Multi-stage photo capture, digital signatures (cryptographic binding), and closing form management.
- **Geolocation Verification** - Geo-fencing to validate technician presence at service locations.
- **Self-Test Logs** - Upload and validation of diagnostic logs from field devices.

## Tech Stack

- **Runtime:** Tokio (async-first)
- **Web:** Axum 0.8.x
- **ORM/DB:** SeaORM (MySQL), MongoDB
- **Cache:** Valkey (Redis)
- **Messaging:** lapin (RabbitMQ)
- **Security:** Argon2 (Password hashing), JWT (Auth)
- **Documentation:** Scalar / utoipa
- **Scheduling:** tokio-cron-scheduler
- **Observability:** tracing + opentelemetry-otlp

## Project Structure

```
Zent-BE/
├── src/
│   ├── main.rs           # Application entry point & orchestration
│   ├── core/             # Shared state, config, and lookup tables
│   ├── entities/         # SeaORM database models
│   ├── extractor/        # Axum extractors (Auth, Role check)
│   ├── handlers/         # API route definitions and controllers
│   ├── infrastructure/   # Database, MQ, Cache, and Observability init
│   │   ├── consumers/    # Background message consumers (Email, FCM, etc.)
│   │   ├── cron_tasks/   # Scheduled background jobs
│   │   └── mq/           # Message queue abstractions
│   ├── model/            # DTOs (Requests/Responses)
│   ├── services/         # Core business logic implementation
│   └── utils/            # Cryptography, Geocoding, and OCI utilities
├── migration/            # SeaORM (MySQL) migrations
├── mongodb_migration/    # MongoDB schema migrations
├── seeder/               # Database initial data seeder
├── templates/            # HTML Email templates
├── docker-compose.yml    # Infrastructure orchestration
└── Cargo.toml            # Workspace and dependencies
```

## Getting Started

### Prerequisites

- Rust 1.75+ (2021 edition)
- Docker & Docker Compose
- [cargo-make](https://github.com/sagiegurari/cargo-make) (optional but recommended)

### Local Development Setup

1. **Clone and Setup Environment:**
   ```bash
   git clone <repository-url>
   cd Zent-BE
   cp .env.example .env
   ```

2. **Launch Infrastructure:**
   ```bash
   docker-compose up -d
   ```
   *This starts MySQL, RabbitMQ, MongoDB, Redis, and Grafana Alloy.*

3. **Run Migrations:**
   ```bash
   cargo run --package migration
   ```

4. **Start the Server:**
   ```bash
   cargo run
   ```

### API Documentation

Interactive API documentation is available at:
- **Scalar:** `http://localhost:3000/api/v1/docs`

## API Overview

### Authentication & User

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/login` | User login |
| POST | `/api/v1/auth/logout` | User logout (Session termination) |
| POST | `/api/v1/auth/refresh` | Session refresh |

### Work Orders

| Method | Endpoint | Role |
|--------|----------|------|
| GET | `/api/v1/work_orders` | All | List work orders |
| POST | `/api/v1/work_orders` | Customer | Create new request |
| GET | `/api/v1/work_orders/{id}` | All | Detailed WO info |
| POST | `/api/v1/work_orders/{id}/start` | Tech | Start execution |
| POST | `/api/v1/work_orders/{id}/complete`| Tech | Complete workflow |
| POST | `/api/v1/work_orders/{id}/assign` | Admin | Assign to technician |

### Inventory & Media

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/inventory/parts` | List available inventory |
| POST | `/api/v1/inventory/work_orders/{id}/parts` | Associate parts with WO |
| POST | `/api/v1/media/work_orders/{id}/closing_form/photos` | Upload service photos |
| POST | `/api/v1/media/work_orders/{id}/closing_form/signature` | Capture customer signature |

## License

This project is developed for educational purposes as part of the SE346.Q22 Mobile Application Development course.

## Acknowledgments

Architected with inspiration from highly scalable field service platforms, emphasizing type safety, observability, and reliable message delivery.
