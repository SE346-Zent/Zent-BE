use uuid::Uuid;

/// Calculate the Employee ID based on role ID and user UUID.
pub fn get_employee_id(role_id: i32, user_id: Uuid) -> Option<String> {
    let id_str = user_id.to_string();
    if id_str.len() < 4 {
        return None;
    }
    let suffix = &id_str[..4];
    match role_id {
        4 => Some(format!("TECH{}", suffix)),
        1 => Some(format!("ADMIN{}", suffix)),
        2 => Some(format!("SA{}", suffix)),
        _ => None,
    }
}
