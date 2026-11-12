use crate::statistics::StatsMap;
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

        // TODO: use fn `safe_intersection` function from trench.rs.
        let intersection_result = std::panic::catch_unwind(|| {
            trench_polygon.boolean_op(feature, geo::OpType::Intersection)
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

pub fn test_features_for_layout<'a>(
    features: &'a Vec<ArchFeature>,
    trenches: &TrenchLayout,
    site: &'a str,
) -> StatsMap<'a> {
    let mut stats_map = StatsMap::new();

    for feature in features {
        let excavated_area = excavated_overlap_area(&feature.multipolygon, trenches);
        stats_map.add_from_feature(
            site,
            &feature.feature_type,
            &feature.monument,
            excavated_area > 0.0,
            excavated_area,
            feature.area - excavated_area,
            trenches.trenches_deployed as u32,
        );
    }
    for stats in stats_map.0.values_mut() {
        stats.update_percentages();
    }
    stats_map
}

// pub fn test_features_for_layout<'a>(
//     features: &'a Vec<ArchFeature>,
//     trenches: &TrenchLayout,
// ) -> TestedLayout<'a> {
//     let mut tested_layout = TestedLayout::new_from_layout(trenches);

//     for feature in features {
//         let excavated_area = excavated_overlap_area(&feature.multipolygon, trenches);
//         tested_layout.tested_features.push(TestedFeature {
//             period: &feature.period,
//             feature_type: &feature.feature_type,
//             monument: &feature.monument,
//             area: feature.area,
//             excavated_area,
//         });
//     }
//     tested_layout
// }
