# Hướng dẫn cài đặt

## Phần mềm yêu cầu

- [Git](https://git-scm.com/install/)
- [Rust \(stable toolchain\)](https://rustup.rs/)
- [MySQL Server](https://dev.mysql.com/doc/mysql-installation-excerpt/8.0/en/) hoặc bất kỳ database nào SeaORM hỗ trợ. Lưu ý code migration có thể không hoạt động với SQLite
- [Valkey](https://valkey.io/topics/installation/) (hoặc [Redis](https://redis.io/docs/latest/operate/oss_and_stack/install/archive/install-redis/)) Server (\*Nhóm chưa tiến hành thử nghiệm với Redis)
- [MongoDB](https://www.mongodb.com/docs/manual/installation/?msockid=01d4d74022d866ca1928c1e223b16787) hoặc document DB khác phù hợp.
- [RabbitMQ](https://www.rabbitmq.com/docs/download)
- [Grafana Alloy](https://grafana.com/docs/alloy/latest/set-up/install/)
- [Tài khoản Google Firebase (cho push notification)](https://firebase.google.com/docs/cloud-messaging/)

## Các yêu cầu khác

- Tài khoản Email Google. Sử dụng [App Password](https://support.google.com/accounts/answer/185833?sjid=16804269141173951397-NC#app-passwords) nếu cần
- [AWS S3](https://docs.aws.amazon.com/AmazonS3/latest/userguide/access-points-naming.html#accessing-a-bucket-through-s3-access-point), [Google Cloud Storage](https://docs.cloud.google.com/storage/docs/request-endpoints), [Oracle Object Storage](https://docs.oracle.com/en-us/iaas/Content/Object/Tasks/usingpreauthenticatedrequests.htm), self-host [MinIO](https://github.com/minio/minio) hoặc phần mềm tương tự (S3-compatible object storage).
  - Lưu ý: do vấn đề chi phí và độ phức tạp, nhóm chưa thử nghiệm trên môi trường AWS, GCS và MinIO.
- Grafana Cloud hoặc Grafana self-host (Prometheus, Loki)
- [Tailscale](https://tailscale.com/docs/integrations) nếu self-host trên máy chủ không có địa chỉ IP public, máy chủ cá nhân trong môi trường mạng dân dụng để kết nối các thành phần vào cùng mạng với nhau.
- [Zeus MRP](https://github.com/SE359-Zeus/Zeus-BE/blob/main/INSTALL_MRP.md) nếu API có sẵn down hoặc cần self-host.
- [Zeus SCM](https://github.com/SE359-Zeus/Zeus-BE/blob/main/INSTALL_SCM.md) nếu API có sẵn down hoặc cần self-host.

## Chạy backend

### Hướng dẫn chung

#### 1. Cài đặt các yêu cầu

1. Truy cập các trang tài liệu tương ứng với loại hạ tầng muốn sử dụng và cài đặt trên thiết bị đích theo nhu cầu.
  - Lưu ý: nếu sử dụng Docker toàn phần, vui lòng cuộn xuống mục Docker. Thay bất kỳ thành phần nào trong Docker với thành phần vật lý nếu muốn.
2. Khởi động tất cả các service và note lại URI truy cập (vd. IP của server, localhost, container network IP, container hostname, v.v.)

#### 2. Edit file .env với các biến môi trường cần thiết

Edit file `.env` với các field sau:

Lưu ý tiến trình backend có thể đọc file `.env` để bổ sung biến môi trường trong runtime. Cho các phần mềm hạ tầng khác, vui lòng set biến môi trường theo hướng dẫn nếu chạy bare metal.

##### Database/Storage

```env
DATABASE_URL=mysql://<username>:password@<db_server_ip_or_url>/<DB nếu cần thiết, VD MySQL>
VALKEY_URL=redis://[<username>][:<password>@]<hostname>[:port][/[<db>][?protocol=<protocol>]]
REDIS_PASSWORD=
MONGODB_URL=mongodb://[username:password@]host1[:port1][,...hostN[:portN]][/[defaultauthdb][?options]]
# DB connection options
DB_MAX_CONNECTIONS=10
DB_MIN_CONNECTIONS=2
DB_CONNECT_TIMEOUT_SECONDS=30
DB_ACQUIRE_TIMEOUT_SECONDS=30
DB_IDLE_TIMEOUT_SECONDS=600
DB_MAX_LIFETIME_SECONDS=1800
# S3-compatible object storage để upload ảnh minh chứng và signature
PAR_READ_WORK_ORDERS=link/to/bucket/
PAR_WRITE_WORK_ORDERS=link/to/bucket/
```

- Lưu ý đối với các multi-DB/multi-schema DBMS như MySQL, cần tạo trước DB trên DBMS trước khi sử dụng.
- `REDIS_PASSWORD` chỉ cần dùng khi có password, và không dùng password trong connection string (sở thích cá nhân của người làm phần cache).
- `PAR_READ_WORK_ORDERS`, `PAR_WRITE_WORK_ORDERS` là basename của S3-compatible object storage đang sử dụng. Tên file ảnh cuối cùng sẽ là `PAR*` + `image_name`. Nếu link PAR không bao gồm dấu `/` ở cuối, tên file sẽ bao gồm particle cuối cùng của đường dẫn, hoặc URL của bucket. Ví dụ, `PAR_WRITE_WORK_ORDERS=link.com/to/bucket` sẽ ghi ảnh `image.png` như sau: `link.com/to/bucketimage.png`.
- Cần điền vào file config của Alloy nếu không load sẵn `.env` vào môi trường.

##### MQ

```env
RABBITMQ_URL=amqp://[username:password@]host[:port][/vhost]
```

Lưu ý vhost `/` cần sử dụng `/%2f`.

##### JWT

```env
JWT_SIGN_KEY=any-thing
ACCESS_TOKEN_TTL_SECONDS=3600
SESSION_TTL_SECONDS=86400
```

##### Server Config

```env
PORT=3000
APP_STAGE=production
RUST_BACKTRACE=full
ZEUS_BASE_URL=https://zeus.ryanandexen.qzz.io/api/v1/scm
ZEUS_API_KEY=scmkey01-admin-20260524
CHECKLIST_SAVE_PATH=zent_checklist
TEMPLATE_DIR=templates
LUA_SCRIPT_DIR=src/infrastructure/lua_script
SYSTEM_USER_ID=00000000-0000-0000-0000-000000000000
```

Lưu ý SYSTEM_USER_ID: Hiện tại seeder seed system user với `Uuid::nil()`, vậy nên thay đổi ID này không làm thay đổi kết quả khởi tạo DB.

##### Observability

```env
OTEL_EXPORTER_OTLP_ENDPOINT=grafana-alloy-endpoint
GCLOUD_HOSTED_METRICS_URL=grafana-prometheus-push-endpoint
GCLOUD_HOSTED_LOGS_URL=grafana-loki-push-endpoint
GCLOUD_SCRAPE_INTERVAL=60s
```

Nếu sử dụng Grafana Cloud:

```env
GCLOUD_HOSTED_METRICS_ID=
GCLOUD_HOSTED_LOGS_ID=
GCLOUD_RW_API_KEY=
GCLOUD_OTLP_URL=
GCLOUD_STACK_ID=
GCLOUD_FM_HOSTED_ID=
```

Cần điền các ID trên vào file config của Alloy nếu không load sẵn `.env` vào môi trường.

##### Mísc/Other

```env
# Firebase
GOOGLE_APPLICATION_CREDENTIALS=path/to/credentials-file.json
GOOGLE_CLIENT_ID=client-id.apps.googleusercontent.com
# Nominatim for geocoding
NOMINATIM_USER_AGENT=ZentBE1.0
# SMTP for email
SMTP_USERNAME=mail-username
SMTP_PASSWORD=mail-password
# API docs Basic auth credentials
DOCS_USERNAME=zent_doc
DOCS_PASSWORD=zent_doc
# Để tránh dup work order
IDEMPOTENCY_CLAIM_TTL_SECONDS=30
IDEMPOTENCY_FINAL_TTL_SECONDS=3600
IDEMPOTENCY_POLL_RETRIES=6
IDEMPOTENCY_POLL_DELAY_MS=500
```

#### 3. Cấu hình Grafana Alloy

Tạo hoặc chỉnh sửa file `config.alloy` tại thư mục cấu hình của Alloy.

Thay thế các giá trị NẰM TRONG DẤU `< >` bằng IP (bao gồm cả IP Tailscale nếu sử dụng) và Port thực tế của hệ thống bạn. Đảm bảo biến môi trường `GCLOUD_FM_HOSTED_ID` và `GCLOUD_RW_API_KEY` đã được load vào môi trường chạy của service Alloy hoặc thay thế bằng giá trị thực nếu sử dụng Grafana Cloud

```alloy
// config.alloy

livedebugging {
  enabled = true
}

otelcol.receiver.otlp "default" {
  grpc { endpoint = "<LISTENING_IP>:<PORT_GRPC>" }
  http { endpoint = "<LISTENING_IP>:<PORT_HTTP>" }

  output {
    metrics = [otelcol.processor.batch.default.input]
    logs    = [otelcol.processor.batch.default.input]
    traces  = [otelcol.processor.batch.default.input]
  }
}

prometheus.exporter.redis "my_valkey" {
  redis_addr     = "<VALKEY_SERVER_IP>:<VALKEY_SERVER_PORT>"
  redis_password = "<VALKEY_PASSWORD>"
}

prometheus.scrape "valkey_scraper" {
  targets         = prometheus.exporter.redis.my_valkey.targets
  scrape_interval = "15s"

  forward_to      = [otelcol.receiver.prometheus.bridge.receiver]
}

prometheus.scrape "rabbitmq_scraper" {
  targets = [{
    __address__ = "<RABBITMQ_SERVER_IP>:<RABBITMQ_PROMETHEUS_PORT>",
    job         = "rabbitmq",
  }]
  scrape_interval = "15s"

  forward_to      = [otelcol.receiver.prometheus.bridge.receiver]
}

prometheus.scrape "nginx_scraper" {
  targets = [{
    __address__ = "<NGINX_SERVER_IP>:<NGINX_PROMETHEUS_EXPORTER_PORT>",
    job         = "nginx",
  }]
  scrape_interval = "15s"

  forward_to      = [otelcol.receiver.prometheus.bridge.receiver]
}

otelcol.receiver.prometheus "bridge" {
  output {
    metrics = [otelcol.processor.transform.add_production_label.input]
  }
}

otelcol.processor.transform "add_production_label" {
  error_mode = "ignore"
  metric_statements {
    context    = "resource"
    statements = [
      "set(attributes[\"environment\"], \"production\")",
    ]
  }

  output {
    metrics = [otelcol.processor.batch.default.input]
  }
}

otelcol.processor.batch "default" {
  output {
    metrics = [otelcol.exporter.otlphttp.grafana_cloud.input]
    logs    = [otelcol.exporter.otlphttp.grafana_cloud.input]
    traces  = [otelcol.exporter.otlphttp.grafana_cloud.input]
  }
}

// Exporter tới Grafana Dashboard
otelcol.exporter.otlphttp "grafana_cloud" {
  client {
    endpoint = "<GRAFANA_OTLP_ENDPOINT>"
    auth     = otelcol.auth.basic.grafana_cloud.handler
  }
}

// Auth cho Grafana Cloud
// Thay đổi section này nếu không dùng Grafana Cloud
otelcol.auth.basic "grafana_cloud" {
  username = sys.env("GCLOUD_FM_HOSTED_ID")
  password = sys.env("GCLOUD_RW_API_KEY")
}
```

Khởi động lại service Alloy sau khi ghi file cấu hình.

#### 4. Build ứng dụng

Mở Terminal ở thư mục gốc của project, chạy lệnh sau để build ứng dụng và seeder:

```bash
cargo build --release --package zent-be --package seeder
```

Sau khi quá trình biên dịch hoàn tất, các file thực thi (binary) sẽ xuất hiện tại:
- `./target/release/zent-be` (Ứng dụng chính)
- `./target/release/seeder` (Công cụ tạo dữ liệu mẫu)

#### 5. Khởi chạy ứng dụng

Chạy migration cho Database

```bash
cargo run -p migration -- up
```

Kiểm tra:

```bash
cargo run -p migration -- status
```

Seed Database:

```bash
./target/release/seeder --db-url "DB-URL" --num-users 0 --work-orders 0 --products 0
```

Bắt đầu tiến trình backend:

```bash
./target/release/zent-be
```

---

### Sử dụng Docker để chạy backend

#### Yêu cầu phần mềm thêm

- Docker
- Docker Compose plugin

### 1. Build Docker image

```bash
docker build -t zent-be:latest .
```

### 2. Khởi chạy ứng dụng

Sao chép nội dung file compose sau đây vào `docker-compose.yml` trong thư mục gốc:

Có thể dùng image dựng sẵn ở [ghcr.io/se346-zent/](ghcr.io/se346-zent/)

Có thể bỏ việc mapping volume cho kiểu mapping dạng thư mục. Cần map đúng các file config vào container.

```yaml
services:
  zent-seeder:
    container_name: zent-seeder
    image: <ZENT-BE-IMAGE>
    command: ["seeder", "--num-users", "0", "--products", "0", "--work-orders", "0", "--warranties", "0"]
    env_file:
      - .env
    depends_on:
      zent-backend:
        condition: service_started
      db:
        condition: service_healthy

  zent-backend:
    container_name: zent-be
    image: <ZENT-BE-IMAGE>
    restart: unless-stopped
    env_file:
      - .env
    volumes:
      - <TEMPLATE_DIR>:/app/<TEMPLATE_DIR>
      - <CHECKLIST_SAVE_PATH>:/app/<CHECKLIST_SAVE_PATH>
      - <GOOGLE_APPLICATION_CREDENTIALS_JSON_PATH>:/app/<GOOGLE_APPLICATION_CREDENTIALS_JSON_PATH>
    depends_on:
      db:
        condition: service_healthy
      mongodb:
        condition: service_started
      valkey:
        condition: service_started
      rabbitmq:
        condition: service_started
      minio:
        condition: service_started

  alloy:
    image: grafana/alloy:latest
    restart: unless-stopped
    env_file:
      - .env
    ports:
      - "12345:12345"
      - "4317:4317"
      - "4318:4318"
    volumes:
      - <ALLOY_CONFIG_LOCATION>:/etc/alloy/config.alloy:ro
      - <ALLOY_DATA_LOCATION>:/var/lib/alloy/data
    command:
      - run
      - --server.http.listen-addr=0.0.0.0:12345
      - --storage.path=/var/lib/alloy/data
      - /etc/alloy/config.alloy

  db:
    image: mysql:8.0
    container_name: zent-mysql
    restart: always
    environment:
      MYSQL_ROOT_PASSWORD: <YOUR_MYSQL_ROOT_PASSWORD>
      MYSQL_DATABASE: ZentDB
      MYSQL_USER: <YOUR_DB_USER>
      MYSQL_PASSWORD: <YOUR_DB_PASSWORD>
    ports:
      - "3306:3306"
    volumes:
      - db_data:/var/lib/mysql
    healthcheck:
      test: ["CMD", "mysqladmin", "ping", "-h", "localhost"]
      timeout: 20s
      retries: 10

  mongodb:
    image: mongo:latest
    container_name: zent-mongodb
    restart: always
    environment:
      MONGO_INITDB_ROOT_USERNAME: <YOUR_MONGO_ROOT_USER>
      MONGO_INITDB_ROOT_PASSWORD: <YOUR_MONGO_ROOT_PASSWORD>
    ports:
      - "27017:27017"
    volumes:
      - mongodb_data:/data/db

  valkey:
    image: valkey/valkey:latest
    container_name: zent-valkey
    restart: always
    ports:
      - "6379:6379"
    volumes:
      - valkey_data:/data

  rabbitmq:
    image: rabbitmq:3-management
    container_name: zent-rabbitmq
    restart: always
    ports:
      - "5672:5672"
      - "15672:15672"
    volumes:
      - mq_data:/var/lib/rabbitmq

  minio:
    image: minio/minio
    container_name: zent-minio
    restart: always
    command: server /data --console-address ":9001"
    environment:
      MINIO_ROOT_USER: <MINIO_ROOT_USER>
      MINIO_ROOT_PASSWORD: <MINIO_ROOT_PASSWORD>
    ports:
      - "9000:9000"
      - "9001:9001"
    volumes:
      - minio_data:/data

volumes:
  db_data:
  mongodb_data:
  valkey_data:
  mq_data:
  minio_data:
```

Khởi chạy compose stack:

```bash
docker compose -f docker-compose.local.yml up -d
```

Kiểm tra các container đã khởi tạo thành công:

```bash
docker compose -f docker-compose.local.yml ps
```
