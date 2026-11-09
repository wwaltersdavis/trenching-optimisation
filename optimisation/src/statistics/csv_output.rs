use crate::{statistics::DetectionStats, TrenchConfig};
use anyhow::Result;
use csv::Writer;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct Row<'a> {
    site: &'a str,
    pattern: &'a str,
    distribution: &'a str,
    category: &'a str,
    category_value: &'a str,
    found: i64,
    missed: i64,
    percentage_found: String,
    area_found: String,
    area_missed: String,
    percentage_area_found: String,
    trench_patterns_tested: u64,
    trenches_deployed: u64,
    mean_trenches_per_pattern: String,
    features_detected_per_trench: String,
    excavated_feature_area_per_trench_m2: String,
    eligible_runs: u64,
    detected_runs: u64,
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
    area_eligible_runs: u64,
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
        config: &'a TrenchConfig,
        distribution_label: &'a str,
        category: &'a str,
        category_value: &'a str,
        stats: &DetectionStats,
        total_trenches: u64,
    ) -> Self {
        Row {
            site,
            pattern: config.pattern_name.as_str(),
            distribution: distribution_label,
            category,
            category_value,
            found: stats.found,
            missed: stats.missed,
            percentage_found: format!("{:.2}", stats.feature_percentage()),
            area_found: format!("{:.2}", stats.area_found),
            area_missed: format!("{:.2}", stats.area_missed),
            percentage_area_found: format!("{:.2}", stats.area_percentage()),
            trench_patterns_tested: total_trenches,
            trenches_deployed: stats.trenches_deployed,
            mean_trenches_per_pattern: format!("{:.2}", stats.mean_trenches_per_pattern()),
            features_detected_per_trench: format!("{:.4}", stats.features_detected_per_trench()),
            excavated_feature_area_per_trench_m2: format!("{:.4}", stats.excavated_feature_area_per_trench()),
            eligible_runs: stats.eligible_runs,
            detected_runs: stats.detected_runs,
            probability_detection: format!("{:.2}", stats.probability_detection()),
            probability_feature_recovery_ge_nominal: format!("{:.2}", stats.probability_feature_recovery_ge_nominal()),
            probability_area_recovery_ge_nominal: format!("{:.2}", stats.probability_area_recovery_ge_nominal()),
            coefficient_of_variation_feature_recovery: format!("{:.4}", stats.coefficient_of_variation_feature_recovery()),
            coefficient_of_variation_area_recovery: format!("{:.4}", stats.coefficient_of_variation_area_recovery()),
            mean_run_percentage_found: format!("{:.2}", stats.mean_run_feature_percentage()),
            sd_run_percentage_found: format!("{:.2}", stats.sd_run_feature_percentage()),
            p05_run_percentage_found: format!("{:.2}", stats.run_feature_percentage(0.05)),
            p25_run_percentage_found: format!("{:.2}", stats.run_feature_percentage(0.25)),
            p50_run_percentage_found: format!("{:.2}", stats.run_feature_percentage(0.5)),
            p75_run_percentage_found: format!("{:.2}", stats.run_feature_percentage(0.75)),
            p95_run_percentage_found: format!("{:.2}", stats.run_feature_percentage(0.95)),
            area_eligible_runs: stats.area_eligible_runs,
            mean_run_percentage_area_found: format!("{:.2}", stats.mean_run_area_percentage()),
            sd_run_percentage_area_found: format!("{:.2}", stats.sd_run_area_percentage()),
            p05_run_percentage_area_found: format!("{:.2}", stats.run_area_percentage(0.05)),
            p25_run_percentage_area_found: format!("{:.2}", stats.run_area_percentage(0.25)),
            p50_run_percentage_area_found: format!("{:.2}", stats.run_area_percentage(0.5)),
            p75_run_percentage_area_found: format!("{:.2}", stats.run_area_percentage(0.75)),
            p95_run_percentage_area_found: format!("{:.2}", stats.run_area_percentage(0.95)),
        }
    }
}

pub fn write_to_csv(
    output_path: &str,
    config: &TrenchConfig,
    distribution_label: &str,
    total_trenches: u64,
    overall_stats: DetectionStats,
    site_stats: HashMap<String, DetectionStats>,
    category_stats: HashMap<String, DetectionStats>,
    site_category_stats: HashMap<(String, String), DetectionStats>,
) -> Result<()> {
    let mut wtr = Writer::from_path(output_path)?;
    wtr.serialize(Row::from_stats(
        "ALL_SITES",
        config,
        distribution_label,
        "TOTAL",
        "ALL",
        &overall_stats,
        total_trenches,
    ))?;

    for ((site, category), stats) in site_category_stats {
        let parts: Vec<&str> = category.splitn(2, "::").collect();
        wtr.serialize(Row::from_stats(
            &site,
            config,
            distribution_label,
            parts[0],
            parts[1],
            &stats,
            total_trenches,
        ))?;
    }

    for (site, stats) in site_stats {
        wtr.serialize(Row::from_stats(
            &site,
            config,
            distribution_label,
            "TOTAL",
            "ALL",
            &stats,
            total_trenches,
        ))?;
    }

    for (category, stats) in category_stats {
        let parts: Vec<&str> = category.splitn(2, "::").collect();
        wtr.serialize(Row::from_stats(
            "ALL_SITES",
            config,
            distribution_label,
            parts[0],
            parts[1],
            &stats,
            total_trenches,
        ))?;
    }
    Ok(())
}
