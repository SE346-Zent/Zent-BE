/// Check whether a specific serial number exists within a provided list of known catalog serials.
///
/// This pure function performs a case-insensitive, whitespace-trimmed comparison
/// to ensure robust validation of product serial numbers against the catalog.
///
/// # Arguments
/// * `provided_serial_number` - The serial number string to be validated.
/// * `known_catalog_serials` - A slice of valid serial number strings from the database.
///
/// # Returns
/// `true` if a matching serial is found in the catalog, `false` otherwise.
pub fn check_serial_exists(
    provided_serial_number: &str,
    known_catalog_serials: &[String],
) -> bool {
    let normalized_input = provided_serial_number.trim().to_lowercase();
    known_catalog_serials.iter().any(|catalog_serial| catalog_serial.to_lowercase() == normalized_input)
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
