# Local Development Setup

This guide sets up Zent-BE (Rust) locally with all required services running in Docker. SCM (Zeus) is hosted externally and does not need to run locally.

---

## Prerequisites

- **Rust** (stable toolchain) — install via [rustup](https://rustup.rs/)
- **Docker Desktop** — for MySQL, Valkey, RabbitMQ, MongoDB
- **Git**

Verify:

```bash
rustc --version
cargo --version
docker --version
```

---

## 1. Clone and enter the project

```bash
git clone <repo-url>
cd Zent-BE
```

---

## 2. Set up environment variables

Edit the `.env` file in the project root with these local values:

```env
# ── Database (MySQL via Docker, port 3308 → 3306 inside container) ──────────
DATABASE_URL=mysql://admin:HUNGDEPZAI@localhost:3308/ZentDB

# ── Valkey (Redis-compatible cache via Docker) ──────────────────────────────
VALKEY_URL=redis://:HUNGDEPZAI@localhost:6380
VALKEY_PASSWORD=HUNGDEPZAI

# ── RabbitMQ via Docker ─────────────────────────────────────────────────────
RABBITMQ_URL=amqp://guest:guest@localhost:5672/%2f

# ── MongoDB via Docker ──────────────────────────────────────────────────────
MONGODB_URL=mongodb://admin:HUNGDEPZAI@localhost:27017/ZentDB?authSource=admin

# ── JWT (use any string for local dev) ──────────────────────────────────────
JWT_SIGN_KEY=local-dev-jwt-signing-key

# ── Session / Token TTL ─────────────────────────────────────────────────────
ACCESS_TOKEN_TTL_SECONDS=3600
SESSION_TTL_SECONDS=86400

# ── Server ──────────────────────────────────────────────────────────────────
PORT=3000
APP_STAGE=local
RUST_BACKTRACE=full

# ── DB Connection Pool ──────────────────────────────────────────────────────
DB_MAX_CONNECTIONS=10
DB_MIN_CONNECTIONS=2
DB_CONNECT_TIMEOUT_SECONDS=30
DB_ACQUIRE_TIMEOUT_SECONDS=30
DB_IDLE_TIMEOUT_SECONDS=600
DB_MAX_LIFETIME_SECONDS=1800

# ── Zeus SCM (hosted — do NOT change) ───────────────────────────────────────
ZEUS_BASE_URL=https://zeus.ryanandexen.qzz.io/api/v1/scm
ZEUS_API_KEY=<ask-the-team-for-this-key>

# ── Docs Swagger ────────────────────────────────────────────────────────────
DOCS_USERNAME=zent_doc
DOCS_PASSWORD=zent_doc

# ── Idempotency ─────────────────────────────────────────────────────────────
IDEMPOTENCY_CLAIM_TTL_SECONDS=30
IDEMPOTENCY_FINAL_TTL_SECONDS=3600
IDEMPOTENCY_POLL_RETRIES=6
IDEMPOTENCY_POLL_DELAY_MS=500

# ── System User (created by seeder with Uuid::nil) ──────────────────────────
SYSTEM_USER_ID=00000000-0000-0000-0000-000000000000

# ── Local Paths ─────────────────────────────────────────────────────────────
CHECKLIST_SAVE_PATH=.\zent_checklist
LUA_SCRIPT_DIR=./src/infrastructure/lua_script/
TEMPLATE_DIR=./templates/

# ── Other (optional — can leave empty for local dev) ────────────────────────
NOMINATIM_USER_AGENT=ZentBE1.0
SMTP_USERNAME=""
SMTP_PASSWORD=""
```

> **Note:**
> - `ZEUS_API_KEY` — ask a teammate for the hosted SCM API key.
> - `SYSTEM_USER_ID` must be `00000000-0000-0000-0000-000000000000` — the seeder creates the system user with `Uuid::nil()`.

---

## 3. Start Docker services

Create a file named `docker-compose.local.yml` in the project root:

```yaml
version: '3.8'

services:
  mysql:
    image: mysql:8.0
    container_name: zent-mysql
    restart: unless-stopped
    ports:
      - "3308:3306"
    environment:
      MYSQL_ROOT_PASSWORD: HUNGDEPZAI
      MYSQL_DATABASE: ZentDB
      MYSQL_USER: admin
      MYSQL_PASSWORD: HUNGDEPZAI
    volumes:
      - mysql-data:/var/lib/mysql

  valkey:
    image: valkey/valkey:8-alpine
    container_name: zent-valkey
    restart: unless-stopped
    ports:
      - "6380:6379"
    command: valkey-server --requirepass HUNGDEPZAI
    volumes:
      - valkey-data:/data

  rabbitmq:
    image: rabbitmq:3-management
    container_name: zent-rabbitmq
    restart: unless-stopped
    ports:
      - "5672:5672"
      - "15672:15672"   # Management UI (guest/guest)
    volumes:
      - rabbitmq-data:/var/lib/rabbitmq

  mongodb:
    image: mongo:7
    container_name: zent-mongodb
    restart: unless-stopped
    ports:
      - "27017:27017"
    environment:
      MONGO_INITDB_ROOT_USERNAME: admin
      MONGO_INITDB_ROOT_PASSWORD: HUNGDEPZAI
      MONGO_INITDB_DATABASE: ZentDB
    volumes:
      - mongo-data:/data/db

volumes:
  mysql-data:
  valkey-data:
  rabbitmq-data:
  mongo-data:
```

Start all services:

```bash
docker compose -f docker-compose.local.yml up -d
```

Verify all containers are running:

```bash
docker compose -f docker-compose.local.yml ps
```

You should see 4 containers: `zent-mysql`, `zent-valkey`, `zent-rabbitmq`, `zent-mongodb` — all with status `Up`.

---

## 4. Run database migrations

MySQL needs ~10 seconds to fully initialize on first start. Wait, then run:

```bash
cargo run -p migration -- up
```

This applies all SeaORM migrations to the `ZentDB` MySQL database.

Verify:

```bash
cargo run -p migration -- status
```

All migrations should show `Applied`.

---

## 5. Seed the database

The seeder populates lookup tables (roles, statuses, conditions, policies), a system user, and optional fake data.

**Minimal seed (lookup tables + system user only):**

```bash
cargo run -p seeder -- --db-url "mysql://admin:HUNGDEPZAI@localhost:3308/ZentDB" --num-users 0 --work-orders 0 --products 0
```

**Full seed (with fake users, work orders, products):**

```bash
cargo run -p seeder -- --db-url "mysql://admin:HUNGDEPZAI@localhost:3308/ZentDB" --num-users 10 --work-orders 20 --products 10 --warranties 5
```

After seeding, the output will print user credentials. Look for lines like:

```
  Email: <email> | Password: hungdepzai123 | Role: Admin
  Email: <email> | Password: hungdepzai123 | Role: Technician
  Email: <email> | Password: hungdepzai123 | Role: Customer
```

All seeded users share the password `hungdepzai123`.

---

## 6. Run the server

```bash
cargo run
```

The server starts on `http://localhost:3000`. On startup it will:

1. Connect to MySQL, Valkey, RabbitMQ, MongoDB
2. Run MongoDB migrations automatically
3. Load lookup tables from MySQL and Zeus SCM
4. Start HTTP server and WebSocket listener

Verify:

```bash
curl http://localhost:3000/health
```

Expected response: `{"status":"ok"}`

---

## 7. Verify the full stack

Login with a seeded user:

```bash
curl -X POST http://localhost:3000/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{"email": "<seeded-email>", "password": "hungdepzai123"}'
```

You should get back a JSON response with `access_token`, `refresh_token`, and `session_id`.

---

## Troubleshooting

| Problem | Solution |
|---|---|
| `cargo run` fails with "connection refused" on MySQL | Wait longer for MySQL to initialize, then retry. Check with `docker logs zent-mysql`. |
| `cargo run` fails with "Unknown database 'ZentDB'" | MySQL auto-creates it via `MYSQL_DATABASE` env var. If not, run: `docker exec -it zent-mysql mysql -u root -pHUNGDEPZAI -e "CREATE DATABASE ZentDB;"` |
| `cargo run` fails with Valkey connection error | Ensure the Valkey container is running: `docker ps`. The password must match `VALKEY_PASSWORD` in `.env`. |
| `cargo run` fails with MongoDB error | Ensure MongoDB container is running. Check: `docker logs zent-mongodb`. |
| `cargo run` fails with "ZEUS_API_KEY" or Zeus 401/403 | The `ZEUS_API_KEY` in `.env` is missing or invalid. Ask a teammate for the correct key. |
| Migration already applied / tables exist | This is fine. Run `cargo run -p migration -- status` to confirm all are applied. |
| Seeder says "role already exists" | The seeder is idempotent. Re-running is safe. |

---

## Quick Reference

```bash
# Start services
docker compose -f docker-compose.local.yml up -d

# Stop services
docker compose -f docker-compose.local.yml down

# Run migrations
cargo run -p migration -- up

# Seed database (full)
cargo run -p seeder -- --db-url "mysql://admin:HUNGDEPZAI@localhost:3308/ZentDB" --num-users 10 --work-orders 20 --products 10 --warranties 5

# Run server
cargo run

# Reset database (drop all tables + re-migrate)
cargo run -p migration -- fresh
```
