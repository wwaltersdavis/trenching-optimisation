use crate::{
    intersects, io,
    statistics::{write_to_csv, DetectionStats},
    trench,
};
use anyhow::Result;
use rayon::prelude::*;
use std::collections::HashMap;
use std::time::Instant;

use trenching_optimisation::{Distribution, TrenchConfig};

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

    write_to_csv(
        output_path.as_str(),
        config,
        distribution_label.as_str(),
        total_trenches,
        overall_stats,
        site_stats,
        category_stats,
        site_category_stats,
    )?;

    println!("Saved CSV: {}", output_path);
    println!("Testing took: {:?}", now.elapsed());
    Ok(())
}
