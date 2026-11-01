mod intersects;
mod io;
mod run;
mod trench;

use trenching_optimisation::{Distribution, Percentage, TrenchConfig};

fn main() {
    let selected_layer: Option<&str> = None;

    let coverage_percentages = [10.0];

    for coverage in coverage_percentages {
        let continuous = TrenchConfig::continuous(
            2.0,
            Distribution::Coverage(Percentage::new_from_percentage(coverage)),
        );

        let parallel_array = TrenchConfig::parallel_array(
            2.0,
            30.0,
            Distribution::Coverage(Percentage::new_from_percentage(coverage)),
        );

        let standard_grid = TrenchConfig::standard_grid(
            2.0,
            30.0,
            Distribution::Coverage(Percentage::new_from_percentage(coverage)),
        );

        let test_pits = TrenchConfig::test_pits(
            1.0,
            Distribution::Coverage(Percentage::new_from_percentage(coverage)),
        );

        for config in [continuous, parallel_array, standard_grid, test_pits].iter() {
            run::all_loes(config, selected_layer);
        }
    }
}
