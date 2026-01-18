use anyhow::{anyhow, Result};
use fs_err::File;
use geo::{coord, LineString, Polygon};
use geojson::{Feature, GeoJson, Geometry, Value};
use std::io::BufReader;
use std::time::Instant;

use trenching_optimisation::TestLocation;

pub fn read_single_test_location_data(
    site_name: String,
    loe_i: String,
    selected_layer: Option<&str>,
) -> Result<TestLocation> {
    let now = Instant::now();
    let limit_of_excavation = read_single_loe_feature(site_name.clone(), loe_i.clone())?;
    let gj = read_single_features_geojson(site_name.clone(), loe_i.clone())?;
    match process_geojson(&gj, selected_layer) {
        Some(features) => {
            println!("Reading files took: {:?}", now.elapsed());
            Ok(TestLocation {
                limit_of_excavation,
                features,
            })
        }
        None => Err(anyhow!(
            "No {:?} at site: {} location: {}",
            selected_layer,
            site_name,
            loe_i
        )),
    }
}

fn read_single_features_geojson(site_name: String, loe_i: String) -> Result<GeoJson> {
    let file = File::open(format!(
        "../data/grouped_by_loe/{}/{}/features.geojson",
        site_name, loe_i
    ))?;
    let reader = BufReader::new(file);
    let gj: GeoJson = serde_json::from_reader(reader)?;
    Ok(gj)
}

fn get_site_outline_of_loe(polygon: Vec<Vec<Vec<f64>>>) -> Polygon {
    if polygon.len() > 1 {
        println!("Warning: more than one polygon found for LOE");
    }
    let poly_exterior = polygon[0]
        .iter()
        .map(|c| {
            coord! { x: c[0], y: c[1] }
        })
        .collect();
    Polygon::new(LineString(poly_exterior), vec![])
}

fn read_single_loe_feature(site_name: String, loe_i: String) -> Result<Polygon> {
    let file = File::open(format!(
        "../data/grouped_by_loe/{}/{}/loe.geojson",
        site_name, loe_i
    ))?;
    let reader = BufReader::new(file);
    let feature: Feature = serde_json::from_reader(reader)?;
    match feature.geometry {
        Some(geometry) => match geometry.value {
            Value::Polygon(polygon) => Ok(get_site_outline_of_loe(polygon)),
            _ => Err(anyhow!("Geometry is not a polygon")),
        },
        // Ok(geometry),
        None => Err(anyhow!("No geometry found in LOE file")),
    }
}

pub fn read_all_test_location_data(selected_layer: Option<&str>) -> Result<Vec<TestLocation>> {
    let now = Instant::now();
    let mut test_locations = Vec::new();

    let sites_location_counts = [
        ("Stansted", 17),
        // ("Heathrow", 5),
        ("A355_BeaconsfieldEasternReliefRoad", 3),
        ("_NDR__", 22),
        ("wingerworth", 2),
    ];
    for (site, location_count) in sites_location_counts.iter() {
        for i in 0..*location_count {
            let limit_of_excavation = read_single_loe_feature(site.to_string(), i.to_string())?;
            let features = read_single_features_geojson(site.to_string(), i.to_string())?;
            match process_geojson(&features, selected_layer) {
                Some(polygons) => {
                    test_locations.push(TestLocation {
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

fn process_geojson(gj: &GeoJson, selected_layer: Option<&str>) -> Option<Vec<Polygon<f64>>> {
    match *gj {
        GeoJson::FeatureCollection(ref collection) => {
            let mut polygons = Vec::new();
            for feature in &collection.features {
                // Skip features that don't match the selected layer
                if let Some(layer) = selected_layer {
                    if feature.property("Layer").unwrap() != layer {
                        continue;
                    }
                }
                if let Some(ref geom) = feature.geometry {
                    if let Some(poly) = geometry_to_polygon(geom) {
                        polygons.push(poly);
                    } else {
                        println!("No polygon found");
                    }
                }
            }
            if polygons.is_empty() {
                None
            } else {
                Some(polygons)
            }
        }
        _ => {
            println!("Non FeatureCollection GeoJSON not supported");
            None
        }
    }
}

// Process GeoJSON geometries
fn geometry_to_polygon(geom: &Geometry) -> Option<Polygon<f64>> {
    match geom.value {
        Value::Polygon(ref polygon) => {
            let poly_exterior = polygon[0]
                .iter()
                .map(|c| {
                    coord! { x: c[0], y: c[1] }
                })
                .collect();
            Some(Polygon::new(LineString(poly_exterior), vec![]))
        }
        _ => {
            // TODO: update this placeholder for other geometry types
            println!("Matched some other geometry");
            None
        }
    }
}
