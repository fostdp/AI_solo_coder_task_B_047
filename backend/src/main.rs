use actix_web::{web, App, HttpServer, middleware, HttpResponse};
use actix_cors::Cors;
use actix_web_lab::middleware::from_fn;
use dotenv::dotenv;
use std::env;
use std::time::Instant;
use tracing_actix_web::TracingLogger;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

mod db;
mod models;
mod config;
mod city_loader;
mod morphology_analyzer;
mod evolution_detector;
mod spatial_syntax;
mod fractal;
mod mann_kendall;
mod errors;
mod metrics;

fn init_tracing() {
    tracing_subscriber::registry()
        .with(EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| "ancient_city_morphology=info,tower_http=info".into()))
        .with(tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true))
        .init();
}

async fn metrics_endpoint() -> HttpResponse {
    let body = metrics::gather_metrics_text();
    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(body)
}

async fn timing_middleware(
    req: actix_web::dev::ServiceRequest,
    srv: actix_web::dev::Service,
) -> Result<actix_web::dev::ServiceResponse, actix_web::Error> {
    let method = req.method().to_string();
    let path = req.path().to_string();
    let start = Instant::now();
    let fut = srv.call(req);
    let res = fut.await?;
    let duration = start.elapsed().as_secs_f64();
    let status = res.status().as_u16();
    metrics::record_request(&method, &path, status);
    metrics::record_request_duration(&method, &path, duration);
    tracing::info!(method = %method, path = %path, status = %status, duration_secs = %duration, "request handled");
    Ok(res)
}

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    init_tracing();
    metrics::init_metrics();

    let database_url = env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost:5432/ancient_city".to_string());
    
    let pool = db::init_pool(&database_url)
        .await
        .expect("Failed to create database pool");

    let port = env::var("PORT")
        .unwrap_or_else(|_| "8080".to_string())
        .parse::<u16>()
        .unwrap_or(8080);

    tracing::info!(port = %port, "Starting ancient city morphology server");

    HttpServer::new(move || {
        let cors = Cors::permissive();
        
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .wrap(cors)
            .wrap(TracingLogger::default())
            .wrap(middleware::Compress::default())
            .wrap_fn(timing_middleware)
            .route("/metrics", web::get().to(metrics_endpoint))
            .route("/api/health", web::get().to(city_loader::health_check))
            .route("/api/dynasties", web::get().to(city_loader::get_dynasties))
            .route("/api/sites", web::get().to(city_loader::get_city_sites))
            .route("/api/sites/{id}", web::get().to(city_loader::get_city_site_by_id))
            .route("/api/sites/dynasty/{dynasty_id}", web::get().to(city_loader::get_sites_by_dynasty))
            .route("/api/zones/{site_id}", web::get().to(city_loader::get_functional_zones))
            .route("/api/roads/{site_id}", web::get().to(city_loader::get_roads))
            .route("/api/buildings/{site_id}", web::get().to(city_loader::get_buildings))
            .route("/api/population/{site_id}", web::get().to(city_loader::get_population))
            .route("/api/morphology/{site_id}", web::get().to(morphology_analyzer::get_morphology))
            .route("/api/morphology/analyze/{site_id}", web::post().to(morphology_analyzer::analyze_morphology))
            .route("/api/syntax/roads/{site_id}", web::get().to(morphology_analyzer::get_road_syntax))
            .route("/api/trends/analyze", web::post().to(evolution_detector::analyze_trends))
            .route("/api/trends", web::get().to(evolution_detector::get_trends))
            .route("/api/compare", web::post().to(evolution_detector::compare_sites))
    })
    .bind(("0.0.0.0", port))?
    .run()
    .await
}
