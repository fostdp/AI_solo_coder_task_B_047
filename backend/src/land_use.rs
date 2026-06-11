use crate::models::{LandUseTimeline, LandUsePeriod, LandUseItem, LandUseChange};
use crate::errors::AppError;
use crate::config::algorithm;
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

pub fn compute_land_use_trend(timeline: &LandUseTimeline) -> Value {
    let mut trends = HashMap::new();

    if timeline.periods.len() < 2 {
        return json!({ "trends": {}, "note": "数据点不足" });
    }

    for land_type in &timeline.land_use_types {
        let values: Vec<f64> = timeline.periods
            .iter()
            .map(|p| {
                p.land_uses
                    .iter()
                    .find(|u| u.land_use_type == *land_type)
                    .map(|u| u.percentage)
                    .unwrap_or(0.0)
            })
            .collect();

        let first = values.first().copied().unwrap_or(0.0);
        let last = values.last().copied().unwrap_or(0.0);
        let change = last - first;
        let change_pct = if first > 0.0 { (change / first) * 100.0 } else { 0.0 };

        let trend_type = if change_pct > 20.0 {
            "significant_increase"
        } else if change_pct > 5.0 {
            "moderate_increase"
        } else if change_pct < -20.0 {
            "significant_decrease"
        } else if change_pct < -5.0 {
            "moderate_decrease"
        } else {
            "stable"
        };

        trends.insert(land_type.clone(), json!({
            "start_percentage": first,
            "end_percentage": last,
            "absolute_change": change,
            "relative_change_percent": change_pct,
            "trend_type": trend_type,
        }));
    }

    let peak_urban = timeline.periods
        .iter()
        .map(|p| {
            p.land_uses
                .iter()
                .find(|u| u.land_use_type == "urban")
                .map(|u| u.percentage)
                .unwrap_or(0.0)
        })
        .fold(0.0_f64, f64::max);

    json!({
        "trends": trends,
        "peak_urban_percentage": peak_urban,
        "urban_decay_rate": if peak_urban > 0.0 {
            (peak_urban - timeline.periods.last().and_then(|p|
                p.land_uses.iter().find(|u| u.land_use_type == "urban").map(|u| u.percentage)
            ).unwrap_or(0.0)) / peak_urban
        } else { 0.0 },
        "num_periods": timeline.periods.len(),
    })
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
        assert_eq!(LAND_USE_TYPES, expected_types);
    }
}
