use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;

use trenching_optimisation::{Distribution, TrenchConfig};

use super::intersects;
use super::io;
use super::trench;

#[derive(Debug, Default, Clone)]
struct DetectionStats {
    found: i64,
    missed: i64,
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
    fn add_run(
        &mut self,
        found: i32,
        missed: i32,
        area_found: f64,
        area_missed: f64,
        trenches_deployed: u64,
        nominal_coverage_percent: f64,
    ) {
        self.found += found as i64;
        self.missed += missed as i64;
        self.area_found += area_found;
        self.area_missed += area_missed;
        self.trenches_deployed += trenches_deployed;

        let feature_total = found + missed;

        if feature_total > 0 {
            let percentage = found as f64 / feature_total as f64 * 100.0;

            self.eligible_runs += 1;

            if found > 0 {
                self.detected_runs += 1;
            }

            if nominal_coverage_percent.is_finite() && percentage >= nominal_coverage_percent {
                self.nominal_feature_recovery_runs += 1;
            }

            self.feature_percentage_sum += percentage;
            self.feature_percentage_sum_squares += percentage * percentage;
            self.feature_percentage_values.push(percentage);
        }

        let area_total = area_found + area_missed;

        if area_total > 0.0 {
            let percentage = area_found / area_total * 100.0;

            self.area_eligible_runs += 1;

            if nominal_coverage_percent.is_finite() && percentage >= nominal_coverage_percent {
                self.nominal_area_recovery_runs += 1;
            }

            self.area_percentage_sum += percentage;
            self.area_percentage_sum_squares += percentage * percentage;
            self.area_percentage_values.push(percentage);
        }
    }

    fn feature_percentage(&self) -> f64 {
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

    fn p05_run_feature_percentage(&self) -> f64 {
        percentile(&self.feature_percentage_values, 0.05)
    }

    fn p25_run_feature_percentage(&self) -> f64 {
        percentile(&self.feature_percentage_values, 0.25)
    }

    fn p50_run_feature_percentage(&self) -> f64 {
        percentile(&self.feature_percentage_values, 0.50)
    }

    fn p75_run_feature_percentage(&self) -> f64 {
        percentile(&self.feature_percentage_values, 0.75)
    }

    fn p95_run_feature_percentage(&self) -> f64 {
        percentile(&self.feature_percentage_values, 0.95)
    }

    fn p05_run_area_percentage(&self) -> f64 {
        percentile(&self.area_percentage_values, 0.05)
    }

    fn p25_run_area_percentage(&self) -> f64 {
        percentile(&self.area_percentage_values, 0.25)
    }

    fn p50_run_area_percentage(&self) -> f64 {
        percentile(&self.area_percentage_values, 0.50)
    }

    fn p75_run_area_percentage(&self) -> f64 {
        percentile(&self.area_percentage_values, 0.75)
    }

    fn p95_run_area_percentage(&self) -> f64 {
        percentile(&self.area_percentage_values, 0.95)
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

fn write_stats_row(
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
        format!("{:.2}", stats.p05_run_feature_percentage()),
        format!("{:.2}", stats.p25_run_feature_percentage()),
        format!("{:.2}", stats.p50_run_feature_percentage()),
        format!("{:.2}", stats.p75_run_feature_percentage()),
        format!("{:.2}", stats.p95_run_feature_percentage()),
        stats.area_eligible_runs.to_string(),
        format!("{:.2}", stats.mean_run_area_percentage()),
        format!("{:.2}", stats.sd_run_area_percentage()),
        format!("{:.2}", stats.p05_run_area_percentage()),
        format!("{:.2}", stats.p25_run_area_percentage()),
        format!("{:.2}", stats.p50_run_area_percentage()),
        format!("{:.2}", stats.p75_run_area_percentage()),
        format!("{:.2}", stats.p95_run_area_percentage()),
    ];

    writeln!(file, "{}", row.join(",")).unwrap();
}

pub fn single_loe(
    config: &TrenchConfig,
    site_name: String,
    loe_i: String,
    selected_layer: Option<&str>,
) {
    println!("\nRunning {:?} on single LOE", config.pattern_name);
    let test_location = io::read_single_test_location_data(site_name, loe_i, selected_layer);
    match test_location {
        Ok(test_location) => {
            let now = Instant::now();
            let trenches = trench::create_layouts(config, test_location.limit_of_excavation);
            println!("Creating trenches took: {:?}", now.elapsed());
            match trenches {
                Some(trenches) => {
                    let now = Instant::now();
                    let _: Vec<(i32, i32)> = trenches
                        .into_par_iter()
                        .map(|trench| {
                            let (features_found, features_missed) =
                                intersects::count_features_hit_or_missed(
                                    &test_location.features,
                                    &trench,
                                );
                            (features_found, features_missed)
                        })
                        .collect();
                    println!("Calculating features hit took: {:?}", now.elapsed());
                }
                None => {
                    println!("No trenches created for LOE");
                }
            }
        }
        Err(e) => {
            println!("{:?}", e);
        }
    }
}

pub fn all_loes(config: &TrenchConfig, selected_layer: Option<&str>) {
    println!("\nRunning {:?} on all LOEs", config.pattern_name);
    let test_locations = io::read_all_test_location_data(selected_layer).unwrap();

    let now = Instant::now();

    let nominal_coverage_percent = match config.distribution {
        Distribution::Coverage(value) => value.0,
        Distribution::Spacing(_) => f64::NAN,
    };

    let mut overall_stats = DetectionStats::default();
    let mut total_trenches: u64 = 0;
    let mut category_stats: HashMap<String, DetectionStats> = HashMap::new();
    let mut site_stats: HashMap<String, DetectionStats> = HashMap::new();
    let mut site_category_stats: HashMap<(String, String), DetectionStats> = HashMap::new();
    let mut total_trench_creation_time = 0;
    let mut total_testing_time = 0;

    for test_location in test_locations {
        let trenches_time = Instant::now();
        let trenches = trench::create_layouts(config, test_location.limit_of_excavation);
        total_trench_creation_time += trenches_time.elapsed().as_millis();

        let testing_time = Instant::now();

        match trenches {
            Some(trenches) => {
                let found_or_missed: Vec<(
                    i32,
                    i32,
                    f64,
                    f64,
                    u64,
                    HashMap<String, (i32, i32, f64, f64)>,
                )> = trenches
                    .into_par_iter()
                    .map(|trench| {
                        let trenches_deployed = trench.1;

                        let (features_found, features_missed, area_found, area_missed, categories) =
                            intersects::count_features_hit_or_missed_by_category(
                                &test_location.features,
                                &trench,
                            );

                        (
                            features_found,
                            features_missed,
                            area_found,
                            area_missed,
                            trenches_deployed,
                            categories,
                        )
                    })
                    .collect();

                for (found, missed, area_found, area_missed, trenches_deployed, categories) in
                    found_or_missed
                {
                    overall_stats.add_run(
                        found,
                        missed,
                        area_found,
                        area_missed,
                        trenches_deployed,
                        nominal_coverage_percent,
                    );
                    total_trenches += 1;

                    site_stats
                        .entry(test_location.site_name.clone())
                        .or_default()
                        .add_run(
                            found,
                            missed,
                            area_found,
                            area_missed,
                            trenches_deployed,
                            nominal_coverage_percent,
                        );

                    for (category, (cat_found, cat_missed, cat_area_found, cat_area_missed)) in
                        categories
                    {
                        category_stats.entry(category.clone()).or_default().add_run(
                            cat_found,
                            cat_missed,
                            cat_area_found,
                            cat_area_missed,
                            trenches_deployed,
                            nominal_coverage_percent,
                        );

                        site_category_stats
                            .entry((test_location.site_name.clone(), category))
                            .or_default()
                            .add_run(
                                cat_found,
                                cat_missed,
                                cat_area_found,
                                cat_area_missed,
                                trenches_deployed,
                                nominal_coverage_percent,
                            );
                    }
                }
            }
            None => {
                println!("No trenches created for LOE");
                continue;
            }
        }

        total_testing_time += testing_time.elapsed().as_millis();
    }

    println!(
        "Creating trenches took: {:?}s",
        total_trench_creation_time as f64 / 1000.0
    );
    println!(
        "Testing trenches took: {:?}s",
        total_testing_time as f64 / 1000.0
    );

    println!(
        "Total features found: {}, total features missed: {}, percentage found: {:.2}%",
        overall_stats.found,
        overall_stats.missed,
        overall_stats.feature_percentage()
    );

    println!("Total trench patterns tested: {}", total_trenches);

    let distribution_label = match config.distribution {
        trenching_optimisation::Distribution::Spacing(value) => {
            format!("spacing_{}m", value)
        }
        trenching_optimisation::Distribution::Coverage(value) => {
            format!("coverage_{}pct", value.0)
        }
    };

    let output_path = format!(
        "../data/results_{:?}_{}.csv",
        config.pattern_name, distribution_label
    );

    let mut file = fs_err::File::create(&output_path).unwrap();

    writeln!(
        file,
        "{}",
        [
            "site",
            "pattern",
            "distribution",
            "category",
            "category_value",
            "found",
            "missed",
            "percentage_found",
            "area_found",
            "area_missed",
            "percentage_area_found",
            "trench_patterns_tested",
            "trenches_deployed",
            "mean_trenches_per_pattern",
            "features_detected_per_trench",
            "excavated_feature_area_per_trench_m2",
            "eligible_runs",
            "detected_runs",
            "probability_detection",
            "probability_feature_recovery_ge_nominal",
            "probability_area_recovery_ge_nominal",
            "coefficient_of_variation_feature_recovery",
            "coefficient_of_variation_area_recovery",
            "mean_run_percentage_found",
            "sd_run_percentage_found",
            "p05_run_percentage_found",
            "p25_run_percentage_found",
            "p50_run_percentage_found",
            "p75_run_percentage_found",
            "p95_run_percentage_found",
            "area_eligible_runs",
            "mean_run_percentage_area_found",
            "sd_run_percentage_area_found",
            "p05_run_percentage_area_found",
            "p25_run_percentage_area_found",
            "p50_run_percentage_area_found",
            "p75_run_percentage_area_found",
            "p95_run_percentage_area_found",
        ]
        .join(",")
    )
    .unwrap();

    write_stats_row(
        &mut file,
        "ALL_SITES",
        config,
        &distribution_label,
        "TOTAL",
        "ALL",
        &overall_stats,
        total_trenches,
    );

    for ((site, category), stats) in site_category_stats {
        let parts: Vec<&str> = category.splitn(2, "::").collect();

        write_stats_row(
            &mut file,
            &site,
            config,
            &distribution_label,
            parts[0],
            parts[1],
            &stats,
            total_trenches,
        );
    }

    for (site, stats) in site_stats {
        write_stats_row(
            &mut file,
            &site,
            config,
            &distribution_label,
            "TOTAL",
            "ALL",
            &stats,
            total_trenches,
        );
    }

    for (category, stats) in category_stats {
        let parts: Vec<&str> = category.splitn(2, "::").collect();

        write_stats_row(
            &mut file,
            "ALL_SITES",
            config,
            &distribution_label,
            parts[0],
            parts[1],
            &stats,
            total_trenches,
        );
    }

    println!("Saved CSV: {}", output_path);
    println!("Testing took: {:?}", now.elapsed());
}
