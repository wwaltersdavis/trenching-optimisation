use std::collections::{HashMap, HashSet};

pub struct TestedFeature {
    pub period: String,
    pub feature_type: String,
    pub monument: String,
    pub area: f64,
    pub excavated_area: f64,
}

impl TestedFeature {
    pub fn iter_categories(&self) -> impl Iterator<Item = (&str, &str)> {
        vec![
            ("Period", self.period.as_str()),
            ("Type", self.feature_type.as_str()),
            ("Monument", self.monument.as_str()),
        ]
        .into_iter()
    }
}

pub struct TestedLayout {
    pub trenches_deployed: u64,
    pub tested_features: Vec<TestedFeature>,
}

pub struct TestedSite {
    pub site_name: String,
    pub tested_layouts: Vec<TestedLayout>,
    pub total_trenches_deployed: u32,
    pub feature_categories: HashSet<(String, String)>, // List of feature categories for this site
}

pub struct Stats {
    pub total_features_found: u32,
    pub total_features_missed: u32,
    pub total_area_found: f64,
    pub total_area_missed: f64,
    pub total_layouts_tested: u32,
    pub total_trenches_deployed: u32,
}

impl Stats {
    pub fn new() -> Self {
        Stats {
            total_features_found: 0,
            total_features_missed: 0,
            total_area_found: 0.0,
            total_area_missed: 0.0,
            total_layouts_tested: 0,
            total_trenches_deployed: 0,
        }
    }
}

// Key: (site, category, category_value)
pub struct StatsMap<'a>(pub HashMap<(&'a str, &'a str, &'a str), Stats>);

impl<'a> StatsMap<'a> {
    pub fn new() -> Self {
        StatsMap(HashMap::new())
    }

    pub fn add_from_feature(
        &mut self,
        site: &'a str,
        category: &'a str,
        category_value: &'a str,
        feature: &TestedFeature,
    ) {
        let key = (site, category, category_value);
        let stats = self.0.entry(key).or_insert_with(Stats::new);

        stats.total_features_found += if feature.excavated_area > 0.0 { 1 } else { 0 };
        stats.total_features_missed += if feature.excavated_area == 0.0 { 1 } else { 0 };
        stats.total_area_found += feature.excavated_area;
        stats.total_area_missed += feature.area - feature.excavated_area;
    }

    pub fn add_from_site(
        &mut self,
        site: &'a str,
        category: &'a str,
        category_value: &'a str,
        tested_site: &TestedSite,
    ) {
        let key = (site, category, category_value);
        let stats = self.0.entry(key).or_insert_with(Stats::new);

        stats.total_layouts_tested += tested_site.tested_layouts.len() as u32;
        stats.total_trenches_deployed += tested_site.total_trenches_deployed;
    }
}

pub fn get_stats_from_sites<'a>(tested_sites: &'a [TestedSite]) -> StatsMap<'a> {
    // Key: (site, category, category_value), Value: Stats
    let mut stats_map = StatsMap::new();

    for site in tested_sites {
        for layout in site.tested_layouts.iter() {
            for feature in layout.tested_features.iter() {
                for (category, category_value) in feature.iter_categories() {
                    stats_map.add_from_feature(&site.site_name, category, category_value, feature);
                    stats_map.add_from_feature("All", category, category_value, feature);
                }
            }
        }
        // Add site-level stats
        for (category, category_value) in site.feature_categories.iter() {
            stats_map.add_from_site(&site.site_name, category, category_value, site);
            stats_map.add_from_site("All", category, category_value, site);
        }
    }
    stats_map
}
