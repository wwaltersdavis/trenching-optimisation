use crate::statistics::{mean, percentile, standard_deviation};
use std::collections::HashMap;
use trenching_optimisation::Percentage;

/// Represents the detection statistics for a given group of features.
/// The features are grouped by site, category, and category value e.g.
/// ("Site A", "Period", "Bronze Age").
pub struct DetectionStats {
    pub total_features_found: u32,
    pub total_features_missed: u32,
    pub total_area_found: f64,
    pub total_area_missed: f64,
    pub total_layouts_tested: u32,
    pub total_trenches_deployed: u32,
    pub features_found_percentages: Vec<Percentage>,
    pub area_found_percentages: Vec<Percentage>,
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

    /// Set the percentages for features found and area found based on the current
    /// totals for a given layout.
    pub fn set_percentages(&mut self) {
        self.features_found_percentages = vec![self.feature_found_percentage()];
        self.area_found_percentages = vec![self.area_found_percentage()];
    }

    pub fn feature_found_percentage(&self) -> Percentage {
        Percentage::from_fraction(
            self.total_features_found as f64,
            (self.total_features_found + self.total_features_missed) as f64,
        )
    }

    pub fn area_found_percentage(&self) -> Percentage {
        Percentage::from_fraction(
            self.total_area_found,
            self.total_area_found + self.total_area_missed,
        )
    }

    pub fn mean_trenches_per_layout(&self) -> f64 {
        self.total_trenches_deployed as f64 / self.total_layouts_tested as f64
    }

    pub fn features_detected_per_trench(&self) -> f64 {
        self.total_features_found as f64 / self.total_trenches_deployed as f64
    }

    pub fn excavated_feature_area_per_trench(&self) -> f64 {
        self.total_area_found / self.total_trenches_deployed as f64
    }

    pub fn probability_of_a_single_feature_detection(&self) -> Percentage {
        Percentage::from_fraction(
            self.layouts_tested_where_a_feature_found as f64,
            self.total_layouts_tested as f64,
        )
    }

    /// Returns the probability of recovering a feature at or above a given
    /// nominal coverage percentage.
    pub fn probability_feature_recovery_ge_nominal(
        &self,
        nominal_coverage_percent: Percentage,
    ) -> Percentage {
        let n_ge_nominal = self
            .features_found_percentages
            .iter()
            .filter(|&&p| p >= nominal_coverage_percent)
            .count();

        Percentage::from_fraction(n_ge_nominal as f64, self.total_layouts_tested as f64)
    }

    /// Returns the probability of recovering an area at or above a given
    /// nominal coverage percentage.
    pub fn probability_area_recovery_ge_nominal(
        &self,
        nominal_coverage_percent: Percentage,
    ) -> Percentage {
        let n_ge_nominal = self
            .area_found_percentages
            .iter()
            .filter(|&&p| p >= nominal_coverage_percent)
            .count();

        Percentage::from_fraction(n_ge_nominal as f64, self.total_layouts_tested as f64)
    }

    pub fn mean_run_feature_percentage(&self) -> Percentage {
        mean(&self.features_found_percentages)
    }

    pub fn sd_run_feature_percentage(&self) -> f64 {
        standard_deviation(&self.features_found_percentages)
    }

    pub fn mean_run_area_percentage(&self) -> Percentage {
        mean(&self.area_found_percentages)
    }

    pub fn sd_run_area_percentage(&self) -> f64 {
        standard_deviation(&self.area_found_percentages)
    }

    pub fn coefficient_of_variation_feature_recovery(&self) -> f64 {
        self.sd_run_feature_percentage() / f64::from(self.mean_run_feature_percentage())
    }

    pub fn coefficient_of_variation_area_recovery(&self) -> f64 {
        self.sd_run_area_percentage() / f64::from(self.mean_run_area_percentage())
    }

    pub fn run_feature_percentage(&self, p: f64) -> Percentage {
        percentile(&self.features_found_percentages, p)
    }

    pub fn run_area_percentage(&self, p: f64) -> Percentage {
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
                stats.feature_found_percentage().to_2dp(),
                stats.total_trenches_deployed,
            )
        } else {
            "No general stats available.".to_string()
        }
    }
}

// TODO: Add tests for StatsMap and DetectionStats methods.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detection_stats_percentages() {
        let mut stats = DetectionStats::new();
        stats.total_features_found = 80;
        stats.total_features_missed = 20;
        stats.total_area_found = 800.0;
        stats.total_area_missed = 200.0;
        stats.set_percentages();
        assert_eq!(stats.feature_found_percentage(), Percentage(80.0));
        assert_eq!(stats.area_found_percentage(), Percentage(80.0));
    }

    #[test]
    fn test_mean_trenches_per_layout() {
        let mut stats = DetectionStats::new();
        stats.total_trenches_deployed = 100;
        stats.total_layouts_tested = 10;
        assert_eq!(stats.mean_trenches_per_layout(), 10.0);
    }

    #[test]
    fn test_features_detected_per_trench() {
        let mut stats = DetectionStats::new();
        stats.total_features_found = 50;
        stats.total_trenches_deployed = 10;
        assert_eq!(stats.features_detected_per_trench(), 5.0);
    }

    #[test]
    fn test_excavated_feature_area_per_trench() {
        let mut stats = DetectionStats::new();
        stats.total_area_found = 500.0;
        stats.total_trenches_deployed = 100;
        assert_eq!(stats.excavated_feature_area_per_trench(), 5.0);
    }

    // #[test]
}
