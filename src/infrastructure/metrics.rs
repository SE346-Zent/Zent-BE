use opentelemetry::metrics::{Counter, Histogram, UpDownCounter};
use std::sync::OnceLock;

/// Central registry for all business-level metrics.
///
/// Each metric is registered once via the global OTLP meter and reused across
/// the application. Metrics flow through the existing OTLP exporter → Alloy →
/// Grafana Cloud pipeline with no additional configuration needed.
pub struct BusinessMetrics {
    // ── Auth ──
    pub auth_login_total: Counter<u64>,
    pub auth_token_refresh_total: Counter<u64>,
    pub auth_password_change_total: Counter<u64>,

    // ── Work Orders ──
    pub wo_state_transition_total: Counter<u64>,
    pub wo_created_total: Counter<u64>,
    pub wo_auto_assign_total: Counter<u64>,

    // ── WebSocket ──
    pub ws_connections_active: UpDownCounter<i64>,
    pub ws_messages_sent_total: Counter<u64>,
    pub ws_closed_total: Counter<u64>,
    pub ws_auth_fail_total: Counter<u64>,

    // ── Notifications ──
    pub notification_sent_total: Counter<u64>,
    pub notification_failed_total: Counter<u64>,

    // ── Cache ──
    pub cache_hit_total: Counter<u64>,
    pub cache_miss_total: Counter<u64>,

    // ── External APIs ──
    pub external_api_duration: Histogram<f64>,
    pub external_api_errors_total: Counter<u64>,

    // ── Cron Jobs ──
    pub cron_job_duration: Histogram<f64>,
    pub cron_job_errors_total: Counter<u64>,

    // ── AppErrors ──
    pub app_error_total: Counter<u64>,
}

static BUSINESS_METRICS: OnceLock<BusinessMetrics> = OnceLock::new();

/// Initialize and return the global `BusinessMetrics` singleton.
///
/// Safe to call multiple times — only the first call registers the instruments.
pub fn init() -> &'static BusinessMetrics {
    BUSINESS_METRICS.get_or_init(|| {
        let m = crate::infrastructure::observability::meter();

        BusinessMetrics {
            // Auth
            auth_login_total: m.u64_counter("auth.login.total")
                .with_description("Total login attempts")
                .build(),
            auth_token_refresh_total: m.u64_counter("auth.token_refresh.total")
                .with_description("Total token refresh attempts")
                .build(),
            auth_password_change_total: m.u64_counter("auth.password_change.total")
                .with_description("Total password changes")
                .build(),

            // Work Orders
            wo_state_transition_total: m.u64_counter("work_order.state_transition.total")
                .with_description("Work order status transitions")
                .build(),
            wo_created_total: m.u64_counter("work_order.created.total")
                .with_description("Total work orders created")
                .build(),
            wo_auto_assign_total: m.u64_counter("work_order.auto_assign.total")
                .with_description("Auto-assign attempts")
                .build(),

            // WebSocket
            ws_connections_active: m.i64_up_down_counter("ws.connections.active")
                .with_description("Currently active WebSocket connections")
                .build(),
            ws_messages_sent_total: m.u64_counter("ws.messages_sent.total")
                .with_description("Total WebSocket messages sent")
                .build(),
            ws_closed_total: m.u64_counter("ws.closed.total")
                .with_description("WebSocket close events by reason")
                .build(),
            ws_auth_fail_total: m.u64_counter("ws.auth_fail.total")
                .with_description("WebSocket authentication failures")
                .build(),

            // Notifications
            notification_sent_total: m.u64_counter("notification.sent.total")
                .with_description("Notifications sent by channel")
                .build(),
            notification_failed_total: m.u64_counter("notification.failed.total")
                .with_description("Notification delivery failures")
                .build(),

            // Cache
            cache_hit_total: m.u64_counter("cache.hit.total")
                .with_description("Cache hits")
                .build(),
            cache_miss_total: m.u64_counter("cache.miss.total")
                .with_description("Cache misses")
                .build(),

            // External APIs
            external_api_duration: m.f64_histogram("external_api.duration")
                .with_description("External API call duration in seconds")
                .with_unit("s")
                .build(),
            external_api_errors_total: m.u64_counter("external_api.errors.total")
                .with_description("External API call failures")
                .build(),

            // Cron
            cron_job_duration: m.f64_histogram("cron.job.duration")
                .with_description("Cron job execution duration in seconds")
                .with_unit("s")
                .build(),
            cron_job_errors_total: m.u64_counter("cron.job.errors.total")
                .with_description("Cron job execution failures")
                .build(),

            // AppErrors
            app_error_total: m.u64_counter("app.error.total")
                .with_description("Application errors by type and status code")
                .build(),
        }
    })
}
