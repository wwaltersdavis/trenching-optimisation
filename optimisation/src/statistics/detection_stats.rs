use crate::statistics::{mean, percentage, percentile, standard_deviation};
use std::collections::HashMap;

pub struct DetectionStats {
    pub total_features_found: u32,
    pub total_features_missed: u32,
    pub total_area_found: f64,
    pub total_area_missed: f64,
    pub total_layouts_tested: u32,
    pub total_trenches_deployed: u32,
    pub features_found_percentages: Vec<f64>,
    pub area_found_percentages: Vec<f64>,
    pub layouts_tested_where_a_feature_found: u32,
}

impl DetectionStats {
    pub fn new() -> Self {
        DetectionStats {
            total_features_found: 0,
            total_features_missed: 0,
            total_area_found: 0.0,
            total_area_missed: 0.0,
            total_layouts_tested: 0,
            total_trenches_deployed: 0,
            features_found_percentages: Vec::new(),
            area_found_percentages: Vec::new(),
            layouts_tested_where_a_feature_found: 0,
        }
    }

    pub fn update_percentages(&mut self) {
        self.features_found_percentages = vec![self.feature_found_percentage()];
        self.area_found_percentages = vec![self.area_found_percentage()];
    }

    pub fn feature_found_percentage(&self) -> f64 {
        percentage(
            self.total_features_found as f64,
            (self.total_features_found + self.total_features_missed) as f64,
        )
    }

    pub fn area_found_percentage(&self) -> f64 {
        percentage(
            self.total_area_found,
            self.total_area_found + self.total_area_missed,
        )
    }

    pub fn mean_trenches_per_pattern(&self) -> f64 {
        self.total_trenches_deployed as f64 / self.total_layouts_tested as f64
    }

    pub fn features_detected_per_trench(&self) -> f64 {
        self.total_features_found as f64 / self.total_trenches_deployed as f64
    }

    pub fn excavated_feature_area_per_trench(&self) -> f64 {
        self.total_area_found / self.total_trenches_deployed as f64
    }

    pub fn probability_detection(&self) -> f64 {
        percentage(
            self.layouts_tested_where_a_feature_found as f64,
            self.total_layouts_tested as f64,
        )
    }

    pub fn probability_feature_recovery_ge_nominal(&self, nominal_coverage_percent: f64) -> f64 {
        if nominal_coverage_percent.is_nan() {
            return f64::NAN;
        }
        let n_ge_nominal = self
            .features_found_percentages
            .iter()
            .filter(|&&p| p >= nominal_coverage_percent)
            .count();

        percentage(n_ge_nominal as f64, self.total_layouts_tested as f64)
    }

    pub fn probability_area_recovery_ge_nominal(&self, nominal_coverage_percent: f64) -> f64 {
        if nominal_coverage_percent.is_nan() {
            return f64::NAN;
        }
        let n_ge_nominal = self
            .area_found_percentages
            .iter()
            .filter(|&&p| p >= nominal_coverage_percent)
            .count();

        percentage(n_ge_nominal as f64, self.total_layouts_tested as f64)
    }

    pub fn mean_run_feature_percentage(&self) -> f64 {
        mean(&self.features_found_percentages)
    }

    pub fn sd_run_feature_percentage(&self) -> f64 {
        standard_deviation(&self.features_found_percentages)
    }

    pub fn mean_run_area_percentage(&self) -> f64 {
        mean(&self.area_found_percentages)
    }

    pub fn sd_run_area_percentage(&self) -> f64 {
        standard_deviation(&self.area_found_percentages)
    }

    pub fn coefficient_of_variation_feature_recovery(&self) -> f64 {
        self.sd_run_feature_percentage() / self.mean_run_feature_percentage()
    }

    pub fn coefficient_of_variation_area_recovery(&self) -> f64 {
        self.sd_run_area_percentage() / self.mean_run_area_percentage()
    }

    pub fn run_feature_percentage(&self, p: f64) -> f64 {
        percentile(&self.features_found_percentages, p)
    }

    pub fn run_area_percentage(&self, p: f64) -> f64 {
        percentile(&self.area_found_percentages, p)
    }
}

// Key: (site, category, category_value)
pub struct StatsMap<'a>(pub HashMap<(&'a str, &'a str, &'a str), DetectionStats>);

impl<'a> StatsMap<'a> {
    pub fn new() -> Self {
        StatsMap(HashMap::new())
    }

    fn get_key(
        site: &'a str,
        category: &'a str,
        category_value: &'a str,
    ) -> (&'a str, &'a str, &'a str) {
        (site, category, category_value)
    }

    pub fn add_from_feature(
        &mut self,
        site: &'a str,
        category: &'a str,
        category_value: &'a str,
        feature_found: bool,
        area_found: f64,
        area_missed: f64,
        trenches_deployed: u32,
    ) {
        for site in [site, "All"] {
            let key = Self::get_key(site, category, category_value);
            let stats = self.0.entry(key).or_insert_with(DetectionStats::new);

            if feature_found {
                stats.total_features_found += 1;
            } else {
                stats.total_features_missed += 1;
            }
            stats.total_area_found += area_found;
            stats.total_area_missed += area_missed;
            // Can just set these every time as very minor cost
            stats.total_layouts_tested = 1;
            stats.total_trenches_deployed = trenches_deployed;
        }
    }

    pub fn add_from_maps(&mut self, stats_maps: &Vec<StatsMap<'a>>) {
        for stats_map in stats_maps {
            for ((site, category, category_value), stats) in &stats_map.0 {
                let key = Self::get_key(site, category, category_value);
                let entry = self.0.entry(key).or_insert_with(DetectionStats::new);

                entry.total_features_found += stats.total_features_found;
                entry.total_features_missed += stats.total_features_missed;
                entry.total_area_found += stats.total_area_found;
                entry.total_area_missed += stats.total_area_missed;
                entry.total_layouts_tested += stats.total_layouts_tested;
                entry.total_trenches_deployed += stats.total_trenches_deployed;
                entry
                    .features_found_percentages
                    .extend(&stats.features_found_percentages);
                entry
                    .area_found_percentages
                    .extend(&stats.area_found_percentages);
                entry.layouts_tested_where_a_feature_found += {
                    if stats.total_features_found > 0 {
                        1
                    } else {
                        0
                    }
                }
            }
        }
    }

    pub fn general_stats_message(&self) -> String {
        if let Some(stats) = self.0.get(&("All", "All", "All")) {
            format!(
                "Total features found: {} \nTotal features missed: {} \nPercentage found: {:.2}% \nTotal trenches deployed: {}",
                stats.total_features_found,
                stats.total_features_missed,
                stats.feature_found_percentage(),
                stats.total_trenches_deployed,
            )
        } else {
            "No general stats available.".to_string()
        }
    }
}

// TODO: Add tests for StatsMap and DetectionStats methods.
