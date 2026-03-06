# Zent - Backend Service

A REST API backend for Zent, a Licensed Field Technician Service Management System. This service supports mobile field operations including work order management, geolocation verification, parts tracking, and service documentation.

## Project Information

| Attribute | Value |
|-----------|-------|
| **Course** | SE346.Q22 - Mobile Application Development |
| **Framework** | Axum (Rust) |
| **Database** | SQLite (dev, mobile) / undecided (via SeaORM) |
| **API Documentation** | OpenAPI/Scalar  |

## Features

### Core Capabilities

- **Authentication & Role Management** - JWT-based authentication with role-based access control for Administrators, Technicians, and Customers
- **Work Order Management** - Creation, assignment, tracking, and completion workflow
- **Geolocation & Geo-Fencing** - Location verification to ensure technicians are at assigned service locations
- **Parts Management** - QR code-based checkout/checkin with stale transaction detection
- **Multi-Stage Documentation** - Photo capture at pre-disassembly, during, and post-reassembly stages
- **Digital Signatures** - Customer confirmation capture with cryptographic binding
- **Self-Test Log Processing** - Upload and validation of diagnostic logs
- **Secure Messaging** - Privacy-preserving communication between technicians and customers
- **Escalation Management** - Two-week post-service window for issue reporting
- **Survey Automation** - Post-service feedback collection

## User Roles

| Role | Key Capabilities |
|------|------------------|
| **Administrator** | User management, work order assignment, inventory oversight, analytics |
| **Technician** | Work order execution, QR scanning, photo documentation, signature capture |
| **Customer** | Service requests, messaging, surveys, escalations |

## Tech Stack

- **Runtime:** Tokio (async)
- **Web Framework:** Axum 0.8.x
- **ORM:** SeaORM 1.1.x
- **Serialization:** serde
- **Password Hashing:** Argon2
- **API Documentation:** Scalar
- **Logging:** tracing + tracing-subscriber

## Getting Started

### Prerequisites

- Rust 1.70+ (2021 edition)
- SQLite database

### Installation

1. Clone the repository:
   ```bash
   git clone <repository-url>
   cd Zent-BE
   ```

2. Create environment configuration:
   ```bash
   cp .env.example .env
   ```

3. Configure `.env`:
   ```env
   DATABASE_URL=sqlite://./data.db
   JWT_SECRET=your-secret-key
   SERVER_HOST=0.0.0.0
   SERVER_PORT=3000
   ```

4. Build and run:
   ```bash
   cargo build
   cargo run
   ```

### API Documentation

Once running, access Swagger UI at:
```
http://localhost:3000/api/v1/
```

## Project Structure

```
Zent-BE/
├── src/
│   ├── main.rs           # Application entry point
│   ├── config/           # Configuration management
│   ├── handlers/         # API route handlers
│   ├── models/           # DTOs
│   ├── entities/         # SeaORM generated entities
│   ├── services/         # Business logic
│   └── middleware/       # Logging, etc.
├── migration/            # Database migration
├── Cargo.toml            # Dependencies
└── .env                  # Environment configuration
```

## API Overview

### Authentication

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/auth/login` | User login |
| POST | `/api/v1/auth/logout` | User logout |
| POST | `/api/v1/auth/refresh` | Session refresh |

### Work Orders

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/work-orders` | List work orders |
| POST | `/api/v1/work-orders` | Create work order |
| GET | `/api/v1/work-orders/:id` | Get work order details |
| PUT | `/api/v1/work-orders/:id` | Update work order |
| POST | `/api/v1/work-orders/:id/complete` | Complete work order |
| POST | `/api/v1/work-orders/:id/refuse` | Refuse work order |

### Parts Management

| Method | Endpoint | Description |
|--------|----------|-------------|
| GET | `/api/v1/parts` | List inventory |
| POST | `/api/v1/parts` | Add part |
| POST | `/api/v1/parts/checkout` | Check out part |
| POST | `/api/v1/parts/checkin` | Check in part |
| GET | `/api/v1/parts/stale` | List stale checkouts |

### Documentation

| Method | Endpoint | Description |
|--------|----------|-------------|
| POST | `/api/v1/photos/upload` | Upload service photo |
| GET | `/api/v1/photos/:work_order` | Get photos for work order |
| POST | `/api/v1/signatures/upload` | Upload signature |
| POST | `/api/v1/logs/upload` | Upload self-test log |

## Development

### Build

```bash
cargo build
```

### Test

```bash
cargo test
```

### Lint

```bash
cargo clippy
cargo fmt --check
```

## License

This project is developed for educational purposes as part of SE346.Q22 Mobile Application Development course.

## Acknowledgments

Design inspired by enterprise field service management systems in the technology industry.
