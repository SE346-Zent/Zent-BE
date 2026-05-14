/// Check whether a serial number exists in the known product model catalog.
pub fn check_serial_exists(
    serial_number: &str,
    known_serials: &[String],
) -> bool {
    let normalized = serial_number.trim().to_lowercase();
    known_serials.iter().any(|s| s.to_lowercase() == normalized)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_finds_match() {
        assert!(check_serial_exists("SN-001", &["SN-001".into(), "SN-002".into()]));
    }

    #[test]
    fn test_no_match() {
        assert!(!check_serial_exists("SN-999", &["SN-001".into()]));
    }

    #[test]
    fn test_case_insensitive() {
        assert!(check_serial_exists("sn-001", &["SN-001".into()]));
    }

    #[test]
    fn test_trims_whitespace() {
        assert!(check_serial_exists("  SN-001  ", &["SN-001".into()]));
    }

    #[test]
    fn test_empty_list() {
        let empty: Vec<String> = vec![];
        assert!(!check_serial_exists("SN-001", &empty));
    }
}
