use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::f64::consts::PI;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpolationPoint {
    pub lon: f64,
    pub lat: f64,
    pub value: f64,
    pub weight: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterpolationResult {
    pub grid_points: Vec<(f64, f64, f64)>,
    pub method: String,
    pub bandwidth_km: f64,
    pub confidence: f64,
}

pub enum InterpolationMethod {
    KernelDensityEstimation,
    InverseDistanceWeighted,
    TriangulatedIrregularNetwork,
}

pub fn gaussian_kernel(distance_km: f64, bandwidth_km: f64) -> f64 {
    if bandwidth_km <= 0.0 {
        return 0.0;
    }
    let norm = distance_km / bandwidth_km;
    (-0.5 * norm * norm).exp()
}

pub fn epanechnikov_kernel(distance_km: f64, bandwidth_km: f64) -> f64 {
    if bandwidth_km <= 0.0 || distance_km > bandwidth_km {
        return 0.0;
    }
    let norm = distance_km / bandwidth_km;
    0.75 * (1.0 - norm * norm)
}

pub fn kernel_density_interpolation(
    center_lon: f64,
    center_lat: f64,
    site_area_km2: f64,
    control_points: &[InterpolationPoint],
    bandwidth_km: f64,
    grid_resolution: usize,
) -> InterpolationResult {
    let radius_km = (site_area_km2 / PI).sqrt();
    let deg_per_km = 1.0 / 111.0;
    let radius_deg = radius_km * deg_per_km;

    let mut grid_points = Vec::with_capacity(grid_resolution * grid_resolution);
    let step = 2.0 * radius_deg / grid_resolution as f64;

    for i in 0..grid_resolution {
        for j in 0..grid_resolution {
            let lon = center_lon - radius_deg + i as f64 * step;
            let lat = center_lat - radius_deg + j as f64 * step;

            let mut weighted_sum = 0.0;
            let mut weight_sum = 0.0;

            for cp in control_points {
                let d_lon = lon - cp.lon;
                let d_lat = lat - cp.lat;
                let distance_km = (d_lon * d_lon + d_lat * d_lat).sqrt() * 111.0;
                let kernel = gaussian_kernel(distance_km, bandwidth_km);
                let w = kernel * cp.weight;

                weighted_sum += cp.value * w;
                weight_sum += w;
            }

            let value = if weight_sum > 1e-9 {
                weighted_sum / weight_sum
            } else {
                0.0
            };

            grid_points.push((lon, lat, value));
        }
    }

    let confidence = if control_points.len() >= 10 {
        0.9
    } else if control_points.len() >= 5 {
        0.7
    } else if !control_points.is_empty() {
        0.5
    } else {
        0.2
    };

    InterpolationResult {
        grid_points,
        method: "kernel_density_estimation".to_string(),
        bandwidth_km,
        confidence,
    }
}

pub fn inverse_distance_weighted_interpolation(
    center_lon: f64,
    center_lat: f64,
    site_area_km2: f64,
    control_points: &[InterpolationPoint],
    power: f64,
    grid_resolution: usize,
) -> InterpolationResult {
    let radius_km = (site_area_km2 / PI).sqrt();
    let deg_per_km = 1.0 / 111.0;
    let radius_deg = radius_km * deg_per_km;

    let mut grid_points = Vec::with_capacity(grid_resolution * grid_resolution);
    let step = 2.0 * radius_deg / grid_resolution as f64;

    for i in 0..grid_resolution {
        for j in 0..grid_resolution {
            let lon = center_lon - radius_deg + i as f64 * step;
            let lat = center_lat - radius_deg + j as f64 * step;

            let mut weighted_sum = 0.0;
            let mut weight_sum = 0.0;

            for cp in control_points {
                let d_lon = lon - cp.lon;
                let d_lat = lat - cp.lat;
                let distance_km = (d_lon * d_lon + d_lat * d_lat).sqrt() * 111.0;

                let w = if distance_km < 1e-6 {
                    1e9
                } else {
                    cp.weight / distance_km.powf(power)
                };

                weighted_sum += cp.value * w;
                weight_sum += w;
            }

            let value = if weight_sum > 1e-9 {
                weighted_sum / weight_sum
            } else {
                0.0
            };

            grid_points.push((lon, lat, value));
        }
    }

    let confidence = if control_points.len() >= 10 {
        0.85
    } else if control_points.len() >= 5 {
        0.65
    } else if !control_points.is_empty() {
        0.45
    } else {
        0.15
    };

    InterpolationResult {
        grid_points,
        method: format!("idw_power_{}", power),
        bandwidth_km: radius_km,
        confidence,
    }
}

pub fn adaptive_bandwidth(
    control_points: &[InterpolationPoint],
    site_area_km2: f64,
) -> f64 {
    if control_points.len() < 2 {
        return (site_area_km2 / PI).sqrt() * 0.3;
    }

    let mut min_dist = f64::INFINITY;
    for i in 0..control_points.len() {
        for j in (i + 1)..control_points.len() {
            let d_lon = control_points[i].lon - control_points[j].lon;
            let d_lat = control_points[i].lat - control_points[j].lat;
            let dist = (d_lon * d_lon + d_lat * d_lat).sqrt() * 111.0;
            if dist < min_dist {
                min_dist = dist;
            }
        }
    }

    let n = control_points.len() as f64;
    let adaptive = min_dist * (n.powf(-0.2)).max(0.5);

    let radius_km = (site_area_km2 / PI).sqrt();
    adaptive.max(radius_km * 0.1).min(radius_km * 0.5)
}

pub fn interpolation_result_to_geojson(result: &InterpolationResult) -> Value {
    let features: Vec<Value> = result
        .grid_points
        .iter()
        .map(|(lon, lat, val)| {
            serde_json::json!({
                "type": "Feature",
                "geometry": {
                    "type": "Point",
                    "coordinates": [lon, lat]
                },
                "properties": {
                    "value": val
                }
            })
        })
        .collect();

    serde_json::json!({
        "type": "FeatureCollection",
        "features": features,
        "properties": {
            "method": result.method,
            "bandwidth_km": result.bandwidth_km,
            "confidence": result.confidence
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gaussian_kernel_at_zero() {
        assert_relative_eq!(gaussian_kernel(0.0, 1.0), 1.0, epsilon = 1e-9);
    }

    #[test]
    fn test_gaussian_kernel_decays() {
        assert!(gaussian_kernel(2.0, 1.0) < gaussian_kernel(1.0, 1.0));
        assert!(gaussian_kernel(1.0, 1.0) > 0.0);
    }

    #[test]
    fn test_kde_basic_interpolation() {
        let points = vec![
            InterpolationPoint { lon: 116.0, lat: 34.0, value: 100.0, weight: 1.0 },
            InterpolationPoint { lon: 116.01, lat: 34.01, value: 80.0, weight: 1.0 },
        ];
        let result = kernel_density_interpolation(116.0, 34.0, 1.0, &points, 0.5, 5);
        assert_eq!(result.grid_points.len(), 25);
        assert!(result.confidence > 0.4);
        for (_, _, v) in &result.grid_points {
            assert!(*v >= 0.0);
        }
    }

    #[test]
    fn test_adaptive_bandwidth_non_empty() {
        let points = vec![
            InterpolationPoint { lon: 116.0, lat: 34.0, value: 100.0, weight: 1.0 },
            InterpolationPoint { lon: 116.01, lat: 34.01, value: 80.0, weight: 1.0 },
        ];
        let bw = adaptive_bandwidth(&points, 1.0);
        assert!(bw > 0.0 && bw < 1.0);
    }
}
