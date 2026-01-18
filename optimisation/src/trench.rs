use geo::{
    coord, Area, BooleanOps, Centroid, EuclideanDistance, LineString, MultiPolygon, Point, Polygon,
    Rotate, Translate,
};
use rayon::prelude::*;
use trenching_optimisation::array::{Configuration, PatternRotationAxis};
use trenching_optimisation::{
    Degree, Distribution, Percentage, Rectangle, Structure, TrenchConfig, TrenchLayout,
};

pub fn create_layouts(
    config: &TrenchConfig,
    limit_of_excavation: Polygon,
) -> Option<Vec<TrenchLayout>> {
    // exclude holes as removed in preprocessing
    let centroid = limit_of_excavation.centroid().unwrap();
    let max_distance_from_centroid = get_max_distance_from_centroid(centroid, &limit_of_excavation);

    match config.distribution {
        Distribution::Spacing(spacing) => {
            Some(get_layouts_from_spacing(
                &limit_of_excavation,
                *config,
                max_distance_from_centroid,
                centroid,
                spacing,
            ))
        }
        Distribution::Coverage(coverage) => {
            get_layouts_from_coverage(
                &limit_of_excavation,
                *config,
                max_distance_from_centroid,
                centroid,
                coverage,
            )
        }
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
            let spacing =
                ((rectangle.width * rectangle.length) / (coverage.percentage_as_decimal())).sqrt();
            if array_config.separated {
                spacing / 2.0
            } else {
                spacing
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
    let trench_centroid = centroid.translate(x_offset as f64 * spacing, y_offset as f64 * spacing);
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

    if array_config.separated & is_alternate_point {
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
                let distance = centroid.euclidean_distance(&p);
                if distance > max_distance_from_centroid {
                    distance
                } else {
                    max_distance_from_centroid
                }
            });
    max_distance_from_centroid
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
        .map(|rotation| {
            let trench_pattern = trenches.rotate_around_point(rotation as f64, centroid);

            // cut trench to site outline
            let intersection =
                limit_of_excavation_as_multi.boolean_op(&trench_pattern, geo::OpType::Intersection);

            let _percentage_coverage =
                calculate_coverage(&intersection, &limit_of_excavation_as_multi);

            TrenchLayout(intersection)
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
    let mut intersection =
        limit_of_excavation.boolean_op(&current_trench_pattern, geo::OpType::Intersection);
    let mut current_coverage = calculate_coverage(&intersection, limit_of_excavation);

    // println!("Target coverage: {}", target_coverage);

    while (iteration <= 10) && !check_coverage(current_coverage, target_coverage) {
        // println!("\nCurrent iteration: {}", iteration);
        // println!("Current spacing: {}", current_spacing);
        // println!("Current coverage: {}", current_coverage);

        // adjust current_spacing to get closer to target_coverage
        let adjustment_factor = 0.82_f64.powf(iteration as f64);
        let error = (target_coverage - current_coverage) / target_coverage;
        if error < 0.0 {
            current_spacing *= 1.0 + -error * adjustment_factor;
        } else {
            current_spacing /= 1.0 + error * adjustment_factor;
        }

        // check spacing is not too small
        if spacing_smaller_than_minimum(&current_spacing, &config.minimum_spacing) {
            // println!("Spacing too small");
            return None;
        }

        current_trench_pattern = get_layout_from_spacing(
            *config,
            *max_distance_from_centroid,
            centroid,
            current_spacing,
        )
        .rotate_around_point(rotation as f64, centroid);

        intersection =
            limit_of_excavation.boolean_op(&current_trench_pattern, geo::OpType::Intersection);

        current_coverage = calculate_coverage(&intersection, limit_of_excavation);
        iteration += 1;
    }

    if check_coverage(current_coverage, target_coverage) {
        // println!("Target coverage hit");
        Some(TrenchLayout(intersection))
    } else {
        // println!("Target coverage not hit");
        None
    }
}
