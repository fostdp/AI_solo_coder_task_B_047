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

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DataCompleteness {
    High,
    Medium,
    Low,
    VeryLow,
}

pub fn assess_data_completeness(
    zones: &[(String, f64)],
    buildings: &[(f64, f64, String)],
    has_total_pop: bool,
) -> DataCompleteness {
    let mut score = 0.0;
    if !zones.is_empty() { score += 0.4; }
    if zones.len() >= 5 { score += 0.1; }
    if !buildings.is_empty() { score += 0.3; }
    if buildings.len() >= 20 { score += 0.1; }
    if has_total_pop { score += 0.1; }

    if score >= 0.85 { DataCompleteness::High }
    else if score >= 0.6 { DataCompleteness::Medium }
    else if score >= 0.3 { DataCompleteness::Low }
    else { DataCompleteness::VeryLow }
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
    let (lit_density, lit_weight, lit_source) = literature_baseline_density(zone_type);

    let (zone_min, zone_max) = get_zone_density_range(zone_type);
    let zone_avg = (zone_min + zone_max) / 2.0;
    let zone_weight = 0.25;

    let (building_density, building_weight) = if buildings_in_zone.is_empty() || zone_area_km2 <= 0.0 {
        (0.0, 0.0)
    } else {
        let mut total_rooms = 0.0;
        for (_, _, btype) in buildings_in_zone {
            let rooms = match btype.as_str() {
                "palace" => 20.0,
                "residential" => 4.0,
                "temple" => 2.0,
                "official" => 8.0,
                "workshop" => 3.0,
                "tomb" => 0.0,
                "storage" => 0.0,
                _ => 3.0,
            };
            total_rooms += rooms;
        }
        let pop_estimate = total_rooms * persons_per_room;
        let density = pop_estimate / zone_area_km2.max(0.001);
        let w = (buildings_in_zone.len() as f64 / 30.0).min(1.0) * 0.6;
        (density, w)
    };

    let total_weight = lit_weight + zone_weight + building_weight;
    if total_weight <= 0.0 {
        return (lit_density, lit_weight, lit_source.to_string());
    }

    let fused = (lit_density * lit_weight + zone_avg * zone_weight + building_density * building_weight)
        / total_weight;

    let confidence = (lit_weight * 0.6 + zone_weight * 0.4 + building_weight)
        / total_weight;

    let method_tag = format!(
        "fusion(lit={:.1}×{:.2}, zone={:.1}×{:.2}, bld={:.1}×{:.2})",
        lit_density, lit_weight, zone_avg, zone_weight, building_density, building_weight
    );

    (fused.max(0.0), confidence.max(0.0).min(1.0), method_tag)
}

pub fn kernel_density_interpolation(
    center_lon: f64,
    center_lat: f64,
    site_area_km2: f64,
    control_points: &[(f64, f64, f64)],
    bandwidth_km: f64,
) -> Vec<PopulationGridCell> {
    let grid_size = algorithm::POPULATION_GRID_SIZE;
    let half_size_km = (site_area_km2.sqrt() / 2.0).max(0.5);
    let deg_per_km = 1.0 / 111.0;
    let half_deg = half_size_km * deg_per_km;
    let cells_per_side = ((half_deg * 2.0) / grid_size).ceil().max(10.0).min(50.0) as i32;
    let cell_area_km2 = (grid_size * 111.0).powi(2);
    let bw_deg = bandwidth_km * deg_per_km;

    let mut grid = Vec::new();

    for i in -cells_per_side/2..cells_per_side/2 {
        for j in -cells_per_side/2..cells_per_side/2 {
            let lon = center_lon + (i as f64 + 0.5) * grid_size;
            let lat = center_lat + (j as f64 + 0.5) * grid_size;

            let mut weighted_sum = 0.0;
            let mut weight_sum = 0.0;

            for (px, py, pv) in control_points {
                let dx = lon - px;
                let dy = lat - py;
                let d2 = dx*dx + dy*dy;
                let bw2 = bw_deg * bw_deg;
                let kernel = (-d2 / (2.0 * bw2)).exp();
                weighted_sum += pv * kernel;
                weight_sum += kernel;
            }

            let density = if weight_sum > 0.0 {
                weighted_sum / weight_sum
            } else {
                0.0
            };

            grid.push(PopulationGridCell {
                lon,
                lat,
                population: density * cell_area_km2,
                density,
                zone_type: None,
            });
        }
    }

    grid
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
    let b = algorithm::POPULATION_ALLOMETRIC_EXPONENT;
    let _scaling_factor = if site_area_km2 > 0.0 {
        total_population / site_area_km2.powf(b)
    } else {
        0.0
    };

    let mut zone_populations: Vec<ZonePopulation> = Vec::new();
    let mut total_zone_pop = 0.0_f64;
    let mut zone_densities: HashMap<String, f64> = HashMap::new();
    let mut zone_confidences: HashMap<String, f64> = HashMap::new();
    let mut control_points: Vec<(f64, f64, f64)> = Vec::new();

    for (zone_type, area_km2) in zones {
        let buildings_in_zone: Vec<_> = buildings.iter()
            .filter(|(_, _, bt)| bt == zone_type)
            .cloned()
            .collect();

        let (fused_density, conf, _method) = multi_source_fusion_density(
            zone_type,
            *area_km2,
            &buildings_in_zone,
            algorithm::POPULATION_PERSONS_PER_ROOM,
        );

        let pop = area_km2 * fused_density;
        let pop = if pop < 0.0 { 0.0 } else { pop };
        total_zone_pop += pop;
        zone_densities.insert(zone_type.clone(), fused_density);
        zone_confidences.insert(zone_type.clone(), conf);

        let sample_count = (area_km2 * 50.0).max(1.0).min(20.0) as usize;
        for k in 0..sample_count {
            let angle = (k as f64) * std::f64::consts::TAU / sample_count as f64;
            let r = (area_km2 / std::f64::consts::PI).sqrt() * 0.5;
            let deg_per_km = 1.0 / 111.0;
            let dx = r * angle.cos() * deg_per_km;
            let dy = r * angle.sin() * deg_per_km;
            let idx = (k + zone_type.len()) % zones.len();
            let angle_offset = (idx as f64) * std::f64::consts::TAU / zones.len().max(1) as f64;
            let radius = (site_area_km2 / std::f64::consts::PI).sqrt() * 0.4 * deg_per_km;
            let zx = center_lon + radius * angle_offset.cos() + dx;
            let zy = center_lat + radius * angle_offset.sin() + dy;
            control_points.push((zx, zy, fused_density));
        }
    }

    for (blon, blat, btype) in buildings {
        let density = zone_densities.get(btype).copied().unwrap_or(3000.0);
        control_points.push((*blon, *blat, density * 1.3));
    }

    if total_zone_pop > 0.0 && total_population > 0.0 {
        let ratio = total_population / total_zone_pop;
        for (zone_type, area_km2) in zones {
            let density = zone_densities.get(zone_type).copied().unwrap_or(500.0) * ratio;
            let pop = area_km2 * density;
            let percentage = (pop / total_population) * 100.0;
            zone_populations.push(ZonePopulation {
                zone_type: zone_type.clone(),
                population: pop,
                area_km2: *area_km2,
                density,
                percentage,
            });
        }

        for cp in control_points.iter_mut() {
            cp.2 *= ratio;
        }
    } else {
        for (zone_type, area_km2) in zones {
            let density = zone_densities.get(zone_type).copied().unwrap_or(500.0);
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
            zp.percentage = if total_zone_pop > 0.0 { (zp.population / total_zone_pop) * 100.0 } else { 0.0 };
        }
    }

    let grid_cells = if control_points.is_empty() {
        generate_population_grid_simple(center_lon, zones, &zone_densities)
    } else {
        let bw = (site_area_km2 / std::f64::consts::PI).sqrt() * 0.25;
        kernel_density_interpolation(center_lon, center_lat, site_area_km2, &control_points, bw.max(0.3))
    };

    let densities: Vec<f64> = grid_cells.iter().map(|c| c.density).collect();
    let avg_density = if densities.is_empty() { 0.0 } else { densities.iter().sum::<f64>() / densities.len() as f64 };
    let max_density = densities.iter().cloned().fold(0.0_f64, f64::max);

    let avg_conf: f64 = if zone_confidences.is_empty() {
        0.5
    } else {
        zone_confidences.values().sum::<f64>() / zone_confidences.len() as f64
    };

    let data_level = assess_data_completeness(zones, buildings, total_population > 0.0);
    let level_bonus = match data_level {
        DataCompleteness::High => 0.15,
        DataCompleteness::Medium => 0.05,
        DataCompleteness::Low => -0.1,
        DataCompleteness::VeryLow => -0.25,
    };

    let confidence = (avg_conf + level_bonus).max(0.1).min(0.98);

    PopulationAnalysisResult {
        site_id: 0,
        total_population: if total_population > 0.0 { total_population } else { total_zone_pop },
        population_density_avg: avg_density,
        population_density_max: max_density,
        model_type: "allometric_growth_multi_source".to_string(),
        confidence,
        grid_cells,
        zone_populations,
    }
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

    let mut control_points: Vec<(f64, f64, f64)> = Vec::new();
    let mut total_pop_from_buildings = 0.0_f64;

    for (lon, lat, btype) in buildings {
        let rooms = match btype.as_str() {
            "palace" => 20.0,
            "residential" => 4.0,
            "temple" => 2.0,
            "official" => 8.0,
            "workshop" => 3.0,
            "tomb" => 0.0,
            "storage" => 0.0,
            _ => 3.0,
        };
        let pop = rooms * persons_per_room;
        total_pop_from_buildings += pop;

        let area_per_building = 0.0002;
        let density = pop / area_per_building;
        control_points.push((*lon, *lat, density));
    }

    let mut zone_populations: Vec<ZonePopulation> = Vec::new();
    let mut total_zone_pop = 0.0_f64;
    let mut zone_densities: HashMap<String, f64> = HashMap::new();

    for (zone_type, area_km2) in zones {
        let buildings_in_zone: Vec<_> = buildings.iter()
            .filter(|(_, _, bt)| bt == zone_type)
            .cloned()
            .collect();

        let (fused_density, _conf, _method) = multi_source_fusion_density(
            zone_type,
            *area_km2,
            &buildings_in_zone,
            persons_per_room,
        );

        let pop = area_km2 * fused_density;
        total_zone_pop += pop;
        zone_densities.insert(zone_type.clone(), fused_density);
        zone_populations.push(ZonePopulation {
            zone_type: zone_type.clone(),
            population: pop,
            area_km2: *area_km2,
            density: fused_density,
            percentage: 0.0,
        });
    }

    for zp in &mut zone_populations {
        zp.percentage = if total_zone_pop > 0.0 { (zp.population / total_zone_pop) * 100.0 } else { 0.0 };
    }

    let grid_cells = if control_points.len() >= 3 {
        let bw = if site_area_km2 > 0.0 {
            (site_area_km2 / std::f64::consts::PI).sqrt() * 0.2
        } else {
            0.5
        };
        kernel_density_interpolation(center_lon, center_lat, site_area_km2.max(1.0), &control_points, bw.max(0.2))
    } else {
        generate_population_grid_simple(center_lon, zones, &zone_densities)
    };

    let densities: Vec<f64> = grid_cells.iter().map(|c| c.density).collect();
    let avg_density = if densities.is_empty() { 0.0 } else { densities.iter().sum::<f64>() / densities.len() as f64 };
    let max_density = densities.iter().cloned().fold(0.0_f64, f64::max);

    let data_level = assess_data_completeness(zones, buildings, false);
    let confidence = match data_level {
        DataCompleteness::High => 0.85,
        DataCompleteness::Medium => 0.7,
        DataCompleteness::Low => 0.55,
        DataCompleteness::VeryLow => 0.35,
    };

    PopulationAnalysisResult {
        site_id: 0,
        total_population: if total_pop_from_buildings > 0.0 { total_pop_from_buildings } else { total_zone_pop },
        population_density_avg: avg_density,
        population_density_max: max_density,
        model_type: "residential_density_kernel".to_string(),
        confidence,
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
    buildings: &[(f64, f64, String)],
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

        let buildings_in_zone: Vec<_> = buildings.iter()
            .filter(|(_, _, bt)| bt == zone_type)
            .cloned()
            .collect();

        let (fused_density, _conf, _method) = multi_source_fusion_density(
            zone_type,
            *area_km2,
            &buildings_in_zone,
            algorithm::POPULATION_PERSONS_PER_ROOM,
        );

        zone_centers.push((zl, zlt, fused_density, zone_type.clone()));
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
            let mut best_weight = 0.0;

            for (zl, zlt, density, zone_type) in &zone_centers {
                let dist = ((lon - zl).powi(2) + (lat - zlt).powi(2)).sqrt();
                let dist = dist.max(0.0001);
                let weight = 1.0 / dist.powi(2);

                weighted_density += density * weight;
                total_weight += weight;

                if weight > best_weight {
                    best_weight = weight;
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
        let buildings_in_zone: Vec<_> = buildings.iter()
            .filter(|(_, _, bt)| bt == zone_type)
            .cloned()
            .collect();
        let (fused_density, _conf, _) = multi_source_fusion_density(
            zone_type,
            *area_km2,
            &buildings_in_zone,
            algorithm::POPULATION_PERSONS_PER_ROOM,
        );
        let pop = area_km2 * fused_density;
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

    let data_level = assess_data_completeness(zones, buildings, total_population > 0.0);
    let confidence = match data_level {
        DataCompleteness::High => 0.8,
        DataCompleteness::Medium => 0.65,
        DataCompleteness::Low => 0.5,
        DataCompleteness::VeryLow => 0.3,
    };

    PopulationAnalysisResult {
        site_id: 0,
        total_population: final_pop,
        population_density_avg: avg_density,
        population_density_max: max_density,
        model_type: "idw_interpolation_multi_source".to_string(),
        confidence,
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
