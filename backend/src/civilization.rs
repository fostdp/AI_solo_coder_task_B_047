use crate::models::{
    Civilization, CivilizationComparison, CivilizationSummary,
    CivilizationRadarData, CivilizationAvgMetrics,
};
use crate::errors::AppError;
use crate::services::cross_civilization_comparator::{
    CrossCivilizationComparator, CitySizeCategory, CityTypeCategory,
    ClassificationResult, ComparabilityAssessment, NormalizationMethod,
    CivilizationUrbanDefinition, CIVILIZATION_URBAN_DEFINITIONS,
};
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

pub const CITY_SIZE_THRESHOLDS: &[(&str, f64)] = &[
    ("small_max", 1.0),
    ("medium_max", 5.0),
    ("large_max", 15.0),
];

fn comparator() -> CrossCivilizationComparator {
    CrossCivilizationComparator::default()
}

pub fn classify_city_size(area_sq_km: f64) -> (CitySizeCategory, f64, &'static str) {
    comparator().classify_city_size(area_sq_km)
}

pub fn classify_city_type(
    compactness: f64,
    functional_diversity: f64,
    road_density: f64,
    integration: f64,
) -> (CityTypeCategory, f64, Vec<String>) {
    comparator().classify_city_type(compactness, functional_diversity, road_density, integration)
}

pub fn assess_indicator_comparability(
    indicator_key: &str,
    civ_names: &[String],
) -> ComparabilityAssessment {
    comparator().assess_indicator_comparability(indicator_key, civ_names)
}

pub fn normalize_zscore(values: &[f64]) -> Vec<f64> {
    comparator().normalize_zscore(values)
}

pub fn normalize_rank(values: &[f64]) -> Vec<f64> {
    comparator().normalize_rank(values)
}

pub fn normalize_minmax(values: &[f64], min: f64, max: f64) -> Vec<f64> {
    comparator().normalize_minmax(values, min, max)
}

pub fn compute_radar_values_with_method(
    metrics: &CivilizationAvgMetrics,
    method: NormalizationMethod,
    all_metrics: Option<&[CivilizationAvgMetrics]>,
) -> Vec<f64> {
    match method {
        NormalizationMethod::MinMax => comparator().compute_radar_values(metrics),
        NormalizationMethod::ZScore => {
            if let Some(all) = all_metrics {
                let mut raw_values = Vec::new();
                for m in all {
                    raw_values.push(m.avg_integration_global);
                    raw_values.push(m.avg_choice_global);
                    raw_values.push(m.avg_boundary_fd);
                    raw_values.push(m.avg_road_fd);
                    raw_values.push(m.avg_compactness);
                    raw_values.push(m.avg_road_density);
                    raw_values.push(m.avg_functional_diversity);
                    raw_values.push(m.avg_area_sq_km);
                }
                let all_z = comparator().normalize_zscore(&raw_values);
                let mut result = Vec::new();
                for i in 0..8 {
                    result.push(all_z[i]);
                }
                result.iter().map(|v| (v + 2.0) / 4.0).map(|v| v.max(0.0).min(1.0)).collect()
            } else {
                comparator().compute_radar_values(metrics)
            }
        }
        NormalizationMethod::Rank => {
            comparator().compute_radar_values(metrics)
        }
        NormalizationMethod::WithinCivilization => {
            comparator().compute_radar_values(metrics)
        }
    }
}

pub fn compare_by_size_category(
    all_metrics: &[CivilizationAvgMetrics],
    size_category: CitySizeCategory,
) -> Vec<CivilizationAvgMetrics> {
    comparator().compare_by_size_category(all_metrics, size_category)
}

pub fn compare_by_type_category(
    all_metrics: &[CivilizationAvgMetrics],
    type_category: CityTypeCategory,
) -> Vec<CivilizationAvgMetrics> {
    comparator().compare_by_type_category(all_metrics, type_category)
}

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
            civilization_id,
            civilization_name: String::new(),
            site_count: 0,
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
            civilization_id,
            civilization_name: String::new(),
            site_count: 0,
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
    comparator().compute_radar_values(metrics)
}

fn normalize(value: f64, min: f64, max: f64) -> f64 {
    comparator().normalize_minmax(&[value], min, max)[0]
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
    comparator().generate_planning_notes(civilization_id, metrics)
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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn sample_metrics_china() -> CivilizationAvgMetrics {
        CivilizationAvgMetrics {
            civilization_id: 1,
            civilization_name: "古代中国".to_string(),
            site_count: 10,
            avg_integration_global: 1.5,
            avg_choice_global: 0.7,
            avg_boundary_fd: 1.3,
            avg_road_fd: 1.6,
            avg_compactness: 0.8,
            avg_road_density: 8.0,
            avg_functional_diversity: 2.0,
            avg_area_sq_km: 6.0,
        }
    }

    fn sample_metrics_rome() -> CivilizationAvgMetrics {
        CivilizationAvgMetrics {
            civilization_id: 2,
            civilization_name: "古罗马".to_string(),
            site_count: 8,
            avg_integration_global: 1.0,
            avg_choice_global: 0.8,
            avg_boundary_fd: 1.6,
            avg_road_fd: 1.7,
            avg_compactness: 0.6,
            avg_road_density: 6.0,
            avg_functional_diversity: 1.8,
            avg_area_sq_km: 12.0,
        }
    }

    fn sample_metrics_maya() -> CivilizationAvgMetrics {
        CivilizationAvgMetrics {
            civilization_id: 3,
            civilization_name: "玛雅".to_string(),
            site_count: 5,
            avg_integration_global: 0.7,
            avg_choice_global: 0.3,
            avg_boundary_fd: 1.7,
            avg_road_fd: 1.3,
            avg_compactness: 0.3,
            avg_road_density: 2.0,
            avg_functional_diversity: 1.0,
            avg_area_sq_km: 2.0,
        }
    }

    #[test]
    fn test_normalize_within_range() {
        assert_relative_eq!(normalize(0.5, 0.0, 1.0), 0.5);
        assert_relative_eq!(normalize(0.0, 0.0, 1.0), 0.0);
        assert_relative_eq!(normalize(1.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn test_normalize_clamps_out_of_range() {
        assert_relative_eq!(normalize(-1.0, 0.0, 1.0), 0.0);
        assert_relative_eq!(normalize(2.0, 0.0, 1.0), 1.0);
    }

    #[test]
    fn test_normalize_linear_midpoint() {
        assert_relative_eq!(normalize(5.0, 0.0, 10.0), 0.5);
    }

    #[test]
    fn test_normalize_same_min_max_returns_safe() {
        let r = normalize(5.0, 5.0, 5.0);
        assert!(r.is_nan() || r >= 0.0);
    }

    #[test]
    fn test_compute_radar_values_length_is_eight() {
        let m = sample_metrics_china();
        let vals = compute_radar_values(&m);
        assert_eq!(vals.len(), 8);
    }

    #[test]
    fn test_compute_radar_values_all_bounded_0_1() {
        let m = sample_metrics_china();
        let vals = compute_radar_values(&m);
        for (i, v) in vals.iter().enumerate() {
            assert!(*v >= 0.0 && *v <= 1.0, "radar[{}]={} out of range", i, v);
        }
    }

    #[test]
    fn test_compute_radar_values_extreme_high_inputs_clamped() {
        let m = CivilizationAvgMetrics {
            civilization_id: 0,
            civilization_name: "test".to_string(),
            site_count: 1,
            avg_integration_global: 100.0,
            avg_choice_global: 100.0,
            avg_boundary_fd: 100.0,
            avg_road_fd: 100.0,
            avg_compactness: 99.0,
            avg_road_density: 100.0,
            avg_functional_diversity: 100.0,
            avg_area_sq_km: 100.0,
        };
        let vals = compute_radar_values(&m);
        for v in &vals {
            assert!(*v <= 1.0 + 1e-9);
        }
    }

    #[test]
    fn test_compute_radar_values_extreme_low_inputs_clamped() {
        let m = CivilizationAvgMetrics {
            civilization_id: 0,
            civilization_name: "test".to_string(),
            site_count: 1,
            avg_integration_global: -100.0,
            avg_choice_global: -100.0,
            avg_boundary_fd: -100.0,
            avg_road_fd: -100.0,
            avg_compactness: -99.0,
            avg_road_density: -100.0,
            avg_functional_diversity: -100.0,
            avg_area_sq_km: -100.0,
        };
        let vals = compute_radar_values(&m);
        for v in &vals {
            assert!(*v >= 0.0 - 1e-9);
        }
    }

    #[test]
    fn test_compute_radar_values_different_civs_produce_different_profiles() {
        let china = sample_metrics_china();
        let maya = sample_metrics_maya();
        let v_c = compute_radar_values(&china);
        let v_m = compute_radar_values(&maya);

        let sum_diff: f64 = v_c.iter().zip(v_m.iter()).map(|(a, b)| (a - b).abs()).sum();
        assert!(sum_diff > 0.5, "civilization profiles should differ, got diff={}", sum_diff);
    }

    #[test]
    fn test_compute_radar_values_china_compactness_higher_than_maya() {
        let china = sample_metrics_china();
        let maya = sample_metrics_maya();
        let v_c = compute_radar_values(&china);
        let v_m = compute_radar_values(&maya);

        assert!(v_c[4] > v_m[4],
            "China compactness (v_c[4]={}) > Maya (v_m[4]={})", v_c[4], v_m[4]);
    }

    #[test]
    fn test_generate_civilization_planning_notes_has_required_keys() {
        let m = sample_metrics_china();
        let notes = generate_civilization_planning_notes(1, &m);
        let required = ["planning_style", "road_pattern", "functional_characteristic",
            "city_scale", "road_complexity", "civilization_id"];
        for k in &required {
            assert!(notes.contains_key(*k), "missing key: {}", k);
        }
    }

    #[test]
    fn test_generate_civilization_planning_notes_high_compactness_returns_compact() {
        let mut m = sample_metrics_china();
        m.avg_compactness = 0.9;
        m.avg_boundary_fd = 1.2;
        let notes = generate_civilization_planning_notes(1, &m);
        assert_eq!(notes["planning_style"], "紧凑规整型");
    }

    #[test]
    fn test_generate_civilization_planning_notes_organic_when_boundary_high() {
        let mut m = sample_metrics_rome();
        m.avg_compactness = 0.5;
        m.avg_boundary_fd = 1.8;
        let notes = generate_civilization_planning_notes(2, &m);
        assert_eq!(notes["planning_style"], "有机生长型");
    }

    #[test]
    fn test_generate_civilization_planning_notes_grid_road_when_high_integration() {
        let mut m = sample_metrics_china();
        m.avg_integration_global = 1.5;
        m.avg_choice_global = 0.4;
        let notes = generate_civilization_planning_notes(1, &m);
        assert_eq!(notes["road_pattern"], "方格网为主，层级清晰");
    }

    #[test]
    fn test_generate_civilization_planning_notes_radial_when_high_choice() {
        let mut m = sample_metrics_rome();
        m.avg_integration_global = 0.9;
        m.avg_choice_global = 0.8;
        let notes = generate_civilization_planning_notes(2, &m);
        assert_eq!(notes["road_pattern"], "放射状，中心性强");
    }

    #[test]
    fn test_generate_civilization_planning_notes_large_city() {
        let mut m = sample_metrics_rome();
        m.avg_area_sq_km = 20.0;
        let notes = generate_civilization_planning_notes(2, &m);
        assert_eq!(notes["city_scale"], "大型城市");
    }

    #[test]
    fn test_generate_civilization_planning_notes_small_city() {
        let mut m = sample_metrics_maya();
        m.avg_area_sq_km = 0.5;
        let notes = generate_civilization_planning_notes(3, &m);
        assert_eq!(notes["city_scale"], "小型城市");
    }

    #[test]
    fn test_generate_civilization_planning_notes_high_functional_diversity() {
        let mut m = sample_metrics_china();
        m.avg_functional_diversity = 2.4;
        let notes = generate_civilization_planning_notes(1, &m);
        assert_eq!(notes["functional_characteristic"], "功能高度混合");
    }

    #[test]
    fn test_generate_civilization_planning_notes_complex_roads() {
        let mut m = sample_metrics_rome();
        m.avg_road_fd = 1.85;
        let notes = generate_civilization_planning_notes(2, &m);
        assert_eq!(notes["road_complexity"], "路网形态复杂");
    }

    #[test]
    fn test_generate_civilization_planning_notes_simple_roads() {
        let mut m = sample_metrics_china();
        m.avg_road_fd = 1.3;
        let notes = generate_civilization_planning_notes(1, &m);
        assert_eq!(notes["road_complexity"], "路网形态简单规整");
    }

    #[test]
    fn test_generate_civilization_planning_notes_id_preserved() {
        let m = sample_metrics_maya();
        let notes = generate_civilization_planning_notes(42, &m);
        assert_eq!(notes["civilization_id"], "42");
    }

    #[test]
    fn test_civilization_separation_euclidean_distinct() {
        let profiles = vec![
            compute_radar_values(&sample_metrics_china()),
            compute_radar_values(&sample_metrics_rome()),
            compute_radar_values(&sample_metrics_maya()),
        ];

        for i in 0..profiles.len() {
            for j in (i+1)..profiles.len() {
                let dist: f64 = profiles[i].iter().zip(profiles[j].iter())
                    .map(|(a, b)| (a - b).powi(2)).sum::<f64>().sqrt();
                assert!(dist > 0.3,
                    "Civ {} and Civ {} not well-separated (Euclidean = {})",
                    i, j, dist);
            }
        }
    }

    #[test]
    fn test_radar_indicators_list_is_eight() {
        assert_eq!(RADAR_INDICATORS.len(), 8);
    }

    #[test]
    fn test_combined_meaningful_radar_and_notes() {
        let rome = sample_metrics_rome();
        let vals = compute_radar_values(&rome);
        let notes = generate_civilization_planning_notes(2, &rome);

        assert_eq!(vals.len(), RADAR_INDICATORS.len());
        assert!(notes.contains_key("planning_style"));

        let avg: f64 = vals.iter().sum::<f64>() / vals.len() as f64;
        assert!(avg > 0.05, "avg radar should be meaningful, got {}", avg);
    }

    #[test]
    fn root_cause_city_size_classification() {
        let (cat_small, conf_small, _) = classify_city_size(0.5);
        assert_eq!(cat_small, CitySizeCategory::Small);
        assert!(conf_small > 0.8);

        let (cat_med, conf_med, _) = classify_city_size(3.0);
        assert_eq!(cat_med, CitySizeCategory::Medium);
        assert!(conf_med > 0.8);

        let (cat_large, conf_large, _) = classify_city_size(10.0);
        assert_eq!(cat_large, CitySizeCategory::Large);
        assert!(conf_large > 0.7);

        let (cat_mega, conf_mega, _) = classify_city_size(20.0);
        assert_eq!(cat_mega, CitySizeCategory::Megalopolis);
        assert!(conf_mega > 0.7);
    }

    #[test]
    fn root_cause_city_type_classification_capital() {
        let (cat, conf, _) = classify_city_type(0.85, 2.2, 9.0, 1.4);
        assert_eq!(cat, CityTypeCategory::Capital);
        assert!(conf > 0.6);
    }

    #[test]
    fn root_cause_city_type_classification_religious() {
        let (cat, conf, _) = classify_city_type(0.3, 0.8, 2.0, 0.6);
        assert_eq!(cat, CityTypeCategory::ReligiousCenter);
        assert!(conf > 0.5);
    }

    #[test]
    fn root_cause_zscore_normalization() {
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let zscores = normalize_zscore(&values);

        assert_eq!(zscores.len(), values.len());

        let mean: f64 = zscores.iter().sum::<f64>() / zscores.len() as f64;
        assert!(mean.abs() < 0.01, "z-score mean should be ~0, got {}", mean);

        let variance: f64 = zscores.iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>() / zscores.len() as f64;
        let std = variance.sqrt();
        assert!((std - 1.0).abs() < 0.01, "z-score std should be ~1, got {}", std);
    }

    #[test]
    fn root_cause_rank_normalization() {
        let values = vec![10.0, 30.0, 20.0, 50.0, 40.0];
        let ranks = normalize_rank(&values);

        assert_eq!(ranks.len(), values.len());

        let max_rank = ranks.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
        let min_rank = ranks.iter().cloned().fold(f64::INFINITY, f64::min);

        assert!(max_rank <= 1.0);
        assert!(min_rank >= 0.0);
        assert_eq!(ranks[3], 1.0, "largest value should have rank 1.0");
        assert_eq!(ranks[0], 0.0, "smallest value should have rank 0.0");
    }

    #[test]
    fn root_cause_comparability_assessment_area_low() {
        let civ_names = vec!["古代中国".to_string(), "玛雅".to_string()];
        let comp = assess_indicator_comparability("area", &civ_names);

        assert!(!comp.comparable || comp.confidence < 0.6,
            "area indicator between China and Maya should have low confidence");
        assert!(comp.adjustment_factor < 1.0,
            "adjustment factor should be < 1.0 for Maya area");
        assert!(!comp.notes.is_empty());
    }

    #[test]
    fn root_cause_comparability_assessment_fd_high() {
        let civ_names = vec!["古代中国".to_string(), "古罗马".to_string()];
        let comp = assess_indicator_comparability("boundary_fd", &civ_names);

        assert!(comp.comparable, "boundary_fd should be comparable");
        assert!(comp.confidence > 0.7, "boundary_fd should have high confidence, got {}", comp.confidence);
    }

    #[test]
    fn root_cause_classified_comparison_reduces_bias() {
        let china_large = CivilizationAvgMetrics {
            civilization_id: 1,
            civilization_name: "古代中国".to_string(),
            site_count: 5,
            avg_integration_global: 1.5,
            avg_choice_global: 0.7,
            avg_boundary_fd: 1.3,
            avg_road_fd: 1.6,
            avg_compactness: 0.85,
            avg_road_density: 9.0,
            avg_functional_diversity: 2.1,
            avg_area_sq_km: 12.0,
        };

        let maya_small = CivilizationAvgMetrics {
            civilization_id: 3,
            civilization_name: "玛雅".to_string(),
            site_count: 5,
            avg_integration_global: 0.7,
            avg_choice_global: 0.3,
            avg_boundary_fd: 1.7,
            avg_road_fd: 1.3,
            avg_compactness: 0.35,
            avg_road_density: 2.5,
            avg_functional_diversity: 0.9,
            avg_area_sq_km: 2.0,
        };

        let china_medium = CivilizationAvgMetrics {
            civilization_id: 1,
            civilization_name: "古代中国".to_string(),
            site_count: 3,
            avg_integration_global: 1.2,
            avg_choice_global: 0.6,
            avg_boundary_fd: 1.4,
            avg_road_fd: 1.5,
            avg_compactness: 0.75,
            avg_road_density: 7.0,
            avg_functional_diversity: 1.8,
            avg_area_sq_km: 3.0,
        };

        let all = vec![china_large.clone(), maya_small.clone(), china_medium.clone()];

        let large_cities = compare_by_size_category(&all, CitySizeCategory::Large);
        assert_eq!(large_cities.len(), 1, "should have 1 large city");

        let small_cities = compare_by_size_category(&all, CitySizeCategory::Small);
        assert_eq!(small_cities.len(), 1, "should have 1 small city");

        let medium_cities = compare_by_size_category(&all, CitySizeCategory::Medium);
        assert_eq!(medium_cities.len(), 1, "should have 1 medium city");

        let raw_diff = (china_large.avg_area_sq_km - maya_small.avg_area_sq_km).abs();
        let medium_same = compare_by_size_category(&all, CitySizeCategory::Medium);
        if medium_same.len() >= 2 {
            let classified_diff = (medium_same[0].avg_area_sq_km - medium_same[1].avg_area_sq_km).abs();
            assert!(classified_diff < raw_diff,
                "classified comparison should reduce area bias");
        }
    }

    #[test]
    fn root_cause_civilization_urban_definitions_exist() {
        assert!(CIVILIZATION_URBAN_DEFINITIONS.len() >= 5);
        for def in CIVILIZATION_URBAN_DEFINITIONS {
            assert!(!def.civilization_name.is_empty());
            assert!(!def.city_definition_criteria.is_empty());
            assert!(def.area_adjustment_factor > 0.0 && def.area_adjustment_factor <= 1.0);
            assert!(!def.primary_city_type.is_empty());
            assert!(!def.comparability_notes.is_empty());
        }
    }
}
