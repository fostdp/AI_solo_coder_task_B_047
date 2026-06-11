use crate::models::{LandUseItem, LandUsePeriod, LandUseTimeline};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DepositionEnvironment {
    Alluvial,
    Lacustrine,
    Aeolian,
    Fluvial,
    Marine,
    Colluvial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DepositionModel {
    pub environment: DepositionEnvironment,
    pub accumulation_rate_cm_per_yr: f64,
    pub compaction_factor: f64,
    pub erosion_rate_cm_per_yr: f64,
    pub confidence: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchaeologicalCheckpoint {
    pub period_name: String,
    pub period_year: i32,
    pub evidence_type: String,
    pub description: String,
    pub confidence: f64,
    pub land_use_constraints: Vec<(String, f64, f64)>,
}

pub const DEPOSITION_MODELS: &[(&str, DepositionModel)] = &[
    (
        "alluvial_plain",
        DepositionModel {
            environment: DepositionEnvironment::Alluvial,
            accumulation_rate_cm_per_yr: 0.1,
            compaction_factor: 0.7,
            erosion_rate_cm_per_yr: 0.02,
            confidence: 0.8,
        },
    ),
    (
        "river_bank",
        DepositionModel {
            environment: DepositionEnvironment::Fluvial,
            accumulation_rate_cm_per_yr: 0.2,
            compaction_factor: 0.65,
            erosion_rate_cm_per_yr: 0.05,
            confidence: 0.75,
        },
    ),
    (
        "lake_side",
        DepositionModel {
            environment: DepositionEnvironment::Lacustrine,
            accumulation_rate_cm_per_yr: 0.05,
            compaction_factor: 0.6,
            erosion_rate_cm_per_yr: 0.01,
            confidence: 0.85,
        },
    ),
    (
        "loess_terrace",
        DepositionModel {
            environment: DepositionEnvironment::Aeolian,
            accumulation_rate_cm_per_yr: 0.08,
            compaction_factor: 0.75,
            erosion_rate_cm_per_yr: 0.03,
            confidence: 0.7,
        },
    ),
];

pub struct LandUseTracer {
    default_model: &'static DepositionModel,
    enable_stratigraphic_interpolation: bool,
    enable_archaeological_validation: bool,
}

impl Default for LandUseTracer {
    fn default() -> Self {
        LandUseTracer {
            default_model: &DEPOSITION_MODELS[0].1,
            enable_stratigraphic_interpolation: true,
            enable_archaeological_validation: true,
        }
    }
}

impl LandUseTracer {
    pub fn new(
        default_model: &'static DepositionModel,
        enable_stratigraphic_interpolation: bool,
        enable_archaeological_validation: bool,
    ) -> Self {
        LandUseTracer {
            default_model,
            enable_stratigraphic_interpolation,
            enable_archaeological_validation,
        }
    }

    pub fn select_deposition_model(
        &self,
        terrain: &str,
        water_proximity: f64,
    ) -> &'static DepositionModel {
        if terrain.contains("river") || water_proximity > 0.8 {
            &DEPOSITION_MODELS[1].1
        } else if terrain.contains("lake") || water_proximity > 0.6 {
            &DEPOSITION_MODELS[2].1
        } else if terrain.contains("loess") || terrain.contains("plain") {
            &DEPOSITION_MODELS[0].1
        } else if terrain.contains("terrace") || terrain.contains("hill") {
            &DEPOSITION_MODELS[3].1
        } else {
            self.default_model
        }
    }

    pub fn estimate_missing_periods(
        &self,
        known_layers: &[StratigraphicLayer],
        target_periods: &[(i32, String)],
        model: &DepositionModel,
    ) -> Vec<StratigraphicLayer> {
        let mut result = Vec::new();
        let known_years: Vec<i32> = known_layers.iter().map(|l| l.period_year).collect();

        for (year, name) in target_periods {
            if known_years.contains(year) {
                if let Some(layer) = known_layers.iter().find(|l| l.period_year == *year) {
                    result.push(layer.clone());
                }
            } else if self.enable_stratigraphic_interpolation {
                let estimated = self.estimate_single_layer(*year, name, known_layers, model);
                result.push(estimated);
            }
        }

        result.sort_by(|a, b| a.period_year.cmp(&b.period_year));
        result
    }

    fn estimate_single_layer(
        &self,
        year: i32,
        name: &str,
        known_layers: &[StratigraphicLayer],
        model: &DepositionModel,
    ) -> StratigraphicLayer {
        if known_layers.is_empty() {
            return StratigraphicLayer {
                layer_id: format!("est_{}", year),
                period_name: name.to_string(),
                period_year: year,
                thickness_cm: model.accumulation_rate_cm_per_yr * 1000.0,
                sediment_type: "estimated".to_string(),
                artifacts: vec![],
                dating_method: "deposition_model".to_string(),
                confidence: 0.3,
            };
        }

        let before = known_layers
            .iter()
            .filter(|l| l.period_year < year)
            .max_by_key(|l| l.period_year);
        let after = known_layers
            .iter()
            .filter(|l| l.period_year > year)
            .min_by_key(|l| l.period_year);

        match (before, after) {
            (Some(b), Some(a)) => {
                let total_span = (a.period_year - b.period_year).abs() as f64;
                let year_span = (year - b.period_year).abs() as f64;
                let t = if total_span > 0.0 {
                    year_span / total_span
                } else {
                    0.5
                };

                let thickness = b.thickness_cm
                    + (a.thickness_cm - b.thickness_cm) * t
                    + model.accumulation_rate_cm_per_yr * year_span * (1.0 - t);

                let conf = (b.confidence.min(a.confidence))
                    * (1.0 - t * 0.3)
                    * (1.0 - (1.0 - t) * 0.3);

                StratigraphicLayer {
                    layer_id: format!("est_{}", year),
                    period_name: name.to_string(),
                    period_year: year,
                    thickness_cm: thickness.max(0.5),
                    sediment_type: self.infer_sediment_type(
                        t,
                        &b.sediment_type,
                        &a.sediment_type,
                    ),
                    artifacts: vec![],
                    dating_method: "stratigraphic_interpolation".to_string(),
                    confidence: conf.max(0.2),
                }
            }
            (Some(b), None) => {
                let year_diff = (year - b.period_year).abs() as f64;
                let thickness = b.thickness_cm
                    + model.accumulation_rate_cm_per_yr * year_diff
                    - model.erosion_rate_cm_per_yr * year_diff * 0.5;

                let conf =
                    b.confidence * (1.0 - year_diff.min(2000.0) / 2000.0 * 0.5);

                StratigraphicLayer {
                    layer_id: format!("est_{}", year),
                    period_name: name.to_string(),
                    period_year: year,
                    thickness_cm: thickness.max(0.5),
                    sediment_type: b.sediment_type.clone(),
                    artifacts: vec![],
                    dating_method: "extrapolation_after".to_string(),
                    confidence: conf.max(0.15),
                }
            }
            (None, Some(a)) => {
                let year_diff = (year - a.period_year).abs() as f64;
                let thickness = a.thickness_cm
                    - model.accumulation_rate_cm_per_yr * year_diff
                    + model.erosion_rate_cm_per_yr * year_diff * 0.5;

                let conf =
                    a.confidence * (1.0 - year_diff.min(2000.0) / 2000.0 * 0.5);

                StratigraphicLayer {
                    layer_id: format!("est_{}", year),
                    period_name: name.to_string(),
                    period_year: year,
                    thickness_cm: thickness.max(0.5),
                    sediment_type: a.sediment_type.clone(),
                    artifacts: vec![],
                    dating_method: "extrapolation_before".to_string(),
                    confidence: conf.max(0.15),
                }
            }
            (None, None) => StratigraphicLayer {
                layer_id: format!("est_{}", year),
                period_name: name.to_string(),
                period_year: year,
                thickness_cm: model.accumulation_rate_cm_per_yr * 500.0,
                sediment_type: "unknown".to_string(),
                artifacts: vec![],
                dating_method: "model_baseline".to_string(),
                confidence: 0.1,
            },
        }
    }

    fn infer_sediment_type(&self, t: f64, before: &str, after: &str) -> String {
        if t < 0.3 {
            before.to_string()
        } else if t > 0.7 {
            after.to_string()
        } else {
            format!("{}-{}_transition", before, after)
        }
    }

    pub fn apply_archaeological_constraints(
        &self,
        layers: &mut [StratigraphicLayer],
        checkpoints: &[ArchaeologicalCheckpoint],
    ) {
        if !self.enable_archaeological_validation {
            return;
        }

        for checkpoint in checkpoints {
            if let Some(layer) = layers.iter_mut().find(|l| l.period_year == checkpoint.period_year)
            {
                layer.confidence = layer.confidence.max(checkpoint.confidence * 0.8);
                if layer.artifacts.is_empty() {
                    layer.artifacts.push(checkpoint.evidence_type.clone());
                }
                layer.dating_method = format!("{}_archaeo_validated", layer.dating_method);
            }
        }
    }

    pub fn assess_stratigraphic_continuity(
        &self,
        layers: &[StratigraphicLayer],
    ) -> (f64, Vec<String>, Vec<i32>) {
        if layers.len() < 2 {
            return (if layers.is_empty() { 0.0 } else { 1.0 }, vec![], vec![]);
        }

        let mut gaps = Vec::new();
        let mut issues = Vec::new();
        let mut total_gap_years = 0;

        for i in 1..layers.len() {
            let gap = layers[i].period_year - layers[i - 1].period_year;
            if gap > 500 {
                gaps.push(layers[i - 1].period_year);
                issues.push(format!(
                    "时期{}至{}存在{}年地层间断",
                    layers[i - 1].period_name, layers[i].period_name, gap
                ));
                total_gap_years += gap;
            }
        }

        let total_span = (layers.last().unwrap().period_year - layers.first().unwrap().period_year)
            .abs() as f64;
        let continuity = if total_span > 0.0 {
            (1.0 - total_gap_years as f64 / total_span).max(0.0)
        } else {
            1.0
        };

        (continuity, issues, gaps)
    }

    pub fn fill_land_use_gaps(
        &self,
        timeline: &LandUseTimeline,
        model: &DepositionModel,
        checkpoints: &[ArchaeologicalCheckpoint],
    ) -> LandUseTimeline {
        let mut periods = timeline.periods.clone();

        if periods.len() < 2 {
            return timeline.clone();
        }

        let mut filled_periods = Vec::new();

        for i in 0..periods.len() {
            filled_periods.push(periods[i].clone());

            if i < periods.len() - 1 {
                let curr = &periods[i];
                let next = &periods[i + 1];
                let gap = next.period_year - curr.period_year;

                if gap > 300 {
                    let num_intermediate = (gap / 200).min(3).max(1);
                    for j in 1..=num_intermediate {
                        let t = j as f64 / (num_intermediate + 1) as f64;
                        let inter_year =
                            curr.period_year + (gap as f64 * t) as i32;
                        let inter = self.interpolate_land_use_period(
                            curr, next, t, inter_year, model,
                        );
                        filled_periods.push(inter);
                    }
                }
            }
        }

        if self.enable_archaeological_validation {
            for checkpoint in checkpoints {
                if let Some(p) = filled_periods
                    .iter_mut()
                    .find(|p| p.period_year == checkpoint.period_year)
                {
                    for (land_type, min_pct, max_pct) in &checkpoint.land_use_constraints {
                        if let Some(item) =
                            p.land_uses.iter_mut().find(|u| u.land_use_type == *land_type)
                        {
                            if item.percentage < *min_pct {
                                item.percentage = *min_pct;
                            }
                            if item.percentage > *max_pct {
                                item.percentage = *max_pct;
                            }
                        }
                    }
                }
            }
        }

        LandUseTimeline {
            site_id: timeline.site_id,
            periods: filled_periods,
            land_use_types: timeline.land_use_types.clone(),
        }
    }

    fn interpolate_land_use_period(
        &self,
        before: &LandUsePeriod,
        after: &LandUsePeriod,
        t: f64,
        year: i32,
        _model: &DepositionModel,
    ) -> LandUsePeriod {
        let mut land_uses = Vec::new();

        for before_item in &before.land_uses {
            let after_item = after
                .land_uses
                .iter()
                .find(|u| u.land_use_type == before_item.land_use_type);

            let after_pct = after_item
                .map(|i| i.percentage)
                .unwrap_or(before_item.percentage);
            let after_area = after_item
                .map(|i| i.area_km2)
                .unwrap_or(before_item.area_km2);

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

    pub fn compute_land_use_trend(
        &self,
        timeline: &LandUseTimeline,
    ) -> Value {
        let mut trends = HashMap::new();

        if timeline.periods.len() < 2 {
            return serde_json::json!({
                "trends": {},
                "note": "数据点不足",
                "num_periods": timeline.periods.len()
            });
        }

        for land_type in &timeline.land_use_types {
            let values: Vec<f64> = timeline
                .periods
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
            let change_pct = if first > 0.0 {
                (change / first) * 100.0
            } else {
                0.0
            };

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

            trends.insert(
                land_type.clone(),
                serde_json::json!({
                    "start_percentage": first,
                    "end_percentage": last,
                    "absolute_change": change,
                    "relative_change_percent": change_pct,
                    "trend_type": trend_type,
                }),
            );
        }

        let peak_urban = timeline
            .periods
            .iter()
            .map(|p| {
                p.land_uses
                    .iter()
                    .find(|u| u.land_use_type == "urban")
                    .map(|u| u.percentage)
                    .unwrap_or(0.0)
            })
            .fold(0.0_f64, f64::max);

        serde_json::json!({
            "trends": trends,
            "peak_urban_percentage": peak_urban,
            "urban_decay_rate": if peak_urban > 0.0 {
                (peak_urban - timeline.periods.last().and_then(|p|
                    p.land_uses.iter().find(|u| u.land_use_type == "urban").map(|u| u.percentage)
                ).unwrap_or(0.0)) / peak_urban
            } else { 0.0 },
            "num_periods": timeline.periods.len(),
            "method": "land_use_tracer_v1",
            "stratigraphic_interpolation": self.enable_stratigraphic_interpolation,
            "archaeological_validation": self.enable_archaeological_validation,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::LandUseItem;
    use approx::assert_relative_eq;

    fn sample_layers() -> Vec<StratigraphicLayer> {
        vec![
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
        ]
    }

    fn sample_timeline() -> LandUseTimeline {
        let periods = vec![
            LandUsePeriod {
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
            },
            LandUsePeriod {
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
            },
        ];

        LandUseTimeline {
            site_id: 1,
            periods,
            land_use_types: vec!["urban".to_string(), "farmland".to_string()],
        }
    }

    #[test]
    fn test_deposition_models_exist() {
        assert!(DEPOSITION_MODELS.len() >= 4);
        for (_, model) in DEPOSITION_MODELS {
            assert!(model.accumulation_rate_cm_per_yr > 0.0);
            assert!(model.confidence > 0.0);
        }
    }

    #[test]
    fn test_select_deposition_model_river() {
        let tracer = LandUseTracer::default();
        let m = tracer.select_deposition_model("river_plain", 0.9);
        assert_eq!(m.environment, DepositionEnvironment::Fluvial);
    }

    #[test]
    fn test_estimate_missing_periods_interpolation() {
        let tracer = LandUseTracer::default();
        let known = sample_layers();
        let targets = vec![(400, "南北朝".to_string())];
        let model = &DEPOSITION_MODELS[0].1;
        let estimated = tracer.estimate_missing_periods(&known, &targets, model);

        assert_eq!(estimated.len(), 1);
        assert_eq!(estimated[0].period_year, 400);
        assert!(estimated[0].confidence > 0.2);
        assert!(estimated[0].thickness_cm > 0.0);
    }

    #[test]
    fn test_assess_stratigraphic_continuity() {
        let tracer = LandUseTracer::default();
        let mut layers = sample_layers();
        layers.push(StratigraphicLayer {
            layer_id: "l3".to_string(),
            period_name: "明代".to_string(),
            period_year: 1400,
            thickness_cm: 120.0,
            sediment_type: "alluvial".to_string(),
            artifacts: vec![],
            dating_method: "typology".to_string(),
            confidence: 0.8,
        });

        let (continuity, issues, gaps) = tracer.assess_stratigraphic_continuity(&layers);

        assert!(continuity > 0.0 && continuity <= 1.0);
        assert!(!issues.is_empty() || gaps.is_empty());
    }

    #[test]
    fn test_apply_archaeological_constraints() {
        let tracer = LandUseTracer::default();
        let mut layers = sample_layers();
        let checkpoints = vec![ArchaeologicalCheckpoint {
            period_name: "汉代".to_string(),
            period_year: -200,
            evidence_type: "城址".to_string(),
            description: "发现汉代城墙".to_string(),
            confidence: 0.9,
            land_use_constraints: vec![],
        }];

        let original_conf = layers[0].confidence;
        tracer.apply_archaeological_constraints(&mut layers, &checkpoints);

        assert!(layers[0].confidence >= original_conf);
        assert!(layers[0].dating_method.contains("archaeo"));
    }

    #[test]
    fn test_fill_land_use_gaps() {
        let tracer = LandUseTracer::default();
        let tl = sample_timeline();
        let model = &DEPOSITION_MODELS[0].1;
        let checkpoints: Vec<ArchaeologicalCheckpoint> = vec![];

        let original_count = tl.periods.len();
        let filled = tracer.fill_land_use_gaps(&tl, model, &checkpoints);

        assert!(filled.periods.len() >= original_count);
        assert_eq!(filled.site_id, tl.site_id);
    }

    #[test]
    fn test_compute_land_use_trend() {
        let tracer = LandUseTracer::default();
        let tl = sample_timeline();
        let trend = tracer.compute_land_use_trend(&tl);

        assert!(trend.get("trends").is_some());
        assert!(trend.get("urban_decay_rate").is_some());
        assert!(trend["urban_decay_rate"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn test_interpolated_land_use_smooth() {
        let tracer = LandUseTracer::default();
        let before = LandUsePeriod {
            period_name: "早期".to_string(),
            period_year: 0,
            land_uses: vec![LandUseItem {
                land_use_type: "urban".to_string(),
                area_km2: 2.0,
                percentage: 40.0,
            }],
        };
        let after = LandUsePeriod {
            period_name: "晚期".to_string(),
            period_year: 1000,
            land_uses: vec![LandUseItem {
                land_use_type: "urban".to_string(),
                area_km2: 0.5,
                percentage: 10.0,
            }],
        };
        let model = &DEPOSITION_MODELS[0].1;

        let mid = tracer.interpolate_land_use_period(&before, &after, 0.5, 500, model);
        assert_eq!(mid.period_year, 500);
        let mid_urban = mid
            .land_uses
            .iter()
            .find(|u| u.land_use_type == "urban")
            .unwrap();
        assert!(mid_urban.percentage > 10.0 && mid_urban.percentage < 40.0);
    }

    #[test]
    fn test_tracer_disable_validation() {
        let tracer = LandUseTracer::new(&DEPOSITION_MODELS[0].1, true, false);
        let mut layers = sample_layers();
        let checkpoints = vec![ArchaeologicalCheckpoint {
            period_name: "汉代".to_string(),
            period_year: -200,
            evidence_type: "城址".to_string(),
            description: "发现汉代城墙".to_string(),
            confidence: 0.9,
            land_use_constraints: vec![],
        }];

        let original_conf = layers[0].confidence;
        let original_method = layers[0].dating_method.clone();
        tracer.apply_archaeological_constraints(&mut layers, &checkpoints);

        assert_relative_eq!(layers[0].confidence, original_conf);
        assert_eq!(layers[0].dating_method, original_method);
    }
}
