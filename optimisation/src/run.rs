use crate::{
    detection_stats::{write_stats_row, DetectionStats},
    intersects, io, trench,
};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::io::Write;
use std::time::Instant;

use trenching_optimisation::{Distribution, TrenchConfig};

pub fn single_loe(
    config: &TrenchConfig,
    site_name: String,
    loe_i: String,
    selected_layer: Option<&str>,
) -> Result<()> {
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
    Ok(())
}

pub fn all_loes(config: &TrenchConfig, selected_layer: Option<&str>) -> Result<()> {
    println!("\nRunning {:?} on all LOEs", config.pattern_name);
    let test_locations = io::read_all_test_location_data(selected_layer)?;

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
                let all_features_stats: Vec<intersects::FeaturesStats> = trenches
                    .into_par_iter()
                    .map(|trench| {
                        intersects::count_features_hit_or_missed_by_category(
                            &test_location.features,
                            &trench,
                        )
                    })
                    .collect();

                for features_stats in all_features_stats {
                    overall_stats.add_run(&features_stats, nominal_coverage_percent);
                    total_trenches += 1;

                    site_stats
                        .entry(test_location.site_name.clone())
                        .or_default()
                        .add_run(&features_stats, nominal_coverage_percent);

                    for (category, stats) in features_stats.to_category_stats() {
                        category_stats
                            .entry(category.clone())
                            .or_default()
                            .add_run(&stats, nominal_coverage_percent);

                        site_category_stats
                            .entry((test_location.site_name.clone(), category))
                            .or_default()
                            .add_run(&stats, nominal_coverage_percent);
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
            format!("spacing_{}m", value).to_string()
        }
        trenching_optimisation::Distribution::Coverage(value) => {
            format!("coverage_{}pct", value.0).to_string()
        }
    };

    let output_path = format!(
        "../data/results_{:?}_{}.csv",
        config.pattern_name, distribution_label
    );

    let mut file = fs_err::File::create(&output_path)?;

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
    )?;

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
    Ok(())
}
