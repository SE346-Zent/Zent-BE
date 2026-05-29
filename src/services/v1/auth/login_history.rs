use crate::{
    entities::login_audit_logs,
    model::responses::auth::login_history_response::LoginHistoryEntry,
};

pub fn decide_login_history(records: Vec<login_audit_logs::Model>) -> Vec<LoginHistoryEntry> {
    records
        .into_iter()
        .map(|record| LoginHistoryEntry {
            id: record.id,
            session_id: record.session_id,
            device_name: record.device_name,
            location: record.location,
            ip_address: record.ip_address,
            created_at: record.created_at,
        })
        .collect()
}