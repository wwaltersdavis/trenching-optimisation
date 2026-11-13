use geo::{
    coord, Area, BooleanOps, Centroid, Distance, Euclidean, Intersects, LineString, MultiPolygon,
    Point, Polygon, Rotate, Translate,
};
use rayon::prelude::*;
use std::panic;
use trenching_optimisation::array::{Configuration, PatternRotationAxis};
use trenching_optimisation::{
    Degree, Distribution, Percentage, Rectangle, Structure, TrenchConfig, TrenchLayout,
};

pub fn create_layouts(
    config: &TrenchConfig,
    limit_of_excavation: Polygon,
) -> Option<Vec<TrenchLayout>> {
    let centroid = limit_of_excavation.centroid().unwrap();
    let max_distance_from_centroid = get_max_distance_from_centroid(centroid, &limit_of_excavation);

    match config.distribution {
        Distribution::Spacing(spacing) => Some(get_layouts_from_spacing(
            &limit_of_excavation,
            *config,
            max_distance_from_centroid,
            centroid,
            spacing,
        )),
        Distribution::Coverage(coverage) => get_layouts_from_coverage(
            &limit_of_excavation,
            *config,
            max_distance_from_centroid,
            centroid,
            coverage,
        ),
    }
}

fn get_size_of_grid(max_distance_from_centroid: &f64, spacing: &f64) -> i32 {
    (max_distance_from_centroid / spacing).floor() as i32
}

fn spacing_smaller_than_minimum(spacing: &f64, minimum_spacing: &f64) -> bool {
    spacing < minimum_spacing
}

fn estimate_spacing(config: &TrenchConfig, coverage: &Percentage) -> f64 {
    match config.structure {
        Structure::Parallel(line) => line.width / (coverage.percentage_as_decimal()),
        Structure::Array(rectangle, array_config) => {
            if array_config.separated {
                let y_spacing = rectangle.length + 1.0;
                (rectangle.width * rectangle.length)
                    / (coverage.percentage_as_decimal() * y_spacing * 2.0)
            } else {
                ((rectangle.width * rectangle.length) / (coverage.percentage_as_decimal())).sqrt()
            }
        }
    }
}

fn trench_of_array_coordinate(
    x_index: usize,
    y_index: usize,
    x_offset: i32,
    y_offset: i32,
    centroid: Point,
    spacing: f64,
    array_config: &Configuration,
    rectangle: Rectangle,
) -> Option<Polygon> {
    let y_spacing = if array_config.separated {
        rectangle.length + 1.0
    } else {
        spacing
    };

    let trench_centroid =
        centroid.translate(x_offset as f64 * spacing, y_offset as f64 * y_spacing);
    let is_alternate_point = (x_index + y_index).is_multiple_of(2);
    let rotation = match array_config.pattern_rotation_axis {
        PatternRotationAxis::ByCell => {
            if is_alternate_point {
                array_config.base_angle
            } else {
                array_config.alternate_angle
            }
        }
        PatternRotationAxis::ByColumn => {
            if x_index.is_multiple_of(2) {
                array_config.base_angle
            } else {
                array_config.alternate_angle
            }
        }
    };

    if array_config.separated && is_alternate_point {
        None
    } else {
        Some(plot_trench(
            trench_centroid,
            rectangle.width,
            rectangle.length,
            rotation,
        ))
    }
}

fn plot_trench(centroid: Point, width: f64, length: f64, rotation: Degree) -> Polygon<f64> {
    let trench_exterior = vec![
        coord! { x: centroid.x() - width / 2.0, y: centroid.y() - length / 2.0 },
        coord! { x: centroid.x() + width / 2.0, y: centroid.y() - length / 2.0 },
        coord! { x: centroid.x() + width / 2.0, y: centroid.y() + length / 2.0 },
        coord! { x: centroid.x() - width / 2.0, y: centroid.y() + length / 2.0 },
        coord! { x: centroid.x() - width / 2.0, y: centroid.y() - length / 2.0 },
    ];
    Polygon::new(LineString(trench_exterior), vec![]).rotate_around_point(rotation.0, centroid)
}

fn get_layouts_from_coverage(
    limit_of_excavation: &Polygon,
    config: TrenchConfig,
    max_distance_from_centroid: f64,
    centroid: Point,
    coverage: Percentage,
) -> Option<Vec<TrenchLayout>> {
    let estimated_spacing = estimate_spacing(&config, &coverage);
    if spacing_smaller_than_minimum(&estimated_spacing, &config.minimum_spacing) {
        return None;
    }
    let trenches = get_layout_from_spacing(
        config,
        max_distance_from_centroid,
        centroid,
        estimated_spacing,
    );
    // in parallel iterate over all
    let limit_of_excavation = MultiPolygon(vec![limit_of_excavation.clone()]);

    let trench_patterns: Vec<TrenchLayout> = (0..config.structure.get_rotational_symmetry())
        // let trench_patterns: Vec<TrenchLayout> = (171..172)
        .into_par_iter()
        .filter_map(|rotation| {
            adjust_trench_layout_to_coverage(
                &trenches,
                coverage.0,
                centroid,
                &limit_of_excavation,
                estimated_spacing,
                &config,
                rotation,
                &max_distance_from_centroid,
            )
        })
        .collect();
    if trench_patterns.is_empty() {
        return None;
    }
    Some(trench_patterns)
}

fn get_layouts_from_spacing(
    limit_of_excavation: &Polygon,
    config: TrenchConfig,
    max_distance_from_centroid: f64,
    centroid: Point,
    spacing: f64,
) -> Vec<TrenchLayout> {
    let trenches = get_layout_from_spacing(config, max_distance_from_centroid, centroid, spacing);
    get_rotated_trench_patterns(
        trenches,
        config.structure.get_rotational_symmetry(),
        centroid,
        limit_of_excavation,
    )
}

fn get_layout_from_spacing(
    config: TrenchConfig,
    max_distance_from_centroid: f64,
    centroid: Point,
    spacing: f64,
) -> MultiPolygon {
    let n = get_size_of_grid(&max_distance_from_centroid, &spacing);
    let x_offsets = -n..n + 1;
    let trenches = match config.structure {
        Structure::Parallel(line) => {
            x_offsets
                .into_par_iter()
                .map(|x_offset| {
                    let trench_centroid = centroid.translate(x_offset as f64 * spacing, 0.0);
                    plot_trench(
                        trench_centroid,
                        line.width,
                        max_distance_from_centroid * 2.0,
                        Degree(0.0),
                    )
                })
                .collect()
            // TODO: test performance of this vs .push() to Vec
        }
        Structure::Array(rectangle, array_config) => {
            let y_offsets = -n..n + 1;
            x_offsets
                .into_par_iter()
                .enumerate()
                .flat_map(|(x_index, x_offset)| {
                    y_offsets
                        .clone()
                        .enumerate()
                        .filter_map(move |(y_index, y_offset)| {
                            trench_of_array_coordinate(
                                x_index,
                                y_index,
                                x_offset,
                                y_offset,
                                centroid,
                                spacing,
                                &array_config,
                                rectangle,
                            )
                        })
                        .collect::<Vec<Polygon>>()
                })
                .collect::<Vec<Polygon>>()
        }
    };
    MultiPolygon(trenches)
}

fn get_max_distance_from_centroid(centroid: Point, limit_of_excavation: &Polygon) -> f64 {
    let max_distance_from_centroid =
        limit_of_excavation
            .exterior()
            .points()
            .fold(0.0, |max_distance_from_centroid, p| {
                let distance = Euclidean.distance(centroid, p);
                if distance > max_distance_from_centroid {
                    distance
                } else {
                    max_distance_from_centroid
                }
            });
    max_distance_from_centroid
}
fn safe_intersection(a: &MultiPolygon<f64>, b: &MultiPolygon<f64>) -> Option<MultiPolygon<f64>> {
    panic::catch_unwind(|| a.boolean_op(b, geo::OpType::Intersection)).ok()
}

fn calculate_coverage(
    trench_layout: &MultiPolygon<f64>,
    limit_of_excavation: &MultiPolygon<f64>,
) -> f64 {
    trench_layout.unsigned_area() / limit_of_excavation.unsigned_area() * 100.0
}

fn get_rotated_trench_patterns(
    trenches: MultiPolygon,
    rotations: i32,
    centroid: Point,
    limit_of_excavation: &Polygon,
) -> Vec<TrenchLayout> {
    let limit_of_excavation_as_multi = MultiPolygon(vec![limit_of_excavation.clone()]);

    (0..rotations)
        .into_par_iter()
        .filter_map(|rotation| {
            let trench_pattern = trenches.rotate_around_point(rotation as f64, centroid);
            let trenches_deployed = trench_pattern
                .iter()
                .filter(|polygon| polygon.intersects(limit_of_excavation))
                .count() as u64;

            // cut trench to site outline
            let Some(intersection) =
                safe_intersection(&limit_of_excavation_as_multi, &trench_pattern)
            else {
                println!(
                    "Skipped rotation {} because site intersection failed",
                    rotation
                );
                return None;
            };

            let _percentage_coverage =
                calculate_coverage(&intersection, &limit_of_excavation_as_multi);

            Some(TrenchLayout {
                layout: intersection,
                trenches_deployed,
            })
        })
        .collect()
    // TODO: return average percentage coverage
}

fn check_coverage(current_coverage: f64, target_coverage: f64) -> bool {
    current_coverage > target_coverage - 0.05 && current_coverage < target_coverage + 0.05
}

fn adjust_trench_layout_to_coverage(
    trench_pattern: &MultiPolygon,
    target_coverage: f64,
    centroid: Point,
    limit_of_excavation: &MultiPolygon,
    estimated_spacing: f64,
    config: &TrenchConfig,
    rotation: i32,
    max_distance_from_centroid: &f64,
) -> Option<TrenchLayout> {
    let mut iteration = 0;
    let mut current_spacing = estimated_spacing;
    let mut current_trench_pattern = trench_pattern.rotate_around_point(rotation as f64, centroid);
    let Some(mut intersection) = safe_intersection(limit_of_excavation, &current_trench_pattern)
    else {
        println!(
            "No trenches created for {:?}: initial site intersection failed at rotation {}",
            config.pattern_name, rotation
        );
        return None;
    };
    let mut current_coverage = calculate_coverage(&intersection, limit_of_excavation);

    // println!("Target coverage: {}", target_coverage);

    while (iteration <= 30) && !check_coverage(current_coverage, target_coverage) {
        // println!("\nCurrent iteration: {}", iteration);
        // println!("Current spacing: {}", current_spacing);
        // println!("Current coverage: {}", current_coverage);

        // adjust current_spacing to get closer to target_coverage
        let adjustment_factor = 0.90_f64.powf(iteration as f64); // Bumped this from 0.82 due to the increase in iterations.
        let error = (target_coverage - current_coverage) / target_coverage;
        if error < 0.0 {
            current_spacing *= 1.0 + -error * adjustment_factor;
        } else {
            current_spacing /= 1.0 + error * adjustment_factor;
        }

        // check spacing is not too small
        if spacing_smaller_than_minimum(&current_spacing, &config.minimum_spacing) {
            println!(
                "Spacing too small for {:?}: target {:.2} current {:.2}",
                config.pattern_name, target_coverage, current_coverage
            );
            return None;
        }

        current_trench_pattern = get_layout_from_spacing(
            *config,
            *max_distance_from_centroid,
            centroid,
            current_spacing,
        )
        .rotate_around_point(rotation as f64, centroid);

        let Some(next_intersection) =
            safe_intersection(limit_of_excavation, &current_trench_pattern)
        else {
            println!(
                "No trenches created for {:?}: site intersection failed at rotation {}, iteration {}",
                config.pattern_name,
                rotation,
                iteration
            );
            return None;
        };

        intersection = next_intersection;

        current_coverage = calculate_coverage(&intersection, limit_of_excavation);
        iteration += 1;
    }

    if check_coverage(current_coverage, target_coverage) {
        // Count the individual trenches or pits that intersect the site.
        let trenches_deployed = current_trench_pattern
            .iter()
            .filter(|polygon| polygon.intersects(limit_of_excavation))
            .count() as u64;
        Some(TrenchLayout {
            layout: intersection,
            trenches_deployed,
        })
    } else {
        println!(
            "Failed {:?}: target {:.2} achieved {:.2} after {} iterations",
            config.pattern_name, target_coverage, current_coverage, iteration
        );
        None
    }
}
