mod intersects;
mod trench;

use geo::Polygon;
use rayon::prelude::*;
use std::time::Instant;

use trenching_optimisation::{
    read_all_test_location_data, read_single_test_location_data, Distribution, Percentage,
    TrenchConfig, TrenchLayout,
};
// fn main() {
//     let new_rectangle = Rectangle::new(2.0, 30.0);
//     test_get_minimum_spacing(new_rectangle, Degree(20.0), Degree(88.0));
// }
fn main() {
    let continous_spacing = TrenchConfig::continuous(2.0, Distribution::Spacing(20.0));
    let parallel_array_spacing =
        TrenchConfig::parallel_array(2.0, 30.0, Distribution::Spacing(20.0));
    let standard_grid_spacing = TrenchConfig::standard_grid(2.0, 30.0, Distribution::Spacing(20.0));
    // let grid_with_wide_trenches_spacing =
    //     TrenchConfig::standard_grid(4.0, 30.0, Distribution::Spacing(40.0));
    // let grid_wtth_short_trenches_spacing =
    //     TrenchConfig::standard_grid(2.0, 20.0, Distribution::Spacing(30.0));
    // let test_pits_spacing = TrenchConfig::test_pits(1.0, Distribution::Spacing(20.0));

    let continous_coverage = TrenchConfig::continuous(
        2.0,
        Distribution::Coverage(Percentage::new_from_percentage(5.0)),
    );
    let parallel_array_coverage = TrenchConfig::parallel_array(
        2.0,
        30.0,
        Distribution::Coverage(Percentage::new_from_percentage(5.0)),
    );
    let standard_grid_coverage = TrenchConfig::standard_grid(
        2.0,
        30.0,
        Distribution::Coverage(Percentage::new_from_percentage(5.0)),
    );
    // let grid_with_wide_trenches_coverage =
    //     TrenchConfig::standard_grid(4.0, 30.0, Distribution::Coverage(Percentage::new_from_percentage(5.0)));
    // let grid_wtth_short_trenches_coverage =
    //     TrenchConfig::standard_grid(2.0, 20.0, Distribution::Coverage(Percentage::new_from_percentage(5.0)));
    // let test_pits_coverage = TrenchConfig::test_pits(1.0, Distribution::Coverage(Percentage::new_from_percentage(5.0)));

    let selected_layer = Some("Middle Bronze Age");
    // let selected_layer: Option<&str> = None;

    for config in [
        continous_spacing,
        parallel_array_spacing,
        standard_grid_spacing,
        // grid_with_wide_trenches_spacing,
        // grid_wtth_short_trenches_spacing,
        // test_pits_spacing,
        continous_coverage,
        parallel_array_coverage,
        standard_grid_coverage,
        // grid_with_wide_trenches_coverage,
        // grid_wtth_short_trenches_coverage,
        // test_pits_coverage,
    ]
    .iter()
    {
        // run_on_single_loe(
        //     &config,
        //     "Stansted".to_string(),
        //     "0".to_string(),
        //     selected_layer,
        // );
        run_on_all_loes(&config, selected_layer);
    }
}

fn count_features_hit_or_missed(
    features: &Vec<Polygon<f64>>,
    trenches: &TrenchLayout,
) -> (i32, i32) {
    let mut features_found = 0;
    let mut features_missed = 0;
    for feature in features {
        if intersects::test(feature, trenches) {
            features_found += 1;
        } else {
            features_missed += 1;
        }
    }
    (features_found, features_missed)
}

fn run_on_single_loe(
    config: &TrenchConfig,
    site_name: String,
    loe_i: String,
    selected_layer: Option<&str>,
) {
    // println!("\nRunning {:?} on single LOE", config.layout);
    let test_location = read_single_test_location_data(site_name, loe_i, selected_layer);
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
                                count_features_hit_or_missed(&test_location.features, &trench);
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

fn run_on_all_loes(config: &TrenchConfig, selected_layer: Option<&str>) {
    // println!("\nRunning {:?} on all LOEs", config.layout);
    println!("\nRunning on all LOEs");
    let test_locations = read_all_test_location_data(selected_layer).unwrap();

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
                            count_features_hit_or_missed(&test_location.features, &trench);
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
    println!("Creating trenches took: {:?}s", total_trench_creation_time as f64 / 1000.0);
    println!("Testing trenches took: {:?}s", total_testing_time as f64 / 1000.0);

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
