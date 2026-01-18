use geo::{Intersects, Polygon};
use trenching_optimisation::TrenchLayout;

fn test(feature: &Polygon<f64>, trenches: &TrenchLayout) -> bool {
    feature.intersects(&trenches.0)
}

pub fn count_features_hit_or_missed(
    features: &Vec<Polygon<f64>>,
    trenches: &TrenchLayout,
) -> (i32, i32) {
    let mut features_found = 0;
    let mut features_missed = 0;
    for feature in features {
        if test(feature, trenches) {
            features_found += 1;
        } else {
            features_missed += 1;
        }
    }
    (features_found, features_missed)
}