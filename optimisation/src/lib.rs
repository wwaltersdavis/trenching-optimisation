use geo::{coord, Coord, MultiPolygon, Polygon};
use std::time::Instant;

#[derive(Debug)]
pub struct TrenchLayout(pub MultiPolygon<f64>);
// TODO: add impl for intersects to TrenchLayout

#[derive(Debug)]
pub struct TestLocation {
    pub limit_of_excavation: Polygon,
    pub features: Vec<Polygon<f64>>,
}

#[derive(Debug, Clone, Copy)]
pub struct Rectangle {
    pub width: f64,
    pub length: f64,
}

impl Rectangle {
    pub fn new(width: f64, length: f64) -> Self {
        Rectangle { width, length }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Line {
    pub width: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Degree(pub f64);

impl Degree {
    pub fn new(value: f64) -> Self {
        Degree(value)
    }
    pub fn add(&self, other: Degree) -> Self {
        Degree(self.0 + other.0)
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Percentage(pub f64);

impl Percentage {
    pub fn new_from_percentage(value: f64) -> Self {
        Percentage(value)
    }
    pub fn new_from_decimal(value: f64) -> Self {
        Percentage(value * 100.0)
    }
    pub fn percentage_as_decimal(&self) -> f64 {
        self.0 / 100.0
    }
}

pub mod array {
    use crate::Degree;
    #[derive(Debug, Clone, Copy)]
    pub enum PatternRotationAxis {
        ByCell,
        ByColumn,
    }

    #[derive(Debug, Clone, Copy)]
    pub struct Configuration {
        pub base_angle: Degree,
        pub alternate_angle: Degree,
        pub pattern_rotation_axis: PatternRotationAxis,
        pub separated: bool,
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Structure {
    Parallel(Line),
    Array(Rectangle, array::Configuration),
}

impl Structure {
    pub fn get_rotational_symmetry(self) -> i32 {
        match self {
            Structure::Parallel(_) => 180,
            Structure::Array(rectangle, array_configuration) => {
                if (rectangle.width == rectangle.length) && !array_configuration.separated {
                    90
                } else {
                    180
                }
            }
        }
    }
}

// TODO: add meters struct?

#[derive(Debug, Clone, Copy)]
pub enum Distribution {
    Spacing(f64),         // meters
    Coverage(Percentage), // percentage coverage
}

fn get_coords(rectangle: Rectangle, angle_from_verticle: Degree) -> [Coord; 4] {
    let half_width = rectangle.width / 2.0;
    let half_length = rectangle.length / 2.0;
    let angle = (90.0 - angle_from_verticle.0).to_radians();
    let w1 = half_width * angle.cos();
    let w2 = half_width * angle.sin();
    let l1 = half_length * angle.cos();
    let l2 = half_length * angle.sin();

    [
        coord! { x: l1 - w2, y: l2 + w1 },
        coord! { x: l1 + w2, y: l2 - w1 },
        coord! { x: -l1 + w2, y: -l2 - w1 },
        coord! { x: -l1 - w2, y: -l2 + w1 },
    ]
}

fn get_index_of_coord_with_max_y(coords: [Coord; 4]) -> usize {
    // TODO: calculate this when finding coords?
    let mut max_y = 0.0;
    let mut max_y_index = 0;
    // println!("{:?}", coords);
    for (i, coord) in coords.iter().enumerate() {
        if coord.y > max_y {
            max_y = coord.y;
            max_y_index = i;
        }
    }
    max_y_index
}

fn is_coord_outside_x_bounds_of_line(coord: Coord, line: geo::Line<f64>) -> bool {
    // println!("line start: {:?}", line.start);
    // println!("line end: {:?}", line.end);
    coord.x < line.start.x || coord.x > line.end.x
}

fn get_previous_and_next_index(i: usize, length: usize) -> (usize, usize) {
    let previous_i = if i == 0 { length - 1 } else { i - 1 };
    let next_i = if i == length - 1 { 0 } else { i + 1 };
    (previous_i, next_i)
}

fn find_y_differences(coords_a: [Coord; 4], coords_b: [Coord; 4], y_differences: &mut Vec<f64>) {
    let i = get_index_of_coord_with_max_y(coords_b);
    let (previous_i, next_i) = get_previous_and_next_index(i, 4);
    let line_b1 = geo::Line::new(coords_b[previous_i], coords_b[i]);
    let line_b2 = geo::Line::new(coords_b[i], coords_b[next_i]);

    for line in [line_b1, line_b2] {
        let gradient = line.slope();
        // skip vertical lines
        if gradient.is_infinite() {
            continue;
        }
        let y_intercept = line.start.y - gradient * line.start.x;
        for coord in coords_a {
            if is_coord_outside_x_bounds_of_line(coord, line) {
                continue;
            }
            y_differences.push(-(coord.y - (gradient * coord.x + y_intercept)));
        }
    }
}

fn minimum_spacing(rectangle: Rectangle, angle_1: Degree, angle_2: Degree) -> f64 {
    let coords_1 = get_coords(rectangle, angle_1);
    let coords_2 = get_coords(rectangle, angle_2);
    let mut y_differences: Vec<f64> = Vec::new();
    find_y_differences(coords_1, coords_2, &mut y_differences);
    find_y_differences(coords_2, coords_1, &mut y_differences);

    y_differences
        .iter()
        .fold(f64::NEG_INFINITY, |max, &val| max.max(val))
}

pub fn test_get_minimum_spacing(rectangle: Rectangle, angle_1: Degree, angle_2: Degree) {
    let now = Instant::now();
    let min_spacing = minimum_spacing(rectangle, angle_1, angle_2);
    println!("Finding minimum spacing took: {:?}", now.elapsed());
    println!("Minimum spacing: {:?}", min_spacing);
}

pub fn get_minimum_spacing(structure: Structure) -> f64 {
    match structure {
        Structure::Parallel(line) => line.width,
        Structure::Array(rectangle, array_configuration) => {
            let horizontal_minimum_spacing = minimum_spacing(
                rectangle,
                array_configuration.alternate_angle.add(Degree(90.0)),
                array_configuration.base_angle.add(Degree(90.0)),
            );
            match array_configuration.pattern_rotation_axis {
                array::PatternRotationAxis::ByCell => {
                    let verticle_minimum_spacing = minimum_spacing(
                        rectangle,
                        array_configuration.alternate_angle,
                        array_configuration.base_angle,
                    );
                    let diagonal_minimum_spacing = minimum_spacing(
                        rectangle,
                        array_configuration.base_angle.add(Degree(45.0)),
                        array_configuration.base_angle.add(Degree(45.0)),
                    );
                    if array_configuration.separated {
                        diagonal_minimum_spacing
                            .max(verticle_minimum_spacing / 2.0)
                            .max(horizontal_minimum_spacing / 2.0)
                    } else {
                        diagonal_minimum_spacing
                            .max(verticle_minimum_spacing)
                            .max(horizontal_minimum_spacing)
                    }
                }
                array::PatternRotationAxis::ByColumn => {
                    let verticle_minimum_spacing_a = minimum_spacing(
                        rectangle,
                        array_configuration.base_angle,
                        array_configuration.base_angle,
                    );
                    let verticle_minimum_spacing_b = minimum_spacing(
                        rectangle,
                        array_configuration.alternate_angle,
                        array_configuration.alternate_angle,
                    );
                    let diagonal_minimum_spacing = minimum_spacing(
                        rectangle,
                        array_configuration.alternate_angle.add(Degree(45.0)),
                        array_configuration.base_angle.add(Degree(45.0)),
                    );
                    if array_configuration.separated {
                        diagonal_minimum_spacing
                            .max(verticle_minimum_spacing_a / 2.0)
                            .max(verticle_minimum_spacing_b / 2.0)
                            .max(horizontal_minimum_spacing / 2.0)
                    } else {
                        diagonal_minimum_spacing
                            .max(verticle_minimum_spacing_a)
                            .max(verticle_minimum_spacing_b)
                            .max(horizontal_minimum_spacing)
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum PatternName {
    Continuous,
    ParallelArray,
    StandardGrid,
    TestPits,
    Herringbone,
    CentreLine,
}

#[derive(Debug, Clone, Copy)]
pub struct TrenchConfig {
    // TODO: add shifts in x/y
    pub structure: Structure,
    pub distribution: Distribution,
    pub minimum_spacing: f64,
    pub pattern_name: PatternName,
}

impl TrenchConfig {
    // TODO: add centre_line
    // TODO: add validate_spacing
    pub fn validate_spacing(minimum_spacing: f64, distribution: Distribution) {
        if let Distribution::Spacing(spacing) = distribution {
            assert!(minimum_spacing < spacing, "Spacing too small");
        }
    }
    pub fn continuous(width: f64, distribution: Distribution) -> Self {
        let structure = Structure::Parallel(Line { width });
        let minimum_spacing = get_minimum_spacing(structure);
        Self::validate_spacing(minimum_spacing, distribution);
        TrenchConfig {
            structure,
            distribution,
            minimum_spacing,
            pattern_name: PatternName::Continuous,
        }
    }
    pub fn parallel_array(width: f64, length: f64, distribution: Distribution) -> Self {
        let structure = Structure::Array(
            Rectangle { width, length },
            array::Configuration {
                base_angle: Degree::new(0.0),
                alternate_angle: Degree::new(0.0),
                pattern_rotation_axis: array::PatternRotationAxis::ByCell,
                separated: true,
            },
        );
        let minimum_spacing = get_minimum_spacing(structure);
        Self::validate_spacing(minimum_spacing, distribution);
        TrenchConfig {
            structure,
            distribution,
            minimum_spacing,
            pattern_name: PatternName::ParallelArray,
        }
    }
    pub fn standard_grid(width: f64, length: f64, distribution: Distribution) -> Self {
        let structure = Structure::Array(
            Rectangle { width, length },
            array::Configuration {
                base_angle: Degree::new(0.0),
                alternate_angle: Degree::new(90.0),
                pattern_rotation_axis: array::PatternRotationAxis::ByCell,
                separated: false,
            },
        );
        let minimum_spacing = get_minimum_spacing(structure);
        Self::validate_spacing(minimum_spacing, distribution);
        TrenchConfig {
            structure,
            distribution,
            minimum_spacing,
            pattern_name: PatternName::StandardGrid,
        }
    }
    pub fn test_pits(width: f64, distribution: Distribution) -> Self {
        let structure = Structure::Array(
            Rectangle {
                width,
                length: width,
            },
            array::Configuration {
                base_angle: Degree::new(0.0),
                alternate_angle: Degree::new(0.0),
                pattern_rotation_axis: array::PatternRotationAxis::ByCell,
                separated: false,
            },
        );
        let minimum_spacing = get_minimum_spacing(structure);
        Self::validate_spacing(minimum_spacing, distribution);
        TrenchConfig {
            structure,
            distribution,
            minimum_spacing,
            pattern_name: PatternName::TestPits,
        }
    }
    pub fn herringbone(width: f64, length: f64, distribution: Distribution) -> Self {
        let structure = Structure::Array(
            Rectangle { width, length },
            array::Configuration {
                base_angle: Degree::new(45.0),
                alternate_angle: Degree::new(315.0),
                pattern_rotation_axis: array::PatternRotationAxis::ByColumn,
                separated: false,
            },
        );
        let minimum_spacing = get_minimum_spacing(structure);
        Self::validate_spacing(minimum_spacing, distribution);
        TrenchConfig {
            structure,
            distribution,
            minimum_spacing,
            pattern_name: PatternName::Herringbone,
        }
    }
    // pub fn centre_line_of_width(width: f64) -> Self {
    //     TrenchConfig {
    //         structure: Structure::Parallel(Line { width }),
    //         distribution: Distribution::Spacing(0.0),
    //     }
    // }

    // pub fn centre_line_coverage(coverage: f64) -> Self {
    //     TrenchConfig {
    //         structure: Structure::Parallel(Line { width: 0.0 }),
    //         distribution: Distribution::Coverage(coverage),
    //     }
    // }
}
