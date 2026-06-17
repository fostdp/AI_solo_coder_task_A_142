mod alert_service;
mod cals10k_model;
mod config;
mod database;
mod errors;
mod handlers;
pub mod micromagnetic_simulation;
mod models;
mod mqtt_service;

use crate::alert_service::AlertService;
use crate::cals10k_model::CALS10KModel;
use crate::config::Config;
use crate::database::Database;
use crate::handlers::*;
use crate::micromagnetic_simulation::MicromagneticSimulator;
use crate::models::ArchaeologyMagneticData;
use crate::mqtt_service::MqttService;
use axum::{
    routing::{get, post},
    Router,
};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tower_http::cors::{Any, CorsLayer};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sinan_backend=debug".into()),
        )
        .init();

    tracing::info!("正在启动司南磁石指向精度仿真系统...");

    let config = Config::from_env()?;
    tracing::info!("配置加载完成");

    let db = Database::new(&config)?;
    tracing::info!("数据库连接成功");

    let mqtt_service = Arc::new(MqttService::new(&config).await);
    tracing::info!(
        "MQTT服务初始化完成，状态: {}",
        if mqtt_service.is_enabled() { "已启用" } else { "未启用" }
    );

    let alert_service = Arc::new(AlertService::new(config.clone(), db.clone(), mqtt_service.clone()));
    tracing::info!("告警服务初始化完成");

    let simulator = Arc::new(MicromagneticSimulator::new());
    tracing::info!("微磁学仿真器初始化完成");

    let mut geomagnetic_model = Arc::new(RwLock::new(CALS10KModel::new()));
    tracing::info!("CALS10K地磁场模型初始化完成");

    let archaeo_data = load_archaeomagnetic_data();
    geomagnetic_model
        .write()
        .load_archaeomagnetic_data(archaeo_data);
    tracing::info!("考古地磁数据已加载并校准");

    let sensor_data_cache = Arc::new(RwLock::new(HashMap::new()));

    let app_state = AppState {
        db: db.clone(),
        alert_service: alert_service.clone(),
        simulator,
        geomagnetic_model,
        sensor_data_cache,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/health", get(health_check))
        .route("/api/v1/sensor", post(receive_sensor_data))
        .route("/api/v1/sensor/data", get(get_sensor_data))
        .route("/api/v1/sensor/latest", get(get_latest_sensor_data))
        .route("/api/v1/sensor/stream", get(sensor_data_stream))
        .route("/api/v1/device/status", get(get_device_status))
        .route("/api/v1/devices", get(get_all_devices))
        .route("/api/v1/geomagnetic/field", get(calculate_geomagnetic_field))
        .route("/api/v1/geomagnetic/vectorfield", post(generate_vector_field))
        .route("/api/v1/geomagnetic/secular", get(get_secular_variation))
        .route("/api/v1/simulation/pointing", post(run_pointing_simulation))
        .route("/api/v1/simulation/results", get(get_simulation_results))
        .route("/api/v1/alerts/active", get(get_active_alerts))
        .route("/api/v1/alerts/acknowledge", post(acknowledge_alert))
        .route("/api/v1/statistics", get(get_statistics))
        .with_state(app_state)
        .layer(cors);

    let addr = format!("{}:{}", config.server_host, config.server_port);
    tracing::info!("HTTP服务器监听地址: {}", addr);

    let alert_service_clone = alert_service.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(60));
        loop {
            interval.tick().await;
            match alert_service_clone.check_and_send_pending_alerts().await {
                Ok(count) if count > 0 => {
                    tracing::info!("已重发 {} 条待推送告警", count);
                }
                Ok(_) => {}
                Err(e) => {
                    tracing::error!("检查待推送告警失败: {}", e);
                }
            }
        }
    });

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("司南磁石指向精度仿真系统启动完成!");
    tracing::info!("API文档:");
    tracing::info!("  POST /api/v1/sensor - 接收传感器数据");
    tracing::info!("  GET  /api/v1/sensor/data - 查询传感器数据");
    tracing::info!("  GET  /api/v1/geomagnetic/field - 计算地磁场");
    tracing::info!("  POST /api/v1/geomagnetic/vectorfield - 生成矢量场");
    tracing::info!("  POST /api/v1/simulation/pointing - 运行指向仿真");

    axum::serve(listener, app).await?;

    Ok(())
}

fn load_archaeomagnetic_data() -> Vec<ArchaeologyMagneticData> {
    vec![
        ArchaeologyMagneticData {
            site_name: "汉长安城遗址".to_string(),
            location_lat: 34.265,
            location_lon: 108.955,
            sample_age: -100.0,
            sample_age_error: 50.0,
            declination: -2.5,
            declination_error: 0.8,
            inclination: 56.2,
            inclination_error: 1.2,
            intensity: 55000.0,
            intensity_error: 3000.0,
            sample_material: "brick".to_string(),
            reference: "考古地磁学报2022".to_string(),
        },
        ArchaeologyMagneticData {
            site_name: "洛阳汉魏故城".to_string(),
            location_lat: 34.667,
            location_lon: 112.483,
            sample_age: -50.0,
            sample_age_error: 30.0,
            declination: -1.8,
            declination_error: 0.6,
            inclination: 55.8,
            inclination_error: 1.0,
            intensity: 54500.0,
            intensity_error: 2500.0,
            sample_material: "brick".to_string(),
            reference: "地球物理学报2021".to_string(),
        },
        ArchaeologyMagneticData {
            site_name: "长沙马王堆汉墓".to_string(),
            location_lat: 28.197,
            location_lon: 113.021,
            sample_age: -165.0,
            sample_age_error: 50.0,
            declination: -3.2,
            declination_error: 1.0,
            inclination: 48.5,
            inclination_error: 1.5,
            intensity: 52000.0,
            intensity_error: 3500.0,
            sample_material: "soil".to_string(),
            reference: "考古与文物2023".to_string(),
        },
        ArchaeologyMagneticData {
            site_name: "西安未央宫遗址".to_string(),
            location_lat: 34.285,
            location_lon: 108.925,
            sample_age: -80.0,
            sample_age_error: 40.0,
            declination: -2.3,
            declination_error: 0.7,
            inclination: 56.0,
            inclination_error: 1.1,
            intensity: 54800.0,
            intensity_error: 2800.0,
            sample_material: "brick".to_string(),
            reference: "考古地磁学报2022".to_string(),
        },
        ArchaeologyMagneticData {
            site_name: "徐州狮子山汉墓".to_string(),
            location_lat: 34.221,
            location_lon: 117.329,
            sample_age: -154.0,
            sample_age_error: 60.0,
            declination: -2.8,
            declination_error: 0.9,
            inclination: 52.3,
            inclination_error: 1.3,
            intensity: 53500.0,
            intensity_error: 3200.0,
            sample_material: "soil".to_string(),
            reference: "华夏考古2022".to_string(),
        },
    ]
}
