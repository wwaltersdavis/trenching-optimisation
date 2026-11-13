use crate::statistics::StatsMap;
use geo::{Area, BooleanOps, BoundingRect, Intersects, MultiPolygon};
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

pub fn test_features_for_layout<'a>(
    features: &'a Vec<ArchFeature>,
    trenches: &TrenchLayout,
    site: &'a str,
) -> StatsMap<'a> {
    let mut stats_map = StatsMap::new();

    for feature in features {
        let excavated_area = excavated_overlap_area(&feature.multipolygon, trenches);
        for (category, category_value) in feature.iter_categories() {
            stats_map.add_from_feature(
                site,
                category,
                category_value,
                excavated_area > 0.0,
                excavated_area,
                feature.area - excavated_area,
                trenches.trenches_deployed as u32,
            );
        }
    }
    for stats in stats_map.0.values_mut() {
        stats.update_percentages();
    }
    stats_map
}
