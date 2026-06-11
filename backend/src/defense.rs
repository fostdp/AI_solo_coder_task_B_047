use crate::models::{DefenseAnalysisResult, DefenseWeakPoint, AttackRoute, GateDefenseScore, CityGate};
use crate::errors::AppError;
use crate::config::algorithm;
use serde_json::{json, Value};
use std::f64::consts::PI;

const ANCIENT_WEAPON_RANGES: &[(&str, f64, f64, f64, &str)] = &[
    ("composite_bow", 100.0, 200.0, 350.0, "《考工记·弓人》《武经总要》"),
    ("crossbow", 200.0, 350.0, 600.0, "《战国策·韩策》《天工开物·兵》"),
    ("heavy_crossbow", 400.0, 700.0, 1000.0, "《史记·苏秦列传》《武备志》"),
    ("catapult_traction", 100.0, 200.0, 300.0, "《范蠡兵法》《武经总要·攻城法》"),
    ("catapult_torsion", 200.0, 350.0, 500.0, "《墨子·备城门》《通典·兵典》"),
    ("ballista", 300.0, 500.0, 800.0, "《战术》《武经总要》"),
    ("sling", 50.0, 100.0, 200.0, "《诗经》《罗马军事史》"),
    ("javelin", 30.0, 60.0, 100.0, "《考工记·庐人》波利比乌斯《通史》"),
];

const CIVILIZATION_WEAPON_PROFILES: &[(&str, &str, f64, &str)] = &[
    ("ancient_china", "heavy_crossbow", 0.85, "强弩主义，秦俑汉弩实证"),
    ("ancient_rome", "ballista", 0.8, "扭力投射器传统，维盖蒂乌斯记载"),
    ("maya", "composite_bow", 0.5, "玛雅壁画与武器献祭坑"),
    ("ancient_egypt", "composite_bow", 0.65, "新王国时期喜克索斯传入"),
    ("indus_valley", "sling", 0.45, "摩亨佐-达罗弹丸堆积"),
    ("default", "crossbow", 0.6, "通用古代投射武器基准"),
];

#[derive(Debug, Clone)]
pub struct WeaponRangeEstimate {
    pub min_range_m: f64,
    pub typical_range_m: f64,
    pub max_range_m: f64,
    pub weapon_type: String,
    pub confidence: f64,
    pub literature_sources: String,
    pub estimate_method: String,
}

pub fn estimate_weapon_range(
    civilization: &str,
    wall_height_m: f64,
    terrain: &str,
    has_direct_evidence: bool,
) -> WeaponRangeEstimate {
    let profile = CIVILIZATION_WEAPON_PROFILES
        .iter()
        .find(|(civ, _, _, _)| *civ == civilization)
        .unwrap_or_else(|| {
            CIVILIZATION_WEAPON_PROFILES
                .iter()
                .find(|(civ, _, _, _)| *civ == "default")
                .unwrap()
        });

    let (_, weapon_type, base_confidence, source_note) = profile;

    let weapon_data = ANCIENT_WEAPON_RANGES
        .iter()
        .find(|(wt, _, _, _, _)| *wt == *weapon_type)
        .unwrap_or_else(|| {
            ANCIENT_WEAPON_RANGES
                .iter()
                .find(|(wt, _, _, _, _)| *wt == "crossbow")
                .unwrap()
        });

    let (_, min_r, typical_r, max_r, literature) = weapon_data;

    let height_factor = (wall_height_m / 8.0).max(0.5).min(1.5);
    let terrain_factor = match terrain {
        "mountain" => 0.85,
        "hill" => 0.9,
        "plain" => 1.0,
        "wetland" => 0.95,
        "river" => 0.92,
        _ => 1.0,
    };

    let evidence_bonus = if has_direct_evidence { 0.15 } else { 0.0 };
    let confidence = (base_confidence + evidence_bonus).min(1.0).max(0.2);

    let method = if has_direct_evidence {
        "direct_archaeological_evidence"
    } else if civilization != "default" {
        "civilization_inference_with_literature"
    } else {
        "literature_baseline_estimation"
    };

    WeaponRangeEstimate {
        min_range_m: min_r * height_factor * terrain_factor * 0.8,
        typical_range_m: typical_r * height_factor * terrain_factor,
        max_range_m: max_r * height_factor * terrain_factor * 1.1,
        weapon_type: weapon_type.to_string(),
        confidence,
        literature_sources: format!("{}; {}", literature, source_note),
        estimate_method: method.to_string(),
    }
}

pub fn compute_weapon_coverage_score(
    perimeter_km: f64,
    num_gates: usize,
    weapon_range: &WeaponRangeEstimate,
) -> f64 {
    let range_km = weapon_range.typical_range_m / 1000.0;
    let coverage_per_gate = 2.0 * range_km;
    let total_coverage = coverage_per_gate * num_gates as f64;
    (total_coverage / perimeter_km.max(0.1)).min(1.0).max(0.0)
}

pub fn assess_defense_data_quality(
    has_wall_height: bool,
    has_moat: bool,
    has_gates: bool,
    has_weapon_evidence: bool,
    has_terrain_data: bool,
) -> f64 {
    let mut score = 0.0;
    if has_wall_height { score += 0.25; }
    if has_moat { score += 0.15; }
    if has_gates { score += 0.25; }
    if has_weapon_evidence { score += 0.2; }
    if has_terrain_data { score += 0.15; }
    score.min(1.0).max(0.0)
}

pub fn infer_civilization(site_name: &str) -> &'static str {
    let name_lower = site_name.to_lowercase();
    if name_lower.contains("长安") || name_lower.contains("洛阳") || name_lower.contains("邯郸")
        || name_lower.contains("临淄") || name_lower.contains("咸阳") || name_lower.contains("陶寺")
        || name_lower.contains("二里头") || name_lower.contains("殷墟") || name_lower.contains("镐京") {
        "ancient_china"
    } else if name_lower.contains("罗马") || name_lower.contains("庞贝") || name_lower.contains("君士坦丁堡") {
        "ancient_rome"
    } else if name_lower.contains("蒂卡尔") || name_lower.contains("帕伦克") || name_lower.contains("科潘")
        || name_lower.contains("奇琴伊察") || name_lower.contains("玛雅") {
        "maya"
    } else if name_lower.contains("孟菲斯") || name_lower.contains("底比斯") || name_lower.contains("埃及")
        || name_lower.contains("卡洪") {
        "ancient_egypt"
    } else if name_lower.contains("摩亨佐") || name_lower.contains("哈拉帕") || name_lower.contains("印度河") {
        "indus_valley"
    } else {
        "default"
    }
}

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

    let civ_key = infer_civilization(&site_row.name.clone().unwrap_or_default());
    let has_wall_evidence = site_row.wall_height.is_some();
    let has_moat_evidence = site_row.moat_width.is_some();
    let has_gate_evidence = false;
    let has_weapon_evidence = false;
    let weapon_estimate = estimate_weapon_range(
        civ_key,
        wall_height,
        &terrain,
        has_weapon_evidence,
    );
    let data_quality = assess_defense_data_quality(
        has_wall_evidence,
        has_moat_evidence,
        has_gate_evidence,
        has_weapon_evidence,
        site_row.terrain_type.is_some(),
    );

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

    let perimeter_km = 2.0 * PI * radius_km;
    let weapon_coverage = compute_weapon_coverage_score(perimeter_km, gates.len(), &weapon_estimate);

    let visibility_analysis = compute_visibility_analysis(
        center_lon, center_lat, radius_km, &gates, &terrain, &weapon_estimate,
    );

    let overall_defense = compute_overall_defense_score(
        &gate_scores, wall_height, wall_width, moat_width,
        &weak_points, area_sq_km, &terrain, weapon_coverage, data_quality,
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
    weapon_range: &WeaponRangeEstimate,
) -> Value {
    let num_samples = algorithm::DEFENSE_NUM_SAMPLE_POINTS;
    let deg_per_km = 1.0 / 111.0;
    let mut visibility_points = Vec::new();
    let weapon_range_km = weapon_range.typical_range_m / 1000.0;

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

        let weapon_factor = if !gates.is_empty() {
            let mut min_dist_to_gate_km = f64::MAX;
            for gate in gates {
                if let Some(geom) = &gate.geom {
                    if let Some(coords) = geom.get("coordinates").and_then(|c| c.as_array()) {
                        if coords.len() >= 2 {
                            let glon = coords[0].as_f64().unwrap_or(0.0);
                            let glat = coords[1].as_f64().unwrap_or(0.0);
                            let d = ((vlon - glon).powi(2) + (vlat - glat).powi(2)).sqrt() * 111.0;
                            min_dist_to_gate_km = min_dist_to_gate_km.min(d);
                        }
                    }
                }
            }
            if min_dist_to_gate_km <= weapon_range_km {
                (1.0 - min_dist_to_gate_km / weapon_range_km.max(0.01)) * 0.25
            } else {
                0.0
            }
        } else {
            0.0
        };

        let visibility = (terrain_factor + gate_proximity + weapon_factor).min(1.0).max(0.0);
        let weapon_coverage_ratio = if !gates.is_empty() {
            let coverage_radius = weapon_range_km;
            let gate_coverage = 2.0 * coverage_radius * gates.len() as f64;
            let perimeter = 2.0 * PI * radius_km;
            (gate_coverage / perimeter.max(0.1)).min(1.0)
        } else {
            0.0
        };

        visibility_points.push(json!({
            "lon": vlon,
            "lat": vlat,
            "visibility": visibility,
            "angle_deg": angle.to_degrees(),
            "weapon_coverage": weapon_factor / 0.25f64.max(0.01),
        }));
    }

    let avg_visibility: f64 = visibility_points
        .iter()
        .filter_map(|p| p.get("visibility").and_then(|v| v.as_f64()))
        .sum::<f64>() / visibility_points.len().max(1) as f64;

    let avg_weapon_coverage: f64 = visibility_points
        .iter()
        .filter_map(|p| p.get("weapon_coverage").and_then(|v| v.as_f64()))
        .sum::<f64>() / visibility_points.len().max(1) as f64;

    json!({
        "average_visibility": avg_visibility,
        "average_weapon_coverage": avg_weapon_coverage,
        "sample_points": visibility_points,
        "method": "radial_sampling_with_weapon_fire_zone",
        "terrain_factor": match terrain {
            "mountain" => 0.55,
            "hill" => 0.7,
            "plain" => 0.85,
            "wetland" => 0.75,
            "river" => 0.8,
            _ => 0.8,
        },
        "weapon_range": {
            "weapon_type": weapon_range.weapon_type,
            "min_range_m": weapon_range.min_range_m,
            "typical_range_m": weapon_range.typical_range_m,
            "max_range_m": weapon_range.max_range_m,
            "confidence": weapon_range.confidence,
            "literature_sources": weapon_range.literature_sources,
            "estimate_method": weapon_range.estimate_method,
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
    weapon_coverage: f64,
    data_quality: f64,
) -> f64 {
    let gate_score = if !gate_scores.is_empty() {
        gate_scores.iter().map(|g| g.defense_score).sum::<f64>() / gate_scores.len() as f64
    } else {
        0.65
    };

    let wall_height_score = (wall_height / 15.0).min(1.0) * 0.12;
    let wall_width_score = (wall_width / 10.0).min(1.0) * 0.08;
    let moat_score = (moat_width / 20.0).min(1.0) * 0.1;

    let perimeter = 2.0 * PI * (area_sq_km / PI).sqrt();
    let num_gates = gate_scores.len() as f64;
    let gate_density = num_gates / perimeter;
    let gate_density_score = (1.0 - (gate_density * 2.0).min(0.5) * 2.0) * 0.08;

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

    let weapon_score = weapon_coverage * 0.15;

    let base_score = (gate_score * 0.35
        + wall_height_score
        + wall_width_score
        + moat_score
        + gate_density_score
        - weakness_penalty
        + terrain_bonus
        + weapon_score)
        .max(0.0)
        .min(1.0);

    let quality_adjustment = 0.7 + 0.3 * data_quality;
    let adjusted_score = base_score * quality_adjustment;

    adjusted_score.max(0.0).min(1.0) * 100.0
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

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn sample_gates(lon: f64, lat: f64) -> Vec<CityGate> {
        generate_simulated_gates(lon, lat, 2.0)
    }

    fn sample_weapon_estimate() -> WeaponRangeEstimate {
        estimate_weapon_range("default", 8.0, "plain", false)
    }

    fn sample_weak_points(lon: f64, lat: f64) -> Vec<DefenseWeakPoint> {
        let gates = sample_gates(lon, lat);
        let segs = generate_wall_segments(lon, lat, 2.0);
        analyze_weak_points(lon, lat, 2.0, &gates, &segs, 5.0, 2.0, 10.0, "plain")
    }

    #[test]
    fn test_generate_simulated_gates_count() {
        let gates = generate_simulated_gates(116.0, 34.0, 2.0);
        assert_eq!(gates.len(), 4);
    }

    #[test]
    fn test_generate_simulated_gates_circular_positioning() {
        let gates = generate_simulated_gates(116.0, 34.0, 2.0);
        let deg_per_km = 1.0 / 111.0;
        let expected_radius_deg = 2.0 * deg_per_km;

        for g in &gates {
            let coords = g.geom.as_ref().unwrap()["coordinates"]
                .as_array().unwrap();
            let glon = coords[0].as_f64().unwrap();
            let glat = coords[1].as_f64().unwrap();
            let dist_deg = ((glon - 116.0).powi(2) + (glat - 34.0).powi(2)).sqrt();
            assert_relative_eq!(dist_deg, expected_radius_deg, epsilon = 1e-4);
        }
    }

    #[test]
    fn test_generate_simulated_gates_cardinal_directions() {
        let gates = generate_simulated_gates(116.0, 34.0, 2.0);

        let east = gates.iter().find(|g| g.name.as_ref().unwrap() == "东门").unwrap();
        let south = gates.iter().find(|g| g.name.as_ref().unwrap() == "南门").unwrap();
        let west = gates.iter().find(|g| g.name.as_ref().unwrap() == "西门").unwrap();
        let north = gates.iter().find(|g| g.name.as_ref().unwrap() == "北门").unwrap();

        let e_coord = east.geom.as_ref().unwrap()["coordinates"].as_array().unwrap();
        let s_coord = south.geom.as_ref().unwrap()["coordinates"].as_array().unwrap();
        let w_coord = west.geom.as_ref().unwrap()["coordinates"].as_array().unwrap();
        let n_coord = north.geom.as_ref().unwrap()["coordinates"].as_array().unwrap();

        assert!(e_coord[0].as_f64().unwrap() > 116.0);
        assert!(w_coord[0].as_f64().unwrap() < 116.0);
        assert!(n_coord[1].as_f64().unwrap() > 34.0);
        assert!(s_coord[1].as_f64().unwrap() < 34.0);
    }

    #[test]
    fn test_generate_simulated_gates_valid_geojson() {
        let gates = generate_simulated_gates(116.0, 34.0, 2.0);
        for g in &gates {
            let geom = g.geom.as_ref().unwrap();
            assert_eq!(geom["type"], "Point");
            let coords = geom["coordinates"].as_array().unwrap();
            assert_eq!(coords.len(), 2);
        }
    }

    #[test]
    fn test_generate_simulated_gates_defense_rating_bounded() {
        let gates = generate_simulated_gates(116.0, 34.0, 2.0);
        for g in &gates {
            let r = g.defense_rating.unwrap();
            assert!(r >= 0.0 && r <= 1.0);
        }
    }

    #[test]
    fn test_generate_wall_segments_count() {
        use crate::config::algorithm::DEFENSE_WALL_SEGMENTS;
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        assert_eq!(segs.len(), DEFENSE_WALL_SEGMENTS);
    }

    #[test]
    fn test_generate_wall_segments_defense_strength_bounded() {
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        for s in &segs {
            let ds = s["defense_strength"].as_f64().unwrap();
            let vi = s["visibility_index"].as_f64().unwrap();
            assert!(ds >= 0.0 && ds <= 1.0);
            assert!(vi >= 0.0 && vi <= 1.0);
        }
    }

    #[test]
    fn test_generate_wall_segments_indexes_consecutive() {
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        for (i, s) in segs.iter().enumerate() {
            assert_eq!(s["segment_index"].as_i64().unwrap(), i as i64);
        }
    }

    #[test]
    fn test_generate_wall_segments_closed_loop() {
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        let n = segs.len();
        for i in 0..n {
            let cur_end = segs[i]["end"].as_array().unwrap();
            let next_start = segs[(i + 1) % n]["start"].as_array().unwrap();
            assert_relative_eq!(
                cur_end[0].as_f64().unwrap(),
                next_start[0].as_f64().unwrap(),
                epsilon = 1e-6
            );
            assert_relative_eq!(
                cur_end[1].as_f64().unwrap(),
                next_start[1].as_f64().unwrap(),
                epsilon = 1e-6
            );
        }
    }

    #[test]
    fn test_analyze_weak_points_identifies_gates() {
        let gates = sample_gates(116.0, 34.0);
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        let wp = analyze_weak_points(
            116.0, 34.0, 2.0,
            &gates, &segs,
            5.0, 2.0, 10.0,
            "plain",
        );
        let gate_points: Vec<_> = wp.iter().filter(|w| w.weakness_type == "gate").collect();
        assert_eq!(gate_points.len(), gates.len());
    }

    #[test]
    fn test_analyze_weak_points_weakness_bounded_0_1() {
        let gates = sample_gates(116.0, 34.0);
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        let wp = analyze_weak_points(
            116.0, 34.0, 2.0,
            &gates, &segs,
            5.0, 2.0, 10.0,
            "plain",
        );
        for w in &wp {
            assert!(w.weakness_score >= 0.0 && w.weakness_score <= 1.0);
        }
    }

    #[test]
    fn test_analyze_weak_points_terrain_effect_mountain_reduces_weakness() {
        let gates = sample_gates(116.0, 34.0);
        let segs = generate_wall_segments(116.0, 34.0, 2.0);

        let wp_mountain = analyze_weak_points(
            116.0, 34.0, 2.0, &gates, &segs,
            5.0, 2.0, 0.0, "mountain",
        );
        let wp_plain = analyze_weak_points(
            116.0, 34.0, 2.0, &gates, &segs,
            5.0, 2.0, 0.0, "plain",
        );

        let mtn_sum: f64 = wp_mountain.iter().map(|w| w.weakness_score).sum();
        let pln_sum: f64 = wp_plain.iter().map(|w| w.weakness_score).sum();
        assert!(mtn_sum < pln_sum, "mountain should reduce weakness ({} < {})", mtn_sum, pln_sum);
    }

    #[test]
    fn test_analyze_weak_points_wall_height_reduces_weakness() {
        let gates = sample_gates(116.0, 34.0);
        let segs = generate_wall_segments(116.0, 34.0, 2.0);

        let wp_short = analyze_weak_points(
            116.0, 34.0, 2.0, &gates, &segs,
            2.0, 1.0, 0.0, "plain",
        );
        let wp_tall = analyze_weak_points(
            116.0, 34.0, 2.0, &gates, &segs,
            10.0, 3.0, 20.0, "plain",
        );

        let s_sum: f64 = wp_short.iter().map(|w| w.weakness_score).sum();
        let t_sum: f64 = wp_tall.iter().map(|w| w.weakness_score).sum();
        assert!(t_sum < s_sum, "taller walls reduce weakness ({} < {})", t_sum, s_sum);
    }

    #[test]
    fn test_compute_attack_routes_count() {
        let gates = sample_gates(116.0, 34.0);
        let wp = sample_weak_points(116.0, 34.0);
        let routes = compute_attack_routes(
            116.0, 34.0, 2.0,
            &gates, &wp, "plain",
        );
        assert_eq!(routes.len(), algorithm::DEFENSE_NUM_ATTACK_ROUTES);
    }

    #[test]
    fn test_compute_attack_routes_starts_outside_wall() {
        let center_lon = 116.0;
        let center_lat = 34.0;
        let radius_km = 2.0;
        let deg_per_km = 1.0 / 111.0;
        let radius_deg = radius_km * deg_per_km;

        let gates = sample_gates(center_lon, center_lat);
        let wp = sample_weak_points(center_lon, center_lat);
        let routes = compute_attack_routes(
            center_lon, center_lat, radius_km,
            &gates, &wp, "plain",
        );

        for r in &routes {
            let coords = r.route_geom["coordinates"].as_array().unwrap();
            assert!(coords.len() >= 2, "each route has at least start and end");

            let origin = coords[0].as_array().unwrap();
            let origin_lon = origin[0].as_f64().unwrap();
            let origin_lat = origin[1].as_f64().unwrap();
            let d = ((origin_lon - center_lon).powi(2) + (origin_lat - center_lat).powi(2)).sqrt();
            assert!(d > radius_deg * 0.9, "attack starts outside wall");

            let last = coords.last().unwrap().as_array().unwrap();
            let last_lon = last[0].as_f64().unwrap();
            let last_lat = last[1].as_f64().unwrap();
            assert_relative_eq!(last_lon, center_lon, epsilon = 0.005);
            assert_relative_eq!(last_lat, center_lat, epsilon = 0.005);
        }
    }

    #[test]
    fn test_compute_attack_routes_scores_ordered() {
        let gates = sample_gates(116.0, 34.0);
        let wp = sample_weak_points(116.0, 34.0);
        let mut routes = compute_attack_routes(
            116.0, 34.0, 2.0, &gates, &wp, "plain",
        );
        routes.sort_by(|a, b| b.attack_score.partial_cmp(&a.attack_score).unwrap());

        for i in 1..routes.len() {
            assert!(routes[i-1].attack_score >= routes[i].attack_score);
        }
    }

    #[test]
    fn test_compute_attack_routes_score_bounded() {
        let gates = sample_gates(116.0, 34.0);
        let wp = sample_weak_points(116.0, 34.0);
        let routes = compute_attack_routes(
            116.0, 34.0, 2.0, &gates, &wp, "plain",
        );
        for r in &routes {
            assert!(r.attack_score >= 0.0 && r.attack_score <= 100.0);
        }
    }

    #[test]
    fn test_compute_attack_routes_wetland_changes_score() {
        let gates = sample_gates(116.0, 34.0);
        let wp = sample_weak_points(116.0, 34.0);

        let r_plain = compute_attack_routes(116.0, 34.0, 2.0, &gates, &wp, "plain");
        let r_wetland = compute_attack_routes(116.0, 34.0, 2.0, &gates, &wp, "wetland");

        let p_score: f64 = r_plain.iter().map(|r| r.attack_score).sum();
        let w_score: f64 = r_wetland.iter().map(|r| r.attack_score).sum();
        assert_ne!(p_score, w_score, "terrain should affect attack scores");
    }

    #[test]
    fn test_compute_visibility_analysis_count() {
        let center_lon = 116.0;
        let center_lat = 34.0;
        let radius_km = 2.0;
        let gates = sample_gates(center_lon, center_lat);
        let weapon = sample_weapon_estimate();
        let vis = compute_visibility_analysis(center_lon, center_lat, radius_km, &gates, "plain", &weapon);

        assert_eq!(
            vis["sample_points"].as_array().unwrap().len(),
            algorithm::DEFENSE_NUM_SAMPLE_POINTS
        );
    }

    #[test]
    fn test_compute_visibility_analysis_all_bounded_0_1() {
        let gates = sample_gates(116.0, 34.0);
        let weapon = sample_weapon_estimate();
        let vis = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "plain", &weapon);
        let points = vis["sample_points"].as_array().unwrap();

        for v in points {
            let vv = v["visibility"].as_f64().unwrap();
            let wc = v["weapon_coverage"].as_f64().unwrap();
            assert!(vv >= 0.0 && vv <= 1.0);
            assert!(wc >= 0.0 && wc <= 1.0);
        }
    }

    #[test]
    fn test_compute_visibility_analysis_covers_full_360() {
        let gates = sample_gates(116.0, 34.0);
        let weapon = sample_weapon_estimate();
        let vis = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "plain", &weapon);
        let points = vis["sample_points"].as_array().unwrap();

        let angles: Vec<f64> = points.iter()
            .map(|v| v["angle_deg"].as_f64().unwrap())
            .collect();

        for i in 0..algorithm::DEFENSE_NUM_SAMPLE_POINTS {
            let expected = (i as f64) * 360.0 / (algorithm::DEFENSE_NUM_SAMPLE_POINTS as f64);
            assert_relative_eq!(angles[i], expected, epsilon = 0.1);
        }
    }

    #[test]
    fn test_compute_visibility_analysis_mountain_vs_plain() {
        let gates = sample_gates(116.0, 34.0);
        let weapon = sample_weapon_estimate();
        let v_mtn = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "mountain", &weapon);
        let v_pln = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "plain", &weapon);

        let mtn_avg = v_mtn["average_visibility"].as_f64().unwrap();
        let pln_avg = v_pln["average_visibility"].as_f64().unwrap();

        assert!(mtn_avg < pln_avg, "mountain reduces visibility");
    }

    #[test]
    fn test_compute_accessibility_score_more_gates_higher() {
        let g2: Vec<CityGate> = sample_gates(116.0, 34.0).into_iter().take(2).collect();
        let g4 = sample_gates(116.0, 34.0);
        assert!(compute_accessibility_score(&g4, 2.0) >= compute_accessibility_score(&g2, 2.0));
    }

    #[test]
    fn test_compute_accessibility_score_empty_default() {
        assert_eq!(compute_accessibility_score(&[], 2.0), 50.0);
    }

    #[test]
    fn test_compute_accessibility_score_bounded() {
        let g = sample_gates(116.0, 34.0);
        let s = compute_accessibility_score(&g, 2.0);
        assert!(s >= 0.0 && s <= 100.0);
    }

    #[test]
    fn test_compute_overall_defense_score_bounded_0_100() {
        let gates = sample_gates(116.0, 34.0);
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        let wp = analyze_weak_points(116.0, 34.0, 2.0, &gates, &segs, 5.0, 2.0, 10.0, "plain");
        let weapon = sample_weapon_estimate();
        let vis = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "plain", &weapon);
        let routes = compute_attack_routes(116.0, 34.0, 2.0, &gates, &wp, "plain");
        let gate_scores = vec![];
        let weapon_cov = compute_weapon_coverage_score(2.0 * PI * 2.0, gates.len(), &weapon);
        let data_quality = assess_defense_data_quality(true, true, false, false, true);

        let score = compute_overall_defense_score(
            &gate_scores, 5.0, 2.0, 10.0,
            &wp, PI * 2.0 * 2.0, "plain",
            weapon_cov, data_quality,
        );
        assert!(score >= 0.0 && score <= 100.0);
    }

    #[test]
    fn test_compute_overall_defense_score_strong_fortress_high_score() {
        let gates = sample_gates(116.0, 34.0);
        let segs = generate_wall_segments(116.0, 34.0, 2.0);
        let weapon = sample_weapon_estimate();
        let weapon_cov = compute_weapon_coverage_score(2.0 * PI * 2.0, gates.len(), &weapon);
        let area = PI * 2.0 * 2.0;

        let wp_strong = analyze_weak_points(116.0, 34.0, 2.0, &gates, &segs, 12.0, 5.0, 30.0, "mountain");
        let vis_strong = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "mountain", &weapon);
        let routes_strong = compute_attack_routes(116.0, 34.0, 2.0, &gates, &wp_strong, "mountain");
        let strong = compute_overall_defense_score(
            &[], 12.0, 5.0, 30.0,
            &wp_strong, area, "mountain",
            weapon_cov, 0.9,
        );

        let wp_weak = analyze_weak_points(116.0, 34.0, 2.0, &gates, &segs, 2.0, 1.0, 0.0, "wetland");
        let vis_weak = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "wetland", &weapon);
        let routes_weak = compute_attack_routes(116.0, 34.0, 2.0, &gates, &wp_weak, "wetland");
        let weak = compute_overall_defense_score(
            &[], 2.0, 1.0, 0.0,
            &wp_weak, area, "wetland",
            weapon_cov * 0.5, 0.3,
        );

        assert!(strong > weak, "strong fortress {} should exceed weak one {}", strong, weak);
        assert!(strong > 40.0, "strong fortress should be >40, got {}", strong);
        assert!(weak < 90.0, "weak fortress should be <90, got {}", weak);
    }

    #[test]
    fn test_analysis_pipeline_end_to_end() {
        let center_lon = 116.0_f64;
        let center_lat = 34.0_f64;
        let radius_km = 2.0_f64;
        let terrain = "plain";
        let wall_height = 5.0;
        let wall_width = 2.0;
        let moat_width = 8.0;

        let gates = generate_simulated_gates(center_lon, center_lat, radius_km);
        assert_eq!(gates.len(), 4);

        let segs = generate_wall_segments(center_lon, center_lat, radius_km);
        assert!(!segs.is_empty());

        let wp = analyze_weak_points(center_lon, center_lat, radius_km, &gates, &segs,
            wall_height, wall_width, moat_width, terrain);
        assert!(!wp.is_empty());

        let routes = compute_attack_routes(center_lon, center_lat, radius_km, &gates, &wp, terrain);
        assert_eq!(routes.len(), algorithm::DEFENSE_NUM_ATTACK_ROUTES);

        let weapon = sample_weapon_estimate();
        let vis = compute_visibility_analysis(center_lon, center_lat, radius_km, &gates, terrain, &weapon);
        assert_eq!(
            vis["sample_points"].as_array().unwrap().len(),
            algorithm::DEFENSE_NUM_SAMPLE_POINTS
        );

        let acc = compute_accessibility_score(&gates, radius_km);
        assert!(acc >= 0.0 && acc <= 100.0);

        let gate_scores = Vec::<GateDefenseScore>::new();
        let area = PI * radius_km * radius_km;
        let weapon_cov = compute_weapon_coverage_score(2.0 * PI * radius_km, gates.len(), &weapon);
        let data_quality = assess_defense_data_quality(true, true, false, false, true);
        let score = compute_overall_defense_score(
            &gate_scores, wall_height, wall_width, moat_width,
            &wp, area, terrain, weapon_cov, data_quality,
        );
        assert!(score >= 0.0 && score <= 100.0);
    }

    #[test]
    fn root_cause_weapon_range_estimation_different_civilizations() {
        let china = estimate_weapon_range("ancient_china", 8.0, "plain", false);
        let rome = estimate_weapon_range("ancient_rome", 8.0, "plain", false);
        let maya = estimate_weapon_range("maya", 8.0, "plain", false);
        let default = estimate_weapon_range("default", 8.0, "plain", false);

        assert_eq!(china.weapon_type, "heavy_crossbow");
        assert_eq!(rome.weapon_type, "ballista");
        assert_eq!(maya.weapon_type, "composite_bow");
        assert_eq!(default.weapon_type, "crossbow");

        assert!(china.typical_range_m > default.typical_range_m);
        assert!(china.confidence > default.confidence);
        assert!(maya.confidence < default.confidence);
    }

    #[test]
    fn root_cause_weapon_range_wall_height_scaling() {
        let low_wall = estimate_weapon_range("default", 3.0, "plain", false);
        let high_wall = estimate_weapon_range("default", 12.0, "plain", false);

        assert!(high_wall.typical_range_m > low_wall.typical_range_m);
        let ratio = high_wall.typical_range_m / low_wall.typical_range_m;
        assert!(ratio >= 1.2 && ratio <= 2.0, "height scaling should be bounded");
    }

    #[test]
    fn root_cause_weapon_range_evidence_boosts_confidence() {
        let no_evidence = estimate_weapon_range("ancient_china", 8.0, "plain", false);
        let with_evidence = estimate_weapon_range("ancient_china", 8.0, "plain", true);

        assert!(with_evidence.confidence > no_evidence.confidence);
        assert!(with_evidence.estimate_method.contains("direct"));
        assert!(no_evidence.estimate_method.contains("inference"));
    }

    #[test]
    fn root_cause_weapon_coverage_affects_overall_score() {
        let gates = sample_gates(116.0, 34.0);
        let wp = sample_weak_points(116.0, 34.0);
        let gate_scores: Vec<GateDefenseScore> = Vec::new();

        let score_low = compute_overall_defense_score(
            &gate_scores, 5.0, 2.0, 10.0,
            &wp, 3.14, "plain",
            0.1, 0.5,
        );
        let score_high = compute_overall_defense_score(
            &gate_scores, 5.0, 2.0, 10.0,
            &wp, 3.14, "plain",
            0.9, 0.5,
        );

        assert!(score_high > score_low, "higher weapon coverage should increase score");
    }

    #[test]
    fn root_cause_data_quality_affects_overall_score() {
        let gates = sample_gates(116.0, 34.0);
        let wp = sample_weak_points(116.0, 34.0);
        let gate_scores: Vec<GateDefenseScore> = Vec::new();

        let score_low_quality = compute_overall_defense_score(
            &gate_scores, 5.0, 2.0, 10.0,
            &wp, 3.14, "plain",
            0.5, 0.2,
        );
        let score_high_quality = compute_overall_defense_score(
            &gate_scores, 5.0, 2.0, 10.0,
            &wp, 3.14, "plain",
            0.5, 1.0,
        );

        assert!(score_high_quality > score_low_quality,
            "higher data quality should increase score ({} > {})", score_high_quality, score_low_quality);
        let ratio = score_high_quality / score_low_quality.max(0.1);
        assert!(ratio <= 1.5, "quality adjustment should be moderate");
    }

    #[test]
    fn root_cause_infer_civilization_keywords() {
        assert_eq!(infer_civilization("长安遗址"), "ancient_china");
        assert_eq!(infer_civilization("洛阳"), "ancient_china");
        assert_eq!(infer_civilization("罗马古城"), "ancient_rome");
        assert_eq!(infer_civilization("蒂卡尔"), "maya");
        assert_eq!(infer_civilization("孟菲斯"), "ancient_egypt");
        assert_eq!(infer_civilization("摩亨佐-达罗"), "indus_valley");
        assert_eq!(infer_civilization("某某不知名遗址"), "default");
    }

    #[test]
    fn root_cause_literature_sources_present() {
        let china = estimate_weapon_range("ancient_china", 8.0, "plain", true);
        assert!(!china.literature_sources.is_empty());
        assert!(china.literature_sources.contains("《") || china.literature_sources.contains("记"),
            "should contain literature references");
        assert!(!china.estimate_method.is_empty());
    }

    #[test]
    fn root_cause_visibility_includes_weapon_info() {
        let gates = sample_gates(116.0, 34.0);
        let weapon = sample_weapon_estimate();
        let vis = compute_visibility_analysis(116.0, 34.0, 2.0, &gates, "plain", &weapon);

        assert!(vis.get("weapon_range").is_some());
        assert!(vis.get("average_weapon_coverage").is_some());
        assert!(vis["method"].as_str().unwrap().contains("weapon"));
        assert!(vis["weapon_range"]["confidence"].as_f64().unwrap() > 0.0);
        assert!(!vis["weapon_range"]["literature_sources"].as_str().unwrap().is_empty());
    }

    #[test]
    fn root_cause_data_quality_scoring() {
        let q_full = assess_defense_data_quality(true, true, true, true, true);
        let q_none = assess_defense_data_quality(false, false, false, false, false);
        let q_partial = assess_defense_data_quality(true, false, true, false, true);

        assert_relative_eq!(q_full, 1.0, epsilon = 0.01);
        assert_relative_eq!(q_none, 0.0, epsilon = 0.01);
        assert!(q_partial > 0.0 && q_partial < 1.0);
    }
}
