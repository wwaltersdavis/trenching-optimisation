use crate::{
    intersects::FeaturesStats,
    statistics::{mean, percentage, percentile, sample_sd},
};

#[derive(Debug, Default, Clone)]
pub struct DetectionStats {
    pub found: i64,
    pub missed: i64,
    pub area_found: f64,
    pub area_missed: f64,
    pub trenches_deployed: u64,
    pub eligible_runs: u64,
    pub detected_runs: u64,
    nominal_feature_recovery_runs: u64,

    feature_percentage_sum: f64,
    feature_percentage_sum_squares: f64,
    // Store run-level recovery values so we can calculate true percentile bands
    // for fan plots: p05/p25/p50/p75/p95.
    feature_percentage_values: Vec<f64>,

    pub area_eligible_runs: u64,
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

    pub fn area_percentage(&self) -> f64 {
        percentage(self.area_found, self.area_found + self.area_missed)
    }

    pub fn probability_detection(&self) -> f64 {
        percentage(self.detected_runs as f64, self.eligible_runs as f64)
    }

    pub fn probability_feature_recovery_ge_nominal(&self) -> f64 {
        percentage(
            self.nominal_feature_recovery_runs as f64,
            self.eligible_runs as f64,
        )
    }

    pub fn probability_area_recovery_ge_nominal(&self) -> f64 {
        percentage(
            self.nominal_area_recovery_runs as f64,
            self.area_eligible_runs as f64,
        )
    }

    pub fn coefficient_of_variation_feature_recovery(&self) -> f64 {
        let mean = self.mean_run_feature_percentage();

        if mean > 0.0 {
            self.sd_run_feature_percentage() / mean
        } else {
            f64::NAN
        }
    }

    pub fn coefficient_of_variation_area_recovery(&self) -> f64 {
        let mean = self.mean_run_area_percentage();

        if mean > 0.0 {
            self.sd_run_area_percentage() / mean
        } else {
            f64::NAN
        }
    }

    pub fn mean_run_feature_percentage(&self) -> f64 {
        mean(self.feature_percentage_sum, self.eligible_runs)
    }

    pub fn sd_run_feature_percentage(&self) -> f64 {
        sample_sd(
            self.feature_percentage_sum,
            self.feature_percentage_sum_squares,
            self.eligible_runs,
        )
    }

    pub fn mean_run_area_percentage(&self) -> f64 {
        mean(self.area_percentage_sum, self.area_eligible_runs)
    }

    pub fn sd_run_area_percentage(&self) -> f64 {
        sample_sd(
            self.area_percentage_sum,
            self.area_percentage_sum_squares,
            self.area_eligible_runs,
        )
    }

    pub fn run_feature_percentage(&self, p: f64) -> f64 {
        percentile(&self.feature_percentage_values, p)
    }

    pub fn run_area_percentage(&self, p: f64) -> f64 {
        percentile(&self.area_percentage_values, p)
    }

    pub fn mean_trenches_per_pattern(&self) -> f64 {
        if self.eligible_runs > 0 {
            self.trenches_deployed as f64 / self.eligible_runs as f64
        } else {
            f64::NAN
        }
    }

    pub fn features_detected_per_trench(&self) -> f64 {
        if self.trenches_deployed > 0 {
            self.found as f64 / self.trenches_deployed as f64
        } else {
            f64::NAN
        }
    }

    pub fn excavated_feature_area_per_trench(&self) -> f64 {
        if self.trenches_deployed > 0 {
            self.area_found / self.trenches_deployed as f64
        } else {
            f64::NAN
        }
    }
}
