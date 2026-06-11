use crate::models::{PopulationAnalysisResult, PopulationGridCell, ZonePopulation, Zone, Building, PopulationResult};
use crate::errors::AppError;
use crate::config::algorithm;
use crate::services::population_reconstructor::{PopulationReconstructor, ReconstructionConfig, DataCompleteness};
use crate::services::interpolation_service;
use serde_json::Value;
use std::collections::HashMap;

const ZONE_DENSITY_MAP: &[(&str, f64, f64)] = &[
    ("palace", 500.0, 1500.0),
    ("residential", 1000.0, 8000.0),
    ("market", 500.0, 2000.0),
    ("workshop", 200.0, 1500.0),
    ("temple", 50.0, 500.0),
    ("official", 300.0, 1200.0),
    ("tomb", 10.0, 100.0),
    ("storage", 50.0, 300.0),
    ("other", 200.0, 1000.0),
];

const LITERATURE_BASELINE_DENSITY: &[(&str, f64, f64, &str)] = &[
    ("palace", 800.0, 0.2, "《考工记·匠人》《三辅黄图》"),
    ("residential", 3500.0, 0.35, "《汉书·地理志》《长安志》"),
    ("market", 1200.0, 0.25, "《洛阳伽蓝记》《东京梦华录》"),
    ("workshop", 800.0, 0.2, "《考工记》《天工开物》"),
    ("temple", 200.0, 0.15, "《洛阳伽蓝记》《建康实录》"),
    ("official", 600.0, 0.2, "《唐六典》《宋会要辑稿》"),
    ("tomb", 30.0, 0.1, "《仪礼》《水经注》"),
    ("storage", 150.0, 0.15, "《史记·平准书》"),
    ("other", 400.0, 0.15, "综合考古遗存估算"),
];

fn convert_zones(zones: &[(String, f64)], center_lon: f64, center_lat: f64) -> Vec<Zone> {
    let angle_step = std::f64::consts::TAU / zones.len().max(1) as f64;
    let radius = 0.01;
    zones.iter().enumerate().map(|(i, (zone_type, area_km2))| {
        let angle = i as f64 * angle_step;
        Zone {
            id: (i + 1) as i32,
            city_site_id: 0,
            zone_type: zone_type.clone(),
            name: None,
            area_sq_km: *area_km2,
            center_longitude: center_lon + radius * angle.cos(),
            center_latitude: center_lat + radius * angle.sin(),
        }
    }).collect()
}

fn convert_buildings(buildings: &[(f64, f64, String)]) -> Vec<Building> {
    buildings.iter().enumerate().map(|(i, (lon, lat, btype))| {
        let num_rooms = match btype.as_str() {
            "palace" => 20,
            "residential" => 4,
            "temple" => 2,
            "official" => 8,
            "workshop" => 3,
            "tomb" => 0,
            "storage" => 0,
            _ => 3,
        };
        Building {
            id: (i + 1) as i32,
            city_site_id: 0,
            zone_id: None,
            building_type: Some(btype.clone()),
            area_sq_m: Some(50.0),
            num_rooms,
            longitude: *lon,
            latitude: *lat,
        }
    }).collect()
}

fn convert_population_result(result: PopulationResult, zones: &[(String, f64)]) -> PopulationAnalysisResult {
    let grid_cells: Vec<PopulationGridCell> = result.grid.iter().map(|g| PopulationGridCell {
        lon: g.lon,
        lat: g.lat,
        population: g.population as f64,
        density: g.density_km2,
        zone_type: None,
    }).collect();

    let total_pop = result.total_population as f64;
    let zone_populations: Vec<ZonePopulation> = zones.iter().map(|(zone_type, area_km2)| {
        let (density_min, density_max) = get_zone_density_range(zone_type);
        let density = (density_min + density_max) / 2.0;
        let population = area_km2 * density;
        ZonePopulation {
            zone_type: zone_type.clone(),
            population,
            area_km2: *area_km2,
            density,
            percentage: if total_pop > 0.0 { (population / total_pop) * 100.0 } else { 0.0 },
        }
    }).collect();

    let densities: Vec<f64> = grid_cells.iter().map(|c| c.density).collect();
    let avg_density = if densities.is_empty() { 0.0 } else { densities.iter().sum::<f64>() / densities.len() as f64 };
    let max_density = densities.iter().cloned().fold(0.0_f64, f64::max);

    PopulationAnalysisResult {
        site_id: 0,
        total_population: total_pop,
        population_density_avg: avg_density,
        population_density_max: max_density,
        model_type: result.method,
        confidence: result.confidence,
        grid_cells,
        zone_populations,
    }
}

pub fn assess_data_completeness(
    zones: &[(String, f64)],
    buildings: &[(f64, f64, String)],
    has_total_pop: bool,
) -> DataCompleteness {
    let reconstructor = PopulationReconstructor::new(ReconstructionConfig::default());
    let converted_zones = convert_zones(zones, 0.0, 0.0);
    let converted_buildings = convert_buildings(buildings);
    let (level, _score) = reconstructor.assess_data_completeness(&converted_zones, &converted_buildings, has_total_pop);
    level
}

pub fn literature_baseline_density(zone_type: &str) -> (f64, f64, &'static str) {
    let zt = zone_type.to_lowercase();
    for (name, density, weight, source) in LITERATURE_BASELINE_DENSITY {
        if name.eq_ignore_ascii_case(&zt) {
            return (*density, *weight, source);
        }
    }
    (400.0, 0.15, "综合考古遗存估算")
}

pub fn multi_source_fusion_density(
    zone_type: &str,
    zone_area_km2: f64,
    buildings_in_zone: &[(f64, f64, String)],
    persons_per_room: f64,
) -> (f64, f64, String) {
    let reconstructor = PopulationReconstructor::new(ReconstructionConfig::default());
    let converted_buildings = convert_buildings(buildings_in_zone);
    reconstructor.multi_source_fusion_density(zone_type, zone_area_km2, &converted_buildings, persons_per_room)
}

pub fn kernel_density_interpolation(
    center_lon: f64,
    center_lat: f64,
    site_area_km2: f64,
    control_points: &[(f64, f64, f64)],
    bandwidth_km: f64,
) -> Vec<PopulationGridCell> {
    let points: Vec<interpolation_service::InterpolationPoint> = control_points.iter().map(|(lon, lat, val)| {
        interpolation_service::InterpolationPoint {
            lon: *lon,
            lat: *lat,
            value: *val,
            weight: 1.0,
        }
    }).collect();

    let result = interpolation_service::kernel_density_interpolation(
        center_lon,
        center_lat,
        site_area_km2,
        &points,
        bandwidth_km,
        algorithm::POPULATION_GRID_RESOLUTION.unwrap_or(16),
    );

    let deg_per_km = 1.0 / 111.0;
    let grid_len = result.grid_points.len() as f64;
    let cell_area_km2 = if grid_len > 0.0 {
        site_area_km2 / grid_len
    } else {
        (algorithm::POPULATION_GRID_SIZE * 111.0).powi(2)
    };

    result.grid_points.iter().map(|(lon, lat, val)| PopulationGridCell {
        lon: *lon,
        lat: *lat,
        population: val * cell_area_km2,
        density: *val,
        zone_type: None,
    }).collect()
}

pub fn get_zone_density_range(zone_type: &str) -> (f64, f64) {
    let zt = zone_type.to_lowercase();
    for (name, min, max) in ZONE_DENSITY_MAP {
        if name.eq_ignore_ascii_case(&zt) {
            return (*min, *max);
        }
    }
    (200.0, 1000.0)
}

pub fn allometric_growth_model(
    site_area_km2: f64,
    total_population: f64,
    center_lon: f64,
    center_lat: f64,
    zones: &[(String, f64)],
    buildings: &[(f64, f64, String)],
) -> PopulationAnalysisResult {
    let reconstructor = PopulationReconstructor::new(ReconstructionConfig::default());
    let converted_zones = convert_zones(zones, center_lon, center_lat);
    let converted_buildings = convert_buildings(buildings);
    let estimated_pop = if total_population > 0.0 { Some(total_population as i32) } else { None };
    let result = reconstructor.allometric_growth_model(
        center_lon,
        center_lat,
        site_area_km2,
        estimated_pop,
        &converted_zones,
        &converted_buildings,
    );
    convert_population_result(result, zones)
}

fn generate_population_grid_simple(
    center_lon: f64,
    zones: &[(String, f64)],
    zone_densities: &HashMap<String, f64>,
) -> Vec<PopulationGridCell> {
    let mut grid_cells = Vec::new();
    let grid_size = 0.002;

    if zones.is_empty() {
        return grid_cells;
    }

    let half_cells = 15i32;

    for i in -half_cells..half_cells {
        for j in -half_cells..half_cells {
            let lon = center_lon + (i as f64) * grid_size;
            let lat = 34.0 + (j as f64) * grid_size;

            let dist_from_center = ((i as f64).powi(2) + (j as f64).powi(2)).sqrt();
            let decay = (-dist_from_center / 10.0).exp();

            let zone_idx = ((i + half_cells + j + half_cells) as usize) % zones.len().max(1);
            let zone_type = zones.get(zone_idx).map(|(z, _)| z.clone()).unwrap_or_else(|| "residential".to_string());
            let base_density = zone_densities.get(&zone_type).cloned().unwrap_or(500.0);

            let density = base_density * (0.4 + 0.6 * decay);
            let cell_area_km2 = (grid_size * 111.0).powi(2);
            let population = density * cell_area_km2;

            grid_cells.push(PopulationGridCell {
                lon,
                lat,
                population,
                density,
                zone_type: Some(zone_type),
            });
        }
    }

    grid_cells
}

pub fn residential_density_model(
    site_area_km2: f64,
    buildings: &[(f64, f64, String)],
    zones: &[(String, f64)],
    persons_per_room: f64,
) -> PopulationAnalysisResult {
    let center_lon = if buildings.is_empty() { 116.0 } else {
        buildings.iter().map(|(lon, _, _)| *lon).sum::<f64>() / buildings.len() as f64
    };
    let center_lat = if buildings.is_empty() { 34.0 } else {
        buildings.iter().map(|(_, lat, _)| *lat).sum::<f64>() / buildings.len() as f64
    };

    let reconstructor = PopulationReconstructor::new(ReconstructionConfig::default());
    let converted_zones = convert_zones(zones, center_lon, center_lat);
    let converted_buildings = convert_buildings(buildings);
    let result = reconstructor.residential_density_model(
        center_lon,
        center_lat,
        site_area_km2,
        &converted_zones,
        &converted_buildings,
        persons_per_room,
    );
    convert_population_result(result, zones)
}

pub fn inverse_distance_weighted(
    center_lon: f64,
    center_lat: f64,
    site_area_km2: f64,
    total_population: f64,
    zones: &[(String, f64)],
    buildings: &[(f64, f64, String)],
) -> PopulationAnalysisResult {
    let reconstructor = PopulationReconstructor::new(ReconstructionConfig::default());
    let converted_zones = convert_zones(zones, center_lon, center_lat);
    let converted_buildings = convert_buildings(buildings);
    let result = reconstructor.inverse_distance_weighted(
        center_lon,
        center_lat,
        site_area_km2,
        &converted_zones,
        &converted_buildings,
    );
    let mut converted = convert_population_result(result, zones);
    if total_population > 0.0 {
        converted.total_population = total_population;
    }
    converted
}

pub async fn analyze_population(
    pool: &sqlx::PgPool,
    site_id: i32,
    model_type: Option<&str>,
) -> Result<PopulationAnalysisResult, AppError> {
    let site_row = sqlx::query!(
        r#"
        SELECT cs.id, cs.name, cs.estimated_population, cs.area_sq_km,
               cs.center_longitude, cs.center_latitude,
               ST_AsGeoJSON(cs.geom)::jsonb as geom
        FROM city_sites cs
        WHERE cs.id = $1
        "#,
        site_id
    )
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound(format!("City site with id {} not found", site_id)))?;

    let total_population = site_row.estimated_population.unwrap_or(0) as f64;
    let area_sq_km = site_row.area_sq_km.unwrap_or(1.0);
    let center_lon = site_row.center_longitude;
    let center_lat = site_row.center_latitude;

    let zones_rows = sqlx::query!(
        r#"
        SELECT fz.zone_type, 
               ST_Area(ST_Transform(fz.geom, 3857)) / 1000000.0 as area_km2
        FROM functional_zones fz
        WHERE fz.city_site_id = $1
        "#,
        site_id
    )
    .fetch_all(pool)
    .await?;

    let zones: Vec<(String, f64)> = zones_rows
        .iter()
        .map(|r| (r.zone_type.clone(), r.area_km2.unwrap_or(0.0)))
        .collect();

    let buildings_rows = sqlx::query!(
        r#"
        SELECT ST_X(bf.geom) as lon, ST_Y(bf.geom) as lat, bf.building_type
        FROM building_foundations bf
        WHERE bf.city_site_id = $1
        "#,
        site_id
    )
    .fetch_all(pool)
    .await?;

    let buildings: Vec<(f64, f64, String)> = buildings_rows
        .iter()
        .filter_map(|r| {
            match (r.lon, r.lat, &r.building_type) {
                (Some(lon), Some(lat), Some(bt)) => Some((lon, lat, bt.clone())),
                _ => None,
            }
        })
        .collect();

    let model = model_type.unwrap_or("allometric");
    let mut result = match model {
        "residential" => residential_density_model(
            area_sq_km,
            &buildings,
            &zones,
            algorithm::POPULATION_PERSONS_PER_ROOM,
        ),
        "idw" => inverse_distance_weighted(
            center_lon,
            center_lat,
            area_sq_km,
            total_population,
            &zones,
            &buildings,
        ),
        _ => allometric_growth_model(
            area_sq_km,
            total_population,
            center_lon,
            center_lat,
            &zones,
            &buildings,
        ),
    };

    result.site_id = site_id;

    if !buildings.is_empty() && model != "residential" {
        result.confidence = (result.confidence + 0.1).min(0.95);
    }
    if zones.len() < 3 {
        result.confidence = (result.confidence - 0.15).max(0.3);
    }

    Ok(result)
}

pub fn parse_geojson_polygon_coords(geom: &Value) -> Option<Vec<(f64, f64)>> {
    let coords = geom.get("coordinates")?.get(0)?.as_array()?;
    let points: Vec<(f64, f64)> = coords
        .iter()
        .filter_map(|c| {
            let lon = c.get(0)?.as_f64()?;
            let lat = c.get(1)?.as_f64()?;
            Some((lon, lat))
        })
        .collect();
    if points.is_empty() { None } else { Some(points) }
}

use actix_web::{web, HttpResponse};
use crate::models::ApiResponse;

pub async fn analyze_population_handler(
    pool: web::Data<sqlx::PgPool>,
    site_id: web::Path<i32>,
    query: web::Query<std::collections::HashMap<String, String>>,
) -> Result<HttpResponse, AppError> {
    let model_type = query.get("model").map(|s| s.as_str());
    let result = analyze_population(&pool, *site_id, model_type).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

pub async fn get_population_distributions_handler(
    pool: web::Data<sqlx::PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, site_id, analysis_id,
               ST_AsGeoJSON(grid_cell_geom)::jsonb as grid_cell_geom,
               ST_AsGeoJSON(grid_cell_centroid)::jsonb as grid_cell_centroid,
               population_estimate, density_per_km2, zone_type, model_type, confidence
        FROM population_distributions
        WHERE site_id = $1
        ORDER BY id
        "#,
        *site_id
    )
    .fetch_all(pool.get_ref())
    .await?;

    if rows.is_empty() {
        let result = analyze_population(&pool, *site_id, None).await?;
        return Ok(HttpResponse::Ok().json(ApiResponse::success(result)));
    }

    let distributions: Vec<crate::models::PopulationDistribution> = rows
        .iter()
        .map(|r| crate::models::PopulationDistribution {
            id: r.id,
            site_id: r.site_id,
            analysis_id: r.analysis_id,
            grid_cell_geom: r.grid_cell_geom.clone(),
            grid_cell_centroid: r.grid_cell_centroid.clone(),
            population_estimate: r.population_estimate.map(|v| v as f64),
            density_per_km2: r.density_per_km2.map(|v| v as f64),
            zone_type: r.zone_type.clone(),
            model_type: r.model_type.clone(),
            confidence: r.confidence.map(|v| v as f64),
            created_at: None,
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(distributions)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn sample_zones() -> Vec<(String, f64)> {
        vec![
            ("palace".to_string(), 0.2),
            ("residential".to_string(), 1.5),
            ("market".to_string(), 0.3),
            ("workshop".to_string(), 0.4),
            ("temple".to_string(), 0.1),
            ("official".to_string(), 0.2),
            ("tomb".to_string(), 0.1),
            ("storage".to_string(), 0.2),
        ]
    }

    fn sample_buildings() -> Vec<(f64, f64, String)> {
        vec![
            (116.001, 34.001, "palace".to_string()),
            (116.002, 34.002, "residential".to_string()),
            (116.003, 34.003, "residential".to_string()),
            (116.004, 34.004, "market".to_string()),
            (116.005, 34.005, "workshop".to_string()),
            (116.006, 34.006, "temple".to_string()),
            (116.007, 34.007, "official".to_string()),
            (116.008, 34.008, "residential".to_string()),
            (116.009, 34.009, "residential".to_string()),
            (116.010, 34.010, "storage".to_string()),
        ]
    }

    #[test]
    fn test_get_zone_density_range_normal() {
        let (min, max) = get_zone_density_range("residential");
        assert_eq!(min, 1000.0);
        assert_eq!(max, 8000.0);
        assert!(max > min);
    }

    #[test]
    fn test_get_zone_density_range_all_types() {
        let types = [
            "palace", "residential", "market", "workshop",
            "temple", "official", "tomb", "storage", "other",
        ];
        for t in types {
            let (min, max) = get_zone_density_range(t);
            assert!(min >= 0.0, "{} min should be >= 0", t);
            assert!(max >= min, "{} max should be >= min", t);
        }
    }

    #[test]
    fn test_get_zone_density_range_unknown_defaults() {
        let (min, max) = get_zone_density_range("unknown_type_xyz");
        assert_eq!(min, 200.0);
        assert_eq!(max, 1000.0);
    }

    #[test]
    fn test_get_zone_density_range_case_insensitive() {
        let r1 = get_zone_density_range("Residential");
        let r2 = get_zone_density_range("RESIDENTIAL");
        let r3 = get_zone_density_range("residential");
        assert_eq!(r1, r2);
        assert_eq!(r2, r3);
    }

    #[test]
    fn test_allometric_growth_model_normal_case() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = allometric_growth_model(
            3.0,
            50000.0,
            116.0,
            34.0,
            &zones,
            &buildings,
        );

        assert_eq!(result.site_id, 0);
        assert_eq!(result.total_population, 50000.0);
        assert_eq!(result.model_type, "allometric_growth_multi_source");
        assert!(result.confidence > 0.0 && result.confidence <= 1.0);
        assert!(!result.grid_cells.is_empty());
        assert_eq!(result.zone_populations.len(), zones.len());
    }

    #[test]
    fn test_allometric_growth_model_population_conservation() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = allometric_growth_model(
            3.0,
            50000.0,
            116.0,
            34.0,
            &zones,
            &buildings,
        );

        let zone_sum: f64 = result.zone_populations.iter().map(|z| z.population).sum();
        assert_relative_eq!(zone_sum, 50000.0, epsilon = 1.0);

        let pct_sum: f64 = result.zone_populations.iter().map(|z| z.percentage).sum();
        assert_relative_eq!(pct_sum, 100.0, epsilon = 0.5);
    }

    #[test]
    fn test_allometric_growth_model_zone_density_ordering() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = allometric_growth_model(
            3.0,
            50000.0,
            116.0,
            34.0,
            &zones,
            &buildings,
        );

        let residential = result.zone_populations.iter()
            .find(|z| z.zone_type == "residential").unwrap();
        let tomb = result.zone_populations.iter()
            .find(|z| z.zone_type == "tomb").unwrap();
        assert!(
            residential.density > tomb.density,
            "residential density should exceed tomb density"
        );
    }

    #[test]
    fn test_allometric_growth_model_zero_population() {
        let zones = sample_zones();
        let buildings = vec![];
        let result = allometric_growth_model(
            3.0,
            0.0,
            116.0,
            34.0,
            &zones,
            &buildings,
        );

        assert!(result.total_population > 0.0, "should fall back to density estimate");
        assert!(!result.grid_cells.is_empty());
    }

    #[test]
    fn test_allometric_growth_model_empty_zones() {
        let zones: Vec<(String, f64)> = vec![];
        let buildings = vec![];
        let result = allometric_growth_model(
            1.0,
            10000.0,
            116.0,
            34.0,
            &zones,
            &buildings,
        );

        assert!(result.zone_populations.is_empty());
        assert!(result.grid_cells.is_empty());
    }

    #[test]
    fn test_allometric_growth_model_grid_coordinates_bounds() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = allometric_growth_model(
            3.0,
            50000.0,
            116.0,
            34.0,
            &zones,
            &buildings,
        );

        for cell in &result.grid_cells {
            assert!(cell.lon >= 115.9 && cell.lon <= 116.1, "lon out of bounds: {}", cell.lon);
            assert!(cell.lat >= 33.9 && cell.lat <= 34.1, "lat out of bounds: {}", cell.lat);
            assert!(cell.density >= 0.0, "density should be non-negative");
            assert!(cell.population >= 0.0, "population should be non-negative");
        }
    }

    #[test]
    fn test_allometric_growth_model_avg_density_reasonable() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = allometric_growth_model(
            3.0,
            50000.0,
            116.0,
            34.0,
            &zones,
            &buildings,
        );

        let expected_avg = 50000.0 / 3.0;
        let ratio = result.population_density_avg / expected_avg;
        assert!(
            ratio > 0.1 && ratio < 10.0,
            "avg density {} out of reasonable range (expected ~{})",
            result.population_density_avg, expected_avg
        );
    }

    #[test]
    fn test_residential_density_model_normal_case() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = residential_density_model(
            3.0,
            &buildings,
            &zones,
            4.5,
        );

        assert_eq!(result.model_type, "residential_density_kernel");
        assert!(result.total_population > 0.0);
        assert!(!result.grid_cells.is_empty());
    }

    #[test]
    fn test_residential_density_model_population_per_room() {
        let zones = sample_zones();
        let buildings = vec![
            (116.0, 34.0, "residential".to_string()),
            (116.001, 34.001, "residential".to_string()),
        ];
        let r_low = residential_density_model(1.0, &buildings, &zones, 2.0);
        let r_high = residential_density_model(1.0, &buildings, &zones, 8.0);
        assert!(r_high.total_population > r_low.total_population);
    }

    #[test]
    fn test_residential_density_model_no_buildings() {
        let zones = sample_zones();
        let buildings = vec![];
        let result = residential_density_model(3.0, &buildings, &zones, 4.5);
        assert_eq!(result.grid_cells.len(), 0);
        assert!(result.total_population > 0.0, "should use zone fallback");
    }

    #[test]
    fn test_residential_density_model_room_types_non_residential_zero_contribution() {
        let zones = vec![];
        let buildings = vec![
            (116.0, 34.0, "tomb".to_string()),
            (116.001, 34.001, "storage".to_string()),
        ];
        let result = residential_density_model(1.0, &buildings, &zones, 4.5);
        assert_eq!(result.total_population, 0.0);
        assert!(result.grid_cells.is_empty());
    }

    #[test]
    fn test_residential_density_model_monotonic_with_building_count() {
        let zones = vec![];
        let b1 = vec![(116.0, 34.0, "residential".to_string())];
        let b2 = vec![
            (116.0, 34.0, "residential".to_string()),
            (116.001, 34.001, "residential".to_string()),
            (116.002, 34.002, "residential".to_string()),
        ];
        let r1 = residential_density_model(1.0, &b1, &zones, 4.5);
        let r2 = residential_density_model(1.0, &b2, &zones, 4.5);
        assert!(r2.total_population > r1.total_population);
    }

    #[test]
    fn test_inverse_distance_weighted_normal_case() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = inverse_distance_weighted(
            116.0,
            34.0,
            3.0,
            50000.0,
            &zones,
            &buildings,
        );

        assert_eq!(result.model_type, "idw_interpolation_multi_source");
        assert_eq!(result.total_population, 50000.0);
        assert!(!result.grid_cells.is_empty());
    }

    #[test]
    fn test_inverse_distance_weighted_center_decay() {
        let zones = vec![("residential".to_string(), 1.0)];
        let buildings = vec![];
        let result = inverse_distance_weighted(
            116.0,
            34.0,
            4.0,
            20000.0,
            &zones,
            &buildings,
        );

        let mut center_density = 0.0_f64;
        let mut edge_density = f64::INFINITY;

        for cell in &result.grid_cells {
            let dist = ((cell.lon - 116.0).powi(2) + (cell.lat - 34.0).powi(2)).sqrt();
            if dist < 0.005 {
                center_density = cell.density;
            }
            if dist > 0.02 {
                edge_density = edge_density.min(cell.density);
            }
        }

        assert!(center_density > edge_density, "center {} should be denser than edge {}", center_density, edge_density);
    }

    #[test]
    fn test_inverse_distance_weighted_scaling_match() {
        let zones = sample_zones();
        let buildings = sample_buildings();
        let result = inverse_distance_weighted(
            116.0,
            34.0,
            3.0,
            50000.0,
            &zones,
            &buildings,
        );

        let grid_sum: f64 = result.grid_cells.iter().map(|c| c.population).sum();
        let ratio = grid_sum / result.total_population;
        assert_relative_eq!(ratio, 1.0, epsilon = 0.05);
    }

    #[test]
    fn test_inverse_distance_weighted_negative_inputs_safe() {
        let zones = sample_zones();
        let buildings = vec![];
        let result = inverse_distance_weighted(
            116.0,
            34.0,
            3.0,
            -100.0,
            &zones,
            &buildings,
        );
        for cell in &result.grid_cells {
            assert!(cell.density >= 0.0);
            assert!(cell.population >= 0.0);
        }
    }

    #[test]
    fn test_inverse_distance_weighted_zone_count_preserved() {
        let zones = sample_zones();
        let buildings = vec![];
        let result = inverse_distance_weighted(
            116.0,
            34.0,
            3.0,
            50000.0,
            &zones,
            &buildings,
        );
        assert_eq!(result.zone_populations.len(), zones.len());
    }

    #[test]
    fn test_three_models_different_results_same_input() {
        let zones = sample_zones();
        let buildings = sample_buildings();

        let r1 = allometric_growth_model(3.0, 50000.0, 116.0, 34.0, &zones, &buildings);
        let r2 = residential_density_model(3.0, &buildings, &zones, 4.5);
        let r3 = inverse_distance_weighted(116.0, 34.0, 3.0, 50000.0, &zones, &buildings);

        assert_ne!(r1.model_type, r2.model_type);
        assert_ne!(r2.model_type, r3.model_type);
        assert_ne!(r1.model_type, r3.model_type);

        assert_ne!(r1.grid_cells.len(), 0);
        assert_ne!(r3.grid_cells.len(), 0);
    }

    #[test]
    fn test_historical_benchmark_changan_like() {
        let zones = vec![
            ("palace".to_string(), 1.0),
            ("residential".to_string(), 6.0),
            ("market".to_string(), 1.0),
            ("official".to_string(), 0.8),
            ("temple".to_string(), 0.2),
            ("workshop".to_string(), 1.0),
        ];
        let buildings = vec![];

        let result = allometric_growth_model(
            10.0,
            1000000.0,
            108.9,
            34.3,
            &zones,
            &buildings,
        );

        let residential = result.zone_populations.iter()
            .find(|z| z.zone_type == "residential").unwrap();
        let palace = result.zone_populations.iter()
            .find(|z| z.zone_type == "palace").unwrap();

        assert!(residential.population > palace.population);
        assert!(residential.percentage > 30.0, "residential should dominate, got {}%", residential.percentage);
        assert!(result.population_density_avg > 50000.0, "Tang Chang'an should be dense");
        assert!(result.confidence >= 0.6);
    }

    #[test]
    fn test_multi_source_fusion_reduces_bias_with_buildings() {
        let zones = sample_zones();
        let no_buildings: Vec<(f64, f64, String)> = vec![];
        let with_buildings = sample_buildings();

        let r_no = allometric_growth_model(3.0, 50000.0, 116.0, 34.0, &zones, &no_buildings);
        let r_yes = allometric_growth_model(3.0, 50000.0, 116.0, 34.0, &zones, &with_buildings);

        assert!(r_yes.confidence > r_no.confidence,
            "with buildings should have higher confidence: yes={} no={}", r_yes.confidence, r_no.confidence);
        assert!(r_no.confidence < 0.7, "no buildings confidence should be lower");
        assert!(r_yes.confidence > 0.5, "with buildings confidence should be reasonable");
    }

    #[test]
    fn test_data_completeness_assessment_levels() {
        let zones_many: Vec<(String, f64)> = vec![
            ("residential".into(), 1.0), ("palace".into(), 0.5),
            ("market".into(), 0.3), ("temple".into(), 0.2),
            ("workshop".into(), 0.4), ("official".into(), 0.3),
        ];
        let zones_few = vec![("residential".to_string(), 1.0)];
        let buildings_many: Vec<(f64, f64, String)> = (0..30)
            .map(|i| (116.0 + i as f64 * 0.001, 34.0 + i as f64 * 0.001, "residential".into()))
            .collect();
        let buildings_none: Vec<(f64, f64, String)> = vec![];

        let high = assess_data_completeness(&zones_many, &buildings_many, true);
        let low = assess_data_completeness(&zones_few, &buildings_none, false);

        assert_eq!(high, DataCompleteness::High);
        assert_eq!(low, DataCompleteness::VeryLow);
    }

    #[test]
    fn test_literature_baseline_known_unknown() {
        let (res_dens, res_w, _src) = literature_baseline_density("residential");
        assert!(res_dens > 1000.0, "residential baseline should be reasonable");
        assert!(res_w > 0.0);

        let (unk_dens, unk_w, src) = literature_baseline_density("unknown_xyz");
        assert!(unk_dens > 0.0);
        assert!(unk_w > 0.0);
        assert!(!src.is_empty());
    }

    #[test]
    fn test_kernel_density_interpolation_basic() {
        let points = vec![
            (116.0, 34.0, 5000.0),
            (116.01, 34.0, 3000.0),
            (116.0, 34.01, 4000.0),
        ];
        let grid = kernel_density_interpolation(116.0, 34.0, 1.0, &points, 0.3);

        assert!(!grid.is_empty());
        assert!(grid.iter().all(|c| c.density >= 0.0));

        let center = grid.iter()
            .find(|c| (c.lon - 116.0).abs() < 0.005 && (c.lat - 34.0).abs() < 0.005)
            .expect("center cell should exist");
        assert!(center.density > 1000.0, "center density should be significant");
    }

    #[test]
    fn test_fusion_weight_changes_with_building_count() {
        let few_buildings = vec![(116.0, 34.0, "residential".to_string())];
        let many_buildings: Vec<(f64, f64, String)> = (0..50)
            .map(|i| (116.0 + i as f64 * 0.0005, 34.0, "residential".into()))
            .collect();

        let (dens_few, conf_few, _) = multi_source_fusion_density("residential", 0.5, &few_buildings, 4.5);
        let (dens_many, conf_many, _) = multi_source_fusion_density("residential", 0.5, &many_buildings, 4.5);

        assert!(conf_many > conf_few, "more buildings should raise confidence");
        assert!(dens_few > 0.0);
        assert!(dens_many > 0.0);
    }
}
