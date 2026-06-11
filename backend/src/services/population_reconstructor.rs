use crate::models::{Building, PopulationGrid, PopulationResult, Zone};
use crate::services::interpolation_service::{
    self, InterpolationPoint,
};
use serde_json::Value;
use std::collections::HashMap;
use std::f64::consts::PI;

pub const LITERATURE_BASELINE_DENSITY: &[(&str, f64, f64, &str)] = &[
    ("palace", 120.0, 0.9, "《考工记·匠人营国》"),
    ("residential_high", 200.0, 0.85, "《汉书·地理志》"),
    ("residential_medium", 150.0, 0.8, "《洛阳伽蓝记》"),
    ("residential_low", 100.0, 0.75, "《宋会要辑稿》"),
    ("commercial", 180.0, 0.7, "《东京梦华录》"),
    ("industrial", 80.0, 0.65, "《天工开物》"),
    ("administrative", 90.0, 0.8, "《唐六典》"),
    ("religious", 60.0, 0.6, "《武经总要》前集"),
    ("open_space", 40.0, 0.5, "《三辅黄图》"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataCompleteness {
    High,
    Medium,
    Low,
    VeryLow,
}

pub struct ReconstructionConfig {
    pub use_literature_baseline: bool,
    pub use_building_density: bool,
    pub interpolation_method: String,
    pub bandwidth_km: Option<f64>,
    pub grid_resolution: usize,
}

impl Default for ReconstructionConfig {
    fn default() -> Self {
        ReconstructionConfig {
            use_literature_baseline: true,
            use_building_density: true,
            interpolation_method: "kde".to_string(),
            bandwidth_km: None,
            grid_resolution: 16,
        }
    }
}

pub struct PopulationReconstructor {
    config: ReconstructionConfig,
}

impl PopulationReconstructor {
    pub fn new(config: ReconstructionConfig) -> Self {
        PopulationReconstructor { config }
    }

    pub fn assess_data_completeness(
        &self,
        zones: &[Zone],
        buildings: &[Building],
        has_total_pop: bool,
    ) -> (DataCompleteness, f64) {
        let zone_coverage = if zones.is_empty() {
            0.0
        } else {
            zones.iter().filter(|z| z.area_sq_km > 0.0).count() as f64 / zones.len() as f64
        };

        let building_coverage = if buildings.is_empty() {
            0.0
        } else {
            let with_rooms = buildings.iter().filter(|b| b.num_rooms > 0).count() as f64;
            (with_rooms / buildings.len() as f64).min(1.0)
        };

        let total_pop_score = if has_total_pop { 1.0 } else { 0.0 };

        let score = (zone_coverage * 0.4 + building_coverage * 0.4 + total_pop_score * 0.2).min(1.0);

        let level = if score >= 0.8 {
            DataCompleteness::High
        } else if score >= 0.5 {
            DataCompleteness::Medium
        } else if score >= 0.2 {
            DataCompleteness::Low
        } else {
            DataCompleteness::VeryLow
        };

        (level, score)
    }

    pub fn literature_baseline_density(&self, zone_type: &str) -> (f64, f64, &'static str) {
        for (t, density, weight, source) in LITERATURE_BASELINE_DENSITY {
            if zone_type.contains(t) || t.contains(zone_type) {
                return (*density, *weight, *source);
            }
        }
        (120.0, 0.5, "通用基准")
    }

    pub fn zone_density_range(&self, zone_type: &str) -> (f64, f64) {
        match zone_type {
            t if t.contains("residential") => (100.0, 250.0),
            t if t.contains("commercial") => (120.0, 220.0),
            t if t.contains("industrial") => (50.0, 120.0),
            t if t.contains("palace") || t.contains("administrative") => (60.0, 150.0),
            t if t.contains("religious") => (30.0, 100.0),
            t if t.contains("open") || t.contains("space") => (10.0, 60.0),
            _ => (80.0, 180.0),
        }
    }

    pub fn multi_source_fusion_density(
        &self,
        zone_type: &str,
        zone_area: f64,
        buildings_in_zone: &[Building],
        persons_per_room: f64,
    ) -> (f64, f64, String) {
        let (lit_density, lit_weight, lit_source) = if self.config.use_literature_baseline {
            self.literature_baseline_density(zone_type)
        } else {
            (120.0, 0.3, "默认基准")
        };

        let (range_min, range_max) = self.zone_density_range(zone_type);
        let range_mid = (range_min + range_max) / 2.0;
        let range_weight = 0.25;

        let (building_density, building_weight) = if self.config.use_building_density && !buildings_in_zone.is_empty() {
            let total_rooms: i32 = buildings_in_zone.iter().map(|b| b.num_rooms.max(1)).sum();
            let total_building_area: f64 = buildings_in_zone.iter().map(|b| b.area_sq_m.unwrap_or(50.0)).sum::<f64>() / 1_000_000.0;
            let density = if total_building_area > 0.0 {
                (total_rooms as f64 * persons_per_room) / total_building_area
            } else {
                range_mid
            };
            let weight = (buildings_in_zone.len() as f64 / 20.0).min(0.6).max(0.1);
            (density.max(range_min * 0.5).min(range_max * 1.5), weight)
        } else {
            (range_mid, 0.1)
        };

        let total_weight = lit_weight + range_weight + building_weight;
        let fused = if total_weight > 0.0 {
            (lit_density * lit_weight + range_mid * range_weight + building_density * building_weight) / total_weight
        } else {
            range_mid
        };

        let confidence = total_weight.min(1.0);

        let method_tag = format!(
            "multi_source_lit({:.0})_range({:.0})_building({:.0})",
            lit_weight * 100.0,
            range_weight * 100.0,
            building_weight * 100.0
        );

        (fused, confidence, method_tag)
    }

    pub fn generate_population_grid(
        &self,
        center_lon: f64,
        center_lat: f64,
        site_area_km2: f64,
        base_density: f64,
        control_points: &[InterpolationPoint],
    ) -> (Vec<PopulationGrid>, f64, String) {
        let bandwidth = self.config.bandwidth_km.unwrap_or_else(|| {
            interpolation_service::adaptive_bandwidth(control_points, site_area_km2)
        });

        let result = if self.config.interpolation_method == "idw" {
            interpolation_service::inverse_distance_weighted_interpolation(
                center_lon,
                center_lat,
                site_area_km2,
                control_points,
                2.0,
                self.config.grid_resolution,
            )
        } else {
            interpolation_service::kernel_density_interpolation(
                center_lon,
                center_lat,
                site_area_km2,
                control_points,
                bandwidth,
                self.config.grid_resolution,
            )
        };

        let grid: Vec<PopulationGrid> = result
            .grid_points
            .iter()
            .map(|(lon, lat, val)| PopulationGrid {
                lon: *lon,
                lat: *lat,
                density_km2: val * base_density,
                population: (val * base_density * site_area_km2 / (result.grid_points.len() as f64)) as i32,
            })
            .collect();

        (grid, result.confidence, result.method)
    }

    pub fn allometric_growth_model(
        &self,
        center_lon: f64,
        center_lat: f64,
        site_area_km2: f64,
        estimated_total_population: Option<i32>,
        zones: &[Zone],
        buildings: &[Building],
    ) -> PopulationResult {
        let (data_level, data_score) = self.assess_data_completeness(zones, buildings, estimated_total_population.is_some());

        let b = 0.85;
        let a = 1500.0;

        let theoretical_pop = a * site_area_km2.powf(b);
        let total_pop = estimated_total_population.unwrap_or(theoretical_pop as i32) as f64;

        let mut total_weighted_area = 0.0;
        let mut zone_densities = HashMap::new();
        let mut control_points = Vec::new();

        for zone in zones {
            let (density, conf, _method) = self.multi_source_fusion_density(
                &zone.zone_type,
                zone.area_sq_km,
                buildings,
                4.5,
            );
            let weighted_area = zone.area_sq_km * conf;
            total_weighted_area += weighted_area;
            zone_densities.insert(zone.id, (density, conf));

            control_points.push(InterpolationPoint {
                lon: zone.center_longitude,
                lat: zone.center_latitude,
                value: density / 200.0,
                weight: conf,
            });
        }

        let base_density = if total_weighted_area > 0.0 {
            total_pop / total_weighted_area
        } else {
            total_pop / site_area_km2.max(0.01)
        };

        let (grid, interpolation_conf, method) = self.generate_population_grid(
            center_lon,
            center_lat,
            site_area_km2,
            base_density,
            &control_points,
        );

        let grid_sum: i32 = grid.iter().map(|g| g.population).sum();
        let scale_factor = if grid_sum > 0 {
            total_pop / grid_sum as f64
        } else {
            1.0
        };

        let scaled_grid: Vec<PopulationGrid> = grid
            .into_iter()
            .map(|g| PopulationGrid {
                population: (g.population as f64 * scale_factor) as i32,
                ..g
            })
            .collect();

        let dynamic_conf = (data_score * 0.4 + interpolation_conf * 0.4 + 0.2).min(1.0);

        PopulationResult {
            total_population: total_pop as i32,
            method: format!("allometric_growth_multi_source_{}", method),
            confidence: dynamic_conf,
            grid: scaled_grid,
            data_quality: match data_level {
                DataCompleteness::High => "high",
                DataCompleteness::Medium => "medium",
                DataCompleteness::Low => "low",
                DataCompleteness::VeryLow => "very_low",
            }
            .to_string(),
        }
    }

    pub fn residential_density_model(
        &self,
        center_lon: f64,
        center_lat: f64,
        site_area_km2: f64,
        zones: &[Zone],
        buildings: &[Building],
        persons_per_room: f64,
    ) -> PopulationResult {
        let (data_level, data_score) = self.assess_data_completeness(zones, buildings, false);

        let mut total_pop = 0.0;
        let mut control_points = Vec::new();

        for zone in zones {
            let buildings_in_zone: Vec<Building> = buildings
                .iter()
                .filter(|b| {
                    if let Some(zid) = b.zone_id {
                        zid == zone.id
                    } else {
                        let d_lon = b.longitude - zone.center_longitude;
                        let d_lat = b.latitude - zone.center_latitude;
                        d_lon * d_lon + d_lat * d_lat < 0.01
                    }
                })
                .cloned()
                .collect();

            let (density, conf, _method) = self.multi_source_fusion_density(
                &zone.zone_type,
                zone.area_sq_km,
                &buildings_in_zone,
                persons_per_room,
            );

            let zone_pop = density * zone.area_sq_km;
            total_pop += zone_pop;

            control_points.push(InterpolationPoint {
                lon: zone.center_longitude,
                lat: zone.center_latitude,
                value: density / 200.0,
                weight: conf,
            });
        }

        if control_points.is_empty() {
            control_points.push(InterpolationPoint {
                lon: center_lon,
                lat: center_lat,
                value: 0.6,
                weight: 0.3,
            });
        }

        let base_density = total_pop / site_area_km2.max(0.01);

        let (grid, interpolation_conf, method) = self.generate_population_grid(
            center_lon,
            center_lat,
            site_area_km2,
            base_density,
            &control_points,
        );

        let grid_sum: i32 = grid.iter().map(|g| g.population).sum();
        let scale_factor = if grid_sum > 0 && total_pop > 0.0 {
            total_pop / grid_sum as f64
        } else {
            1.0
        };

        let scaled_grid: Vec<PopulationGrid> = grid
            .into_iter()
            .map(|g| PopulationGrid {
                population: (g.population as f64 * scale_factor) as i32,
                ..g
            })
            .collect();

        let dynamic_conf = (data_score * 0.4 + interpolation_conf * 0.4 + 0.2).min(1.0);

        PopulationResult {
            total_population: total_pop as i32,
            method: format!("residential_density_kernel_{}", method),
            confidence: dynamic_conf,
            grid: scaled_grid,
            data_quality: match data_level {
                DataCompleteness::High => "high",
                DataCompleteness::Medium => "medium",
                DataCompleteness::Low => "low",
                DataCompleteness::VeryLow => "very_low",
            }
            .to_string(),
        }
    }

    pub fn inverse_distance_weighted(
        &self,
        center_lon: f64,
        center_lat: f64,
        site_area_km2: f64,
        zones: &[Zone],
        buildings: &[Building],
    ) -> PopulationResult {
        let (data_level, data_score) = self.assess_data_completeness(zones, buildings, false);

        let mut control_points = Vec::new();
        let mut total_pop = 0.0;

        for zone in zones {
            let buildings_in_zone: Vec<Building> = buildings
                .iter()
                .filter(|b| b.zone_id == Some(zone.id))
                .cloned()
                .collect();

            let (density, conf, _method) = self.multi_source_fusion_density(
                &zone.zone_type,
                zone.area_sq_km,
                &buildings_in_zone,
                4.5,
            );

            total_pop += density * zone.area_sq_km;

            control_points.push(InterpolationPoint {
                lon: zone.center_longitude,
                lat: zone.center_latitude,
                value: density / 200.0,
                weight: conf,
            });
        }

        if control_points.is_empty() {
            control_points.push(InterpolationPoint {
                lon: center_lon,
                lat: center_lat,
                value: 0.6,
                weight: 0.3,
            });
            total_pop = 120.0 * site_area_km2;
        }

        let mut idw_config = self.config.clone();
        idw_config.interpolation_method = "idw".to_string();
        let idw_reconstructor = PopulationReconstructor::new(idw_config);

        let base_density = total_pop / site_area_km2.max(0.01);
        let (grid, interpolation_conf, method) = idw_reconstructor.generate_population_grid(
            center_lon,
            center_lat,
            site_area_km2,
            base_density,
            &control_points,
        );

        let grid_sum: i32 = grid.iter().map(|g| g.population).sum();
        let scale_factor = if grid_sum > 0 && total_pop > 0.0 {
            total_pop / grid_sum as f64
        } else {
            1.0
        };

        let scaled_grid: Vec<PopulationGrid> = grid
            .into_iter()
            .map(|g| PopulationGrid {
                population: (g.population as f64 * scale_factor) as i32,
                ..g
            })
            .collect();

        let dynamic_conf = (data_score * 0.4 + interpolation_conf * 0.4 + 0.2).min(1.0);

        PopulationResult {
            total_population: total_pop as i32,
            method: format!("idw_interpolation_multi_source_{}", method),
            confidence: dynamic_conf,
            grid: scaled_grid,
            data_quality: match data_level {
                DataCompleteness::High => "high",
                DataCompleteness::Medium => "medium",
                DataCompleteness::Low => "low",
                DataCompleteness::VeryLow => "very_low",
            }
            .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn sample_zones() -> Vec<Zone> {
        vec![
            Zone {
                id: 1,
                city_site_id: 1,
                zone_type: "residential".to_string(),
                name: Some("居住区".to_string()),
                area_sq_km: 1.5,
                center_longitude: 116.0,
                center_latitude: 34.0,
            },
            Zone {
                id: 2,
                city_site_id: 1,
                zone_type: "commercial".to_string(),
                name: Some("商业区".to_string()),
                area_sq_km: 0.8,
                center_longitude: 116.005,
                center_latitude: 34.005,
            },
        ]
    }

    fn sample_buildings() -> Vec<Building> {
        vec![
            Building {
                id: 1,
                city_site_id: 1,
                zone_id: Some(1),
                building_type: Some("residential".to_string()),
                area_sq_m: Some(80.0),
                num_rooms: 3,
                longitude: 116.001,
                latitude: 34.001,
            },
            Building {
                id: 2,
                city_site_id: 1,
                zone_id: Some(1),
                building_type: Some("residential".to_string()),
                area_sq_m: Some(100.0),
                num_rooms: 4,
                longitude: 116.002,
                latitude: 34.002,
            },
        ]
    }

    #[test]
    fn test_reconstructor_default_config() {
        let r = PopulationReconstructor::new(Default::default());
        assert!(r.config.use_literature_baseline);
        assert_eq!(r.config.grid_resolution, 16);
    }

    #[test]
    fn test_assess_data_completeness_high() {
        let r = PopulationReconstructor::new(Default::default());
        let zones = sample_zones();
        let buildings = sample_buildings();
        let (level, score) = r.assess_data_completeness(&zones, &buildings, true);
        assert_eq!(level, DataCompleteness::High);
        assert!(score >= 0.7);
    }

    #[test]
    fn test_literature_baseline_palace() {
        let r = PopulationReconstructor::new(Default::default());
        let (density, weight, source) = r.literature_baseline_density("palace");
        assert!(density > 0.0);
        assert!(weight > 0.0);
        assert!(!source.is_empty());
    }

    #[test]
    fn test_multi_source_fusion_density() {
        let r = PopulationReconstructor::new(Default::default());
        let buildings = sample_buildings();
        let (density, conf, method) = r.multi_source_fusion_density(
            "residential", 1.5, &buildings, 4.5,
        );
        assert!(density > 50.0 && density < 300.0);
        assert!(conf > 0.0 && conf <= 1.0);
        assert!(method.contains("multi_source"));
    }

    #[test]
    fn test_allometric_growth_model() {
        let r = PopulationReconstructor::new(Default::default());
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = r.allometric_growth_model(
            116.0, 34.0, 5.0, Some(10000), &zones, &buildings,
        );
        assert_eq!(result.total_population, 10000);
        assert!(!result.grid.is_empty());
        assert!(result.confidence > 0.0 && result.confidence <= 1.0);
        assert!(result.method.contains("allometric_growth_multi_source"));
    }

    #[test]
    fn test_residential_density_model() {
        let r = PopulationReconstructor::new(Default::default());
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = r.residential_density_model(
            116.0, 34.0, 5.0, &zones, &buildings, 4.5,
        );
        assert!(result.total_population > 0);
        assert!(!result.grid.is_empty());
        assert!(result.method.contains("residential_density_kernel"));
    }

    #[test]
    fn test_idw_model() {
        let r = PopulationReconstructor::new(Default::default());
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = r.inverse_distance_weighted(
            116.0, 34.0, 5.0, &zones, &buildings,
        );
        assert!(result.total_population > 0);
        assert!(!result.grid.is_empty());
        assert!(result.method.contains("idw_interpolation_multi_source"));
    }

    #[test]
    fn test_zone_density_range() {
        let r = PopulationReconstructor::new(Default::default());
        let (min, max) = r.zone_density_range("residential_high");
        assert!(min < max);
        assert!(min > 0.0);
    }
}
