use actix_web::{web, HttpResponse, Responder};
use serde_json::json;
use sqlx::PgPool;

use crate::models::*;
use crate::errors::AppError;

pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "success": true,
        "message": "古代城市遗址空间结构复原与形态演化分析系统 API 运行正常",
        "version": "1.0.0"
    }))
}

pub async fn get_dynasties(pool: web::Data<PgPool>) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, name, start_year, end_year, period, description, created_at
        FROM dynasties
        ORDER BY start_year ASC
        "#
    )
    .fetch_all(pool.get_ref())
    .await?;

    let dynasties: Vec<Dynasty> = rows
        .into_iter()
        .map(|row| Dynasty {
            id: row.id,
            name: row.name,
            start_year: row.start_year,
            end_year: row.end_year,
            period: row.period,
            description: row.description,
            created_at: row.created_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(dynasties)))
}

pub async fn get_city_sites(pool: web::Data<PgPool>) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT cs.id, cs.name, cs.dynasty_id, cs.location, cs.center_longitude,
               cs.center_latitude, cs.estimated_population, cs.area_sq_km,
               cs.description, cs.archaeological_notes,
               ST_AsGeoJSON(cs.geom)::jsonb as geom_json,
               cs.created_at, cs.updated_at,
               d.name as dynasty_name,
               cs.civilization_id, c.name as civilization_name,
               cs.terrain_type, cs.elevation,
               cs.wall_height, cs.wall_width, cs.moat_width, cs.num_gates
        FROM city_sites cs
        JOIN dynasties d ON cs.dynasty_id = d.id
        LEFT JOIN civilizations c ON cs.civilization_id = c.id
        ORDER BY d.start_year ASC, cs.name ASC
        "#
    )
    .fetch_all(pool.get_ref())
    .await?;

    let sites: Vec<CitySite> = rows
        .into_iter()
        .map(|row| CitySite {
            id: row.id,
            name: row.name,
            dynasty_id: row.dynasty_id,
            location: row.location,
            center_longitude: row.center_longitude,
            center_latitude: row.center_latitude,
            estimated_population: row.estimated_population,
            area_sq_km: row.area_sq_km,
            description: row.description,
            archaeological_notes: row.archaeological_notes,
            geom: row.geom_json.map(|j| serde_json::from_value(j).unwrap_or(json!({}))),
            created_at: row.created_at.map(|dt| dt.and_utc()),
            updated_at: row.updated_at.map(|dt| dt.and_utc()),
            dynasty_name: Some(row.dynasty_name),
            civilization_id: row.civilization_id,
            civilization_name: row.civilization_name,
            terrain_type: row.terrain_type,
            elevation: row.elevation.map(|v| v as f64),
            wall_height: row.wall_height.map(|v| v as f64),
            wall_width: row.wall_width.map(|v| v as f64),
            moat_width: row.moat_width.map(|v| v as f64),
            num_gates: row.num_gates,
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(sites)))
}

pub async fn get_city_site_by_id(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT cs.id, cs.name, cs.dynasty_id, cs.location, cs.center_longitude,
               cs.center_latitude, cs.estimated_population, cs.area_sq_km,
               cs.description, cs.archaeological_notes,
               ST_AsGeoJSON(cs.geom)::jsonb as geom_json,
               cs.created_at, cs.updated_at,
               d.name as dynasty_name,
               cs.civilization_id, c.name as civilization_name,
               cs.terrain_type, cs.elevation,
               cs.wall_height, cs.wall_width, cs.moat_width, cs.num_gates
        FROM city_sites cs
        JOIN dynasties d ON cs.dynasty_id = d.id
        LEFT JOIN civilizations c ON cs.civilization_id = c.id
        WHERE cs.id = $1
        "#,
        site_id.into_inner()
    )
    .fetch_optional(pool.get_ref())
    .await?;

    match row {
        Some(row) => {
            let site = CitySite {
                id: row.id,
                name: row.name,
                dynasty_id: row.dynasty_id,
                location: row.location,
                center_longitude: row.center_longitude,
                center_latitude: row.center_latitude,
                estimated_population: row.estimated_population,
                area_sq_km: row.area_sq_km,
                description: row.description,
                archaeological_notes: row.archaeological_notes,
                geom: row.geom_json.map(|j| serde_json::from_value(j).unwrap_or(json!({}))),
                created_at: row.created_at.map(|dt| dt.and_utc()),
                updated_at: row.updated_at.map(|dt| dt.and_utc()),
                dynasty_name: Some(row.dynasty_name),
                civilization_id: row.civilization_id,
                civilization_name: row.civilization_name,
                terrain_type: row.terrain_type,
                elevation: row.elevation.map(|v| v as f64),
                wall_height: row.wall_height.map(|v| v as f64),
                wall_width: row.wall_width.map(|v| v as f64),
                moat_width: row.moat_width.map(|v| v as f64),
                num_gates: row.num_gates,
            };
            Ok(HttpResponse::Ok().json(ApiResponse::success(site)))
        }
        None => Err(AppError::NotFound("City site not found".to_string())),
    }
}

pub async fn get_sites_by_dynasty(
    pool: web::Data<PgPool>,
    dynasty_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT cs.id, cs.name, cs.dynasty_id, cs.location, cs.center_longitude,
               cs.center_latitude, cs.estimated_population, cs.area_sq_km,
               cs.description, cs.archaeological_notes,
               ST_AsGeoJSON(cs.geom)::jsonb as geom_json,
               cs.created_at, cs.updated_at,
               d.name as dynasty_name,
               cs.civilization_id, c.name as civilization_name,
               cs.terrain_type, cs.elevation,
               cs.wall_height, cs.wall_width, cs.moat_width, cs.num_gates
        FROM city_sites cs
        JOIN dynasties d ON cs.dynasty_id = d.id
        LEFT JOIN civilizations c ON cs.civilization_id = c.id
        WHERE cs.dynasty_id = $1
        ORDER BY cs.name ASC
        "#,
        dynasty_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await?;

    let sites: Vec<CitySite> = rows
        .into_iter()
        .map(|row| CitySite {
            id: row.id,
            name: row.name,
            dynasty_id: row.dynasty_id,
            location: row.location,
            center_longitude: row.center_longitude,
            center_latitude: row.center_latitude,
            estimated_population: row.estimated_population,
            area_sq_km: row.area_sq_km,
            description: row.description,
            archaeological_notes: row.archaeological_notes,
            geom: row.geom_json.map(|j| serde_json::from_value(j).unwrap_or(json!({}))),
            created_at: row.created_at.map(|dt| dt.and_utc()),
            updated_at: row.updated_at.map(|dt| dt.and_utc()),
            dynasty_name: Some(row.dynasty_name),
            civilization_id: row.civilization_id,
            civilization_name: row.civilization_name,
            terrain_type: row.terrain_type,
            elevation: row.elevation.map(|v| v as f64),
            wall_height: row.wall_height.map(|v| v as f64),
            wall_width: row.wall_width.map(|v| v as f64),
            moat_width: row.moat_width.map(|v| v as f64),
            num_gates: row.num_gates,
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(sites)))
}

pub async fn get_functional_zones(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, city_site_id, zone_type, name, description,
               archaeological_findings, functional_inference, confidence_level,
               ST_AsGeoJSON(geom)::jsonb as geom_json, created_at
        FROM functional_zones
        WHERE city_site_id = $1
        ORDER BY zone_type, name
        "#,
        site_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await?;

    let zones: Vec<FunctionalZone> = rows
        .into_iter()
        .map(|row| FunctionalZone {
            id: row.id,
            city_site_id: row.city_site_id,
            zone_type: row.zone_type,
            name: row.name,
            description: row.description,
            archaeological_findings: row.archaeological_findings,
            functional_inference: row.functional_inference,
            confidence_level: row.confidence_level,
            geom: row.geom_json.map(|j| serde_json::from_value(j).unwrap_or(json!({}))),
            created_at: row.created_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(zones)))
}

pub async fn get_roads(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, city_site_id, road_name, road_type, width, description,
               ST_AsGeoJSON(geom)::jsonb as geom_json, created_at
        FROM roads
        WHERE city_site_id = $1
        ORDER BY road_name
        "#,
        site_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await?;

    let roads: Vec<Road> = rows
        .into_iter()
        .map(|row| Road {
            id: row.id,
            city_site_id: row.city_site_id,
            road_name: row.road_name,
            road_type: row.road_type,
            width: row.width,
            description: row.description,
            geom: row.geom_json.map(|j| serde_json::from_value(j).unwrap_or(json!({}))),
            created_at: row.created_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(roads)))
}

pub async fn get_buildings(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, city_site_id, building_type, name, area_sq_m, rooms_count,
               description, archaeological_findings,
               ST_AsGeoJSON(geom)::jsonb as geom_json, created_at
        FROM building_foundations
        WHERE city_site_id = $1
        ORDER BY building_type, name
        "#,
        site_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await?;

    let buildings: Vec<BuildingFoundation> = rows
        .into_iter()
        .map(|row| BuildingFoundation {
            id: row.id,
            city_site_id: row.city_site_id,
            building_type: row.building_type,
            name: row.name,
            area_sq_m: row.area_sq_m,
            rooms_count: row.rooms_count,
            description: row.description,
            archaeological_findings: row.archaeological_findings,
            geom: row.geom_json.map(|j| serde_json::from_value(j).unwrap_or(json!({}))),
            created_at: row.created_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(buildings)))
}

pub async fn get_population(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, city_site_id, estimate_year, population_min, population_max,
               population_mean, estimation_method, source, confidence_level, created_at
        FROM population_estimates
        WHERE city_site_id = $1
        ORDER BY estimate_year
        "#,
        site_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await?;

    let estimates: Vec<PopulationEstimate> = rows
        .into_iter()
        .map(|row| PopulationEstimate {
            id: row.id,
            city_site_id: row.city_site_id,
            estimate_year: row.estimate_year,
            population_min: row.population_min,
            population_max: row.population_max,
            population_mean: row.population_mean,
            estimation_method: row.estimation_method,
            source: row.source,
            confidence_level: row.confidence_level,
            created_at: row.created_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(estimates)))
}
