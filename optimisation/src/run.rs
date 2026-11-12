use crate::{
    intersects, io,
    statistics::{write_to_csv, StatsMap},
    trench,
};
use anyhow::Result;
use rayon::prelude::*;
use std::time::Instant;

use trenching_optimisation::TrenchConfig;

pub fn all_loes(config: &TrenchConfig, selected_layer: Option<&str>) -> Result<()> {
    println!("\nRunning {:?} on all LOEs", config.pattern_name);
    let test_locations = io::read_all_test_location_data(selected_layer)?;

    let now = Instant::now();

    let mut total_trench_creation_time = 0;
    let mut total_testing_time = 0;

    let mut all_stats_map: StatsMap = StatsMap::new();

    for test_location in &test_locations {
        let trenches_time = Instant::now();
        let trenches = trench::create_layouts(config, test_location.limit_of_excavation.clone());
        total_trench_creation_time += trenches_time.elapsed().as_millis();

        let testing_time = Instant::now();

        match trenches {
            Some(trenches) => {
                let stats_maps: Vec<StatsMap> = trenches
                    .into_par_iter()
                    .map(|trench| {
                        intersects::test_features_for_layout(
                            &test_location.features,
                            &trench,
                            test_location.site_name.as_str(),
                        )
                    })
                    .collect();
                all_stats_map.add_from_maps(&stats_maps);
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

    // println!(
    //     "Total features found: {}, total features missed: {}, percentage found: {:.2}%",
    //     overall_stats.found,
    //     overall_stats.missed,
    //     overall_stats.feature_percentage()
    // );

    // println!("Total trench patterns tested: {}", total_trenches);

    let output_path = format!(
        "../data/results_{:?}_{}.csv",
        config.pattern_name,
        config.distribution.to_file_string()
    );

    write_to_csv(
        output_path.as_str(),
        config,
        &config.distribution,
        all_stats_map,
    )?;

    println!("Saved CSV: {}", output_path);
    println!("Testing took: {:?}", now.elapsed());
    Ok(())
}

// // Add the tested layouts to the overall stats.
// let (layouts_tested, total_trenches_deployed) = &trenches
//     .iter()
//     .fold((0, 0), |(layouts, trenches), layout| {
//         (layouts + 1, trenches + layout.trenches_deployed as u32)
//     });
// for (category, category_value) in test_location.unique_categories() {
//     all_stats_map.add_trenches_deployed(
//         &test_location.site_name,
//         category,
//         category_value,
//         *total_trenches_deployed,
//         *layouts_tested,
//     );
// }
