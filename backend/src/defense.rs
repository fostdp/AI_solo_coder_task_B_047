use crate::models::{DefenseAnalysisResult, DefenseWeakPoint, AttackRoute, GateDefenseScore, CityGate};
use crate::errors::AppError;
use crate::config::algorithm;
use serde_json::{json, Value};
use std::f64::consts::PI;

pub async fn analyze_defense(
    pool: &sqlx::PgPool,
    site_id: i32,
) -> Result<DefenseAnalysisResult, AppError> {
    let site_row = sqlx::query!(
        r#"
        SELECT cs.id, cs.name, cs.area_sq_km, cs.wall_height, cs.wall_width,
               cs.moat_width, cs.num_gates, cs.terrain_type,
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

    let center_lon = site_row.center_longitude;
    let center_lat = site_row.center_latitude;
    let area_sq_km = site_row.area_sq_km.unwrap_or(1.0);
    let radius_km = (area_sq_km / PI).sqrt();
    let wall_height = site_row.wall_height.unwrap_or(8.0);
    let wall_width = site_row.wall_width.unwrap_or(5.0);
    let moat_width = site_row.moat_width.unwrap_or(0.0);
    let terrain = site_row.terrain_type.clone().unwrap_or_else(|| "plain".to_string());

    let gates_rows = sqlx::query!(
        r#"
        SELECT id, site_id, name, gate_type, defense_rating,
               ST_AsGeoJSON(geom)::jsonb as geom, description
        FROM city_gates
        WHERE site_id = $1
        ORDER BY id
        "#,
        site_id
    )
    .fetch_all(pool)
    .await?;

    let mut gates: Vec<CityGate> = Vec::new();
    for row in &gates_rows {
        gates.push(CityGate {
            id: row.id,
            site_id: row.site_id,
            name: row.name.clone(),
            gate_type: row.gate_type.clone(),
            defense_rating: row.defense_rating.map(|r| r as f64),
            geom: row.geom.clone(),
            description: row.description.clone(),
            created_at: None,
        });
    }

    if gates.is_empty() {
        gates = generate_simulated_gates(center_lon, center_lat, radius_km);
    }

    let wall_segments = generate_wall_segments(center_lon, center_lat, radius_km);

    let weak_points = analyze_weak_points(
        center_lon, center_lat, radius_km,
        &gates, &wall_segments, wall_height, wall_width, moat_width, &terrain,
    );

    let attack_routes = compute_attack_routes(
        center_lon, center_lat, radius_km,
        &gates, &weak_points, &terrain,
    );

    let gate_scores: Vec<GateDefenseScore> = gates
        .iter()
        .map(|g| {
            let base_defense = g.defense_rating.unwrap_or(0.7);
            let wall_bonus = (wall_height / 15.0).min(0.3) + (wall_width / 10.0).min(0.2);
            let moat_bonus = (moat_width / 20.0).min(0.2);
            let defense_score = (base_defense + wall_bonus + moat_bonus).min(1.0);
            let vulnerability = 1.0 - defense_score;

            GateDefenseScore {
                gate_id: g.id,
                gate_name: g.name.clone(),
                defense_score,
                vulnerability,
            }
        })
        .collect();

    let visibility_analysis = compute_visibility_analysis(
        center_lon, center_lat, radius_km, &gates, &terrain,
    );

    let overall_defense = compute_overall_defense_score(
        &gate_scores, wall_height, wall_width, moat_width,
        &weak_points, area_sq_km, &terrain,
    );

    let accessibility_score = compute_accessibility_score(&gates, radius_km);

    Ok(DefenseAnalysisResult {
        id: None,
        site_id,
        overall_defense_score: overall_defense,
        visibility_analysis: Some(json!(visibility_analysis)),
        weak_points,
        optimal_attack_routes: attack_routes,
        accessibility_score,
        gate_defense_scores: gate_scores,
        wall_segments: Some(json!(wall_segments)),
        gates,
        created_at: None,
    })
}

fn generate_simulated_gates(
    center_lon: f64,
    center_lat: f64,
    radius_km: f64,
) -> Vec<CityGate> {
    let num_gates = 4;
    let mut gates = Vec::new();
    let deg_per_km = 1.0 / 111.0;
    let gate_names = ["东门", "南门", "西门", "北门"];
    let gate_types = ["main", "main", "main", "main"];

    for i in 0..num_gates {
        let angle = (i as f64) * (2.0 * PI) / (num_gates as f64) - PI / 2.0;
        let glon = center_lon + radius_km * deg_per_km * angle.cos();
        let glat = center_lat + radius_km * deg_per_km * angle.sin();

        gates.push(CityGate {
            id: (i + 1) as i32,
            site_id: 0,
            name: Some(gate_names[i].to_string()),
            gate_type: Some(gate_types[i].to_string()),
            defense_rating: Some(0.7 + (i % 2) as f64 * 0.1),
            geom: Some(json!({
                "type": "Point",
                "coordinates": [glon, glat]
            })),
            description: None,
            created_at: None,
        });
    }

    gates
}

fn generate_wall_segments(
    center_lon: f64,
    center_lat: f64,
    radius_km: f64,
) -> Vec<Value> {
    let num_segments = algorithm::DEFENSE_WALL_SEGMENTS;
    let deg_per_km = 1.0 / 111.0;
    let mut segments = Vec::new();

    for i in 0..num_segments {
        let angle1 = (i as f64) * (2.0 * PI) / (num_segments as f64);
        let angle2 = ((i + 1) as f64) * (2.0 * PI) / (num_segments as f64);

        let lon1 = center_lon + radius_km * deg_per_km * angle1.cos();
        let lat1 = center_lat + radius_km * deg_per_km * angle1.sin();
        let lon2 = center_lon + radius_km * deg_per_km * angle2.cos();
        let lat2 = center_lat + radius_km * deg_per_km * angle2.sin();

        let defense_strength = 0.6 + 0.4 * ((i as f64 * 0.5).sin() * 0.5 + 0.5);

        segments.push(json!({
            "segment_index": i,
            "start": [lon1, lat1],
            "end": [lon2, lat2],
            "defense_strength": defense_strength,
            "wall_condition": "intact",
            "visibility_index": 0.5 + 0.3 * ((i as f64 * 0.7).cos() * 0.5 + 0.5),
        }));
    }

    segments
}

fn analyze_weak_points(
    center_lon: f64,
    center_lat: f64,
    radius_km: f64,
    gates: &[CityGate],
    wall_segments: &[Value],
    wall_height: f64,
    wall_width: f64,
    moat_width: f64,
    terrain: &str,
) -> Vec<DefenseWeakPoint> {
    let mut weak_points = Vec::new();
    let deg_per_km = 1.0 / 111.0;

    for gate in gates {
        if let Some(geom) = &gate.geom {
            if let Some(coords) = geom.get("coordinates").and_then(|c| c.as_array()) {
                if coords.len() >= 2 {
                    let lon = coords[0].as_f64().unwrap_or(center_lon);
                    let lat = coords[1].as_f64().unwrap_or(center_lat);

                    let defense_rating = gate.defense_rating.unwrap_or(0.7);
                    let weakness_score = (1.0 - defense_rating) * 0.8;
                    let gate_factor = 0.3;
                    let weakness_score = (weakness_score + gate_factor).min(1.0);

                    let terrain_factor = match terrain {
                        "mountain" => -0.15,
                        "hill" => -0.1,
                        "plain" => 0.0,
                        "wetland" => 0.1,
                        "river" => 0.15,
                        _ => 0.0,
                    };
                    let weakness_score = (weakness_score + terrain_factor).max(0.0).min(1.0);

                    weak_points.push(DefenseWeakPoint {
                        lon,
                        lat,
                        weakness_score,
                        weakness_type: "gate".to_string(),
                        description: Some(format!("{}为防御薄弱点，城门易受攻击", gate.name.clone().unwrap_or_else(|| "城门".to_string()))),
                    });
                }
            }
        }
    }

    let num_wall_weak = 3;
    for i in 0..num_wall_weak {
        let segment_idx = ((i + 1) * 7 + 3) % wall_segments.len().max(1);
        if let Some(seg) = wall_segments.get(segment_idx) {
            let start = seg.get("start").and_then(|s| s.as_array()).unwrap_or(&vec![]);
            let end = seg.get("end").and_then(|e| e.as_array()).unwrap_or(&vec![]);

            if start.len() >= 2 && end.len() >= 2 {
                let mid_lon = (start[0].as_f64().unwrap_or(0.0) + end[0].as_f64().unwrap_or(0.0)) / 2.0;
                let mid_lat = (start[1].as_f64().unwrap_or(0.0) + end[1].as_f64().unwrap_or(0.0)) / 2.0;

                let defense_strength = seg.get("defense_strength").and_then(|d| d.as_f64()).unwrap_or(0.6);
                let mut weakness_score = 1.0 - defense_strength;

                if wall_height < 6.0 { weakness_score += 0.15; }
                if wall_width < 4.0 { weakness_score += 0.1; }
                if moat_width < 2.0 { weakness_score += 0.1; }

                let weakness_score = weakness_score.max(0.2).min(0.9);

                weak_points.push(DefenseWeakPoint {
                    lon: mid_lon,
                    lat: mid_lat,
                    weakness_score,
                    weakness_type: "wall".to_string(),
                    description: Some("城墙段防御强度较低，为潜在突破口".to_string()),
                });
            }
        }
    }

    let terrain_angle = 2.3;
    let tlon = center_lon + radius_km * 0.7 * deg_per_km * terrain_angle.cos();
    let tlat = center_lat + radius_km * 0.7 * deg_per_km * terrain_angle.sin();
    let terrain_weakness = match terrain {
        "river" => 0.75,
        "wetland" => 0.65,
        "plain" => 0.45,
        "hill" => 0.35,
        "mountain" => 0.25,
        _ => 0.5,
    };
    weak_points.push(DefenseWeakPoint {
        lon: tlon,
        lat: tlat,
        weakness_score: terrain_weakness,
        weakness_type: "terrain".to_string(),
        description: Some("地形因素导致该区域防御效能下降".to_string()),
    });

    weak_points.sort_by(|a, b| b.weakness_score.partial_cmp(&a.weakness_score).unwrap_or(std::cmp::Ordering::Equal));
    weak_points
}

fn compute_attack_routes(
    center_lon: f64,
    center_lat: f64,
    radius_km: f64,
    gates: &[CityGate],
    weak_points: &[DefenseWeakPoint],
    terrain: &str,
) -> Vec<AttackRoute> {
    let mut routes = Vec::new();
    let deg_per_km = 1.0 / 111.0;
    let num_routes = algorithm::DEFENSE_NUM_ATTACK_ROUTES.min(gates.len() + 3);

    for i in 0..num_routes {
        let start_angle = (i as f64) * (2.0 * PI) / (num_routes as f64);
        let start_dist = radius_km * 1.5;
        let start_lon = center_lon + start_dist * deg_per_km * start_angle.cos();
        let start_lat = center_lat + start_dist * deg_per_km * start_angle.sin();

        let (end_lon, end_lat, route_type, attack_score) = if i < gates.len() {
            let gate = &gates[i];
            let (glon, glat) = if let Some(geom) = &gate.geom {
                if let Some(coords) = geom.get("coordinates").and_then(|c| c.as_array()) {
                    (coords[0].as_f64().unwrap_or(center_lon),
                     coords[1].as_f64().unwrap_or(center_lat))
                } else {
                    (center_lon, center_lat)
                }
            } else {
                (center_lon, center_lat)
            };

            let defense_rating = gate.defense_rating.unwrap_or(0.7);
            let score = (1.0 - defense_rating) * 0.7 + 0.3;

            (glon, glat, "gate_attack".to_string(), score)
        } else {
            let wp_idx = (i - gates.len()) % weak_points.len().max(1);
            if let Some(wp) = weak_points.get(wp_idx) {
                (wp.lon, wp.lat, wp.weakness_type.clone(), wp.weakness_score * 0.9 + 0.1)
            } else {
                (center_lon + radius_km * 0.5 * deg_per_km,
                 center_lat + radius_km * 0.5 * deg_per_km,
                 "wall_breach".to_string(), 0.5)
            }
        };

        let route_geom = generate_attack_route_geom(start_lon, start_lat, end_lon, end_lat, terrain, i as f64);

        let terrain_modifier = match terrain {
            "mountain" => 0.85,
            "hill" => 0.9,
            "plain" => 1.0,
            "wetland" => 1.1,
            "river" => 1.15,
            _ => 1.0,
        };
        let final_score = (attack_score * terrain_modifier).min(1.0).max(0.0);

        routes.push(AttackRoute {
            route_geom,
            start_lon,
            start_lat,
            end_lon,
            end_lat,
            attack_score: final_score,
            route_type,
        });
    }

    routes.sort_by(|a, b| b.attack_score.partial_cmp(&a.attack_score).unwrap_or(std::cmp::Ordering::Equal));
    routes
}

fn generate_attack_route_geom(
    start_lon: f64,
    start_lat: f64,
    end_lon: f64,
    end_lat: f64,
    terrain: &str,
    seed: f64,
) -> Value {
    let num_points = 8;
    let mut coordinates = Vec::new();
    coordinates.push(vec![start_lon, start_lat]);

    for i in 1..num_points {
        let t = i as f64 / num_points as f64;
        let lon = start_lon + (end_lon - start_lon) * t;
        let lat = start_lat + (end_lat - start_lat) * t;

        let perp_lon = -(end_lat - start_lat);
        let perp_lat = end_lon - start_lon;
        let perp_len = (perp_lon.powi(2) + perp_lat.powi(2)).sqrt().max(0.0001);
        let perp_lon = perp_lon / perp_len;
        let perp_lat = perp_lat / perp_len;

        let curve = (t * PI).sin() * 0.15 * (seed.sin() * 0.5 + 0.5)
            * match terrain {
                "mountain" => 2.0,
                "hill" => 1.5,
                "wetland" => 1.3,
                _ => 1.0,
            };

        let offset = curve * (seed * 0.3).sin();

        let rlon = lon + perp_lon * offset;
        let rlat = lat + perp_lat * offset;

        coordinates.push(vec![rlon, rlat]);
    }

    coordinates.push(vec![end_lon, end_lat]);

    json!({
        "type": "LineString",
        "coordinates": coordinates
    })
}

fn compute_visibility_analysis(
    center_lon: f64,
    center_lat: f64,
    radius_km: f64,
    gates: &[CityGate],
    terrain: &str,
) -> Value {
    let num_samples = algorithm::DEFENSE_NUM_SAMPLE_POINTS;
    let deg_per_km = 1.0 / 111.0;
    let mut visibility_points = Vec::new();

    for i in 0..num_samples {
        let angle = (i as f64) * (2.0 * PI) / (num_samples as f64);
        let dist = radius_km * (0.8 + 0.2 * (i as f64 * 0.5).sin());
        let vlon = center_lon + dist * deg_per_km * angle.cos();
        let vlat = center_lat + dist * deg_per_km * angle.sin();

        let terrain_factor = match terrain {
            "mountain" => 0.55,
            "hill" => 0.7,
            "plain" => 0.85,
            "wetland" => 0.75,
            "river" => 0.8,
            _ => 0.8,
        };

        let gate_proximity = if !gates.is_empty() {
            let mut min_dist_km = f64::MAX;
            for gate in gates {
                if let Some(geom) = &gate.geom {
                    if let Some(coords) = geom.get("coordinates").and_then(|c| c.as_array()) {
                        if coords.len() >= 2 {
                            let glon = coords[0].as_f64().unwrap_or(0.0);
                            let glat = coords[1].as_f64().unwrap_or(0.0);
                            let d = ((vlon - glon).powi(2) + (vlat - glat).powi(2)).sqrt() * 111.0;
                            min_dist_km = min_dist_km.min(d);
                        }
                    }
                }
            }
            (1.0 - (min_dist_km / radius_km).min(1.0)) * 0.3
        } else {
            0.0
        };

        let visibility = (terrain_factor + gate_proximity).min(1.0).max(0.0);

        visibility_points.push(json!({
            "lon": vlon,
            "lat": vlat,
            "visibility": visibility,
            "angle_deg": angle.to_degrees(),
        }));
    }

    let avg_visibility: f64 = visibility_points
        .iter()
        .filter_map(|p| p.get("visibility").and_then(|v| v.as_f64()))
        .sum::<f64>() / visibility_points.len().max(1) as f64;

    json!({
        "average_visibility": avg_visibility,
        "sample_points": visibility_points,
        "method": "radial_sampling",
        "terrain_factor": match terrain {
            "mountain" => 0.55,
            "hill" => 0.7,
            "plain" => 0.85,
            "wetland" => 0.75,
            "river" => 0.8,
            _ => 0.8,
        }
    })
}

fn compute_overall_defense_score(
    gate_scores: &[GateDefenseScore],
    wall_height: f64,
    wall_width: f64,
    moat_width: f64,
    weak_points: &[DefenseWeakPoint],
    area_sq_km: f64,
    terrain: &str,
) -> f64 {
    let gate_score = if !gate_scores.is_empty() {
        gate_scores.iter().map(|g| g.defense_score).sum::<f64>() / gate_scores.len() as f64
    } else {
        0.65
    };

    let wall_height_score = (wall_height / 15.0).min(1.0) * 0.15;
    let wall_width_score = (wall_width / 10.0).min(1.0) * 0.1;
    let moat_score = (moat_width / 20.0).min(1.0) * 0.1;

    let perimeter = 2.0 * PI * (area_sq_km / PI).sqrt();
    let num_gates = gate_scores.len() as f64;
    let gate_density = num_gates / perimeter;
    let gate_density_score = (1.0 - (gate_density * 2.0).min(0.5) * 2.0) * 0.1;

    let weak_avg = if !weak_points.is_empty() {
        weak_points.iter().map(|w| w.weakness_score).sum::<f64>() / weak_points.len() as f64
    } else {
        0.5
    };
    let weakness_penalty = weak_avg * 0.15;

    let terrain_bonus = match terrain {
        "mountain" => 0.15,
        "hill" => 0.1,
        "plain" => 0.0,
        "wetland" => -0.05,
        "river" => -0.08,
        _ => 0.0,
    };

    let total = (gate_score * 0.4
        + wall_height_score
        + wall_width_score
        + moat_score
        + gate_density_score
        - weakness_penalty
        + terrain_bonus)
        .max(0.0)
        .min(1.0);

    total * 100.0
}

fn compute_accessibility_score(gates: &[CityGate], radius_km: f64) -> f64 {
    if gates.is_empty() { return 50.0; }

    let perimeter = 2.0 * PI * radius_km;
    let gate_density = gates.len() as f64 / perimeter;

    let avg_gate_defense: f64 = gates
        .iter()
        .map(|g| g.defense_rating.unwrap_or(0.7))
        .sum::<f64>() / gates.len() as f64;

    let density_score = (gate_density * 3.0).min(1.0) * 60.0;
    let quality_score = (1.0 - avg_gate_defense) * 40.0;

    (density_score + quality_score).max(0.0).min(100.0)
}

pub async fn save_defense_analysis(
    pool: &sqlx::PgPool,
    result: &DefenseAnalysisResult,
) -> Result<i32, AppError> {
    let weak_points_json = serde_json::to_value(&result.weak_points)?;
    let attack_routes_json = serde_json::to_value(&result.optimal_attack_routes)?;
    let gate_scores_json = serde_json::to_value(&result.gate_defense_scores)?;

    let rec = sqlx::query!(
        r#"
        INSERT INTO defense_analyses (
            site_id, overall_defense_score, visibility_analysis,
            weak_points, optimal_attack_routes, accessibility_score,
            gate_defense_scores, wall_segments
        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
        RETURNING id
        "#,
        result.site_id,
        result.overall_defense_score,
        result.visibility_analysis.clone(),
        weak_points_json,
        attack_routes_json,
        result.accessibility_score,
        gate_scores_json,
        result.wall_segments.clone(),
    )
    .fetch_one(pool)
    .await?;

    Ok(rec.id)
}

use actix_web::{web, HttpResponse};
use crate::models::ApiResponse;

pub async fn analyze_defense_handler(
    pool: web::Data<sqlx::PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let result = analyze_defense(&pool, *site_id).await?;
    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}

pub async fn get_defense_analysis_handler(
    pool: web::Data<sqlx::PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT id, site_id, overall_defense_score, visibility_analysis,
               weak_points, optimal_attack_routes, accessibility_score,
               gate_defense_scores, wall_segments, created_at
        FROM defense_analyses
        WHERE site_id = $1
        ORDER BY id DESC
        LIMIT 1
        "#,
        *site_id
    )
    .fetch_optional(pool.get_ref())
    .await?;

    if row.is_none() {
        let result = analyze_defense(&pool, *site_id).await?;
        return Ok(HttpResponse::Ok().json(ApiResponse::success(result)));
    }

    let r = row.unwrap();
    let weak_points: Vec<DefenseWeakPoint> = serde_json::from_value(
        r.weak_points.unwrap_or(serde_json::json!([]))
    ).unwrap_or_default();
    let attack_routes: Vec<AttackRoute> = serde_json::from_value(
        r.optimal_attack_routes.unwrap_or(serde_json::json!([]))
    ).unwrap_or_default();
    let gate_scores: Vec<GateDefenseScore> = serde_json::from_value(
        r.gate_defense_scores.unwrap_or(serde_json::json!([]))
    ).unwrap_or_default();

    let gates_rows = sqlx::query!(
        r#"
        SELECT id, site_id, name, gate_type, defense_rating,
               ST_AsGeoJSON(geom)::jsonb as geom, description
        FROM city_gates
        WHERE site_id = $1
        ORDER BY id
        "#,
        *site_id
    )
    .fetch_all(pool.get_ref())
    .await
    .unwrap_or_default();

    let gates: Vec<CityGate> = gates_rows
        .iter()
        .map(|gr| CityGate {
            id: gr.id,
            site_id: gr.site_id,
            name: gr.name.clone(),
            gate_type: gr.gate_type.clone(),
            defense_rating: gr.defense_rating.map(|dr| dr as f64),
            geom: gr.geom.clone(),
            description: gr.description.clone(),
            created_at: None,
        })
        .collect();

    let result = DefenseAnalysisResult {
        id: Some(r.id),
        site_id: r.site_id,
        overall_defense_score: r.overall_defense_score,
        visibility_analysis: r.visibility_analysis,
        weak_points,
        optimal_attack_routes: attack_routes,
        accessibility_score: r.accessibility_score,
        gate_defense_scores: gate_scores,
        wall_segments: r.wall_segments,
        gates,
        created_at: r.created_at,
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(result)))
}
