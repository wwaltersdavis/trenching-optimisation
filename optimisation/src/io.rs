use anyhow::{anyhow, Result};
use fs_err::File;
use geo::{coord, Area, BooleanOps, Contains, LineString, MultiPolygon, Point, Polygon};
use geojson::{Feature, GeoJson, Geometry, Value};
use std::io::BufReader;
use std::time::Instant;

use trenching_optimisation::{ArchFeature, TestLocation};

pub fn read_single_test_location_data(
    site_name: String,
    loe_i: String,
    selected_layer: Option<&str>,
) -> Result<TestLocation> {
    let now = Instant::now();
    let limit_of_excavation = read_single_loe_feature(site_name.clone(), loe_i.clone())?;
    let gj = read_single_features_geojson(site_name.clone(), loe_i.clone())?;
    match process_geojson(&gj, selected_layer, &limit_of_excavation) {
        Some(features) => {
            println!("Reading files took: {:?}", now.elapsed());
            Ok(TestLocation {
                site_name,
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

fn process_geojson(
    gj: &GeoJson,
    selected_layer: Option<&str>,
    limit_of_excavation: &Polygon<f64>,
) -> Option<Vec<ArchFeature>> {
    match *gj {
        GeoJson::FeatureCollection(ref collection) => {
            let mut features = Vec::new();
            let loe = MultiPolygon(vec![limit_of_excavation.clone()]);

            for feature in &collection.features {
                if let Some(layer) = selected_layer {
                    if feature.property("Layer").unwrap() != layer {
                        continue;
                    }
                }

                if let Some(ref geom) = feature.geometry {
                    if let Some(poly) = geometry_to_multipolygon(geom) {
                        let clipped = poly.boolean_op(&loe, geo::OpType::Intersection);
                        if clipped.0.is_empty() {
                            continue;
                        }
                        let area = clipped.unsigned_area();
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

                        features.push(ArchFeature {
                            polygon: clipped,
                            period,
                            feature_type,
                            monument,
                            area,
                        });
                    }
                }
            }

            if features.is_empty() {
                None
            } else {
                Some(features)
            }
        }
        _ => {
            println!("Non FeatureCollection GeoJSON not supported");
            None
        }
    }
}

// Convert a coordinate ring into a closed LineString.
fn ring_to_linestring(ring: &[Vec<f64>]) -> LineString<f64> {
    let mut coordinates: Vec<_> = ring.iter().map(|c| coord! { x: c[0], y: c[1] }).collect();

    if coordinates.first() != coordinates.last() {
        if let Some(first) = coordinates.first().copied() {
            coordinates.push(first);
        }
    }

    LineString(coordinates)
}

// Signed area is used to distinguish exterior rings from interior holes.
// The converted shapefile data can contain several exterior rings within
// one GeoJSON Polygon value.
fn signed_ring_area(ring: &[Vec<f64>]) -> f64 {
    if ring.len() < 3 {
        return 0.0;
    }

    let mut area = 0.0;

    for pair in ring.windows(2) {
        area += pair[0][0] * pair[1][1] - pair[1][0] * pair[0][1];
    }

    if ring.first() != ring.last() {
        let first = &ring[0];
        let last = &ring[ring.len() - 1];
        area += last[0] * first[1] - first[0] * last[1];
    }

    area / 2.0
}

fn polygon_coordinates_to_multipolygon(rings: &[Vec<Vec<f64>>]) -> Option<MultiPolygon<f64>> {
    if rings.is_empty() {
        return None;
    }

    let exterior_sign = signed_ring_area(&rings[0]).signum();

    if exterior_sign == 0.0 {
        return None;
    }

    let mut exteriors = Vec::new();
    let mut holes = Vec::new();

    for ring in rings {
        let area = signed_ring_area(ring);

        if area == 0.0 {
            continue;
        }

        let line = ring_to_linestring(ring);

        if area.signum() == exterior_sign {
            exteriors.push(line);
        } else {
            holes.push(line);
        }
    }

    if exteriors.is_empty() {
        return None;
    }

    let mut assigned_holes: Vec<Vec<LineString<f64>>> = vec![Vec::new(); exteriors.len()];
    let mut unassigned_rings = Vec::new();

    for hole in holes {
        let Some(first_coordinate) = hole.0.first() else {
            continue;
        };

        let point = Point::new(first_coordinate.x, first_coordinate.y);

        let containing_exterior = exteriors
            .iter()
            .position(|exterior| Polygon::new(exterior.clone(), vec![]).contains(&point));

        if let Some(index) = containing_exterior {
            assigned_holes[index].push(hole);
        } else {
            // Preserve an unassigned ring rather than silently discarding it.
            unassigned_rings.push(hole);
        }
    }

    let mut polygons: Vec<Polygon<f64>> = exteriors
        .into_iter()
        .enumerate()
        .map(|(index, exterior)| Polygon::new(exterior, assigned_holes[index].clone()))
        .collect();

    for ring in unassigned_rings {
        polygons.push(Polygon::new(ring, vec![]));
    }

    Some(MultiPolygon(polygons))
}

fn geometry_to_multipolygon(geom: &Geometry) -> Option<MultiPolygon<f64>> {
    match &geom.value {
        Value::Polygon(rings) => polygon_coordinates_to_multipolygon(rings),

        Value::MultiPolygon(polygons) => {
            let mut output = Vec::new();

            for rings in polygons {
                if let Some(multipolygon) = polygon_coordinates_to_multipolygon(rings) {
                    output.extend(multipolygon.0);
                }
            }

            if output.is_empty() {
                None
            } else {
                Some(MultiPolygon(output))
            }
        }

        _ => {
            println!("Unsupported non-polygon feature geometry");
            None
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    fn square(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Vec<Vec<f64>> {
        vec![
            vec![min_x, min_y],
            vec![max_x, min_y],
            vec![max_x, max_y],
            vec![min_x, max_y],
            vec![min_x, min_y],
        ]
    }

    #[test]
    fn preserves_multiple_exterior_parts() {
        let geometry = Geometry::new(Value::Polygon(vec![
            square(0.0, 0.0, 10.0, 10.0),
            square(20.0, 0.0, 30.0, 10.0),
        ]));

        let feature = geometry_to_multipolygon(&geometry).unwrap();

        assert_eq!(feature.0.len(), 2);
        assert!((feature.unsigned_area() - 200.0).abs() < 0.000001);
    }

    #[test]
    fn preserves_interior_holes() {
        let outer = square(0.0, 0.0, 10.0, 10.0);

        let hole = vec![
            vec![2.0, 2.0],
            vec![2.0, 4.0],
            vec![4.0, 4.0],
            vec![4.0, 2.0],
            vec![2.0, 2.0],
        ];

        let geometry = Geometry::new(Value::Polygon(vec![outer, hole]));
        let feature = geometry_to_multipolygon(&geometry).unwrap();

        assert_eq!(feature.0.len(), 1);
        assert_eq!(feature.0[0].interiors().len(), 1);
        assert!((feature.unsigned_area() - 96.0).abs() < 0.000001);
    }

    #[test]
    fn clips_feature_area_to_loe() {
        let geometry = Geometry::new(Value::Polygon(vec![square(0.0, 0.0, 10.0, 10.0)]));

        let feature = geometry_to_multipolygon(&geometry).unwrap();

        let loe = MultiPolygon(vec![Polygon::new(
            ring_to_linestring(&square(0.0, 0.0, 5.0, 10.0)),
            vec![],
        )]);

        let clipped = feature.boolean_op(&loe, geo::OpType::Intersection);

        assert!((clipped.unsigned_area() - 50.0).abs() < 0.000001);
    }
}
