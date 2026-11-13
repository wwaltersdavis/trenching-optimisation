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
    trench_patterns_tested: u32,
    trenches_deployed: u32,
    mean_trenches_per_pattern: String,
    features_detected_per_trench: String,
    excavated_feature_area_per_trench_m2: String,
    detected_runs: u32,
    probability_detection: String,
    probability_feature_recovery_ge_nominal: String,
    probability_area_recovery_ge_nominal: String,
    coefficient_of_variation_feature_recovery: String,
    coefficient_of_variation_area_recovery: String,
    mean_run_percentage_found: String,
    sd_run_percentage_found: String,
    p05_run_percentage_found: String,
    p25_run_percentage_found: String,
    p50_run_percentage_found: String,
    p75_run_percentage_found: String,
    p95_run_percentage_found: String,
    // area_eligible_runs: u64,
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
        Row {
            site,
            pattern: pattern_name,
            distribution: distribution.to_file_string(),
            category,
            category_value,
            found: stats.total_features_found,
            missed: stats.total_features_missed,
            percentage_found: two_dp(stats.feature_found_percentage()),
            area_found: two_dp(stats.total_area_found),
            area_missed: two_dp(stats.total_area_missed),
            percentage_area_found: two_dp(stats.area_found_percentage()),
            trench_patterns_tested: stats.total_layouts_tested,
            trenches_deployed: stats.total_trenches_deployed,
            mean_trenches_per_pattern: two_dp(stats.mean_trenches_per_pattern()),
            features_detected_per_trench: four_dp(stats.features_detected_per_trench()),
            excavated_feature_area_per_trench_m2: four_dp(stats.excavated_feature_area_per_trench()),
            detected_runs: stats.layouts_tested_where_a_feature_found,
            probability_detection: two_dp(stats.probability_detection()),
            probability_feature_recovery_ge_nominal: two_dp(stats.probability_feature_recovery_ge_nominal(distribution.nominal_coverage_percent())),
            probability_area_recovery_ge_nominal: two_dp(stats.probability_area_recovery_ge_nominal(distribution.nominal_coverage_percent())),
            coefficient_of_variation_feature_recovery: four_dp(stats.coefficient_of_variation_feature_recovery()),
            coefficient_of_variation_area_recovery: four_dp(stats.coefficient_of_variation_area_recovery()),
            mean_run_percentage_found: two_dp(stats.mean_run_feature_percentage()),
            sd_run_percentage_found: two_dp(stats.sd_run_feature_percentage()),
            p05_run_percentage_found: two_dp(stats.run_feature_percentage(0.05)),
            p25_run_percentage_found: two_dp(stats.run_feature_percentage(0.25)),
            p50_run_percentage_found: two_dp(stats.run_feature_percentage(0.5)),
            p75_run_percentage_found: two_dp(stats.run_feature_percentage(0.75)),
            p95_run_percentage_found: two_dp(stats.run_feature_percentage(0.95)),
            mean_run_percentage_area_found: two_dp(stats.mean_run_area_percentage()),
            sd_run_percentage_area_found: two_dp(stats.sd_run_area_percentage()),
            p05_run_percentage_area_found: two_dp(stats.run_area_percentage(0.05)),
            p25_run_percentage_area_found: two_dp(stats.run_area_percentage(0.25)),
            p50_run_percentage_area_found: two_dp(stats.run_area_percentage(0.5)),
            p75_run_percentage_area_found: two_dp(stats.run_area_percentage(0.75)),
            p95_run_percentage_area_found: two_dp(stats.run_area_percentage(0.95)),
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
