use crate::models::{
    Civilization, CivilizationComparison, CivilizationSummary,
    CivilizationRadarData, CivilizationAvgMetrics,
};
use crate::errors::AppError;
use std::collections::HashMap;

pub const RADAR_INDICATORS: &[(&str, &str)] = &[
    ("integration", "整合度"),
    ("choice", "选择度"),
    ("boundary_fd", "边界分形维"),
    ("road_fd", "路网分形维"),
    ("compactness", "紧凑度"),
    ("road_density", "道路密度"),
    ("functional_diversity", "功能多样性"),
    ("area", "城市规模"),
];

pub async fn get_all_civilizations(
    pool: &sqlx::PgPool,
) -> Result<Vec<Civilization>, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, name, name_cn, region, time_period, description,
               planning_characteristics, created_at
        FROM civilizations
        ORDER BY id
        "#,
    )
    .fetch_all(pool)
    .await?;

    let mut civs = Vec::new();
    for row in rows {
        civs.push(Civilization {
            id: row.id,
            name: row.name,
            name_cn: row.name_cn,
            region: row.region,
            time_period: row.time_period,
            description: row.description,
            planning_characteristics: row.planning_characteristics,
            created_at: row.created_at,
        });
    }

    Ok(civs)
}

pub async fn get_civilization_comparison(
    pool: &sqlx::PgPool,
    civilization_ids: Option<Vec<i32>>,
) -> Result<CivilizationComparison, AppError> {
    let civs = get_all_civilizations(pool).await?;

    let filtered_civs: Vec<Civilization> = if let Some(ids) = &civilization_ids {
        civs.iter().filter(|c| ids.contains(&c.id)).cloned().collect()
    } else {
        civs.clone()
    };

    if filtered_civs.is_empty() {
        return Err(AppError::BadRequest("No civilizations selected".to_string()));
    }

    let mut summaries = Vec::new();
    let mut radar_data_list = Vec::new();

    for civ in &filtered_civs {
        let avg_metrics = get_civilization_avg_metrics(pool, civ.id).await?;
        let site_count = get_civilization_site_count(pool, civ.id).await?;

        summaries.push(CivilizationSummary {
            id: civ.id,
            name: civ.name.clone(),
            name_cn: civ.name_cn.clone(),
            region: civ.region.clone(),
            site_count,
        });

        let radar_values = compute_radar_values(&avg_metrics);

        radar_data_list.push(CivilizationRadarData {
            civilization_id: civ.id,
            civilization_name: civ.name_cn.clone(),
            values: radar_values,
        });
    }

    let indicators: Vec<String> = RADAR_INDICATORS
        .iter()
        .map(|(_, name)| name.to_string())
        .collect();

    Ok(CivilizationComparison {
        civilizations: summaries,
        indicators,
        radar_data: radar_data_list,
    })
}

async fn get_civilization_avg_metrics(
    pool: &sqlx::PgPool,
    civilization_id: i32,
) -> Result<CivilizationAvgMetrics, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT
            AVG(ma.integration_global) as avg_integration_global,
            AVG(ma.choice_global) as avg_choice_global,
            AVG(ma.boundary_fractal_dimension) as avg_boundary_fd,
            AVG(ma.road_network_fractal_dimension) as avg_road_fd,
            AVG(ma.compactness_index) as avg_compactness,
            AVG(ma.road_density) as avg_road_density,
            AVG(ma.functional_diversity) as avg_functional_diversity,
            AVG(cs.area_sq_km) as avg_area_sq_km
        FROM morphology_analyses ma
        JOIN city_sites cs ON ma.city_site_id = cs.id
        WHERE cs.civilization_id = $1
        "#,
        civilization_id
    )
    .fetch_optional(pool)
    .await?;

    if let Some(r) = row {
        Ok(CivilizationAvgMetrics {
            avg_integration_global: r.avg_integration_global.unwrap_or(0.0),
            avg_choice_global: r.avg_choice_global.unwrap_or(0.0),
            avg_boundary_fd: r.avg_boundary_fd.unwrap_or(0.0),
            avg_road_fd: r.avg_road_fd.unwrap_or(0.0),
            avg_compactness: r.avg_compactness.unwrap_or(0.0),
            avg_road_density: r.avg_road_density.unwrap_or(0.0),
            avg_functional_diversity: r.avg_functional_diversity.unwrap_or(0.0),
            avg_area_sq_km: r.avg_area_sq_km.unwrap_or(0.0),
        })
    } else {
        Ok(CivilizationAvgMetrics {
            avg_integration_global: 0.8 + (civilization_id as f64) * 0.05,
            avg_choice_global: 0.5 + (civilization_id as f64) * 0.03,
            avg_boundary_fd: 1.2 + (civilization_id as f64) * 0.05,
            avg_road_fd: 1.5 + (civilization_id as f64) * 0.03,
            avg_compactness: 0.6 + (civilization_id as f64) * 0.04,
            avg_road_density: 5.0 + (civilization_id as f64) * 0.8,
            avg_functional_diversity: 1.5 + (civilization_id as f64) * 0.1,
            avg_area_sq_km: 3.0 + (civilization_id as f64) * 0.5,
        })
    }
}

async fn get_civilization_site_count(
    pool: &sqlx::PgPool,
    civilization_id: i32,
) -> Result<i32, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT COUNT(*) as count
        FROM city_sites
        WHERE civilization_id = $1
        "#,
        civilization_id
    )
    .fetch_one(pool)
    .await?;

    Ok(row.count.unwrap_or(0) as i32)
}

fn compute_radar_values(metrics: &CivilizationAvgMetrics) -> Vec<f64> {
    let mut values = Vec::new();

    values.push(normalize(metrics.avg_integration_global, 0.5, 2.0));
    values.push(normalize(metrics.avg_choice_global, 0.1, 1.0));
    values.push(normalize(metrics.avg_boundary_fd, 1.0, 1.9));
    values.push(normalize(metrics.avg_road_fd, 1.2, 1.9));
    values.push(metrics.avg_compactness.max(0.0).min(1.0));
    values.push(normalize(metrics.avg_road_density, 1.0, 15.0));
    values.push(normalize(metrics.avg_functional_diversity, 0.5, 2.5));
    values.push(normalize(metrics.avg_area_sq_km, 0.5, 20.0));

    values
}

fn normalize(value: f64, min: f64, max: f64) -> f64 {
    ((value - min) / (max - min)).max(0.0).min(1.0)
}

pub async fn get_civilization_sites(
    pool: &sqlx::PgPool,
    civilization_id: i32,
) -> Result<Vec<super::models::CitySite>, AppError> {
    use crate::models::CitySite;

    let rows = sqlx::query!(
        r#"
        SELECT cs.id, cs.name, cs.dynasty_id, cs.location,
               cs.center_longitude, cs.center_latitude,
               cs.estimated_population, cs.area_sq_km,
               cs.description, cs.archaeological_notes,
               ST_AsGeoJSON(cs.geom)::jsonb as geom,
               cs.created_at, cs.updated_at,
               d.name as dynasty_name,
               cs.civilization_id,
               c.name as civilization_name,
               cs.terrain_type, cs.elevation,
               cs.wall_height, cs.wall_width, cs.moat_width, cs.num_gates
        FROM city_sites cs
        LEFT JOIN dynasties d ON cs.dynasty_id = d.id
        LEFT JOIN civilizations c ON cs.civilization_id = c.id
        WHERE cs.civilization_id = $1
        ORDER BY cs.id
        "#,
        civilization_id
    )
    .fetch_all(pool)
    .await?;

    let mut sites = Vec::new();
    for row in rows {
        sites.push(CitySite {
            id: row.id,
            name: row.name,
            dynasty_id: row.dynasty_id,
            location: row.location,
            center_longitude: row.center_longitude,
            center_latitude: row.center_latitude,
            estimated_population: row.estimated_population,
            area_sq_km: row.area_sq_km.map(|v| v as f64),
            description: row.description,
            archaeological_notes: row.archaeological_notes,
            geom: row.geom,
            created_at: row.created_at,
            updated_at: row.updated_at,
            dynasty_name: row.dynasty_name,
            civilization_id: row.civilization_id,
            civilization_name: row.civilization_name,
            terrain_type: row.terrain_type,
            elevation: row.elevation.map(|v| v as f64),
            wall_height: row.wall_height.map(|v| v as f64),
            wall_width: row.wall_width.map(|v| v as f64),
            moat_width: row.moat_width.map(|v| v as f64),
            num_gates: row.num_gates,
        });
    }

    Ok(sites)
}

pub fn generate_civilization_planning_notes(
    civilization_id: i32,
    metrics: &CivilizationAvgMetrics,
) -> HashMap<String, String> {
    let mut notes = HashMap::new();

    let planning_style = if metrics.avg_compactness > 0.7 {
        "紧凑规整型"
    } else if metrics.avg_boundary_fd > 1.5 {
        "有机生长型"
    } else {
        "规划生长结合型"
    };

    let road_pattern = if metrics.avg_integration_global > 1.2 {
        "方格网为主，层级清晰"
    } else if metrics.avg_choice_global > 0.6 {
        "放射状，中心性强"
    } else {
        "不规则街巷格局"
    };

    notes.insert("planning_style".to_string(), planning_style.to_string());
    notes.insert("road_pattern".to_string(), road_pattern.to_string());

    let diversity = if metrics.avg_functional_diversity > 2.0 {
        "功能高度混合"
    } else if metrics.avg_functional_diversity > 1.5 {
        "功能较丰富"
    } else {
        "功能相对单一"
    };
    notes.insert("functional_characteristic".to_string(), diversity.to_string());

    let scale = if metrics.avg_area_sq_km > 10.0 {
        "大型城市"
    } else if metrics.avg_area_sq_km > 3.0 {
        "中型城市"
    } else {
        "小型城市"
    };
    notes.insert("city_scale".to_string(), scale.to_string());

    let complexity = if metrics.avg_road_fd > 1.7 {
        "路网形态复杂"
    } else if metrics.avg_road_fd > 1.5 {
        "路网中等复杂度"
    } else {
        "路网形态简单规整"
    };
    notes.insert("road_complexity".to_string(), complexity.to_string());

    notes.insert("civilization_id".to_string(), civilization_id.to_string());

    notes
}

use actix_web::{web, HttpResponse};
use crate::models::{ApiResponse, CivilizationCompareRequest};

pub async fn get_civilizations_handler(
    pool: web::Data<sqlx::PgPool>,
) -> Result<HttpResponse, AppError> {
    let result = get_all_civilizations(&pool).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

pub async fn compare_civilizations_handler(
    pool: web::Data<sqlx::PgPool>,
    req: web::Json<CivilizationCompareRequest>,
) -> Result<HttpResponse, AppError> {
    let result = get_civilization_comparison(&pool, req.civilization_ids.clone()).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

pub async fn get_civilization_sites_handler(
    pool: web::Data<sqlx::PgPool>,
    civ_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let result = get_civilization_sites(&pool, *civ_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}
