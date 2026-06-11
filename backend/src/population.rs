use crate::models::{PopulationAnalysisResult, PopulationGridCell, ZonePopulation};
use crate::errors::AppError;
use crate::config::algorithm;
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
    let b = 0.85;
    let _scaling_factor = total_population / site_area_km2.powf(b);

    let mut zone_populations: Vec<ZonePopulation> = Vec::new();
    let mut total_zone_pop = 0.0_f64;
    let mut zone_densities: HashMap<String, f64> = HashMap::new();

    for (zone_type, area_km2) in zones {
        let (density_min, density_max) = get_zone_density_range(zone_type);
        let density = density_min + (density_max - density_min) * 0.6;
        let pop = area_km2 * 1000000.0 / 1000000.0 * density;
        let pop = if pop < 0.0 { 0.0 } else { pop };
        total_zone_pop += pop;
        zone_densities.insert(zone_type.clone(), density);
    }

    if total_zone_pop > 0.0 && total_population > 0.0 {
        let ratio = total_population / total_zone_pop;
        for (zone_type, area_km2) in zones {
            let density = zone_densities.get(zone_type).cloned().unwrap_or(500.0);
            let pop = area_km2 * density * ratio;
            let percentage = (pop / total_population) * 100.0;
            zone_populations.push(ZonePopulation {
                zone_type: zone_type.clone(),
                population: pop,
                area_km2: *area_km2,
                density: pop / area_km2,
                percentage,
            });
        }
    } else {
        for (zone_type, area_km2) in zones {
            let density = zone_densities.get(zone_type).cloned().unwrap_or(500.0);
            let pop = area_km2 * density;
            total_zone_pop += pop;
            zone_populations.push(ZonePopulation {
                zone_type: zone_type.clone(),
                population: pop,
                area_km2: *area_km2,
                density,
                percentage: 0.0,
            });
        }
        for zp in &mut zone_populations {
            zp.percentage = (zp.population / total_zone_pop) * 100.0;
        }
    }

    let grid_cells = generate_population_grid(center_lon, center_lat, zones, &zone_densities, total_population, total_zone_pop);

    let densities: Vec<f64> = grid_cells.iter().map(|c| c.density).collect();
    let avg_density = if densities.is_empty() { 0.0 } else { densities.iter().sum::<f64>() / densities.len() as f64 };
    let max_density = densities.iter().cloned().fold(0.0_f64, f64::max);

    PopulationAnalysisResult {
        site_id: 0,
        total_population: if total_population > 0.0 { total_population } else { total_zone_pop },
        population_density_avg: avg_density,
        population_density_max: max_density,
        model_type: "allometric_growth".to_string(),
        confidence: 0.75,
        grid_cells,
        zone_populations,
    }
}

fn generate_population_grid(
    center_lon: f64,
    center_lat: f64,
    zones: &[(String, f64)],
    zone_densities: &HashMap<String, f64>,
    _total_population: f64,
    _total_zone_pop: f64,
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
            let lat = center_lat + (j as f64) * grid_size;

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
    _site_area_km2: f64,
    buildings: &[(f64, f64, String)],
    zones: &[(String, f64)],
    persons_per_room: f64,
) -> PopulationAnalysisResult {
    let mut total_population = 0.0_f64;
    let mut building_pop_map: HashMap<(i32, i32), f64> = HashMap::new();
    let grid_size = 0.002;

    for (lon, lat, btype) in buildings {
        let grid_i = (lon / grid_size).floor() as i32;
        let grid_j = (lat / grid_size).floor() as i32;

        let rooms = match btype.as_str() {
            "palace" => 20,
            "residential" => 4,
            "temple" => 2,
            "official" => 8,
            "workshop" => 3,
            "tomb" => 0,
            "storage" => 0,
            _ => 3,
        };

        let pop = rooms as f64 * persons_per_room;
        *building_pop_map.entry((grid_i, grid_j)).or_insert(0.0) += pop;
        total_population += pop;
    }

    let mut grid_cells: Vec<PopulationGridCell> = Vec::new();
    let cell_area_km2 = (grid_size * 111.0).powi(2);

    for ((grid_i, grid_j), pop) in &building_pop_map {
        let lon = (*grid_i as f64 + 0.5) * grid_size;
        let lat = (*grid_j as f64 + 0.5) * grid_size;
        let density = pop / cell_area_km2;

        grid_cells.push(PopulationGridCell {
            lon,
            lat,
            population: *pop,
            density,
            zone_type: None,
        });
    }

    let mut zone_populations: Vec<ZonePopulation> = Vec::new();
    for (zone_type, area_km2) in zones {
        let (density_min, density_max) = get_zone_density_range(zone_type);
        let density = (density_min + density_max) / 2.0;
        let pop = area_km2 * density;
        zone_populations.push(ZonePopulation {
            zone_type: zone_type.clone(),
            population: pop,
            area_km2: *area_km2,
            density,
            percentage: 0.0,
        });
    }

    let total_zone_pop: f64 = zone_populations.iter().map(|z| z.population).sum();
    for zp in &mut zone_populations {
        zp.percentage = if total_zone_pop > 0.0 { (zp.population / total_zone_pop) * 100.0 } else { 0.0 };
    }

    let final_pop = if total_population > 0.0 { total_population } else { total_zone_pop };
    let avg_density = if grid_cells.is_empty() { 0.0 } else { grid_cells.iter().map(|c| c.density).sum::<f64>() / grid_cells.len() as f64 };
    let max_density = grid_cells.iter().map(|c| c.density).fold(0.0_f64, f64::max);

    PopulationAnalysisResult {
        site_id: 0,
        total_population: final_pop,
        population_density_avg: avg_density,
        population_density_max: max_density,
        model_type: "residential_density".to_string(),
        confidence: 0.7,
        grid_cells,
        zone_populations,
    }
}

pub fn inverse_distance_weighted(
    center_lon: f64,
    center_lat: f64,
    site_area_km2: f64,
    total_population: f64,
    zones: &[(String, f64)],
) -> PopulationAnalysisResult {
    let grid_size = algorithm::POPULATION_GRID_SIZE;
    let half_size_km = (site_area_km2.sqrt() / 2.0).max(0.5);
    let deg_per_km = 1.0 / 111.0;
    let half_deg = half_size_km * deg_per_km;

    let cells_per_side = ((half_deg * 2.0) / grid_size).ceil() as i32;
    let cells_per_side = cells_per_side.max(10).min(50);

    let mut zone_centers: Vec<(f64, f64, f64, String)> = Vec::new();
    let mut angle = 0.0;
    let angle_step = std::f64::consts::TAU / zones.len().max(1) as f64;

    for (zone_type, area_km2) in zones {
        let r = half_deg * 0.6 * (angle.cos().abs() * 0.5 + 0.5);
        let zl = center_lon + r * angle.cos();
        let zlt = center_lat + r * angle.sin();

        let (density_min, density_max) = get_zone_density_range(zone_type);
        let density = (density_min + density_max) / 2.0;

        zone_centers.push((zl, zlt, density, zone_type.clone()));
        angle += angle_step;
    }

    let mut grid_cells: Vec<PopulationGridCell> = Vec::new();
    let cell_area_km2 = (grid_size * 111.0).powi(2);

    for i in 0..cells_per_side {
        for j in 0..cells_per_side {
            let lon = center_lon - half_deg + (i as f64 + 0.5) * grid_size;
            let lat = center_lat - half_deg + (j as f64 + 0.5) * grid_size;

            let mut total_weight = 0.0;
            let mut weighted_density = 0.0;
            let mut best_zone = None;
            let mut best_density = 0.0;

            for (zl, zlt, density, zone_type) in &zone_centers {
                let dist = ((lon - zl).powi(2) + (lat - zlt).powi(2)).sqrt();
                let dist = dist.max(0.0001);
                let weight = 1.0 / dist.powi(2);

                weighted_density += density * weight;
                total_weight += weight;

                if weight > best_density {
                    best_density = weight;
                    best_zone = Some(zone_type.clone());
                }
            }

            let density = if total_weight > 0.0 { weighted_density / total_weight } else { 0.0 };
            let dist_from_center = ((lon - center_lon).powi(2) + (lat - center_lat).powi(2)).sqrt();
            let center_decay = (-dist_from_center / half_deg * 1.5).exp().max(0.1);
            let density = density * center_decay;

            let population = density * cell_area_km2;

            grid_cells.push(PopulationGridCell {
                lon,
                lat,
                population,
                density,
                zone_type: best_zone,
            });
        }
    }

    let grid_total_pop: f64 = grid_cells.iter().map(|c| c.population).sum();
    if grid_total_pop > 0.0 && total_population > 0.0 {
        let scale = total_population / grid_total_pop;
        for cell in &mut grid_cells {
            cell.population *= scale;
            cell.density *= scale;
        }
    }

    let mut zone_pop_map: HashMap<String, (f64, f64)> = HashMap::new();
    for (zone_type, area_km2) in zones {
        let (density_min, density_max) = get_zone_density_range(zone_type);
        let density = (density_min + density_max) / 2.0;
        let pop = area_km2 * density;
        zone_pop_map.insert(zone_type.clone(), (pop, *area_km2));
    }

    let total_zone_pop: f64 = zone_pop_map.values().map(|(p, _)| *p).sum();
    let mut zone_populations: Vec<ZonePopulation> = zone_pop_map
        .into_iter()
        .map(|(zone_type, (pop, area))| ZonePopulation {
            zone_type,
            population: pop,
            area_km2: area,
            density: pop / area.max(0.0001),
            percentage: if total_zone_pop > 0.0 { (pop / total_zone_pop) * 100.0 } else { 0.0 },
        })
        .collect();

    zone_populations.sort_by(|a, b| b.population.partial_cmp(&a.population).unwrap_or(std::cmp::Ordering::Equal));

    let final_pop = if total_population > 0.0 { total_population } else { grid_total_pop };
    let avg_density = if grid_cells.is_empty() { 0.0 } else { grid_cells.iter().map(|c| c.density).sum::<f64>() / grid_cells.len() as f64 };
    let max_density = grid_cells.iter().map(|c| c.density).fold(0.0_f64, f64::max);

    PopulationAnalysisResult {
        site_id: 0,
        total_population: final_pop,
        population_density_avg: avg_density,
        population_density_max: max_density,
        model_type: "idw_interpolation".to_string(),
        confidence: 0.65,
        grid_cells,
        zone_populations,
    }
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
