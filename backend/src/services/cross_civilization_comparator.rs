use crate::models::CivilizationAvgMetrics;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CitySizeCategory {
    Small,
    Medium,
    Large,
    Megalopolis,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CityTypeCategory {
    Capital,
    RegionalCity,
    ReligiousCenter,
    TradingCenter,
    General,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizationMethod {
    MinMax,
    ZScore,
    WithinCivilization,
    Rank,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComparabilityAssessment {
    pub indicator: String,
    pub comparable: bool,
    pub confidence: f64,
    pub notes: String,
    pub adjustment_factor: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassificationResult {
    pub size_category: CitySizeCategory,
    pub type_category: CityTypeCategory,
    pub size_confidence: f64,
    pub type_confidence: f64,
    pub basis: Vec<String>,
}

pub struct CivilizationUrbanDefinition {
    pub civilization_name: &'static str,
    pub city_definition_criteria: &'static [&'static str],
    pub area_adjustment_factor: f64,
    pub population_density_adjustment: f64,
    pub primary_city_type: &'static str,
    pub comparability_notes: &'static str,
}

pub const CIVILIZATION_URBAN_DEFINITIONS: &[CivilizationUrbanDefinition] = &[
    CivilizationUrbanDefinition {
        civilization_name: "古代中国",
        city_definition_criteria: &["城墙围合", "行政中心", "里坊制", "市场规制"],
        area_adjustment_factor: 1.0,
        population_density_adjustment: 1.1,
        primary_city_type: "行政都城",
        comparability_notes: "以城墙内面积计，含官署、居民区、市场，功能高度混合",
    },
    CivilizationUrbanDefinition {
        civilization_name: "古罗马",
        city_definition_criteria: &["市政建筑", "输水道", "广场", "斗兽场"],
        area_adjustment_factor: 0.85,
        population_density_adjustment: 0.9,
        primary_city_type: "殖民城市",
        comparability_notes: "以建成区计，含大量公共建筑，居住密度相对较低",
    },
    CivilizationUrbanDefinition {
        civilization_name: "玛雅",
        city_definition_criteria: &["金字塔神庙", "宫殿", "球场", "石碑"],
        area_adjustment_factor: 0.6,
        population_density_adjustment: 0.7,
        primary_city_type: "祭祀中心",
        comparability_notes: "以仪式建筑群为核心，外围分散居住区，城市概念偏仪式中心",
    },
    CivilizationUrbanDefinition {
        civilization_name: "古埃及",
        city_definition_criteria: &["神庙", "法老宫殿", "居住区", "码头"],
        area_adjustment_factor: 0.75,
        population_density_adjustment: 0.85,
        primary_city_type: "宗教政治中心",
        comparability_notes: "沿尼罗河分布，神庙为核心，城市形态受河流影响大",
    },
    CivilizationUrbanDefinition {
        civilization_name: "印度河",
        city_definition_criteria: &["城堡区", "下城区", "大浴池", "网格布局"],
        area_adjustment_factor: 0.9,
        population_density_adjustment: 1.0,
        primary_city_type: "商业城市",
        comparability_notes: "高度规划的网格城市，功能分区明确，商业发达",
    },
];

pub struct CrossCivilizationComparator {
    normalization_method: NormalizationMethod,
    enable_classification: bool,
    enable_comparability_assessment: bool,
}

impl Default for CrossCivilizationComparator {
    fn default() -> Self {
        CrossCivilizationComparator {
            normalization_method: NormalizationMethod::MinMax,
            enable_classification: true,
            enable_comparability_assessment: true,
        }
    }
}

impl CrossCivilizationComparator {
    pub fn new(
        normalization_method: NormalizationMethod,
        enable_classification: bool,
        enable_comparability_assessment: bool,
    ) -> Self {
        CrossCivilizationComparator {
            normalization_method,
            enable_classification,
            enable_comparability_assessment,
        }
    }

    pub fn classify_city_size(&self, area_sq_km: f64) -> (CitySizeCategory, f64, &'static str) {
        if area_sq_km <= 1.0 {
            (CitySizeCategory::Small, 0.9, "面积≤1平方公里")
        } else if area_sq_km <= 5.0 {
            (CitySizeCategory::Medium, 0.85, "面积1-5平方公里")
        } else if area_sq_km <= 15.0 {
            (CitySizeCategory::Large, 0.8, "面积5-15平方公里")
        } else {
            (CitySizeCategory::Megalopolis, 0.75, "面积>15平方公里")
        }
    }

    pub fn classify_city_type(
        &self,
        compactness: f64,
        functional_diversity: f64,
        road_density: f64,
        integration: f64,
    ) -> (CityTypeCategory, f64, Vec<String>) {
        let mut scores = HashMap::new();
        let mut basis = Vec::new();

        let capital_score = if compactness > 0.75 && integration > 1.2 { 0.8 } else { 0.3 }
            + if functional_diversity > 2.0 { 0.1 } else { 0.0 };
        scores.insert(CityTypeCategory::Capital, capital_score);
        if compactness > 0.75 {
            basis.push("高紧凑度".to_string());
        }
        if integration > 1.2 {
            basis.push("高整合度".to_string());
        }

        let religious_score = if compactness < 0.5 && functional_diversity < 1.2 { 0.7 } else { 0.2 }
            + if road_density < 3.0 { 0.15 } else { 0.0 };
        scores.insert(CityTypeCategory::ReligiousCenter, religious_score);
        if compactness < 0.5 {
            basis.push("低紧凑度-分散布局".to_string());
        }
        if functional_diversity < 1.2 {
            basis.push("功能单一-祭祀为主".to_string());
        }

        let trading_score = if road_density > 8.0 && functional_diversity > 1.8 { 0.75 } else { 0.25 }
            + if integration > 1.0 { 0.1 } else { 0.0 };
        scores.insert(CityTypeCategory::TradingCenter, trading_score);
        if road_density > 8.0 {
            basis.push("高道路密度".to_string());
        }
        if functional_diversity > 1.8 {
            basis.push("功能多样".to_string());
        }

        let regional_score = if compactness > 0.5 && compactness < 0.8 && functional_diversity > 1.5 {
            0.6
        } else {
            0.3
        };
        scores.insert(CityTypeCategory::RegionalCity, regional_score);

        let mut best_type = CityTypeCategory::General;
        let mut best_score = 0.0;
        for (t, s) in &scores {
            if *s > best_score {
                best_score = *s;
                best_type = *t;
            }
        }

        if best_score < 0.4 {
            best_type = CityTypeCategory::General;
            best_score = 0.5;
            basis.push("综合型城市".to_string());
        }

        (best_type, best_score.min(1.0), basis)
    }

    pub fn classify_metrics(
        &self,
        metrics: &CivilizationAvgMetrics,
    ) -> ClassificationResult {
        let (size_category, size_confidence, _) = self.classify_city_size(metrics.avg_area_sq_km);
        let (type_category, type_confidence, basis) = self.classify_city_type(
            metrics.avg_compactness,
            metrics.avg_functional_diversity,
            metrics.avg_road_density,
            metrics.avg_integration_global,
        );

        ClassificationResult {
            size_category,
            type_category,
            size_confidence,
            type_confidence,
            basis,
        }
    }

    pub fn assess_indicator_comparability(
        &self,
        indicator_key: &str,
        civ_names: &[String],
    ) -> ComparabilityAssessment {
        let notes = match indicator_key {
            "area" => "不同文明城市定义差异大，面积直接比较需谨慎",
            "integration" => "受城市形态影响，网格城市天然较高",
            "choice" => "受路网结构影响，放射状城市较高",
            "boundary_fd" => "受地形影响较大，平原城市更规整",
            "road_fd" => "受规划程度影响，规划城市分维低",
            "compactness" => "受城市定义影响，祭祀中心普遍偏低",
            "road_density" => "受人口密度和功能影响，可比性中等",
            "functional_diversity" => "受城市类型影响，都城功能更全",
            _ => "可比性一般",
        };

        let base_confidence = match indicator_key {
            "boundary_fd" | "road_fd" => 0.85,
            "compactness" | "integration" | "choice" => 0.75,
            "road_density" | "functional_diversity" => 0.7,
            "area" => 0.5,
            _ => 0.65,
        };

        let adjustment = if civ_names.iter().any(|n| n.contains("玛雅")) && indicator_key == "area" {
            0.6
        } else if civ_names.iter().any(|n| n.contains("罗马")) && indicator_key == "area" {
            0.85
        } else {
            1.0
        };

        ComparabilityAssessment {
            indicator: indicator_key.to_string(),
            comparable: base_confidence > 0.5,
            confidence: base_confidence * adjustment,
            notes: notes.to_string(),
            adjustment_factor: adjustment,
        }
    }

    pub fn normalize_zscore(&self, values: &[f64]) -> Vec<f64> {
        if values.is_empty() {
            return vec![];
        }

        let mean: f64 = values.iter().sum::<f64>() / values.len() as f64;
        let variance: f64 = values
            .iter()
            .map(|v| (v - mean).powi(2))
            .sum::<f64>()
            / values.len() as f64;
        let std = variance.sqrt();

        if std < 1e-9 {
            return vec![0.0; values.len()];
        }

        values.iter().map(|v| (v - mean) / std).collect()
    }

    pub fn normalize_rank(&self, values: &[f64]) -> Vec<f64> {
        let mut indexed: Vec<(usize, f64)> =
            values.iter().enumerate().map(|(i, v)| (i, *v)).collect();
        indexed.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());

        let n = values.len() as f64;
        let mut result = vec![0.0; values.len()];
        for (rank, (idx, _)) in indexed.iter().enumerate() {
            result[*idx] = 1.0 - rank as f64 / n.max(1.0);
        }
        result
    }

    pub fn normalize_minmax(&self, values: &[f64], min: f64, max: f64) -> Vec<f64> {
        values
            .iter()
            .map(|v| ((v - min) / (max - min)).max(0.0).min(1.0))
            .collect()
    }

    pub fn compute_radar_values(
        &self,
        metrics: &CivilizationAvgMetrics,
    ) -> Vec<f64> {
        let mut values = Vec::new();

        values.push(self.normalize_minmax(&[metrics.avg_integration_global], 0.5, 2.0)[0]);
        values.push(self.normalize_minmax(&[metrics.avg_choice_global], 0.1, 1.0)[0]);
        values.push(self.normalize_minmax(&[metrics.avg_boundary_fd], 1.0, 1.9)[0]);
        values.push(self.normalize_minmax(&[metrics.avg_road_fd], 1.2, 1.9)[0]);
        values.push(metrics.avg_compactness.max(0.0).min(1.0));
        values.push(self.normalize_minmax(&[metrics.avg_road_density], 1.0, 15.0)[0]);
        values.push(self.normalize_minmax(
            &[metrics.avg_functional_diversity],
            0.5,
            2.5,
        )[0]);
        values.push(self.normalize_minmax(&[metrics.avg_area_sq_km], 0.5, 20.0)[0]);

        values
    }

    pub fn compare_by_size_category(
        &self,
        all_metrics: &[CivilizationAvgMetrics],
        size_category: CitySizeCategory,
    ) -> Vec<CivilizationAvgMetrics> {
        all_metrics
            .iter()
            .filter(|m| {
                let (cat, _, _) = self.classify_city_size(m.avg_area_sq_km);
                cat == size_category
            })
            .cloned()
            .collect()
    }

    pub fn compare_by_type_category(
        &self,
        all_metrics: &[CivilizationAvgMetrics],
        type_category: CityTypeCategory,
    ) -> Vec<CivilizationAvgMetrics> {
        all_metrics
            .iter()
            .filter(|m| {
                let (cat, _, _) = self.classify_city_type(
                    m.avg_compactness,
                    m.avg_functional_diversity,
                    m.avg_road_density,
                    m.avg_integration_global,
                );
                cat == type_category
            })
            .cloned()
            .collect()
    }

    pub fn get_urban_definition(&self, civilization_name: &str) -> Option<&CivilizationUrbanDefinition> {
        CIVILIZATION_URBAN_DEFINITIONS
            .iter()
            .find(|d| d.civilization_name == civilization_name)
    }

    pub fn generate_planning_notes(
        &self,
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
        notes.insert(
            "functional_characteristic".to_string(),
            diversity.to_string(),
        );

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
    fn test_classify_city_size_small() {
        let comparator = CrossCivilizationComparator::default();
        let (cat, conf, _) = comparator.classify_city_size(0.5);
        assert_eq!(cat, CitySizeCategory::Small);
        assert!(conf > 0.8);
    }

    #[test]
    fn test_classify_city_size_megalopolis() {
        let comparator = CrossCivilizationComparator::default();
        let (cat, conf, _) = comparator.classify_city_size(20.0);
        assert_eq!(cat, CitySizeCategory::Megalopolis);
        assert!(conf > 0.7);
    }

    #[test]
    fn test_classify_city_type_capital() {
        let comparator = CrossCivilizationComparator::default();
        let (cat, conf, _) = comparator.classify_city_type(0.85, 2.2, 9.0, 1.4);
        assert_eq!(cat, CityTypeCategory::Capital);
        assert!(conf > 0.6);
    }

    #[test]
    fn test_classify_city_type_religious() {
        let comparator = CrossCivilizationComparator::default();
        let (cat, conf, _) = comparator.classify_city_type(0.3, 0.8, 2.0, 0.6);
        assert_eq!(cat, CityTypeCategory::ReligiousCenter);
        assert!(conf > 0.5);
    }

    #[test]
    fn test_normalize_zscore() {
        let comparator = CrossCivilizationComparator::default();
        let values = vec![1.0, 2.0, 3.0, 4.0, 5.0];
        let z = comparator.normalize_zscore(&values);
        let mean: f64 = z.iter().sum::<f64>() / z.len() as f64;
        assert!(mean.abs() < 0.01);
    }

    #[test]
    fn test_normalize_rank() {
        let comparator = CrossCivilizationComparator::default();
        let values = vec![10.0, 30.0, 20.0, 50.0, 40.0];
        let ranks = comparator.normalize_rank(&values);
        assert_relative_eq!(ranks[3], 1.0);
        assert_relative_eq!(ranks[0], 0.0);
    }

    #[test]
    fn test_comparability_assessment_area_low() {
        let comparator = CrossCivilizationComparator::default();
        let names = vec!["古代中国".to_string(), "玛雅".to_string()];
        let comp = comparator.assess_indicator_comparability("area", &names);
        assert!(comp.confidence < 0.6);
        assert!(comp.adjustment_factor < 1.0);
    }

    #[test]
    fn test_comparability_assessment_fd_high() {
        let comparator = CrossCivilizationComparator::default();
        let names = vec!["古代中国".to_string(), "古罗马".to_string()];
        let comp = comparator.assess_indicator_comparability("boundary_fd", &names);
        assert!(comp.comparable);
        assert!(comp.confidence > 0.7);
    }

    #[test]
    fn test_compare_by_size_category() {
        let comparator = CrossCivilizationComparator::default();
        let all = vec![sample_metrics_china(), sample_metrics_maya()];

        let medium = comparator.compare_by_size_category(&all, CitySizeCategory::Medium);
        assert_eq!(medium.len(), 1);
        assert_eq!(medium[0].civilization_id, 1);

        let small = comparator.compare_by_size_category(&all, CitySizeCategory::Small);
        assert_eq!(small.len(), 1);
        assert_eq!(small[0].civilization_id, 3);
    }

    #[test]
    fn test_urban_definitions_exist() {
        assert!(CIVILIZATION_URBAN_DEFINITIONS.len() >= 5);
        for def in CIVILIZATION_URBAN_DEFINITIONS {
            assert!(!def.civilization_name.is_empty());
            assert!(def.area_adjustment_factor > 0.0);
        }
    }

    #[test]
    fn test_get_urban_definition_china() {
        let comparator = CrossCivilizationComparator::default();
        let def = comparator.get_urban_definition("古代中国");
        assert!(def.is_some());
        assert_eq!(def.unwrap().primary_city_type, "行政都城");
    }

    #[test]
    fn test_generate_planning_notes() {
        let comparator = CrossCivilizationComparator::default();
        let m = sample_metrics_china();
        let notes = comparator.generate_planning_notes(1, &m);
        assert!(notes.contains_key("planning_style"));
        assert_eq!(notes["planning_style"], "紧凑规整型");
    }

    #[test]
    fn test_compute_radar_values_length() {
        let comparator = CrossCivilizationComparator::default();
        let m = sample_metrics_china();
        let vals = comparator.compute_radar_values(&m);
        assert_eq!(vals.len(), 8);
        for v in vals {
            assert!(v >= 0.0 && v <= 1.0);
        }
    }
}
