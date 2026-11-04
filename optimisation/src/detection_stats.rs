use crate::{intersects::FeaturesStats, TrenchConfig};
use std::io::Write;

#[derive(Debug, Default, Clone)]
pub struct DetectionStats {
    pub found: i64,
    pub missed: i64,
    area_found: f64,
    area_missed: f64,
    trenches_deployed: u64,
    eligible_runs: u64,
    detected_runs: u64,
    nominal_feature_recovery_runs: u64,

    feature_percentage_sum: f64,
    feature_percentage_sum_squares: f64,
    // Store run-level recovery values so we can calculate true percentile bands
    // for fan plots: p05/p25/p50/p75/p95.
    feature_percentage_values: Vec<f64>,

    area_eligible_runs: u64,
    nominal_area_recovery_runs: u64,
    area_percentage_sum: f64,
    area_percentage_sum_squares: f64,

    // Store run-level area recovery values for true area fan plots.
    area_percentage_values: Vec<f64>,
}

impl DetectionStats {
    pub fn add_run(&mut self, features_stats: &FeaturesStats, nominal_coverage_percent: f64) {
        self.found += features_stats.features_found as i64;
        self.missed += features_stats.features_missed as i64;
        self.area_found += features_stats.area_found;
        self.area_missed += features_stats.area_missed;
        self.trenches_deployed += features_stats.trenches_deployed;

        let feature_total = features_stats.features_found + features_stats.features_missed;

        if feature_total > 0 {
            let percentage = features_stats.features_found as f64 / feature_total as f64 * 100.0;

            self.eligible_runs += 1;

            if features_stats.features_found > 0 {
                self.detected_runs += 1;
            }

            if nominal_coverage_percent.is_finite() && percentage >= nominal_coverage_percent {
                self.nominal_feature_recovery_runs += 1;
            }

            self.feature_percentage_sum += percentage;
            self.feature_percentage_sum_squares += percentage * percentage;
            self.feature_percentage_values.push(percentage);
        }

        let area_total = features_stats.area_found + features_stats.area_missed;

        if area_total > 0.0 {
            let percentage = features_stats.area_found / area_total * 100.0;

            self.area_eligible_runs += 1;

            if nominal_coverage_percent.is_finite() && percentage >= nominal_coverage_percent {
                self.nominal_area_recovery_runs += 1;
            }

            self.area_percentage_sum += percentage;
            self.area_percentage_sum_squares += percentage * percentage;
            self.area_percentage_values.push(percentage);
        }
    }

    pub fn feature_percentage(&self) -> f64 {
        percentage(self.found as f64, (self.found + self.missed) as f64)
    }

    fn area_percentage(&self) -> f64 {
        percentage(self.area_found, self.area_found + self.area_missed)
    }

    fn probability_detection(&self) -> f64 {
        percentage(self.detected_runs as f64, self.eligible_runs as f64)
    }

    fn probability_feature_recovery_ge_nominal(&self) -> f64 {
        percentage(
            self.nominal_feature_recovery_runs as f64,
            self.eligible_runs as f64,
        )
    }

    fn probability_area_recovery_ge_nominal(&self) -> f64 {
        percentage(
            self.nominal_area_recovery_runs as f64,
            self.area_eligible_runs as f64,
        )
    }

    fn coefficient_of_variation_feature_recovery(&self) -> f64 {
        let mean = self.mean_run_feature_percentage();

        if mean > 0.0 {
            self.sd_run_feature_percentage() / mean
        } else {
            f64::NAN
        }
    }

    fn coefficient_of_variation_area_recovery(&self) -> f64 {
        let mean = self.mean_run_area_percentage();

        if mean > 0.0 {
            self.sd_run_area_percentage() / mean
        } else {
            f64::NAN
        }
    }

    fn mean_run_feature_percentage(&self) -> f64 {
        mean(self.feature_percentage_sum, self.eligible_runs)
    }

    fn sd_run_feature_percentage(&self) -> f64 {
        sample_sd(
            self.feature_percentage_sum,
            self.feature_percentage_sum_squares,
            self.eligible_runs,
        )
    }

    fn mean_run_area_percentage(&self) -> f64 {
        mean(self.area_percentage_sum, self.area_eligible_runs)
    }

    fn sd_run_area_percentage(&self) -> f64 {
        sample_sd(
            self.area_percentage_sum,
            self.area_percentage_sum_squares,
            self.area_eligible_runs,
        )
    }

    fn run_feature_percentage(&self, p: f64) -> f64 {
        percentile(&self.feature_percentage_values, p)
    }

    fn run_area_percentage(&self, p: f64) -> f64 {
        percentile(&self.area_percentage_values, p)
    }

    fn mean_trenches_per_pattern(&self) -> f64 {
        if self.eligible_runs > 0 {
            self.trenches_deployed as f64 / self.eligible_runs as f64
        } else {
            f64::NAN
        }
    }

    fn features_detected_per_trench(&self) -> f64 {
        if self.trenches_deployed > 0 {
            self.found as f64 / self.trenches_deployed as f64
        } else {
            f64::NAN
        }
    }

    fn excavated_feature_area_per_trench(&self) -> f64 {
        if self.trenches_deployed > 0 {
            self.area_found / self.trenches_deployed as f64
        } else {
            f64::NAN
        }
    }
}

fn percentage(numerator: f64, denominator: f64) -> f64 {
    if denominator > 0.0 {
        numerator / denominator * 100.0
    } else {
        f64::NAN
    }
}

fn mean(sum: f64, count: u64) -> f64 {
    if count > 0 {
        sum / count as f64
    } else {
        f64::NAN
    }
}

fn sample_sd(sum: f64, sum_squares: f64, count: u64) -> f64 {
    if count > 1 {
        let variance = (sum_squares - (sum * sum / count as f64)) / (count - 1) as f64;

        variance.max(0.0).sqrt()
    } else if count == 1 {
        0.0
    } else {
        f64::NAN
    }
}

fn percentile(values: &[f64], percentile: f64) -> f64 {
    let mut clean_values: Vec<f64> = values
        .iter()
        .copied()
        .filter(|value| value.is_finite())
        .collect();

    if clean_values.is_empty() {
        return f64::NAN;
    }

    clean_values.sort_by(|a, b| a.partial_cmp(b).unwrap());

    if clean_values.len() == 1 {
        return clean_values[0];
    }

    let rank = percentile.clamp(0.0, 1.0) * (clean_values.len() - 1) as f64;
    let lower_index = rank.floor() as usize;
    let upper_index = rank.ceil() as usize;

    if lower_index == upper_index {
        clean_values[lower_index]
    } else {
        let weight = rank - lower_index as f64;
        clean_values[lower_index] * (1.0 - weight) + clean_values[upper_index] * weight
    }
}

pub fn write_stats_row(
    file: &mut fs_err::File,
    site: &str,
    config: &TrenchConfig,
    distribution_label: &str,
    category: &str,
    category_value: &str,
    stats: &DetectionStats,
    total_trenches: u64,
) {
    let row = vec![
        site.to_string(),
        format!("{:?}", config.pattern_name),
        distribution_label.to_string(),
        category.to_string(),
        category_value.to_string(),
        stats.found.to_string(),
        stats.missed.to_string(),
        format!("{:.2}", stats.feature_percentage()),
        format!("{:.2}", stats.area_found),
        format!("{:.2}", stats.area_missed),
        format!("{:.2}", stats.area_percentage()),
        total_trenches.to_string(),
        stats.trenches_deployed.to_string(),
        format!("{:.2}", stats.mean_trenches_per_pattern()),
        format!("{:.4}", stats.features_detected_per_trench()),
        format!("{:.4}", stats.excavated_feature_area_per_trench()),
        stats.eligible_runs.to_string(),
        stats.detected_runs.to_string(),
        format!("{:.2}", stats.probability_detection()),
        format!("{:.2}", stats.probability_feature_recovery_ge_nominal()),
        format!("{:.2}", stats.probability_area_recovery_ge_nominal()),
        format!("{:.4}", stats.coefficient_of_variation_feature_recovery()),
        format!("{:.4}", stats.coefficient_of_variation_area_recovery()),
        format!("{:.2}", stats.mean_run_feature_percentage()),
        format!("{:.2}", stats.sd_run_feature_percentage()),
        format!("{:.2}", stats.run_feature_percentage(0.05)),
        format!("{:.2}", stats.run_feature_percentage(0.25)),
        format!("{:.2}", stats.run_feature_percentage(0.5)),
        format!("{:.2}", stats.run_feature_percentage(0.75)),
        format!("{:.2}", stats.run_feature_percentage(0.95)),
        stats.area_eligible_runs.to_string(),
        format!("{:.2}", stats.mean_run_area_percentage()),
        format!("{:.2}", stats.sd_run_area_percentage()),
        format!("{:.2}", stats.run_area_percentage(0.05)),
        format!("{:.2}", stats.run_area_percentage(0.25)),
        format!("{:.2}", stats.run_area_percentage(0.5)),
        format!("{:.2}", stats.run_area_percentage(0.75)),
        format!("{:.2}", stats.run_area_percentage(0.95)),
    ];

    writeln!(file, "{}", row.join(",")).unwrap();
}
