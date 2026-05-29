use crate::model::responses::inventory::admin_analytics_response::{
    AdminAnalyticsResponse, JobCompletionTrend, PartCategoryEntry, TechnicianPerformanceEntry,
    TotalMetric,
};
use chrono::{DateTime, Duration, Utc};

pub struct AnalyticsInput {
    pub current_orders: Vec<DateTime<Utc>>,
    pub previous_orders: Vec<DateTime<Utc>>,
    pub current_completed_orders: Vec<DateTime<Utc>>,
    pub previous_completed_orders: Vec<DateTime<Utc>>,
    pub current_imported_parts: i64,
    pub previous_imported_parts: i64,
    pub current_returned_parts: i64,
    pub previous_returned_parts: i64,
    pub part_type_counts: Vec<(String, i64)>,
    pub technician_performance: Vec<TechnicianPerformanceEntry>,
}

pub fn decide_admin_analytics(input: AnalyticsInput, period_days: i64) -> AdminAnalyticsResponse {
    let total_orders = TotalMetric {
        value: input.current_orders.len() as i64,
        percent_change: compute_percent_change(
            input.previous_orders.len() as i64,
            input.current_orders.len() as i64,
        ),
    };

    let total_imported_parts = TotalMetric {
        value: input.current_imported_parts,
        percent_change: compute_percent_change(
            input.previous_imported_parts,
            input.current_imported_parts,
        ),
    };

    let total_returned_parts = TotalMetric {
        value: input.current_returned_parts,
        percent_change: compute_percent_change(
            input.previous_returned_parts,
            input.current_returned_parts,
        ),
    };

    let job_completion_trend = build_job_completion_trend(
        &input.current_completed_orders,
        &input.previous_completed_orders,
        period_days,
    );

    let part_categories = build_part_categories(input.part_type_counts);

    AdminAnalyticsResponse {
        total_orders,
        total_imported_parts,
        total_returned_parts,
        job_completion_trend,
        part_categories,
        technician_performance: input.technician_performance,
    }
}

fn compute_percent_change(previous: i64, current: i64) -> f64 {
    if previous == 0 {
        if current == 0 {
            0.0
        } else {
            100.0
        }
    } else {
        ((current - previous) as f64 / previous as f64) * 100.0
    }
}

fn build_job_completion_trend(
    current: &[DateTime<Utc>],
    previous: &[DateTime<Utc>],
    period_days: i64,
) -> JobCompletionTrend {
    let num_buckets = if period_days <= 7 { 7 } else { 4 };
    let now = Utc::now();
    let period_start = now - Duration::days(period_days);
    let prev_start = period_start - Duration::days(period_days);

    let bucket_duration = Duration::seconds((period_days * 86400) / num_buckets);

    let mut labels = Vec::with_capacity(num_buckets as usize);
    let mut current_counts = vec![0i64; num_buckets as usize];
    let mut previous_counts = vec![0i64; num_buckets as usize];

    for i in 0..num_buckets {
        let bucket_start = period_start + bucket_duration * (i as i32);
        let label = if period_days <= 7 {
            bucket_start.format("%a").to_string()
        } else {
            format!("Week {}", i + 1)
        };
        labels.push(label);
    }

    for &dt in current {
        if dt >= period_start && dt < now {
            let offset = dt - period_start;
            let bucket_idx = (offset.num_seconds() / bucket_duration.num_seconds()) as usize;
            if bucket_idx < current_counts.len() {
                current_counts[bucket_idx] += 1;
            }
        }
    }

    for &dt in previous {
        if dt >= prev_start && dt < period_start {
            let offset = dt - prev_start;
            let bucket_idx = (offset.num_seconds() / bucket_duration.num_seconds()) as usize;
            if bucket_idx < previous_counts.len() {
                previous_counts[bucket_idx] += 1;
            }
        }
    }

    JobCompletionTrend {
        labels,
        current: current_counts,
        previous: previous_counts,
    }
}

fn build_part_categories(part_type_counts: Vec<(String, i64)>) -> Vec<PartCategoryEntry> {
    let total: i64 = part_type_counts.iter().map(|(_, c)| c).sum();
    if total == 0 {
        return vec![];
    }

    let mut entries: Vec<PartCategoryEntry> = part_type_counts
        .into_iter()
        .map(|(category_name, count)| {
            let percent = (count as f64 / total as f64) * 100.0;
            PartCategoryEntry {
                category_name,
                count,
                percent,
            }
        })
        .collect();

    entries.sort_by(|a, b| b.count.cmp(&a.count));
    entries.truncate(5);
    entries
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn test_compute_percent_change_positive() {
        assert!((compute_percent_change(10, 12) - 20.0).abs() < 0.01);
    }

    #[test]
    fn test_compute_percent_change_negative() {
        assert!((compute_percent_change(10, 8) - (-20.0)).abs() < 0.01);
    }

    #[test]
    fn test_compute_percent_change_zero_previous() {
        assert_eq!(compute_percent_change(0, 5), 100.0);
    }

    #[test]
    fn test_compute_percent_change_both_zero() {
        assert_eq!(compute_percent_change(0, 0), 0.0);
    }

    #[test]
    fn test_build_part_categories() {
        let counts = vec![
            ("Battery".to_string(), 50),
            ("Screen".to_string(), 30),
            ("Motor".to_string(), 20),
        ];
        let result = build_part_categories(counts);
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].category_name, "Battery");
        assert!((result[0].percent - 50.0).abs() < 0.01);
    }

    #[test]
    fn test_build_part_categories_truncates_to_5() {
        let counts: Vec<(String, i64)> = (0..8)
            .map(|i| (format!("Type{}", i), 10 - i as i64))
            .collect();
        let result = build_part_categories(counts);
        assert_eq!(result.len(), 5);
    }

    #[test]
    fn test_decide_admin_analytics_basic() {
        let now = Utc::now();
        let input = AnalyticsInput {
            current_orders: vec![now],
            previous_orders: vec![now - Duration::days(8)],
            current_completed_orders: vec![now],
            previous_completed_orders: vec![now - Duration::days(8)],
            current_imported_parts: 15,
            previous_imported_parts: 10,
            current_returned_parts: 3,
            previous_returned_parts: 2,
            part_type_counts: vec![("Battery".to_string(), 10)],
            technician_performance: vec![],
        };
        let result = decide_admin_analytics(input, 7);
        assert_eq!(result.total_orders.value, 1);
        assert_eq!(result.total_imported_parts.value, 15);
        assert_eq!(result.total_returned_parts.value, 3);
        assert_eq!(result.part_categories.len(), 1);
    }
}
