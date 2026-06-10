use actix_web::{web, HttpResponse};
use serde_json::json;
use sqlx::PgPool;

use crate::models::*;
use crate::errors::AppError;
use crate::spatial_syntax::*;
use crate::fractal::*;
use crate::config::algorithm::*;
use crate::metrics;

pub async fn get_morphology(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let row = sqlx::query!(
        r#"
        SELECT id, city_site_id, analysis_date, integration_global, integration_local,
               choice_global, choice_local, mean_depth, total_depth, connectivity,
               boundary_fractal_dimension, road_network_fractal_dimension,
               compactness_index, elongation_ratio, road_density, intersection_density,
               functional_diversity, functional_mixing,
               boundary_fd_quality, road_fd_quality,
               boundary_fd_confidence_lower, boundary_fd_confidence_upper,
               road_fd_confidence_lower, road_fd_confidence_upper,
               notes, created_at
        FROM morphology_analyses
        WHERE city_site_id = $1
        ORDER BY analysis_date DESC
        LIMIT 1
        "#,
        site_id.into_inner()
    )
    .fetch_optional(pool.get_ref())
    .await?;

    match row {
        Some(row) => {
            let analysis = MorphologyAnalysis {
                id: row.id,
                city_site_id: row.city_site_id,
                analysis_date: row.analysis_date.map(|dt| dt.and_utc()),
                integration_global: row.integration_global,
                integration_local: row.integration_local,
                choice_global: row.choice_global,
                choice_local: row.choice_local,
                mean_depth: row.mean_depth,
                total_depth: row.total_depth,
                connectivity: row.connectivity,
                boundary_fractal_dimension: row.boundary_fractal_dimension,
                road_network_fractal_dimension: row.road_network_fractal_dimension,
                compactness_index: row.compactness_index,
                elongation_ratio: row.elongation_ratio,
                road_density: row.road_density,
                intersection_density: row.intersection_density,
                functional_diversity: row.functional_diversity,
                functional_mixing: row.functional_mixing,
                boundary_fd_quality: row.boundary_fd_quality,
                road_fd_quality: row.road_fd_quality,
                boundary_fd_confidence_lower: row.boundary_fd_confidence_lower,
                boundary_fd_confidence_upper: row.boundary_fd_confidence_upper,
                road_fd_confidence_lower: row.road_fd_confidence_lower,
                road_fd_confidence_upper: row.road_fd_confidence_upper,
                notes: row.notes,
                created_at: row.created_at.map(|dt| dt.and_utc()),
            };
            Ok(HttpResponse::Ok().json(ApiResponse::success(analysis)))
        }
        None => Err(AppError::NotFound("Morphology analysis not found".to_string())),
    }
}

pub async fn analyze_morphology(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let site_id = site_id.into_inner();
    tracing::info!(site_id = %site_id, "Starting morphology analysis");
    metrics::inc_syntax_compute();

    let city_row = sqlx::query!(
        r#"
        SELECT ST_AsGeoJSON(geom)::jsonb as geom_json, area_sq_km
        FROM city_sites WHERE id = $1
        "#,
        site_id
    )
    .fetch_optional(pool.get_ref())
    .await?;

    let city_row = match city_row {
        Some(r) => r,
        None => return Err(AppError::NotFound("City site not found".to_string())),
    };

    let roads_rows = sqlx::query!(
        r#"
        SELECT id, ST_AsGeoJSON(geom)::jsonb as geom_json
        FROM roads WHERE city_site_id = $1
        "#,
        site_id
    )
    .fetch_all(pool.get_ref())
    .await?;

    let zones_rows = sqlx::query!(
        r#"
        SELECT id, zone_type, ST_Area(geom::geography) as area
        FROM functional_zones WHERE city_site_id = $1
        "#,
        site_id
    )
    .fetch_all(pool.get_ref())
    .await?;

    let boundary_points = extract_polygon_points(&city_row.geom_json);
    metrics::inc_fractal_compute();
    let boundary_fd_result = boundary_fractal_dimension_robust(
        &boundary_points,
        FRACTAL_NUM_SCALES,
        FRACTAL_QUALITY_THRESHOLD,
        FRACTAL_BOOTSTRAP_SAMPLES_MIN,
        FRACTAL_BOOTSTRAP_SAMPLES_MAX,
    );
    let boundary_fd = boundary_fd_result.weighted_average;

    let road_geoms: Vec<Option<serde_json::Value>> = roads_rows.iter()
        .map(|r| r.geom_json.clone())
        .collect();
    let road_segments = extract_road_segments(&road_geoms);
    let road_fd_result = network_fractal_dimension_robust(
        &road_segments,
        FRACTAL_NUM_SCALES,
        FRACTAL_QUALITY_THRESHOLD,
        FRACTAL_BOOTSTRAP_SAMPLES_MIN,
        FRACTAL_BOOTSTRAP_SAMPLES_MAX,
    );
    let road_fd = road_fd_result.weighted_average;

    let (min_x, max_x, min_y, max_y) = bounding_box_points(&boundary_points);
    let area = polygon_area(&boundary_points);
    let perimeter = polygon_perimeter(&boundary_points);
    let compactness = compactness_index(area, perimeter);
    let elongation = elongation_ratio(min_x, max_x, min_y, max_y);

    let mut axial_lines = Vec::with_capacity(road_segments.len());
    for (i, segment) in road_segments.iter().enumerate() {
        let line = AxialLine::new(i, segment.0.0, segment.0.1, segment.1.0, segment.1.1);
        axial_lines.push(line);
    }

    let graph = if axial_lines.len() > SYNTAX_NODE_THRESHOLD_OPTIMIZED_GRAPH {
        build_axial_graph_optimized(&axial_lines)
    } else {
        let mut g = SpatialGraph::with_capacity(axial_lines.len());
        for line in &axial_lines {
            let (mx, my) = line.midpoint();
            g.add_node(mx, my);
        }
        for i in 0..axial_lines.len() {
            for j in (i + 1)..axial_lines.len() {
                if lines_intersect(
                    axial_lines[i].start_x, axial_lines[i].start_y,
                    axial_lines[i].end_x, axial_lines[i].end_y,
                    axial_lines[j].start_x, axial_lines[j].start_y,
                    axial_lines[j].end_x, axial_lines[j].end_y,
                ) {
                    g.add_edge(i, j);
                }
            }
        }
        g.optimize_memory();
        g
    };

    let avg_integration = graph.average_integration_global();
    let avg_connectivity = graph.average_connectivity();
    let avg_mean_depth = graph.average_mean_depth();
    let avg_total_depth = graph.average_total_depth();
    let local_integration = if graph.node_count() > 0 {
        graph.integration_local(0, SYNTAX_LOCAL_RADIUS)
    } else {
        0.0
    };

    let all_choice = if graph.node_count() > SYNTAX_NODE_THRESHOLD_CHUNKED {
        graph.choice_global_brandes_chunked(SYNTAX_CHUNK_SIZE)
    } else {
        graph.choice_global_brandes()
    };
    let avg_choice = if all_choice.is_empty() {
        0.0
    } else {
        all_choice.iter().sum::<f64>() / all_choice.len() as f64
    };
    let local_choice = if graph.node_count() > 0 && !all_choice.is_empty() {
        *all_choice.get(0).unwrap_or(&0.0)
    } else {
        0.0
    };

    let per_road_metrics = if graph.node_count() > 0 && axial_lines.len() >= graph.node_count() {
        graph.compute_all_metrics(SYNTAX_LOCAL_RADIUS)
    } else {
        Vec::new()
    };

    let road_density = if area > 0.0 {
        road_segments.len() as f64 / area * 1_000_000.0
    } else {
        0.0
    };

    let intersection_density = if area > 0.0 {
        graph.node_count() as f64 / area * 1_000_000.0
    } else {
        0.0
    };

    let mut zone_type_areas = std::collections::HashMap::new();
    for row in &zones_rows {
        let zone_area = row.area.unwrap_or(0.0);
        let entry = zone_type_areas.entry(row.zone_type.clone()).or_insert(0.0);
        *entry += zone_area;
    }

    let zone_counts: Vec<usize> = zone_type_areas.values().map(|_| 1).collect();
    let functional_diversity = shannon_diversity(&zone_counts);

    let zone_areas: Vec<f64> = zone_type_areas.values().copied().collect();
    let functional_mixing = functional_mixing_index(&zone_areas);

    if !per_road_metrics.is_empty() {
        for (i, metrics) in per_road_metrics.iter().enumerate() {
            if i < roads_rows.len() {
                let road_id = roads_rows[i].id;
                sqlx::query!(
                    r#"
                    INSERT INTO road_syntax_results
                    (road_id, city_site_id, integration, choice, depth, connectivity, control)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (road_id) DO UPDATE
                    SET integration = EXCLUDED.integration,
                        choice = EXCLUDED.choice,
                        depth = EXCLUDED.depth,
                        connectivity = EXCLUDED.connectivity,
                        control = EXCLUDED.control
                    "#,
                    road_id,
                    site_id,
                    metrics.integration,
                    metrics.choice,
                    metrics.depth,
                    metrics.connectivity,
                    metrics.control
                )
                .execute(pool.get_ref())
                .await?;
            }
        }
    } else {
        for (i, _line) in axial_lines.iter().enumerate() {
            if i < roads_rows.len() {
                let road_id = roads_rows[i].id;
                let integration = graph.integration_global(i);
                let choice = all_choice.get(i).copied().unwrap_or(0.0);
                let depth = graph.mean_depth(i);
                let connectivity = graph.connectivity(i) as i32;
                let control = graph.control(i);

                sqlx::query!(
                    r#"
                    INSERT INTO road_syntax_results
                    (road_id, city_site_id, integration, choice, depth, connectivity, control)
                    VALUES ($1, $2, $3, $4, $5, $6, $7)
                    ON CONFLICT (road_id) DO UPDATE
                    SET integration = EXCLUDED.integration,
                        choice = EXCLUDED.choice,
                        depth = EXCLUDED.depth,
                        connectivity = EXCLUDED.connectivity,
                        control = EXCLUDED.control
                    "#,
                    road_id,
                    site_id,
                    integration,
                    choice,
                    depth,
                    connectivity,
                    control
                )
                .execute(pool.get_ref())
                .await?;
            }
        }
    }

    let result = sqlx::query!(
        r#"
        INSERT INTO morphology_analyses
        (city_site_id, integration_global, integration_local, choice_global, choice_local,
         mean_depth, total_depth, connectivity, boundary_fractal_dimension,
         road_network_fractal_dimension, compactness_index, elongation_ratio,
         road_density, intersection_density, functional_diversity, functional_mixing,
         boundary_fd_quality, road_fd_quality,
         boundary_fd_confidence_lower, boundary_fd_confidence_upper,
         road_fd_confidence_lower, road_fd_confidence_upper, notes)
        VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16,
                $17, $18, $19, $20, $21, $22, $23)
        RETURNING id, city_site_id, analysis_date, integration_global, integration_local,
                  choice_global, choice_local, mean_depth, total_depth, connectivity,
                  boundary_fractal_dimension, road_network_fractal_dimension,
                  compactness_index, elongation_ratio, road_density, intersection_density,
                  functional_diversity, functional_mixing,
                  boundary_fd_quality, road_fd_quality,
                  boundary_fd_confidence_lower, boundary_fd_confidence_upper,
                  road_fd_confidence_lower, road_fd_confidence_upper,
                  notes, created_at
        "#,
        site_id,
        avg_integration,
        local_integration,
        avg_choice,
        local_choice,
        avg_mean_depth,
        avg_total_depth,
        avg_connectivity,
        boundary_fd,
        road_fd,
        compactness,
        elongation,
        road_density,
        intersection_density,
        functional_diversity,
        functional_mixing,
        Some(boundary_fd_result.overall_quality),
        Some(road_fd_result.overall_quality),
        Some(boundary_fd_result.confidence_lower),
        Some(boundary_fd_result.confidence_upper),
        Some(road_fd_result.confidence_lower),
        Some(road_fd_result.confidence_upper),
        Some("自动生成的形态分析结果".to_string())
    )
    .fetch_one(pool.get_ref())
    .await?;

    let analysis = MorphologyAnalysis {
        id: result.id,
        city_site_id: result.city_site_id,
        analysis_date: result.analysis_date.map(|dt| dt.and_utc()),
        integration_global: result.integration_global,
        integration_local: result.integration_local,
        choice_global: result.choice_global,
        choice_local: result.choice_local,
        mean_depth: result.mean_depth,
        total_depth: result.total_depth,
        connectivity: result.connectivity,
        boundary_fractal_dimension: result.boundary_fractal_dimension,
        road_network_fractal_dimension: result.road_network_fractal_dimension,
        compactness_index: result.compactness_index,
        elongation_ratio: result.elongation_ratio,
        road_density: result.road_density,
        intersection_density: result.intersection_density,
        functional_diversity: result.functional_diversity,
        functional_mixing: result.functional_mixing,
        boundary_fd_quality: result.boundary_fd_quality,
        road_fd_quality: result.road_fd_quality,
        boundary_fd_confidence_lower: result.boundary_fd_confidence_lower,
        boundary_fd_confidence_upper: result.boundary_fd_confidence_upper,
        road_fd_confidence_lower: result.road_fd_confidence_lower,
        road_fd_confidence_upper: result.road_fd_confidence_upper,
        notes: result.notes,
        created_at: result.created_at.map(|dt| dt.and_utc()),
    };

    Ok(HttpResponse::Ok().json(ApiResponse::success(analysis)))
}

pub async fn get_road_syntax(
    pool: web::Data<PgPool>,
    site_id: web::Path<i32>,
) -> Result<HttpResponse, AppError> {
    let rows = sqlx::query!(
        r#"
        SELECT id, road_id, city_site_id, integration, choice, depth,
               connectivity, control, created_at
        FROM road_syntax_results
        WHERE city_site_id = $1
        ORDER BY integration DESC
        "#,
        site_id.into_inner()
    )
    .fetch_all(pool.get_ref())
    .await?;

    let results: Vec<RoadSyntaxResult> = rows
        .into_iter()
        .map(|row| RoadSyntaxResult {
            id: row.id,
            road_id: row.road_id,
            city_site_id: row.city_site_id,
            integration: row.integration,
            choice: row.choice,
            depth: row.depth,
            connectivity: row.connectivity,
            control: row.control,
            created_at: row.created_at.map(|dt| dt.and_utc()),
        })
        .collect();

    Ok(HttpResponse::Ok().json(ApiResponse::success(results)))
}

fn extract_polygon_points(geom_json: &Option<serde_json::Value>) -> Vec<(f64, f64)> {
    let mut points = Vec::new();
    if let Some(geom) = geom_json {
        if let Some(coords) = geom.get("coordinates") {
            if let Some(ring) = coords.get(0) {
                if let Some(ring_arr) = ring.as_array() {
                    for p in ring_arr {
                        if let Some(arr) = p.as_array() {
                            if arr.len() >= 2 {
                                let x = arr[0].as_f64().unwrap_or(0.0);
                                let y = arr[1].as_f64().unwrap_or(0.0);
                                points.push((x, y));
                            }
                        }
                    }
                }
            }
        }
    }
    points
}

fn extract_road_segments(
    road_geoms: &[Option<serde_json::Value>],
) -> Vec<((f64, f64), (f64, f64))> {
    let mut segments = Vec::new();

    for geom_opt in road_geoms {
        if let Some(geom) = geom_opt {
            if let Some(coords) = geom.get("coordinates") {
                if let Some(coords_arr) = coords.as_array() {
                    for i in 0..coords_arr.len().saturating_sub(1) {
                        if let (Some(p1), Some(p2)) = (coords_arr[i].as_array(), coords_arr[i + 1].as_array()) {
                            if p1.len() >= 2 && p2.len() >= 2 {
                                let x1 = p1[0].as_f64().unwrap_or(0.0);
                                let y1 = p1[1].as_f64().unwrap_or(0.0);
                                let x2 = p2[0].as_f64().unwrap_or(0.0);
                                let y2 = p2[1].as_f64().unwrap_or(0.0);
                                segments.push(((x1, y1), (x2, y2)));
                            }
                        }
                    }
                }
            }
        }
    }

    segments
}

fn bounding_box_points(points: &[(f64, f64)]) -> (f64, f64, f64, f64) {
    if points.is_empty() {
        return (0.0, 0.0, 0.0, 0.0);
    }
    let mut min_x = points[0].0;
    let mut max_x = points[0].0;
    let mut min_y = points[0].1;
    let mut max_y = points[0].1;

    for &(x, y) in points {
        min_x = min_x.min(x);
        max_x = max_x.max(x);
        min_y = min_y.min(y);
        max_y = max_y.max(y);
    }

    (min_x, max_x, min_y, max_y)
}
