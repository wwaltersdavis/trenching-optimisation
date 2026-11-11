use geo::{Area, BooleanOps, BoundingRect, Intersects, MultiPolygon};
use std::collections::HashMap;
use trenching_optimisation::{ArchFeature, TrenchLayout};

fn excavated_overlap_area(feature: &MultiPolygon<f64>, trenches: &TrenchLayout) -> f64 {
    if !feature.intersects(&trenches.layout) {
        return 0.0;
    }

    let Some(feature_bbox) = feature.bounding_rect() else {
        return 0.0;
    };

    let mut total_overlap_area = 0.0;

    for trench_polygon in &trenches.layout {
        let Some(trench_bbox) = trench_polygon.bounding_rect() else {
            continue;
        };

        if !feature_bbox.intersects(&trench_bbox) {
            continue;
        }

        if !feature.intersects(trench_polygon) {
            continue;
        }

        let trench_as_multi = MultiPolygon(vec![trench_polygon.clone()]);

        // TODO: use fn `safe_intersection` function from trench.rs.
        let intersection_result = std::panic::catch_unwind(|| {
            feature.boolean_op(&trench_as_multi, geo::OpType::Intersection)
        });

        if let Ok(intersection) = intersection_result {
            total_overlap_area += intersection.unsigned_area();
        }
    }

    total_overlap_area
}

pub struct FeaturesStats {
    pub features_found: i32,
    pub features_missed: i32,
    pub area_found: f64,
    pub area_missed: f64,
    pub trenches_deployed: u64,
    pub category_stats: HashMap<String, (i32, i32, f64, f64)>,
}

impl FeaturesStats {
    // TODO: Remove this hacky solution with the nested category stats.
    pub fn to_category_stats(&self) -> Vec<(String, Self)> {
        self.category_stats
            .iter()
            .map(|(category, cs)| {
                (
                    category.clone(),
                    FeaturesStats {
                        features_found: cs.0,
                        features_missed: cs.1,
                        area_found: cs.2,
                        area_missed: cs.3,
                        trenches_deployed: self.trenches_deployed,
                        category_stats: HashMap::new(),
                    },
                )
            })
            .collect()
    }
}

pub fn count_features_hit_or_missed_by_category(
    features: &Vec<ArchFeature>,
    trenches: &TrenchLayout,
) -> FeaturesStats {
    let mut features_found = 0;
    let mut features_missed = 0;
    let mut area_found = 0.0;
    let mut area_missed = 0.0;
    let mut category_stats: HashMap<String, (i32, i32, f64, f64)> = HashMap::new();

    for feature in features {
        let excavated_area = excavated_overlap_area(&feature.multipolygon, trenches);

        let unexcavated_area = (feature.area - excavated_area).max(0.0);

        let found = excavated_area > 0.0;

        if found {
            features_found += 1;
        } else {
            features_missed += 1;
        }

        area_found += excavated_area;
        area_missed += unexcavated_area;

        for category in [
            format!("Period::{}", feature.period),
            format!("Type::{}", feature.feature_type),
            format!("MONUMENT::{}", feature.monument),
        ] {
            let entry = category_stats.entry(category).or_insert((0, 0, 0.0, 0.0));

            if found {
                entry.0 += 1;
            } else {
                entry.1 += 1;
            }

            entry.2 += excavated_area;
            entry.3 += unexcavated_area;
        }
    }

    FeaturesStats {
        features_found,
        features_missed,
        area_found,
        area_missed,
        trenches_deployed: trenches.trenches_deployed,
        category_stats,
    }
}
