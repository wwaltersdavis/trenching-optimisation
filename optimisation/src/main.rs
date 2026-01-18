mod io;
mod intersects;
mod run;
mod trench;

use trenching_optimisation::{Distribution, Percentage, TrenchConfig};

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
        run::all_loes(config, selected_layer);
    }
}
