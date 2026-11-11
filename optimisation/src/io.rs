use anyhow::{anyhow, Result};
use fs_err::File;
use geo::{Area, BooleanOps, Geometry, MultiPolygon, OpType, Polygon};
use geojson::{Feature, GeoJson};
use std::io::BufReader;
use std::time::Instant;

use trenching_optimisation::{ArchFeature, TestLocation};

fn read_single_features_geojson(site_name: String, loe_i: String) -> Result<GeoJson> {
    let file = File::open(format!(
        "../data/grouped_by_loe/{}/{}/features.geojson",
        site_name, loe_i
    ))?;
    let reader = BufReader::new(file);
    let gj: GeoJson = serde_json::from_reader(reader)?;
    Ok(gj)
}

fn read_single_loe_feature(site_name: String, loe_i: String) -> Result<Polygon> {
    let file = File::open(format!(
        "../data/grouped_by_loe/{}/{}/loe.geojson",
        site_name, loe_i
    ))?;
    let reader = BufReader::new(file);
    let feature: Feature = serde_json::from_reader(reader)?;
    match feature.clone().geometry {
        Some(geometry) => {
            let geo_geometry = Geometry::try_from(geometry).unwrap_or_else(|_| {
                panic!(
                    "Failed to convert GeoJSON geometry to Geometry: {}",
                    feature
                )
            });
            match geo_geometry {
                Geometry::Polygon(polygon) => Ok(polygon),
                _ => Err(anyhow!("Geometry {} is not a polygon", feature)),
            }
        }
        None => Err(anyhow!("No geometry found in LOE file")),
    }
}

pub fn read_all_test_location_data(selected_layer: Option<&str>) -> Result<Vec<TestLocation>> {
    let now = Instant::now();
    let mut test_locations = Vec::new();

    let sites_location_counts = [
        ("A355_BeaconsfieldEasternReliefRoad", 3),
        ("Galloper", 1),
        ("Heathrow", 5),
        ("Stansted", 17),
        ("wingerworth", 2),
    ];
    for (site, location_count) in sites_location_counts.iter() {
        for i in 0..*location_count {
            let limit_of_excavation = read_single_loe_feature(site.to_string(), i.to_string())?;
            let features = read_single_features_geojson(site.to_string(), i.to_string())?;
            match process_geojson(&features, selected_layer, &limit_of_excavation) {
                Some(arch_features) => {
                    test_locations.push(TestLocation {
                        site_name: site.to_string(),
                        limit_of_excavation,
                        features: arch_features,
                    });
                }
                None => {
                    // println!("Unable to make polygons for site: {} location: {}", site, i);
                }
            }
        }
    }
    println!("Reading files took: {:?}", now.elapsed());
    Ok(test_locations)
}

fn process_geojson(
    gj: &GeoJson,
    selected_layer: Option<&str>,
    limit_of_excavation: &Polygon,
) -> Option<Vec<ArchFeature>> {
    match *gj {
        GeoJson::FeatureCollection(ref collection) => {
            let mut features: Vec<ArchFeature> = Vec::new();

            for feature in &collection.features {
                if let Some(layer) = selected_layer {
                    if feature.property("Layer").unwrap() != layer {
                        continue;
                    }
                }

                if let Some(ref geom) = feature.geometry {
                    let geo_geometry = Geometry::try_from(geom.clone()).unwrap_or_else(|_| {
                        panic!(
                            "Failed to convert GeoJSON geometry to Geometry: {}",
                            feature
                        )
                    });
                    match geo_geometry {
                        Geometry::Polygon(polygon) => {
                            let multipolygon = MultiPolygon(vec![polygon.clone()]);
                            push_to_features(
                                &mut features,
                                multipolygon,
                                feature,
                                limit_of_excavation,
                            );
                        }
                        Geometry::MultiPolygon(multipolygon) => {
                            push_to_features(
                                &mut features,
                                multipolygon.clone(),
                                feature,
                                limit_of_excavation,
                            );
                        }
                        _ => {
                            panic!("Non Polygon GeoJSON not supported: {}", feature);
                        }
                    }
                }
            }

            if features.is_empty() {
                None
            } else {
                Some(features)
            }
        }
        _ => panic!("Non FeatureCollection GeoJSON not supported: {}", gj),
    }
}

fn push_to_features(
    features: &mut Vec<ArchFeature>,
    multipolygon: MultiPolygon<f64>,
    feature: &Feature,
    limit_of_excavation: &Polygon<f64>,
) {
    let period = get_property_as_string(feature, "Period");
    let feature_type = get_property_as_string(feature, "Type");
    let monument = get_property_as_string(feature, "MONUMENT");

    let clipped_multipolygon = clip_to_loe(&multipolygon, limit_of_excavation);
    let area = clipped_multipolygon.unsigned_area();

    if area == 0.0 {
        println!(
            "WARNING: Feature {} skipped as no intersection with LOE",
            feature
        );
        return;
    }

    features.push(ArchFeature {
        multipolygon,
        period,
        feature_type,
        monument,
        area,
    });
}

fn get_property_as_string(feature: &Feature, property_name: &str) -> String {
    feature
        .property(property_name)
        .map(|v| match v {
            serde_json::Value::String(s) => s.clone(),
            other => other.to_string(),
        })
        .unwrap_or_else(|| "Unknown".to_string())
}

fn clip_to_loe(
    multipolygon: &MultiPolygon<f64>,
    limit_of_excavation: &Polygon<f64>,
) -> MultiPolygon<f64> {
    multipolygon.boolean_op(limit_of_excavation, OpType::Intersection)
}

#[cfg(test)]
mod tests {
    use super::*;
    use geo::{polygon, MultiPolygon};

    #[test]
    fn test_get_property_as_string() {
        let feature = Feature {
            properties: Some(
                [("Period".to_string(), "Iron Age".into())]
                    .iter()
                    .cloned()
                    .collect(),
            ),
            ..Default::default()
        };
        assert_eq!(
            get_property_as_string(&feature, "Period"),
            "Iron Age".to_string()
        );
        assert_eq!(
            get_property_as_string(&feature, "NonExistentProperty"),
            "Unknown".to_string()
        );
    }

    #[test]
    fn test_clip_to_loe() {
        let loe = polygon![
            (x: 0.0, y: 0.0),
            (x: 10.0, y: 0.0),
            (x: 10.0, y: 10.0),
            (x: 0.0, y: 10.0),
            (x: 0.0, y: 0.0)
        ];
        let feature = MultiPolygon(vec![polygon![
            (x: 5.0, y: 5.0),
            (x: 15.0, y: 5.0),
            (x: 15.0, y: 15.0),
            (x: 5.0, y: 15.0),
            (x: 5.0, y: 5.0)
        ]]);
        let clipped = clip_to_loe(&feature, &loe);
        assert_eq!(clipped.unsigned_area(), 25.0);
    }
}
