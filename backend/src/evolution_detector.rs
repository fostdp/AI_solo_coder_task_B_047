use actix_web::{web, HttpResponse};
use serde_json::json;
use sqlx::PgPool;

use crate::models::*;
use crate::errors::AppError;
use crate::mann_kendall::*;
use crate::config::algorithm::*;
use crate::metrics;

pub async fn analyze_trends(
    pool: web::Data<PgPool>,
    req: web::Json<TrendAnalysisRequest>,
) -> Result<HttpResponse, AppError> {
    let indicator = &req.indicator;
    tracing::info!(indicator = %indicator, "Starting Mann-Kendall trend analysis");
    metrics::inc_mk_test();

    let rows = sqlx::query!(
        r#"
        SELECT d.start_year, d.name as dynasty_name,
               CASE $1
                   WHEN 'integration_global' THEN ma.integration_global
                   WHEN 'integration_local' THEN ma.integration_local
                   WHEN 'choice_global' THEN ma.choice_global
                   WHEN 'choice_local' THEN ma.choice_local
                   WHEN 'mean_depth' THEN ma.mean_depth
                   WHEN 'connectivity' THEN ma.connectivity
                   WHEN 'boundary_fractal_dimension' THEN ma.boundary_fractal_dimension
                   WHEN 'road_network_fractal_dimension' THEN ma.road_network_fractal_dimension
                   WHEN 'compactness_index' THEN ma.compactness_index
                   WHEN 'elongation_ratio' THEN ma.elongation_ratio
                   WHEN 'road_density' THEN ma.road_density
                   WHEN 'intersection_density' THEN ma.intersection_density
                   WHEN 'functional_diversity' THEN ma.functional_diversity
                   WHEN 'functional_mixing' THEN ma.functional_mixing
                   ELSE ma.integration_global
               END as value
        FROM morphology_analyses ma
        JOIN city_sites cs ON ma.city_site_id = cs.id
        JOIN dynasties d ON cs.dynasty_id = d.id
        ORDER BY d.start_year ASC
        "#,
        indicator
    )
    .fetch_all(pool.get_ref())
    .await?;

    let values: Vec<f64> = rows
        .iter()
        .filter_map(|r| r.value)
        .collect();

    let years: Vec<i32> = rows
        .iter()
        .map(|r| r.start_year)
        .collect();

    let dynasty_names: Vec<String> = rows
        .iter()
        .map(|r| r.dynasty_name.clone())
        .collect();

    let mk_result = mann_kendall_test(&values, MK_ALPHA);

    let time_points_json = json!(years);
    let values_json = json!(values);

    let result = sqlx::query!(
        r#"
        INSERT INTO evolution_trends
        (analysis_name, indicator_name, mk_statistic, mk_p_value, mk_z_score,
         sen_slope, trend_direction, trend_significance, time_points, values, description)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9::jsonb, $10::jsonb, $11)
        RETURNING id, analysis_name, indicator_name, mk_statistic, mk_p_value, mk_z_score,
                  sen_slope, trend_direction, trend_significance,
                  time_points, values, description, created_at
        "#,
        format!("{} 演化趋势分析", indicator),
        indicator,
        mk_result.s,
        mk_result.p_value,
        mk_result.z_score,
        mk_result.sen_slope,
        mk_result.trend_direction,
        mk_result.trend_significance,
        time_points_json,
        values_json,
        Some(format!(
            "基于Mann-Kendall检验的{}演化趋势分析，涉及{}个朝代：{}",
            indicator,
            dynasty_names.len(),
            dynasty_names.join(", ")
        ))
    )
    .fetch_one(pool.get_ref())
    .await?;

    let trend = EvolutionTrend {
        id: result.id,
        analysis_name: result.analysis_name,
        indicator_name: result.indicator_name,
        mk_statistic: result.mk_statistic,
        mk_p_value: result.mk_p_value,
        mk_z_score: result.mk_z_score,
        sen_slope: result.sen_slope,
        trend_direction: result.trend_direction,
        trend_significance: result.trend_significance,
        time_points: result.time_points.map(|j| serde_json::from_value(j).unwrap_or(json!([]))),
        values: result.values.map(|j| serde_json::from_value(j).unwrap_or(json!([]))),
        description: result.description,
        created_at: result.created_at.map(|dt| dt.and_utc()),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(trend)))
}

pub async fn get_trends(pool: web::Data<PgPool>) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, analysis_name, indicator_name, mk_statistic, mk_p_value, mk_z_score,
               sen_slope, trend_direction, trend_significance,
               time_points, values, description, created_at
        FROM evolution_trends
        ORDER BY created_at DESC
        LIMIT 20
        "#
    )
    .fetch_all(pool.get_ref())
    .await?;

    let trends: Vec<EvolutionTrend> = rows
        .into_iter()
        .map(|row| EvolutionTrend {
            id: row.id,
            analysis_name: row.analysis_name,
            indicator_name: row.indicator_name,
            mk_statistic: row.mk_statistic,
            mk_p_value: row.mk_p_value,
            mk_z_score: row.mk_z_score,
            sen_slope: row.sen_slope,
            trend_direction: row.trend_direction,
            trend_significance: row.trend_significance,
            time_points: row.time_points.map(|j| serde_json::from_value(j).unwrap_or(json!([]))),
            values: row.values.map(|j| serde_json::from_value(j).unwrap_or(json!([]))),
            description: row.description,
            created_at: row.created_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(trends)))
}

pub async fn compare_sites(
    pool: web::Data<PgPool>,
    req: web::Json<CompareRequest>,
) -> Result<HttpResponse, AppError> {
    let mut comparisons = Vec::new();

    for &site_id in &req.site_ids {
        let site_row = sqlx::query!(
            r#"
            SELECT cs.id, cs.name, d.name as dynasty_name, d.start_year,
                   cs.estimated_population, cs.area_sq_km,
                   ST_AsGeoJSON(cs.geom)::jsonb as geom_json
            FROM city_sites cs
            JOIN dynasties d ON cs.dynasty_id = d.id
            WHERE cs.id = $1
            "#,
            site_id
        )
        .fetch_optional(pool.get_ref())
        .await?;

        if let Some(site) = site_row {
            let morph_row = sqlx::query!(
                r#"
                SELECT * FROM morphology_analyses
                WHERE city_site_id = $1
                ORDER BY analysis_date DESC
                LIMIT 1
                "#,
                site_id
            )
            .fetch_optional(pool.get_ref())
            .await?;

            let site_data = json!({
                "id": site.id,
                "name": site.name,
                "dynasty": site.dynasty_name,
                "start_year": site.start_year,
                "population": site.estimated_population,
                "area_sq_km": site.area_sq_km,
                "morphology": morph_row.map(|m| {
                    json!({
                        "integration_global": m.integration_global,
                        "choice_global": m.choice_global,
                        "boundary_fractal_dimension": m.boundary_fractal_dimension,
                        "road_network_fractal_dimension": m.road_network_fractal_dimension,
                        "compactness_index": m.compactness_index,
                        "elongation_ratio": m.elongation_ratio,
                        "functional_diversity": m.functional_diversity
                    })
                })
            });

            comparisons.push(site_data);
        }
    }

    Ok(HttpResponse::Ok().json(ApiResponse::success(comparisons)))
}
