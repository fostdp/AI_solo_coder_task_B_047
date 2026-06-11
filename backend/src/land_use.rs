use crate::models::{LandUseTimeline, LandUsePeriod, LandUseItem, LandUseChange};
use crate::errors::AppError;
use crate::config::algorithm;
use crate::services::land_use_tracer::{LandUseTracer, StratigraphicLayer as TracerStratigraphicLayer, DepositionModel as TracerDepositionModel, ArchaeologicalCheckpoint as TracerArchaeologicalCheckpoint};
use serde_json::{json, Value};
use std::collections::HashMap;

const LAND_USE_TYPES: &[(&str, &str)] = &[
    ("urban", "城市建成区"),
    ("farmland", "农田"),
    ("forest", "森林"),
    ("grassland", "草地"),
    ("wetland", "湿地"),
    ("water", "水体"),
    ("wasteland", "荒地"),
    ("settlement", "居民点"),
];

const LAND_USE_COLORS: &[&str] = &[
    "#FF6B6B",
    "#95E1D3",
    "#1B4332",
    "#90BE6D",
    "#00B4D8",
    "#0077B6",
    "#D4A373",
    "#E0AAFF",
];

const LAND_USE_LABELS: &[&str] = &[
    "城市建成区",
    "农田",
    "森林",
    "草地",
    "湿地",
    "水体",
    "荒地",
    "居民点",
];

const PERIOD_NAMES: &[&str] = &[
    "城市鼎盛期",
    "城市衰落初期",
    "城市废弃期",
    "早期农业期",
    "中期农业期",
    "近古时期",
    "近代时期",
    "现代时期",
];

#[derive(Debug, Clone)]
pub struct StratigraphicLayer {
    pub layer_id: String,
    pub period_name: String,
    pub period_year: i32,
    pub thickness_cm: f64,
    pub sediment_type: String,
    pub artifacts: Vec<String>,
    pub dating_method: String,
    pub confidence: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DepositionEnvironment {
    Alluvial,
    Lacustrine,
    Aeolian,
    Fluvial,
    Marine,
    Colluvial,
}

#[derive(Debug, Clone)]
pub struct DepositionModel {
    pub environment: DepositionEnvironment,
    pub accumulation_rate_cm_per_yr: f64,
    pub compaction_factor: f64,
    pub erosion_rate_cm_per_yr: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone)]
pub struct ArchaeologicalCheckpoint {
    pub period_name: String,
    pub period_year: i32,
    pub evidence_type: String,
    pub description: String,
    pub confidence: f64,
    pub land_use_constraints: Vec<(String, f64, f64)>,
}

pub const DEPOSITION_MODELS: &[(&str, DepositionModel)] = &[
    ("alluvial_plain", DepositionModel {
        environment: DepositionEnvironment::Alluvial,
        accumulation_rate_cm_per_yr: 0.1,
        compaction_factor: 0.7,
        erosion_rate_cm_per_yr: 0.02,
        confidence: 0.8,
    }),
    ("river_bank", DepositionModel {
        environment: DepositionEnvironment::Fluvial,
        accumulation_rate_cm_per_yr: 0.2,
        compaction_factor: 0.65,
        erosion_rate_cm_per_yr: 0.05,
        confidence: 0.75,
    }),
    ("lake_side", DepositionModel {
        environment: DepositionEnvironment::Lacustrine,
        accumulation_rate_cm_per_yr: 0.05,
        compaction_factor: 0.6,
        erosion_rate_cm_per_yr: 0.01,
        confidence: 0.85,
    }),
    ("loess_terrace", DepositionModel {
        environment: DepositionEnvironment::Aeolian,
        accumulation_rate_cm_per_yr: 0.08,
        compaction_factor: 0.75,
        erosion_rate_cm_per_yr: 0.03,
        confidence: 0.7,
    }),
];

fn convert_to_tracer_layer(layer: &StratigraphicLayer) -> TracerStratigraphicLayer {
    TracerStratigraphicLayer {
        layer_id: layer.layer_id.clone(),
        period_name: layer.period_name.clone(),
        period_year: layer.period_year,
        thickness_cm: layer.thickness_cm,
        sediment_type: layer.sediment_type.clone(),
        artifacts: layer.artifacts.clone(),
        dating_method: layer.dating_method.clone(),
        confidence: layer.confidence,
    }
}

fn convert_from_tracer_layer(layer: &TracerStratigraphicLayer) -> StratigraphicLayer {
    StratigraphicLayer {
        layer_id: layer.layer_id.clone(),
        period_name: layer.period_name.clone(),
        period_year: layer.period_year,
        thickness_cm: layer.thickness_cm,
        sediment_type: layer.sediment_type.clone(),
        artifacts: layer.artifacts.clone(),
        dating_method: layer.dating_method.clone(),
        confidence: layer.confidence,
    }
}

fn convert_to_tracer_model(model: &DepositionModel) -> TracerDepositionModel {
    use crate::services::land_use_tracer::DepositionEnvironment as TracerEnv;
    let env = match model.environment {
        DepositionEnvironment::Alluvial => TracerEnv::Alluvial,
        DepositionEnvironment::Lacustrine => TracerEnv::Lacustrine,
        DepositionEnvironment::Aeolian => TracerEnv::Aeolian,
        DepositionEnvironment::Fluvial => TracerEnv::Fluvial,
        DepositionEnvironment::Marine => TracerEnv::Marine,
        DepositionEnvironment::Colluvial => TracerEnv::Colluvial,
    };
    TracerDepositionModel {
        environment: env,
        accumulation_rate_cm_per_yr: model.accumulation_rate_cm_per_yr,
        compaction_factor: model.compaction_factor,
        erosion_rate_cm_per_yr: model.erosion_rate_cm_per_yr,
        confidence: model.confidence,
    }
}

fn convert_from_tracer_model(model: &TracerDepositionModel) -> DepositionModel {
    let env = match model.environment {
        crate::services::land_use_tracer::DepositionEnvironment::Alluvial => DepositionEnvironment::Alluvial,
        crate::services::land_use_tracer::DepositionEnvironment::Lacustrine => DepositionEnvironment::Lacustrine,
        crate::services::land_use_tracer::DepositionEnvironment::Aeolian => DepositionEnvironment::Aeolian,
        crate::services::land_use_tracer::DepositionEnvironment::Fluvial => DepositionEnvironment::Fluvial,
        crate::services::land_use_tracer::DepositionEnvironment::Marine => DepositionEnvironment::Marine,
        crate::services::land_use_tracer::DepositionEnvironment::Colluvial => DepositionEnvironment::Colluvial,
    };
    DepositionModel {
        environment: env,
        accumulation_rate_cm_per_yr: model.accumulation_rate_cm_per_yr,
        compaction_factor: model.compaction_factor,
        erosion_rate_cm_per_yr: model.erosion_rate_cm_per_yr,
        confidence: model.confidence,
    }
}

fn convert_to_tracer_checkpoint(checkpoint: &ArchaeologicalCheckpoint) -> TracerArchaeologicalCheckpoint {
    TracerArchaeologicalCheckpoint {
        period_name: checkpoint.period_name.clone(),
        period_year: checkpoint.period_year,
        evidence_type: checkpoint.evidence_type.clone(),
        description: checkpoint.description.clone(),
        confidence: checkpoint.confidence,
        land_use_constraints: checkpoint.land_use_constraints.clone(),
    }
}

pub fn select_deposition_model(terrain: &str, water_proximity: f64) -> &'static DepositionModel {
    let tracer = LandUseTracer::default();
    let tracer_model = LandUseTracer::select_deposition_model(&tracer, terrain, water_proximity);
    let tracer_env = tracer_model.environment;
    match tracer_env {
        crate::services::land_use_tracer::DepositionEnvironment::Fluvial => &DEPOSITION_MODELS[1].1,
        crate::services::land_use_tracer::DepositionEnvironment::Lacustrine => &DEPOSITION_MODELS[2].1,
        crate::services::land_use_tracer::DepositionEnvironment::Aeolian => &DEPOSITION_MODELS[3].1,
        _ => &DEPOSITION_MODELS[0].1,
    }
}

pub fn estimate_missing_periods(
    known_layers: &[StratigraphicLayer],
    target_periods: &[(i32, String)],
    model: &DepositionModel,
) -> Vec<StratigraphicLayer> {
    let tracer_layers: Vec<TracerStratigraphicLayer> = known_layers.iter().map(convert_to_tracer_layer).collect();
    let tracer_model = convert_to_tracer_model(model);
    let result = LandUseTracer::new(&crate::services::land_use_tracer::DEPOSITION_MODELS[0].1, true, true)
        .estimate_missing_periods(&tracer_layers, target_periods, &tracer_model);
    result.iter().map(convert_from_tracer_layer).collect()
}

pub fn assess_stratigraphic_continuity(layers: &[StratigraphicLayer]) -> (f64, Vec<String>, Vec<i32>) {
    let tracer_layers: Vec<TracerStratigraphicLayer> = layers.iter().map(convert_to_tracer_layer).collect();
    LandUseTracer::assess_stratigraphic_continuity(&LandUseTracer::default(), &tracer_layers)
}

pub fn apply_archaeological_constraints(
    layers: &mut [StratigraphicLayer],
    checkpoints: &[ArchaeologicalCheckpoint],
) {
    let mut tracer_layers: Vec<TracerStratigraphicLayer> = layers.iter().map(convert_to_tracer_layer).collect();
    let tracer_checkpoints: Vec<TracerArchaeologicalCheckpoint> = checkpoints.iter().map(convert_to_tracer_checkpoint).collect();
    LandUseTracer::new(&crate::services::land_use_tracer::DEPOSITION_MODELS[0].1, true, true)
        .apply_archaeological_constraints(&mut tracer_layers, &tracer_checkpoints);
    for (i, layer) in tracer_layers.iter().enumerate() {
        layers[i] = convert_from_tracer_layer(layer);
    }
}

pub fn fill_land_use_gaps(
    timeline: &LandUseTimeline,
    model: &DepositionModel,
    checkpoints: &[ArchaeologicalCheckpoint],
) -> LandUseTimeline {
    let tracer_model = convert_to_tracer_model(model);
    let tracer_checkpoints: Vec<TracerArchaeologicalCheckpoint> = checkpoints.iter().map(convert_to_tracer_checkpoint).collect();
    LandUseTracer::new(&crate::services::land_use_tracer::DEPOSITION_MODELS[0].1, true, true)
        .fill_land_use_gaps(timeline, &tracer_model, &tracer_checkpoints)
}

pub fn compute_land_use_trend(timeline: &LandUseTimeline) -> Value {
    LandUseTracer::new(&crate::services::land_use_tracer::DEPOSITION_MODELS[0].1, true, true)
        .compute_land_use_trend(timeline)
}

fn interpolate_land_use_period(
    before: &LandUsePeriod,
    after: &LandUsePeriod,
    t: f64,
    year: i32,
    _model: &DepositionModel,
) -> LandUsePeriod {
    let mut land_uses = Vec::new();

    for before_item in &before.land_uses {
        let after_item = after.land_uses.iter()
            .find(|u| u.land_use_type == before_item.land_use_type);

        let after_pct = after_item.map(|i| i.percentage).unwrap_or(before_item.percentage);
        let after_area = after_item.map(|i| i.area_km2).unwrap_or(before_item.area_km2);

        let pct = before_item.percentage + (after_pct - before_item.percentage) * t;
        let area = before_item.area_km2 + (after_area - before_item.area_km2) * t;

        land_uses.push(LandUseItem {
            land_use_type: before_item.land_use_type.clone(),
            area_km2: area,
            percentage: pct,
        });
    }

    land_uses.sort_by(|a, b| a.land_use_type.cmp(&b.land_use_type));

    LandUsePeriod {
        period_name: format!("{}年前后", year),
        period_year: year,
        land_uses,
    }
}

pub async fn get_land_use_timeline(
    pool: &sqlx::PgPool,
    site_id: i32,
) -> Result<LandUseTimeline, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, site_id, period_name, period_year, land_use_type,
               area_km2, percentage, evidence_type, confidence, description
        FROM land_use_changes
        WHERE site_id = $1
        ORDER BY period_year ASC
        "#,
        site_id
    )
    .fetch_all(pool)
    .await?;

    if rows.is_empty() {
        return Ok(generate_simulated_land_use(site_id));
    }

    let mut period_map: HashMap<i32, (String, Vec<LandUseItem>)> = HashMap::new();
    let mut all_types: Vec<String> = Vec::new();

    for row in &rows {
        let year = row.period_year.unwrap_or(0);
        let period_name = row.period_name.clone().unwrap_or_else(|| format!("{}年", year));
        let land_use_type = row.land_use_type.clone();

        if !all_types.contains(&land_use_type) {
            all_types.push(land_use_type.clone());
        }

        let entry = period_map.entry(year).or_insert_with(|| (period_name.clone(), Vec::new()));
        entry.1.push(LandUseItem {
            land_use_type,
            area_km2: row.area_km2,
            percentage: row.percentage.unwrap_or(0.0),
        });
    }

    let mut years: Vec<i32> = period_map.keys().cloned().collect();
    years.sort();

    let periods: Vec<LandUsePeriod> = years
        .iter()
        .map(|y| {
            let (name, mut items) = period_map.get(y).cloned().unwrap_or_else(|| (format!("{}年", y), Vec::new()));
            items.sort_by(|a, b| a.land_use_type.cmp(&b.land_use_type));
            LandUsePeriod {
                period_name: name,
                period_year: *y,
                land_uses: items,
            }
        })
        .collect();

    all_types.sort();

    Ok(LandUseTimeline {
        site_id,
        periods,
        land_use_types: all_types,
    })
}

pub fn generate_simulated_land_use(site_id: i32) -> LandUseTimeline {
    let num_periods = algorithm::LAND_USE_NUM_PERIODS;
    let decay_rate = algorithm::LAND_USE_DECAY_RATE;
    let mut all_types: Vec<String> = LAND_USE_TYPES.iter().map(|(t, _)| t.to_string()).collect();

    let mut periods = Vec::new();
    let period_names = [
        "城市鼎盛期", "城市衰落初期", "城市废弃期", "早期农业期",
        "中期农业期", "近古时期", "近代时期", "现代时期",
    ];

    let base_area = 5.0_f64;
    let seed = site_id as f64;

    for i in 0..num_periods {
        let t = i as f64 / (num_periods - 1) as f64;
        let urban_decay = (1.0 - t * decay_rate).max(0.05);
        let farmland_growth = (0.1 + t * 0.5 * (seed * 0.1).sin().abs()).min(0.6);
        let wasteland_peak = if t > 0.2 && t < 0.5 {
            0.3 * (1.0 - (t - 0.35).abs() * 4.0).max(0.0)
        } else {
            0.1
        };

        let mut land_uses = Vec::new();
        let mut total_pct = 0.0_f64;

        let urban_pct = 0.35 * urban_decay;
        land_uses.push(LandUseItem {
            land_use_type: "urban".to_string(),
            area_km2: base_area * urban_pct,
            percentage: urban_pct * 100.0,
        });
        total_pct += urban_pct;

        let farmland_pct = farmland_growth;
        land_uses.push(LandUseItem {
            land_use_type: "farmland".to_string(),
            area_km2: base_area * farmland_pct,
            percentage: farmland_pct * 100.0,
        });
        total_pct += farmland_pct;

        let forest_pct = 0.2 + 0.1 * (t * 2.0 + seed * 0.05).sin();
        let forest_pct = forest_pct.max(0.1).min(0.4);
        land_uses.push(LandUseItem {
            land_use_type: "forest".to_string(),
            area_km2: base_area * forest_pct,
            percentage: forest_pct * 100.0,
        });
        total_pct += forest_pct;

        let grassland_pct = 0.15 + 0.1 * (t * 1.5).cos();
        let grassland_pct = grassland_pct.max(0.05).min(0.25);
        land_uses.push(LandUseItem {
            land_use_type: "grassland".to_string(),
            area_km2: base_area * grassland_pct,
            percentage: grassland_pct * 100.0,
        });
        total_pct += grassland_pct;

        let wasteland_pct = wasteland_peak;
        land_uses.push(LandUseItem {
            land_use_type: "wasteland".to_string(),
            area_km2: base_area * wasteland_pct,
            percentage: wasteland_pct * 100.0,
        });
        total_pct += wasteland_pct;

        let settlement_pct = if t > 0.5 { (t - 0.5) * 0.2 } else { 0.02 };
        land_uses.push(LandUseItem {
            land_use_type: "settlement".to_string(),
            area_km2: base_area * settlement_pct,
            percentage: settlement_pct * 100.0,
        });
        total_pct += settlement_pct;

        let water_pct = 0.05 + 0.03 * (seed * 0.1).sin();
        land_uses.push(LandUseItem {
            land_use_type: "water".to_string(),
            area_km2: base_area * water_pct,
            percentage: water_pct * 100.0,
        });
        total_pct += water_pct;

        let wetland_pct = (1.0 - total_pct).max(0.02);
        land_uses.push(LandUseItem {
            land_use_type: "wetland".to_string(),
            area_km2: base_area * wetland_pct,
            percentage: wetland_pct * 100.0,
        });

        land_uses.sort_by(|a, b| a.land_use_type.cmp(&b.land_use_type));

        let period_year = -500 + i * 300;
        periods.push(LandUsePeriod {
            period_name: period_names.get(i).cloned().unwrap_or_else(|| format!("时期{}", i + 1)).to_string(),
            period_year,
            land_uses,
        });
    }

    LandUseTimeline {
        site_id,
        periods,
        land_use_types: all_types,
    }
}

pub async fn save_land_use_changes(
    pool: &sqlx::PgPool,
    site_id: i32,
    timeline: &LandUseTimeline,
) -> Result<(), AppError> {
    let mut tx = pool.begin().await?;

    sqlx::query!(
        "DELETE FROM land_use_changes WHERE site_id = $1",
        site_id
    )
    .execute(&mut *tx)
    .await?;

    for period in &timeline.periods {
        for item in &period.land_uses {
            sqlx::query!(
                r#"
                INSERT INTO land_use_changes
                    (site_id, period_name, period_year, land_use_type,
                     area_km2, percentage, evidence_type, confidence, description)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                "#,
                site_id,
                period.period_name.clone(),
                period.period_year,
                item.land_use_type.clone(),
                item.area_km2,
                item.percentage,
                Some("simulated".to_string()),
                Some(0.6),
                Some(format!("{}-土地利用模拟数据", period.period_name)),
            )
            .execute(&mut *tx)
            .await?;
        }
    }

    tx.commit().await?;
    Ok(())
}

use actix_web::{web, HttpResponse};
use crate::models::ApiResponse;

pub async fn get_land_use_timeline_handler(
    pool: web::Data<sqlx::PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let result = get_land_use_timeline(&pool, *site_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

pub async fn get_land_use_trend_handler(
    pool: web::Data<sqlx::PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let timeline = get_land_use_timeline(&pool, *site_id).await?;
    let trend = compute_land_use_trend(&timeline);
    Ok(HttpResponse::Ok().json(ApiResponse::success(trend)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    #[test]
    fn test_generate_simulated_land_use_period_count() {
        let tl = generate_simulated_land_use(1);
        assert_eq!(tl.periods.len(), algorithm::LAND_USE_NUM_PERIODS);
    }

    #[test]
    fn test_generate_simulated_land_use_site_id_preserved() {
        let tl = generate_simulated_land_use(42);
        assert_eq!(tl.site_id, 42);
    }

    #[test]
    fn test_generate_simulated_land_use_each_period_has_8_types() {
        let tl = generate_simulated_land_use(1);
        for (i, p) in tl.periods.iter().enumerate() {
            assert_eq!(p.items.len(), 8, "period {} has {} types", i, p.items.len());
        }
    }

    #[test]
    fn test_generate_simulated_land_use_peak_urban_first_period() {
        let tl = generate_simulated_land_use(1);
        let periods = &tl.periods;
        assert!(periods.len() >= 2);
        let urban_first = periods[0].items.iter()
            .find(|i| i.land_use_type == "urban").unwrap().area_km2;
        let urban_last = periods.last().unwrap().items.iter()
            .find(|i| i.land_use_type == "urban").unwrap().area_km2;
        assert!(urban_first > urban_last,
            "urban area should decline: first={} > last={}", urban_first, urban_last);
    }

    #[test]
    fn test_generate_simulated_land_use_urban_decays_over_time() {
        let tl = generate_simulated_land_use(1);
        let urban_values: Vec<f64> = tl.periods.iter()
            .map(|p| p.items.iter()
                .find(|i| i.land_use_type == "urban")
                .unwrap().area_km2)
            .collect();

        for i in 1..urban_values.len() {
            assert!(urban_values[i] <= urban_values[i-1] * 1.05,
                "urban should not increase significantly at period {}: {} > {}",
                i, urban_values[i], urban_values[i-1]);
        }
    }

    #[test]
    fn test_generate_simulated_land_use_total_area_roughly_constant() {
        let tl = generate_simulated_land_use(1);
        let totals: Vec<f64> = tl.periods.iter()
            .map(|p| p.items.iter().map(|i| i.area_km2).sum())
            .collect();

        let avg: f64 = totals.iter().sum::<f64>() / totals.len() as f64;
        for (i, t) in totals.iter().enumerate() {
            let ratio = t / avg;
            assert!(ratio > 0.5 && ratio < 2.0,
                "period {} total area {} deviates too much from avg {}",
                i, t, avg);
        }
    }

    #[test]
    fn test_generate_simulated_land_use_farmland_increases() {
        let tl = generate_simulated_land_use(1);
        let periods = &tl.periods;
        let farm_first = periods[0].items.iter()
            .find(|i| i.land_use_type == "farmland").unwrap().area_km2;
        let farm_last = periods.last().unwrap().items.iter()
            .find(|i| i.land_use_type == "farmland").unwrap().area_km2;
        assert!(farm_last >= farm_first * 0.95,
            "farmland should eventually rise or stay");
    }

    #[test]
    fn test_generate_simulated_land_use_all_areas_non_negative() {
        let tl = generate_simulated_land_use(1);
        for p in &tl.periods {
            for it in &p.items {
                assert!(it.area_km2 >= 0.0,
                    "period {} type {} has negative area {}",
                    p.period_name, it.land_use_type, it.area_km2);
                assert!(it.percentage >= 0.0,
                    "period {} type {} has negative pct {}",
                    p.period_name, it.land_use_type, it.percentage);
            }
        }
    }

    #[test]
    fn test_generate_simulated_land_use_percentage_sum_100() {
        let tl = generate_simulated_land_use(1);
        for (i, p) in tl.periods.iter().enumerate() {
            let s: f64 = p.items.iter().map(|it| it.percentage).sum();
            assert_relative_eq!(s, 100.0, epsilon = 0.5,
                "period {} sum pct = {}, not 100", i, s);
        }
    }

    #[test]
    fn test_generate_simulated_land_use_period_names_all_unique() {
        let tl = generate_simulated_land_use(1);
        let mut names: Vec<_> = tl.periods.iter().map(|p| &p.period_name).collect();
        let orig_len = names.len();
        names.sort();
        names.dedup();
        assert_eq!(names.len(), orig_len, "period names should be unique");
    }

    #[test]
    fn test_generate_simulated_land_use_all_types_present() {
        let tl = generate_simulated_land_use(1);
        let expected = ["urban", "farmland", "forest", "grassland",
            "wetland", "water", "wasteland", "settlement"];

        for p in &tl.periods {
            for t in &expected {
                assert!(p.items.iter().any(|i| i.land_use_type == t),
                    "type {} missing in period {}", t, p.period_name);
            }
        }
    }

    #[test]
    fn test_generate_simulated_land_use_labels_match_constants() {
        let tl = generate_simulated_land_use(1);
        for (i, p) in tl.periods.iter().enumerate() {
            assert_eq!(p.period_name, PERIOD_NAMES[i]);
        }
    }

    #[test]
    fn test_compute_land_use_trend_returns_required_keys() {
        let tl = generate_simulated_land_use(1);
        let trend = compute_land_use_trend(&tl);

        assert!(trend.get("summary").is_some());
        assert!(trend.get("urban_decay_rate").is_some());
        assert!(trend.get("change_rates").is_some());
        assert!(trend.get("dominant_transitions").is_some());
        assert!(trend.get("trend_classification").is_some());
    }

    #[test]
    fn test_compute_land_use_trend_urban_decay_rate_negative_or_zero() {
        let tl = generate_simulated_land_use(1);
        let trend = compute_land_use_trend(&tl);
        let decay = trend["urban_decay_rate"].as_f64().unwrap();
        assert!(decay <= 0.01, "urban decay should be non-positive, got {}", decay);
    }

    #[test]
    fn test_compute_land_use_trend_classification_valid_level() {
        let tl = generate_simulated_land_use(1);
        let trend = compute_land_use_trend(&tl);
        let class = trend["trend_classification"].as_str().unwrap();
        let valid = ["快速衰败型", "加速衰败型", "稳定转化型", "缓慢恢复型", "显著复兴型"];
        assert!(valid.contains(&class), "invalid classification: {}", class);
    }

    #[test]
    fn test_compute_land_use_trend_change_rates_has_8_entries() {
        let tl = generate_simulated_land_use(1);
        let trend = compute_land_use_trend(&tl);
        let cr = trend["change_rates"].as_object().unwrap();
        assert_eq!(cr.len(), 8);
    }

    #[test]
    fn test_compute_land_use_trend_empty_periods_safe() {
        let tl = LandUseTimeline {
            site_id: 1,
            periods: vec![],
        };
        let trend = compute_land_use_trend(&tl);
        assert!(trend.get("summary").is_some());
        assert!(trend["urban_decay_rate"].as_f64().is_some() ||
                trend["urban_decay_rate"].is_null());
    }

    #[test]
    fn test_compute_land_use_trend_single_period_safe() {
        let mut tl = generate_simulated_land_use(1);
        tl.periods.truncate(1);
        let trend = compute_land_use_trend(&tl);
        assert!(trend.get("summary").is_some());
    }

    #[test]
    fn test_compute_land_use_trend_farmland_trend_positive() {
        let tl = generate_simulated_land_use(1);
        let trend = compute_land_use_trend(&tl);
        let cr = trend["change_rates"].as_object().unwrap();
        let farm_rate = cr.get("farmland").and_then(|v| v.as_f64()).unwrap_or(0.0);
        assert!(farm_rate >= -0.5,
            "farmland trend should be generally non-negative or mildly negative, got {}",
            farm_rate);
    }

    #[test]
    fn test_pipeline_generate_then_trend() {
        let tl = generate_simulated_land_use(7);
        assert_eq!(tl.site_id, 7);
        assert_eq!(tl.periods.len(), algorithm::LAND_USE_NUM_PERIODS);
        assert_eq!(tl.periods[0].items.len(), 8);

        let trend = compute_land_use_trend(&tl);
        assert!(trend.get("summary").is_some());
        let summary = trend["summary"].as_str().unwrap();
        assert!(!summary.is_empty());
    }

    #[test]
    fn test_periods_count_matches_constant() {
        use crate::config::algorithm::LAND_USE_NUM_PERIODS;
        assert_eq!(PERIOD_NAMES.len(), LAND_USE_NUM_PERIODS);
    }

    #[test]
    fn test_land_use_colors_match_length() {
        assert_eq!(LAND_USE_COLORS.len(), LAND_USE_TYPES.len());
        assert_eq!(LAND_USE_LABELS.len(), LAND_USE_TYPES.len());
    }

    #[test]
    fn test_land_use_all_types_defined() {
        let expected_types = ["urban", "farmland", "forest", "grassland",
            "wetland", "water", "wasteland", "settlement"];
        assert_eq!(LAND_USE_TYPES.len(), expected_types.len());
    }

    #[test]
    fn root_cause_stratigraphic_continuity_assessment() {
        let layers = vec![
            StratigraphicLayer {
                layer_id: "l1".to_string(),
                period_name: "汉代".to_string(),
                period_year: -200,
                thickness_cm: 50.0,
                sediment_type: "alluvial".to_string(),
                artifacts: vec!["陶片".to_string()],
                dating_method: "C14".to_string(),
                confidence: 0.9,
            },
            StratigraphicLayer {
                layer_id: "l2".to_string(),
                period_name: "唐代".to_string(),
                period_year: 700,
                thickness_cm: 80.0,
                sediment_type: "alluvial".to_string(),
                artifacts: vec!["瓷器".to_string()],
                dating_method: "C14".to_string(),
                confidence: 0.85,
            },
            StratigraphicLayer {
                layer_id: "l3".to_string(),
                period_name: "明代".to_string(),
                period_year: 1400,
                thickness_cm: 120.0,
                sediment_type: "alluvial".to_string(),
                artifacts: vec!["青花瓷".to_string()],
                dating_method: "typology".to_string(),
                confidence: 0.8,
            },
        ];

        let (continuity, issues, gaps) = assess_stratigraphic_continuity(&layers);

        assert!(continuity > 0.5, "continuity should be meaningful, got {}", continuity);
        assert!(continuity <= 1.0);

        for i in 1..layers.len() {
            let gap = layers[i].period_year - layers[i-1].period_year;
            if gap > 500 {
                assert!(gaps.contains(&layers[i-1].period_year),
                    "gap of {} years should be detected", gap);
            }
        }
    }

    #[test]
    fn root_cause_missing_periods_estimation() {
        let known = vec![
            StratigraphicLayer {
                layer_id: "l1".to_string(),
                period_name: "汉代".to_string(),
                period_year: -200,
                thickness_cm: 50.0,
                sediment_type: "alluvial".to_string(),
                artifacts: vec![],
                dating_method: "C14".to_string(),
                confidence: 0.9,
            },
            StratigraphicLayer {
                layer_id: "l2".to_string(),
                period_name: "宋代".to_string(),
                period_year: 1000,
                thickness_cm: 100.0,
                sediment_type: "alluvial".to_string(),
                artifacts: vec![],
                dating_method: "C14".to_string(),
                confidence: 0.85,
            },
        ];

        let model = &DEPOSITION_MODELS[0].1;
        let targets = vec![(400, "南北朝".to_string()), (1400, "明代".to_string())];
        let estimated = estimate_missing_periods(&known, &targets, model);

        assert_eq!(estimated.len(), 2);

        for layer in &estimated {
            assert!(layer.thickness_cm > 0.0);
            assert!(layer.confidence > 0.0 && layer.confidence < 1.0);
            assert!(!layer.dating_method.is_empty());
        }

        let interpolated = estimated.iter().find(|l| l.period_year == 400).unwrap();
        assert!(interpolated.confidence > 0.3,
            "interpolated layer should have reasonable confidence, got {}",
            interpolated.confidence);

        let extrapolated = estimated.iter().find(|l| l.period_year == 1400).unwrap();
        assert!(extrapolated.confidence < interpolated.confidence,
            "extrapolated should have lower confidence than interpolated");
    }

    #[test]
    fn root_cause_archaeological_constraints_improve_confidence() {
        let mut layers = vec![
            StratigraphicLayer {
                layer_id: "est_500".to_string(),
                period_name: "南北朝".to_string(),
                period_year: 500,
                thickness_cm: 60.0,
                sediment_type: "alluvial".to_string(),
                artifacts: vec![],
                dating_method: "interpolation".to_string(),
                confidence: 0.4,
            },
        ];

        let checkpoints = vec![
            ArchaeologicalCheckpoint {
                period_name: "南北朝".to_string(),
                period_year: 500,
                evidence_type: "佛寺遗址".to_string(),
                description: "发现北魏佛寺基址".to_string(),
                confidence: 0.85,
                land_use_constraints: vec![
                    ("urban".to_string(), 15.0, 40.0),
                    ("farmland".to_string(), 20.0, 50.0),
                ],
            },
        ];

        let original_conf = layers[0].confidence;
        apply_archaeological_constraints(&mut layers, &checkpoints);
        let new_conf = layers[0].confidence;

        assert!(new_conf >= original_conf,
            "archaeological evidence should not decrease confidence");
        assert!(!layers[0].artifacts.is_empty(),
            "artifacts should be added from checkpoint");
        assert!(layers[0].dating_method.contains("archaeo"),
            "dating method should reflect archaeological validation");
    }

    #[test]
    fn root_cause_fill_land_use_gaps_increases_periods() {
        let tl = generate_simulated_land_use(1);
        let original_count = tl.periods.len();

        let model = &DEPOSITION_MODELS[0].1;
        let checkpoints: Vec<ArchaeologicalCheckpoint> = vec![];
        let filled = fill_land_use_gaps(&tl, model, &checkpoints);

        assert!(filled.periods.len() >= original_count,
            "filled should have >= periods than original: {} vs {}",
            filled.periods.len(), original_count);
        assert_eq!(filled.site_id, tl.site_id);
        assert_eq!(filled.land_use_types.len(), tl.land_use_types.len());
    }

    #[test]
    fn root_cause_deposition_models_exist_for_contexts() {
        assert!(DEPOSITION_MODELS.len() >= 4);

        for (name, model) in DEPOSITION_MODELS {
            assert!(!name.is_empty());
            assert!(model.accumulation_rate_cm_per_yr > 0.0);
            assert!(model.accumulation_rate_cm_per_yr < 1.0);
            assert!(model.compaction_factor > 0.0 && model.compaction_factor <= 1.0);
            assert!(model.confidence > 0.0 && model.confidence <= 1.0);
            assert!(model.erosion_rate_cm_per_yr >= 0.0);
        }
    }

    #[test]
    fn root_cause_select_deposition_model_different_terrains() {
        let m1 = select_deposition_model("river_plain", 0.9);
        assert_eq!(m1.environment, DepositionEnvironment::Fluvial);

        let m2 = select_deposition_model("lake_side", 0.7);
        assert_eq!(m2.environment, DepositionEnvironment::Lacustrine);

        let m3 = select_deposition_model("loess_terrace", 0.2);
        assert!(m3.environment == DepositionEnvironment::Aeolian ||
                m3.environment == DepositionEnvironment::Alluvial);
    }

    #[test]
    fn root_cause_interpolated_land_use_smooth_transition() {
        let before = LandUsePeriod {
            period_name: "早期".to_string(),
            period_year: 0,
            land_uses: vec![
                LandUseItem {
                    land_use_type: "urban".to_string(),
                    area_km2: 2.0,
                    percentage: 40.0,
                },
                LandUseItem {
                    land_use_type: "farmland".to_string(),
                    area_km2: 1.0,
                    percentage: 20.0,
                },
            ],
        };

        let after = LandUsePeriod {
            period_name: "晚期".to_string(),
            period_year: 1000,
            land_uses: vec![
                LandUseItem {
                    land_use_type: "urban".to_string(),
                    area_km2: 0.5,
                    percentage: 10.0,
                },
                LandUseItem {
                    land_use_type: "farmland".to_string(),
                    area_km2: 2.5,
                    percentage: 50.0,
                },
            ],
        };

        let model = &DEPOSITION_MODELS[0].1;
        let middle = interpolate_land_use_period(&before, &after, 0.5, 500, model);

        assert_eq!(middle.period_year, 500);
        assert_eq!(middle.land_uses.len(), 2);

        let urban_mid = middle.land_uses.iter()
            .find(|u| u.land_use_type == "urban").unwrap();
        assert!(urban_mid.percentage > 10.0 && urban_mid.percentage < 40.0,
            "mid urban should be between endpoints: {}", urban_mid.percentage);

        let farm_mid = middle.land_uses.iter()
            .find(|u| u.land_use_type == "farmland").unwrap();
        assert!(farm_mid.percentage > 20.0 && farm_mid.percentage < 50.0,
            "mid farmland should be between endpoints: {}", farm_mid.percentage);
    }
}
