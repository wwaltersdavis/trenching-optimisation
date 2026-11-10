use anyhow::{anyhow, Result};
use fs_err::File;
use geo::{Geometry, Polygon};
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
            match process_geojson(&features, selected_layer) {
                Some(polygons) => {
                    test_locations.push(TestLocation {
                        site_name: site.to_string(),
                        limit_of_excavation,
                        features: polygons,
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

fn process_geojson(gj: &GeoJson, selected_layer: Option<&str>) -> Option<Vec<ArchFeature>> {
    match *gj {
        GeoJson::FeatureCollection(ref collection) => {
            let mut features = Vec::new();

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
                            let period = feature
                                .property("Period")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "Unknown".to_string());

                            let feature_type = feature
                                .property("Type")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "Unknown".to_string());

                            let monument = feature
                                .property("MONUMENT")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "Unknown".to_string());

                            let area_text = feature
                                .property("POLY_AREA")
                                .map(|v| v.to_string())
                                .unwrap_or_else(|| "0".to_string());

                            let area = area_text
                                .replace("\"Float(Some(", "")
                                .replace("))\"", "")
                                .replace("\"", "")
                                .parse::<f64>()
                                .unwrap_or(0.0);

                            features.push(ArchFeature {
                                polygon,
                                period,
                                feature_type,
                                monument,
                                area,
                            });
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
