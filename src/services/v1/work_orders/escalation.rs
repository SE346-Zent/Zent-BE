/// Describes the outcome of an escalation decision.
#[derive(Debug)]
pub struct EscalationEffect {
    /// 1 = 110%, 2 = 125%, 3 = 150%
    pub target_level: i32,
    pub level_label: &'static str,
}

/// Pure logic: decide whether a work order should be escalated based on
/// elapsed time vs. baseline.
///
/// Returns `None` if no threshold has been crossed or if the highest
/// already-notified level covers the current elapsed time.
pub fn decide_escalation(
    baseline_minutes: i64,
    elapsed_minutes: i64,
    highest_notified_level: i32,  // 0 = never notified, 1 = 110%, 2 = 125%, 3 = 150%
) -> Option<EscalationEffect> {
    let threshold_110 = (baseline_minutes as f64 * 1.10) as i64;
    let threshold_125 = (baseline_minutes as f64 * 1.25) as i64;
    let threshold_150 = (baseline_minutes as f64 * 1.50) as i64;

    let (target_level, level_label) = if elapsed_minutes >= threshold_150 {
        (3, "150%")
    } else if elapsed_minutes >= threshold_125 {
        (2, "125%")
    } else if elapsed_minutes >= threshold_110 {
        (1, "110%")
    } else {
        return None;
    };

    if target_level <= highest_notified_level {
        return None; // Already notified at or above this level
    }

    Some(EscalationEffect {
        target_level,
        level_label,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_escalation_below_110() {
        // 50 min elapsed, 60 min baseline → below 110% (66 min)
        assert!(decide_escalation(60, 50, 0).is_none());
    }

    #[test]
    fn test_escalation_at_110() {
        // 70 min elapsed, 60 min baseline → above 110% (66 min)
        let effect = decide_escalation(60, 70, 0);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.target_level, 1);
        assert_eq!(e.level_label, "110%");
    }

    #[test]
    fn test_already_notified_skips() {
        // Already notified at 125%, so 110% should be skipped
        assert!(decide_escalation(60, 70, 2).is_none());
    }

    #[test]
    fn test_escalation_at_150() {
        // 100 min elapsed, 60 min baseline → above 150% (90 min)
        let effect = decide_escalation(60, 100, 0);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.target_level, 3);
        assert_eq!(e.level_label, "150%");
    }

    #[test]
    fn test_progression_from_110_to_125() {
        // Level 1 already sent, now at 80 min → 125% threshold (75 min)
        let effect = decide_escalation(60, 80, 1);
        assert!(effect.is_some());
        let e = effect.unwrap();
        assert_eq!(e.target_level, 2);
    }
}
