#![cfg(test)]

use ancient_city_morphology::population::{
    get_zone_density_range, allometric_growth_model, residential_density_model,
    inverse_distance_weighted,
};
use ancient_city_morphology::defense::{
    generate_simulated_gates, generate_wall_segments, analyze_weak_points,
    compute_attack_routes, compute_visibility_analysis, compute_accessibility_score,
    compute_overall_defense_score,
};
use ancient_city_morphology::civilization::{
    compute_radar_values, generate_civilization_planning_notes, normalize, RADAR_INDICATORS,
    CivilizationAvgMetrics,
};
use ancient_city_morphology::land_use::{
    generate_simulated_land_use, compute_land_use_trend,
    PERIOD_NAMES, LAND_USE_TYPES, LAND_USE_COLORS, LAND_USE_LABELS,
};
use approx::assert_relative_eq;

fn sample_zones() -> Vec<(String, f64)> {
    vec![
        ("palace".into(), 0.2),
        ("residential".into(), 1.5),
        ("market".into(), 0.3),
        ("workshop".into(), 0.4),
        ("temple".into(), 0.1),
        ("official".into(), 0.2),
        ("tomb".into(), 0.1),
        ("storage".into(), 0.2),
    ]
}
fn sample_buildings() -> Vec<(f64, f64, String)> {
    vec![
        (116.001, 34.001, "palace".into()),
        (116.002, 34.002, "residential".into()),
        (116.003, 34.003, "residential".into()),
        (116.004, 34.004, "market".into()),
        (116.008, 34.008, "residential".into()),
        (116.009, 34.009, "residential".into()),
        (116.010, 34.010, "storage".into()),
    ]
}
fn sample_gates(lon: f64, lat: f64) -> Vec<ancient_city_morphology::models::CityGate> {
    generate_simulated_gates(lon, lat, 2.0)
}

#[test]
fn it_zoo_population_model_consistency() {
    let zones = sample_zones();
    let buildings = sample_buildings();

    let allo = allometric_growth_model(3.0, 50000.0, 116.0, 34.0, &zones, &buildings);
    let dens = residential_density_model(3.0, &buildings, &zones, 4.5);
    let idw  = inverse_distance_weighted(116.0, 34.0, 3.0, 50000.0, &zones);

    assert_eq!(allo.total_population, 50000.0);
    assert_eq!(idw.total_population, 50000.0);
    assert!(dens.total_population > 0.0);

    let zsum: f64 = allo.zone_populations.iter().map(|z| z.population).sum();
    assert_relative_eq!(zsum, 50000.0, epsilon = 1.0);

    let (min_r, max_r) = get_zone_density_range("residential");
    assert!(max_r > min_r);

    for c in &allo.grid_cells {
        assert!(c.density >= 0.0);
        assert!(c.population >= 0.0);
    }
}

#[test]
fn it_zoo_defense_pipeline_strong_gt_weak() {
    let lon = 116.0; let lat = 34.0; let r_km = 2.0;
    let gates = sample_gates(lon, lat);
    let segs = generate_wall_segments(lon, lat, r_km);

    let wp_strong = analyze_weak_points(lon, lat, r_km, &gates, &segs, 12.0, 5.0, 30.0, "mountain");
    let vis_strong = compute_visibility_analysis(lon, lat, r_km, &segs, "mountain");
    let rts_strong = compute_attack_routes(lon, lat, r_km, &gates, &segs, "mountain");
    let acc = compute_accessibility_score(&gates, r_km);
    let strong = compute_overall_defense_score(12.0, 5.0, 30.0, &wp_strong, &vis_strong, &rts_strong, &[], acc);

    let wp_weak = analyze_weak_points(lon, lat, r_km, &gates, &segs, 2.0, 1.0, 0.0, "wetland");
    let vis_weak = compute_visibility_analysis(lon, lat, r_km, &segs, "wetland");
    let rts_weak = compute_attack_routes(lon, lat, r_km, &gates, &segs, "wetland");
    let weak = compute_overall_defense_score(2.0, 1.0, 0.0, &wp_weak, &vis_weak, &rts_weak, &[], acc);

    assert!(strong > weak, "strong={} > weak={}", strong, weak);
    assert!(strong >= 0.0 && strong <= 100.0);
    assert!(weak   >= 0.0 && weak   <= 100.0);

    assert_eq!(gates.len(), 4);
    assert_eq!(vis_strong.len(), ancient_city_morphology::config::algorithm::DEFENSE_NUM_SAMPLE_POINTS);
    assert_eq!(rts_strong.len(), ancient_city_morphology::config::algorithm::DEFENSE_NUM_ATTACK_ROUTES);

    for r in &rts_strong {
        assert!(r.feasibility >= 0.0 && r.feasibility <= 1.0);
        assert!(r.risk_level <= 3);
    }
    for v in &vis_strong {
        assert!(v["visibility"].as_f64().unwrap() <= 1.0);
    }
}

#[test]
fn it_zoo_civilization_radar_well_separated() {
    let china = CivilizationAvgMetrics {
        civilization_id: 1, civilization_name: "中".into(), site_count: 10,
        avg_integration_global: 1.5, avg_choice_global: 0.7, avg_boundary_fd: 1.3,
        avg_road_fd: 1.6, avg_compactness: 0.85, avg_road_density: 8.0,
        avg_functional_diversity: 2.0, avg_area_sq_km: 6.0,
    };
    let rome = CivilizationAvgMetrics {
        civilization_id: 2, civilization_name: "罗".into(), site_count: 8,
        avg_integration_global: 1.0, avg_choice_global: 0.8, avg_boundary_fd: 1.6,
        avg_road_fd: 1.7, avg_compactness: 0.6, avg_road_density: 6.0,
        avg_functional_diversity: 1.8, avg_area_sq_km: 12.0,
    };
    let maya = CivilizationAvgMetrics {
        civilization_id: 3, civilization_name: "玛".into(), site_count: 5,
        avg_integration_global: 0.7, avg_choice_global: 0.3, avg_boundary_fd: 1.7,
        avg_road_fd: 1.3, avg_compactness: 0.3, avg_road_density: 2.0,
        avg_functional_diversity: 1.0, avg_area_sq_km: 2.0,
    };

    let vc = compute_radar_values(&china);
    let vr = compute_radar_values(&rome);
    let vm = compute_radar_values(&maya);

    for v in [&vc, &vr, &vm] {
        assert_eq!(v.len(), 8);
        v.iter().for_each(|x| assert!(*x >= 0.0 && *x <= 1.0));
    }
    assert_eq!(RADAR_INDICATORS.len(), 8);

    let d_cm: f64 = vc.iter().zip(vm.iter()).map(|(a,b)| (a-b).powi(2)).sum::<f64>().sqrt();
    let d_cr: f64 = vc.iter().zip(vr.iter()).map(|(a,b)| (a-b).powi(2)).sum::<f64>().sqrt();
    assert!(d_cm > 0.3, "china-maya Euclidean = {}", d_cm);
    assert!(d_cr > 0.1, "china-rome Euclidean = {}", d_cr);

    assert!(vc[4] > vm[4], "china compactness > maya");

    let notes = generate_civilization_planning_notes(1, &china);
    for k in ["planning_style","road_pattern","functional_characteristic",
              "city_scale","road_complexity","civilization_id"] {
        assert!(notes.contains_key(k), "missing {}", k);
    }

    let mut hi = china.clone(); hi.avg_compactness = 0.95; hi.avg_boundary_fd = 1.2;
    let n_hi = generate_civilization_planning_notes(1, &hi);
    assert_eq!(n_hi["planning_style"], "紧凑规整型");

    let mut or = rome.clone(); or.avg_compactness = 0.4; or.avg_boundary_fd = 1.8;
    let n_or = generate_civilization_planning_notes(2, &or);
    assert_eq!(n_or["planning_style"], "有机生长型");

    assert_relative_eq!(normalize(5.0, 0.0, 10.0), 0.5);
    assert_relative_eq!(normalize(15.0, 0.0, 10.0), 1.0);
    assert_relative_eq!(normalize(-1.0, 0.0, 10.0), 0.0);
}

#[test]
fn it_zoo_land_use_decay_and_farmland_rise() {
    let tl = generate_simulated_land_use(99);
    assert_eq!(tl.site_id, 99);
    assert_eq!(tl.periods.len(), ancient_city_morphology::config::algorithm::LAND_USE_NUM_PERIODS);
    assert_eq!(PERIOD_NAMES.len(), tl.periods.len());
    assert_eq!(LAND_USE_TYPES.len(), 8);
    assert_eq!(LAND_USE_COLORS.len(), 8);
    assert_eq!(LAND_USE_LABELS.len(), 8);

    let first = tl.periods.first().unwrap();
    let last  = tl.periods.last().unwrap();
    assert_eq!(first.items.len(), 8);
    for p in &tl.periods {
        for i in &p.items {
            assert!(i.area_km2 >= 0.0);
            assert!(i.percentage >= 0.0);
        }
        let s: f64 = p.items.iter().map(|i| i.percentage).sum();
        assert_relative_eq!(s, 100.0, epsilon = 0.5);
    }

    let u_first = first.items.iter().find(|i| i.land_use_type == "urban").unwrap().area_km2;
    let u_last  = last.items.iter().find(|i| i.land_use_type == "urban").unwrap().area_km2;
    assert!(u_first > u_last, "urban decays: {} > {}", u_first, u_last);

    let f_first = first.items.iter().find(|i| i.land_use_type == "farmland").unwrap().area_km2;
    let f_last  = last.items.iter().find(|i| i.land_use_type == "farmland").unwrap().area_km2;
    assert!(f_last >= f_first * 0.95, "farmland stays or grows");

    let trend = compute_land_use_trend(&tl);
    assert!(trend.get("urban_decay_rate").is_some());
    assert!(trend.get("trend_classification").is_some());
    let decay = trend["urban_decay_rate"].as_f64().unwrap_or(0.0);
    assert!(decay <= 0.01, "urban decay should be <= 0, got {}", decay);

    let empty_tl = ancient_city_morphology::land_use::LandUseTimeline { site_id: 0, periods: vec![] };
    let trend2 = compute_land_use_trend(&empty_tl);
    assert!(trend2.get("summary").is_some());
}
