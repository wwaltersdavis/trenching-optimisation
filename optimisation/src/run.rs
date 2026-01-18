use rayon::prelude::*;
use std::time::Instant;

use trenching_optimisation::TrenchConfig;

use super::io;
use super::intersects;
use super::trench;

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
                                intersects::count_features_hit_or_missed(&test_location.features, &trench);
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

    let mut total_found = 0;
    let mut total_missed = 0;
    let mut total_trenches = 0;

    let mut total_trench_creation_time = 0;
    let mut total_testing_time = 0;

    for test_location in test_locations {
        let trenches_time = Instant::now();
        let trenches = trench::create_layouts(config, test_location.limit_of_excavation);
        total_trench_creation_time += trenches_time.elapsed().as_millis();
        let testing_time = Instant::now();
        match trenches {
            Some(trenches) => {
                let found_or_missed: Vec<(i32, i32)> = trenches
                    .into_par_iter()
                    .map(|trench| {
                        let (features_found, features_missed) =
                            intersects::count_features_hit_or_missed(&test_location.features, &trench);
                        (features_found, features_missed)
                    })
                    .collect();
                for (found, missed) in found_or_missed {
                    total_found += found;
                    total_missed += missed;
                    total_trenches += 1;
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

    let percentage_found = total_found as f64 / (total_found + total_missed) as f64 * 100.0;

    // println!(
    //     "Testing {}m wide {:?} trenches",
    //     config.width, config.layout
    // );
    println!(
        "Total features found: {}, total features missed: {}, percentage found: {:.2}%",
        total_found, total_missed, percentage_found
    );
    println!("Total trench patterns tested: {}", total_trenches);
    println!("Testing took: {:?}", now.elapsed());
}
