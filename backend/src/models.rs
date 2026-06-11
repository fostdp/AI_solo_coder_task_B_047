use serde::{Serialize, Deserialize};
use chrono::{DateTime, Utc};
use serde_json::Value;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Dynasty {
    pub id: i32,
    pub name: String,
    pub start_year: i32,
    pub end_year: i32,
    pub period: String,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CitySite {
    pub id: i32,
    pub name: String,
    pub dynasty_id: i32,
    pub location: Option<String>,
    pub center_longitude: f64,
    pub center_latitude: f64,
    pub estimated_population: Option<i32>,
    pub area_sq_km: Option<f64>,
    pub description: Option<String>,
    pub archaeological_notes: Option<String>,
    pub geom: Option<Value>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub dynasty_name: Option<String>,
    pub civilization_id: Option<i32>,
    pub civilization_name: Option<String>,
    pub terrain_type: Option<String>,
    pub elevation: Option<f64>,
    pub wall_height: Option<f64>,
    pub wall_width: Option<f64>,
    pub moat_width: Option<f64>,
    pub num_gates: Option<i32>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FunctionalZone {
    pub id: i32,
    pub city_site_id: i32,
    pub zone_type: String,
    pub name: Option<String>,
    pub description: Option<String>,
    pub archaeological_findings: Option<String>,
    pub functional_inference: Option<String>,
    pub confidence_level: Option<f64>,
    pub geom: Option<Value>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Road {
    pub id: i32,
    pub city_site_id: i32,
    pub road_name: Option<String>,
    pub road_type: Option<String>,
    pub width: Option<f64>,
    pub description: Option<String>,
    pub geom: Option<Value>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BuildingFoundation {
    pub id: i32,
    pub city_site_id: i32,
    pub building_type: Option<String>,
    pub name: Option<String>,
    pub area_sq_m: Option<f64>,
    pub rooms_count: Option<i32>,
    pub description: Option<String>,
    pub archaeological_findings: Option<String>,
    pub geom: Option<Value>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PopulationEstimate {
    pub id: i32,
    pub city_site_id: i32,
    pub estimate_year: Option<i32>,
    pub population_min: Option<i32>,
    pub population_max: Option<i32>,
    pub population_mean: Option<i32>,
    pub estimation_method: Option<String>,
    pub source: Option<String>,
    pub confidence_level: Option<f64>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MorphologyAnalysis {
    pub id: i32,
    pub city_site_id: i32,
    pub analysis_date: Option<DateTime<Utc>>,
    pub integration_global: Option<f64>,
    pub integration_local: Option<f64>,
    pub choice_global: Option<f64>,
    pub choice_local: Option<f64>,
    pub mean_depth: Option<f64>,
    pub total_depth: Option<f64>,
    pub connectivity: Option<f64>,
    pub boundary_fractal_dimension: Option<f64>,
    pub road_network_fractal_dimension: Option<f64>,
    pub compactness_index: Option<f64>,
    pub elongation_ratio: Option<f64>,
    pub road_density: Option<f64>,
    pub intersection_density: Option<f64>,
    pub functional_diversity: Option<f64>,
    pub functional_mixing: Option<f64>,
    pub boundary_fd_quality: Option<f64>,
    pub road_fd_quality: Option<f64>,
    pub boundary_fd_confidence_lower: Option<f64>,
    pub boundary_fd_confidence_upper: Option<f64>,
    pub road_fd_confidence_lower: Option<f64>,
    pub road_fd_confidence_upper: Option<f64>,
    pub notes: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RoadSyntaxResult {
    pub id: i32,
    pub road_id: i32,
    pub city_site_id: i32,
    pub integration: Option<f64>,
    pub choice: Option<f64>,
    pub depth: Option<f64>,
    pub connectivity: Option<i32>,
    pub control: Option<f64>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EvolutionTrend {
    pub id: i32,
    pub analysis_name: String,
    pub indicator_name: String,
    pub mk_statistic: Option<f64>,
    pub mk_p_value: Option<f64>,
    pub mk_z_score: Option<f64>,
    pub sen_slope: Option<f64>,
    pub trend_direction: Option<String>,
    pub trend_significance: Option<bool>,
    pub time_points: Option<Value>,
    pub values: Option<Value>,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct TrendAnalysisRequest {
    pub indicator: String,
    pub dynasty_ids: Option<Vec<i32>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CompareRequest {
    pub site_ids: Vec<i32>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiResponse<T> {
    pub success: bool,
    pub data: Option<T>,
    pub message: Option<String>,
}

impl<T> ApiResponse<T> {
    pub fn success(data: T) -> Self {
        ApiResponse {
            success: true,
            data: Some(data),
            message: None,
        }
    }

    pub fn error(message: &str) -> Self
    where
        T: Default,
    {
        ApiResponse {
            success: false,
            data: None,
            message: Some(message.to_string()),
        }
    }
}

pub struct GeometryUtils;

impl GeometryUtils {
    pub fn wkt_to_geojson_polygon(coords: &[(f64, f64)]) -> Value {
        let coordinates: Vec<Vec<f64>> = coords
            .iter()
            .map(|(x, y)| vec![*x, *y])
            .collect();
        
        serde_json::json!({
            "type": "Polygon",
            "coordinates": [coordinates]
        })
    }

    pub fn wkt_to_geojson_point(lon: f64, lat: f64) -> Value {
        serde_json::json!({
            "type": "Point",
            "coordinates": [lon, lat]
        })
    }

    pub fn wkt_to_geojson_linestring(coords: &[(f64, f64)]) -> Value {
        let coordinates: Vec<Vec<f64>> = coords
            .iter()
            .map(|(x, y)| vec![*x, *y])
            .collect();
        
        serde_json::json!({
            "type": "LineString",
            "coordinates": coordinates
        })
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Civilization {
    pub id: i32,
    pub name: String,
    pub name_cn: String,
    pub region: Option<String>,
    pub time_period: Option<String>,
    pub description: Option<String>,
    pub planning_characteristics: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CityGate {
    pub id: i32,
    pub site_id: i32,
    pub name: Option<String>,
    pub gate_type: Option<String>,
    pub defense_rating: Option<f64>,
    pub geom: Option<Value>,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PopulationDistribution {
    pub id: i32,
    pub site_id: i32,
    pub analysis_id: Option<i32>,
    pub grid_cell_geom: Option<Value>,
    pub grid_cell_centroid: Option<Value>,
    pub population_estimate: Option<f64>,
    pub density_per_km2: Option<f64>,
    pub zone_type: Option<String>,
    pub model_type: Option<String>,
    pub confidence: Option<f64>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PopulationAnalysisResult {
    pub site_id: i32,
    pub total_population: f64,
    pub population_density_avg: f64,
    pub population_density_max: f64,
    pub model_type: String,
    pub confidence: f64,
    pub grid_cells: Vec<PopulationGridCell>,
    pub zone_populations: Vec<ZonePopulation>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PopulationGridCell {
    pub lon: f64,
    pub lat: f64,
    pub population: f64,
    pub density: f64,
    pub zone_type: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ZonePopulation {
    pub zone_type: String,
    pub population: f64,
    pub area_km2: f64,
    pub density: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefenseAnalysisResult {
    pub id: Option<i32>,
    pub site_id: i32,
    pub overall_defense_score: f64,
    pub visibility_analysis: Option<Value>,
    pub weak_points: Vec<DefenseWeakPoint>,
    pub optimal_attack_routes: Vec<AttackRoute>,
    pub accessibility_score: f64,
    pub gate_defense_scores: Vec<GateDefenseScore>,
    pub wall_segments: Option<Value>,
    pub gates: Vec<CityGate>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DefenseWeakPoint {
    pub lon: f64,
    pub lat: f64,
    pub weakness_score: f64,
    pub weakness_type: String,
    pub description: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AttackRoute {
    pub route_geom: Value,
    pub start_lon: f64,
    pub start_lat: f64,
    pub end_lon: f64,
    pub end_lat: f64,
    pub attack_score: f64,
    pub route_type: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GateDefenseScore {
    pub gate_id: i32,
    pub gate_name: Option<String>,
    pub defense_score: f64,
    pub vulnerability: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LandUseChange {
    pub id: i32,
    pub site_id: i32,
    pub period_name: Option<String>,
    pub period_year: Option<i32>,
    pub land_use_type: String,
    pub area_km2: f64,
    pub percentage: f64,
    pub evidence_type: Option<String>,
    pub confidence: Option<f64>,
    pub description: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LandUseTimeline {
    pub site_id: i32,
    pub periods: Vec<LandUsePeriod>,
    pub land_use_types: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LandUsePeriod {
    pub period_name: String,
    pub period_year: i32,
    pub land_uses: Vec<LandUseItem>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LandUseItem {
    pub land_use_type: String,
    pub area_km2: f64,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CivilizationCompareRequest {
    pub civilization_ids: Option<Vec<i32>>,
    pub indicator: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CivilizationComparison {
    pub civilizations: Vec<CivilizationSummary>,
    pub indicators: Vec<String>,
    pub radar_data: Vec<CivilizationRadarData>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CivilizationSummary {
    pub id: i32,
    pub name: String,
    pub name_cn: String,
    pub region: Option<String>,
    pub site_count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CivilizationRadarData {
    pub civilization_id: i32,
    pub civilization_name: String,
    pub values: Vec<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CivilizationAvgMetrics {
    pub civilization_id: i32,
    pub civilization_name: String,
    pub site_count: i32,
    pub avg_integration_global: f64,
    pub avg_choice_global: f64,
    pub avg_boundary_fd: f64,
    pub avg_road_fd: f64,
    pub avg_compactness: f64,
    pub avg_road_density: f64,
    pub avg_functional_diversity: f64,
    pub avg_area_sq_km: f64,
}

