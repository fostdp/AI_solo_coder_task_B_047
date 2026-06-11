use crate::models::{
    CityGate, DefenseAnalysis, DefenseWeakPoint, WeaponRangeEstimate as ModelWeaponRange,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::f64::consts::PI;
use std::sync::Arc;
use tokio::task;

pub const ANCIENT_WEAPON_RANGES: &[(&str, f64, f64, f64, &str)] = &[
    ("compound_bow", 50.0, 150.0, 300.0, "《武经总要》卷十三"),
    ("crossbow", 100.0, 250.0, 450.0, "《墨子·备城门》"),
    ("heavy_crossbow", 200.0, 500.0, 800.0, "《史记·苏秦列传》"),
    ("catapult", 100.0, 300.0, 500.0, "《通典·兵典》"),
    ("torsion_catapult", 150.0, 350.0, 600.0, "波利比乌斯《通史》"),
    ("ballista", 200.0, 400.0, 700.0, "维格蒂乌斯《兵法简述》"),
    ("trebuchet", 100.0, 250.0, 400.0, "《襄阳守城录》"),
    ("javelin", 20.0, 50.0, 80.0, "《左传》"),
];

pub const CIVILIZATION_WEAPON_PROFILES: &[(&str, &str, f64, &str)] = &[
    ("ancient_china", "heavy_crossbow", 0.85, "强弩为特色，射程远"),
    ("ancient_rome", "ballista", 0.8, "扭力弩炮为代表"),
    ("maya", "compound_bow", 0.7, "复合弓为主，丛林作战"),
    ("ancient_egypt", "compound_bow", 0.75, "复合弓+战车"),
    ("indus_valley", "trebuchet", 0.65, "投石器发达"),
    ("mesopotamia", "javelin", 0.7, "标枪+战车"),
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeaponRangeEstimate {
    pub min_range_m: f64,
    pub typical_range_m: f64,
    pub max_range_m: f64,
    pub weapon_type: String,
    pub confidence: f64,
    pub literature_sources: Vec<String>,
    pub estimate_method: String,
}

#[derive(Debug, Clone)]
pub struct VisibilityAnalysisConfig {
    pub num_sample_points: usize,
    pub include_weapon_factor: bool,
    pub terrain_attenuation: bool,
}

impl Default for VisibilityAnalysisConfig {
    fn default() -> Self {
        VisibilityAnalysisConfig {
            num_sample_points: 36,
            include_weapon_factor: true,
            terrain_attenuation: true,
        }
    }
}

pub struct DefenseAnalyzer {
    visibility_config: VisibilityAnalysisConfig,
}

impl DefenseAnalyzer {
    pub fn new(visibility_config: VisibilityAnalysisConfig) -> Self {
        DefenseAnalyzer { visibility_config }
    }

    pub fn infer_civilization(&self, site_name: &str) -> &'static str {
        let name_lower = site_name.to_lowercase();
        for (key, weapon, _, _) in CIVILIZATION_WEAPON_PROFILES {
            let civ_lower = key.to_lowercase();
            if name_lower.contains(&civ_lower) {
                return key;
            }
        }

        if site_name.contains("长安") || site_name.contains("洛阳") || site_name.contains("邯郸") {
            "ancient_china"
        } else if site_name.contains("罗马") || site_name.contains("庞贝") {
            "ancient_rome"
        } else if site_name.contains("蒂卡尔") || site_name.contains("帕伦克") {
            "maya"
        } else if site_name.contains("底比斯") || site_name.contains("孟菲斯") {
            "ancient_egypt"
        } else if site_name.contains("摩亨佐") || site_name.contains("哈拉帕") {
            "indus_valley"
        } else {
            "general"
        }
    }

    pub fn estimate_weapon_range(
        &self,
        civilization: &str,
        wall_height_m: f64,
        terrain: &str,
        has_direct_evidence: bool,
    ) -> WeaponRangeEstimate {
        let profile = CIVILIZATION_WEAPON_PROFILES
            .iter()
            .find(|(key, _, _, _)| *key == civilization)
            .unwrap_or(&CIVILIZATION_WEAPON_PROFILES[5]);

        let weapon_type = profile.1;
        let weapon = ANCIENT_WEAPON_RANGES
            .iter()
            .find(|(key, _, _, _, _)| *key == weapon_type)
            .unwrap_or(&ANCIENT_WEAPON_RANGES[0]);

        let height_factor = (wall_height_m / 8.0).max(0.5).min(1.5);
        let terrain_factor = match terrain {
            "mountain" => 0.8,
            "hilly" => 0.9,
            "wetland" => 0.85,
            "plain" => 1.0,
            "desert" => 1.05,
            _ => 1.0,
        };

        let evidence_bonus = if has_direct_evidence { 1.15 } else { 1.0 };

        let min_range = weapon.1 * height_factor * terrain_factor * 0.9;
        let typical_range = weapon.2 * height_factor * terrain_factor;
        let max_range = weapon.3 * height_factor * terrain_factor * evidence_bonus * 1.05;

        let base_confidence = profile.2;
        let conf_adjustment = if wall_height_m > 0.0 { 0.9 } else { 0.7 };
        let confidence = (base_confidence * conf_adjustment * evidence_bonus).min(0.95);

        let mut sources = vec![weapon.4.to_string()];
        sources.push(profile.3.to_string());

        let method = if has_direct_evidence {
            "direct_evidence_with_adjustment"
        } else {
            "civilization_profile_inference"
        }
        .to_string();

        WeaponRangeEstimate {
            min_range_m: min_range.max(10.0),
            typical_range_m: typical_range.max(20.0),
            max_range_m: max_range.max(30.0),
            weapon_type: weapon_type.to_string(),
            confidence,
            literature_sources: sources,
            estimate_method: method,
        }
    }

    pub fn compute_weapon_coverage_score(
        &self,
        perimeter_km: f64,
        num_gates: usize,
        weapon_range: &WeaponRangeEstimate,
    ) -> f64 {
        if perimeter_km <= 0.0 || num_gates == 0 {
            return 0.0;
        }

        let effective_range_km = weapon_range.typical_range_m / 1000.0;
        let coverage_per_gate = 2.0 * effective_range_km;
        let total_coverage = coverage_per_gate * num_gates as f64;

        (total_coverage / perimeter_km).min(1.0) * 100.0
    }

    pub fn assess_defense_data_quality(
        &self,
        has_wall_height: bool,
        has_moat: bool,
        has_gates: bool,
        has_weapon_evidence: bool,
        has_terrain_data: bool,
    ) -> f64 {
        let mut score = 0.0;
        score += if has_wall_height { 0.25 } else { 0.0 };
        score += if has_moat { 0.15 } else { 0.0 };
        score += if has_gates { 0.25 } else { 0.0 };
        score += if has_weapon_evidence { 0.2 } else { 0.0 };
        score += if has_terrain_data { 0.15 } else { 0.0 };
        score
    }

    pub async fn compute_visibility_analysis_async(
        &self,
        center_lon: f64,
        center_lat: f64,
        radius_km: f64,
        gates: Arc<Vec<CityGate>>,
        terrain: String,
        weapon_range: WeaponRangeEstimate,
    ) -> Value {
        let config = self.visibility_config.clone();
        task::spawn_blocking(move || {
            Self::compute_visibility_analysis_sync(
                center_lon,
                center_lat,
                radius_km,
                &gates,
                &terrain,
                &weapon_range,
                &config,
            )
        })
        .await
        .unwrap_or_else(|e| {
            json!({
                "error": format!("visibility calculation failed: {}", e),
                "sample_points": [],
                "average_visibility": 0.0,
                "method": "fallback_empty"
            })
        })
    }

    pub fn compute_visibility_analysis_sync(
        center_lon: f64,
        center_lat: f64,
        radius_km: f64,
        gates: &[CityGate],
        terrain: &str,
        weapon_range: &WeaponRangeEstimate,
        config: &VisibilityAnalysisConfig,
    ) -> Value {
        let num_points = config.num_sample_points;
        let mut sample_points = Vec::with_capacity(num_points);
        let mut total_visibility = 0.0;

        let deg_per_km = 1.0 / 111.0;
        let radius_deg = radius_km * deg_per_km;

        let terrain_factor = match terrain {
            "mountain" => 0.6,
            "hilly" => 0.75,
            "wetland" => 0.8,
            "plain" => 1.0,
            _ => 0.9,
        };

        for i in 0..num_points {
            let angle = 2.0 * PI * i as f64 / num_points as f64;
            let dist_ratio = 0.3 + (i % 3) as f64 * 0.3;

            let lon = center_lon + radius_deg * dist_ratio * angle.cos();
            let lat = center_lat + radius_deg * dist_ratio * angle.sin();

            let mut min_gate_dist = f64::INFINITY;
            for gate in gates {
                let d_lon = lon - gate.longitude;
                let d_lat = lat - gate.latitude;
                let dist_km = (d_lon * d_lon + d_lat * d_lat).sqrt() * 111.0;
                min_gate_dist = min_gate_dist.min(dist_km);
            }

            let distance_factor = if min_gate_dist < f64::INFINITY {
                (1.0 - min_gate_dist / radius_km.max(0.01)).max(0.0)
            } else {
                0.5
            };

            let weapon_factor = if config.include_weapon_factor {
                let range_km = weapon_range.typical_range_m / 1000.0;
                (min_gate_dist / range_km.max(0.01)).min(1.0)
            } else {
                1.0
            };

            let attenuation = if config.terrain_attenuation {
                terrain_factor
            } else {
                1.0
            };

            let visibility = (0.25 * distance_factor + 0.25 * weapon_factor + 0.25 * attenuation + 0.25)
                .max(0.0)
                .min(1.0);

            total_visibility += visibility;

            sample_points.push(json!({
                "lon": lon,
                "lat": lat,
                "visibility": visibility,
                "distance_to_gate_km": min_gate_dist,
                "angle_deg": angle * 180.0 / PI,
            }));
        }

        let avg_visibility = total_visibility / num_points as f64;

        let weapon_effective = weapon_range.typical_range_m / 1000.0;
        let avg_weapon_coverage = (weapon_effective * gates.len() as f64) / (2.0 * PI * radius_km.max(0.01));

        json!({
            "sample_points": sample_points,
            "average_visibility": avg_visibility,
            "weapon_range": {
                "min_m": weapon_range.min_range_m,
                "typical_m": weapon_range.typical_range_m,
                "max_m": weapon_range.max_range_m,
                "weapon_type": weapon_range.weapon_type,
                "confidence": weapon_range.confidence,
                "sources": weapon_range.literature_sources,
                "method": weapon_range.estimate_method,
            },
            "average_weapon_coverage": avg_weapon_coverage.min(1.0),
            "method": format!("visibility_analysis_{}_points_threaded", num_points),
            "terrain_factor": terrain_factor,
        })
    }

    pub fn compute_overall_defense_score(
        &self,
        gate_scores: &[f64],
        wall_height_m: f64,
        wall_width_m: f64,
        moat_width_m: f64,
        weak_points: &[DefenseWeakPoint],
        site_area_km2: f64,
        terrain: &str,
        weapon_coverage: f64,
        data_quality: f64,
    ) -> f64 {
        let perimeter = 2.0 * PI * (site_area_km2 / PI).sqrt();

        let height_score = (wall_height_m / 12.0).min(1.0) * 100.0;
        let width_score = (wall_width_m / 5.0).min(1.0) * 100.0;
        let moat_score = (moat_width_m / 30.0).min(1.0) * 100.0;

        let num_gates = gate_scores.len().max(1);
        let gate_density = num_gates as f64 / perimeter.max(0.01);
        let gate_density_score = (1.0 - (gate_density * 2.0).min(1.0)) * 100.0;

        let weakness_penalty = if !weak_points.is_empty() {
            let avg_weakness: f64 = weak_points.iter().map(|w| w.weakness_score).sum::<f64>()
                / weak_points.len() as f64;
            avg_weakness * 30.0
        } else {
            0.0
        };

        let terrain_bonus = match terrain {
            "mountain" => 15.0,
            "hilly" => 10.0,
            "wetland" => 8.0,
            _ => 0.0,
        };

        let weapon_score = (weapon_coverage / 100.0).min(1.0) * 15.0;

        let base_score = (height_score * 0.175
            + width_score * 0.125
            + moat_score * 0.125
            + gate_density_score * 0.35
            - weakness_penalty
            + terrain_bonus
            + weapon_score)
            .max(0.0)
            .min(100.0);

        let quality_adjustment = 0.7 + 0.3 * data_quality;
        (base_score * quality_adjustment).max(0.0).min(100.0)
    }

    pub fn weapon_to_model_format(&self, estimate: &WeaponRangeEstimate) -> ModelWeaponRange {
        ModelWeaponRange {
            min_range_m: estimate.min_range_m,
            typical_range_m: estimate.typical_range_m,
            max_range_m: estimate.max_range_m,
            weapon_type: estimate.weapon_type.clone(),
            confidence: estimate.confidence,
            literature_sources: estimate.literature_sources.clone(),
            estimate_method: estimate.estimate_method.clone(),
        }
    }
}

impl Default for DefenseAnalyzer {
    fn default() -> Self {
        DefenseAnalyzer::new(VisibilityAnalysisConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::assert_relative_eq;

    fn sample_gates() -> Vec<CityGate> {
        vec![
            CityGate {
                id: 1,
                site_id: 1,
                name: Some("南门".to_string()),
                longitude: 116.0,
                latitude: 33.99,
                defense_rating: Some(0.8),
                width_m: Some(10.0),
                gate_type: Some("main".to_string()),
                geom: Some(json!({
                    "type": "Point",
                    "coordinates": [116.0, 33.99]
                })),
                description: None,
                created_at: None,
            },
            CityGate {
                id: 2,
                site_id: 1,
                name: Some("北门".to_string()),
                longitude: 116.0,
                latitude: 34.01,
                defense_rating: Some(0.7),
                width_m: Some(8.0),
                gate_type: Some("main".to_string()),
                geom: Some(json!({
                    "type": "Point",
                    "coordinates": [116.0, 34.01]
                })),
                description: None,
                created_at: None,
            },
        ]
    }

    #[test]
    fn test_infer_civilization_changan() {
        let analyzer = DefenseAnalyzer::default();
        assert_eq!(analyzer.infer_civilization("长安城遗址"), "ancient_china");
    }

    #[test]
    fn test_estimate_weapon_range_china() {
        let analyzer = DefenseAnalyzer::default();
        let estimate = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", false);
        assert_eq!(estimate.weapon_type, "heavy_crossbow");
        assert!(estimate.typical_range_m > 300.0);
        assert!(estimate.confidence > 0.5);
        assert!(!estimate.literature_sources.is_empty());
    }

    #[test]
    fn test_estimate_weapon_range_with_evidence() {
        let analyzer = DefenseAnalyzer::default();
        let with_evidence = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", true);
        let without_evidence = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", false);
        assert!(with_evidence.max_range_m >= without_evidence.max_range_m);
        assert!(with_evidence.confidence >= without_evidence.confidence);
    }

    #[test]
    fn test_compute_weapon_coverage() {
        let analyzer = DefenseAnalyzer::default();
        let weapon = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", true);
        let coverage = analyzer.compute_weapon_coverage_score(6.28, 4, &weapon);
        assert!(coverage > 0.0 && coverage <= 100.0);
    }

    #[test]
    fn test_assess_defense_data_quality_full() {
        let analyzer = DefenseAnalyzer::default();
        let quality = analyzer.assess_defense_data_quality(true, true, true, true, true);
        assert_relative_eq!(quality, 1.0, epsilon = 0.01);
    }

    #[test]
    fn test_assess_defense_data_quality_partial() {
        let analyzer = DefenseAnalyzer::default();
        let quality = analyzer.assess_defense_data_quality(true, false, true, false, true);
        assert!(quality > 0.5 && quality < 1.0);
    }

    #[test]
    fn test_visibility_analysis_sync() {
        let analyzer = DefenseAnalyzer::default();
        let gates = sample_gates();
        let weapon = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", true);
        let config = VisibilityAnalysisConfig {
            num_sample_points: 12,
            ..Default::default()
        };
        let result = DefenseAnalyzer::compute_visibility_analysis_sync(
            116.0, 34.0, 2.0, &gates, "plain", &weapon, &config,
        );

        assert_eq!(
            result["sample_points"].as_array().unwrap().len(),
            12
        );
        assert!(result["average_visibility"].as_f64().unwrap() >= 0.0);
        assert!(result["average_weapon_coverage"].as_f64().unwrap() >= 0.0);
    }

    #[test]
    fn test_compute_overall_defense_score() {
        let analyzer = DefenseAnalyzer::default();
        let gate_scores = vec![0.8, 0.7];
        let weak_points = vec![];
        let weapon = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", true);
        let coverage = analyzer.compute_weapon_coverage_score(6.28, 2, &weapon);
        let quality = analyzer.assess_defense_data_quality(true, true, true, false, true);

        let score = analyzer.compute_overall_defense_score(
            &gate_scores, 8.0, 3.0, 10.0, &weak_points, 12.56, "plain", coverage, quality,
        );

        assert!(score >= 0.0 && score <= 100.0);
    }

    #[test]
    fn test_wall_height_scaling_effect() {
        let analyzer = DefenseAnalyzer::default();
        let gate_scores = vec![0.8, 0.7];
        let weak_points = vec![];
        let weapon = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", true);
        let coverage = analyzer.compute_weapon_coverage_score(6.28, 2, &weapon);
        let quality = analyzer.assess_defense_data_quality(true, true, true, false, true);

        let tall_wall = analyzer.compute_overall_defense_score(
            &gate_scores, 12.0, 5.0, 20.0, &weak_points, 12.56, "plain", coverage, quality,
        );

        let short_wall = analyzer.compute_overall_defense_score(
            &gate_scores, 2.0, 1.0, 0.0, &weak_points, 12.56, "plain", coverage, quality,
        );

        assert!(tall_wall > short_wall, "tall wall {} > short wall {}", tall_wall, short_wall);
    }

    #[tokio::test]
    async fn test_visibility_analysis_async_threaded() {
        let analyzer = DefenseAnalyzer::default();
        let gates = Arc::new(sample_gates());
        let weapon = analyzer.estimate_weapon_range("ancient_china", 8.0, "plain", true);

        let result = analyzer
            .compute_visibility_analysis_async(
                116.0, 34.0, 2.0, gates, "plain".to_string(), weapon,
            )
            .await;

        assert!(result["average_visibility"].as_f64().unwrap() >= 0.0);
        assert!(result["sample_points"].as_array().unwrap().len() > 0);
        assert!(result["method"].as_str().unwrap().contains("threaded"));
    }
}
