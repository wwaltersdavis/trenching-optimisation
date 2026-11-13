use crate::{
    statistics::{four_dp, two_dp, DetectionStats, StatsMap},
    TrenchConfig,
};
use anyhow::Result;
use csv::Writer;
use serde::Serialize;
use trenching_optimisation::Distribution;

#[derive(Debug, Serialize)]
struct Row<'a> {
    site: &'a str,
    pattern: &'a str,
    distribution: String,
    category: &'a str,
    category_value: &'a str,
    found: u32,
    missed: u32,
    percentage_found: String,
    area_found: String,
    area_missed: String,
    percentage_area_found: String,
    trench_layouts_tested: u32,
    trenches_deployed: u32,
    mean_trenches_per_layout: String,
    features_detected_per_trench: String,
    excavated_feature_area_per_trench_m2: String,
    detected_runs: u32,
    probability_of_a_single_feature_detection: String,
    probability_feature_recovery_ge_nominal: Option<String>,
    probability_area_recovery_ge_nominal: Option<String>,
    coefficient_of_variation_feature_recovery: String,
    coefficient_of_variation_area_recovery: String,
    mean_run_percentage_found: String,
    sd_run_percentage_found: String,
    p05_run_percentage_found: String,
    p25_run_percentage_found: String,
    p50_run_percentage_found: String,
    p75_run_percentage_found: String,
    p95_run_percentage_found: String,
    mean_run_percentage_area_found: String,
    sd_run_percentage_area_found: String,
    p05_run_percentage_area_found: String,
    p25_run_percentage_area_found: String,
    p50_run_percentage_area_found: String,
    p75_run_percentage_area_found: String,
    p95_run_percentage_area_found: String,
}

impl<'a> Row<'a> {
    #[rustfmt::skip]
    pub fn from_stats(
        site: &'a str,
        category: &'a str,
        category_value: &'a str,
        stats: &DetectionStats,
        distribution: &Distribution,
        pattern_name: &'a str,
    ) -> Self {
        // If distribution is Coverage, calculate the probability of feature and area 
        // recovery at or above the nominal coverage percentage.
        let (probability_feature_recovery_ge_nominal, probability_area_recovery_ge_nominal) = match distribution.nominal_coverage_percent() {
            Some(nominal_coverage_percent) => (
                Some(stats.probability_feature_recovery_ge_nominal(nominal_coverage_percent).to_2dp()),
                Some(stats.probability_area_recovery_ge_nominal(nominal_coverage_percent).to_2dp()),
            ),
            None => (None, None),
        };
        Row {
            site,
            pattern: pattern_name,
            distribution: distribution.to_file_string(),
            category,
            category_value,
            found: stats.total_features_found,
            missed: stats.total_features_missed,
            percentage_found: stats.feature_found_percentage().to_2dp(),
            area_found: two_dp(stats.total_area_found),
            area_missed: two_dp(stats.total_area_missed),
            percentage_area_found: stats.area_found_percentage().to_2dp(),
            trench_layouts_tested: stats.total_layouts_tested,
            trenches_deployed: stats.total_trenches_deployed,
            mean_trenches_per_layout: two_dp(stats.mean_trenches_per_layout()),
            features_detected_per_trench: four_dp(stats.features_detected_per_trench()),
            excavated_feature_area_per_trench_m2: four_dp(stats.excavated_feature_area_per_trench()),
            detected_runs: stats.layouts_tested_where_a_feature_found,
            probability_of_a_single_feature_detection: stats.probability_of_a_single_feature_detection().to_2dp(),
            probability_feature_recovery_ge_nominal,
            probability_area_recovery_ge_nominal,
            coefficient_of_variation_feature_recovery: four_dp(stats.coefficient_of_variation_feature_recovery()),
            coefficient_of_variation_area_recovery: four_dp(stats.coefficient_of_variation_area_recovery()),
            mean_run_percentage_found: stats.mean_run_feature_percentage().to_2dp(),
            sd_run_percentage_found: two_dp(stats.sd_run_feature_percentage()),
            p05_run_percentage_found: stats.run_feature_percentage(0.05).to_2dp(),
            p25_run_percentage_found: stats.run_feature_percentage(0.25).to_2dp(),
            p50_run_percentage_found: stats.run_feature_percentage(0.5).to_2dp(),
            p75_run_percentage_found: stats.run_feature_percentage(0.75).to_2dp(),
            p95_run_percentage_found: stats.run_feature_percentage(0.95).to_2dp(),
            mean_run_percentage_area_found: stats.mean_run_area_percentage().to_2dp(),
            sd_run_percentage_area_found: two_dp(stats.sd_run_area_percentage()),
            p05_run_percentage_area_found: stats.run_area_percentage(0.05).to_2dp(),
            p25_run_percentage_area_found: stats.run_area_percentage(0.25).to_2dp(),
            p50_run_percentage_area_found: stats.run_area_percentage(0.5).to_2dp(),
            p75_run_percentage_area_found: stats.run_area_percentage(0.75).to_2dp(),
            p95_run_percentage_area_found: stats.run_area_percentage(0.95).to_2dp(),
        }
    }
}

pub fn write_to_csv(
    output_path: &str,
    config: &TrenchConfig,
    distribution: &Distribution,
    all_stats_map: StatsMap,
) -> Result<()> {
    let mut wtr = Writer::from_path(output_path)?;
    for ((site, category, category_value), stats) in all_stats_map.0 {
        wtr.serialize(Row::from_stats(
            site,
            category,
            category_value,
            &stats,
            distribution,
            config.pattern_name.as_str(),
        ))?;
    }
    Ok(())
}
