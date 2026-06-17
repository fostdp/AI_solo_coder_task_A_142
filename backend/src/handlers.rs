use crate::alert_service::AlertService;
use crate::cals10k_model::CALS10KModel;
use crate::database::Database;
use crate::errors::Result;
use crate::micromagnetic_simulation::MicromagneticSimulator;
use crate::models::{
    AlertAcknowledgeRequest, PointingSimulationParams, SinanSensorData, VectorFieldRequest,
};
use axum::{
    extract::{Query, State},
    http::StatusCode,
    Json,
    response::sse::{Event, Sse},
};
use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio_stream::StreamExt;
use uuid::Uuid;

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub alert_service: Arc<AlertService>,
    pub simulator: Arc<MicromagneticSimulator>,
    pub geomagnetic_model: Arc<RwLock<CALS10KModel>>,
    pub sensor_data_cache: Arc<RwLock<HashMap<String, SinanSensorData>>>,
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339(),
        "service": "sinan-backend",
        "version": "1.0.0"
    }))
}

pub async fn receive_sensor_data(
    State(state): State<AppState>,
    Json(mut data): Json<SinanSensorData>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    data.id = Uuid::new_v4();
    data.timestamp = Utc::now();

    let moment_vec = state.simulator.calculate_magnetic_moment_from_sensor(
        data.magnetic_moment_x,
        data.magnetic_moment_y,
        data.magnetic_moment_z,
    );
    data.magnetic_moment_magnitude = moment_vec.magnitude();

    let geo_field = state
        .geomagnetic_model
        .read()
        .get_field_vector(data.location_lat, data.location_lon, 2024.0)?;

    data.pointing_deviation = state
        .simulator
        .calculate_pointing_deviation(moment_vec, geo_field)?;

    let alert = state.alert_service.process_sensor_data(&mut data).await?;

    state.db.insert_sensor_data(&data).await?;

    state
        .sensor_data_cache
        .write()
        .insert(data.device_id.clone(), data.clone());

    let mut response = serde_json::json!({
        "status": "success",
        "message": "传感器数据已接收",
        "data": {
            "id": data.id.to_string(),
            "timestamp": data.timestamp.to_rfc3339(),
            "device_id": data.device_id,
            "pointing_deviation": data.pointing_deviation,
            "is_alert": data.is_alert,
            "magnetic_moment_magnitude": data.magnetic_moment_magnitude,
        }
    });

    if let Some(alert) = alert {
        response["alert"] = serde_json::json!({
            "alert_id": alert.id.to_string(),
            "alert_level": alert.alert_level,
            "message": alert.message,
            "mqtt_published": alert.mqtt_published,
        });
    }

    Ok((StatusCode::OK, Json(response)))
}

pub async fn get_sensor_data(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let device_id = params.get("device_id").cloned();
    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);
    let offset = params
        .get("offset")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0);

    let start_time = params
        .get("start_time")
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc));
    let end_time = params
        .get("end_time")
        .and_then(|v| DateTime::parse_from_rfc3339(v).ok())
        .map(|dt| dt.with_timezone(&Utc));

    let data = state
        .db
        .query_sensor_data(
            device_id.as_deref(),
            start_time,
            end_time,
            limit,
            offset,
        )
        .await?;

    Ok(Json(serde_json::json!({
        "count": data.len(),
        "data": data,
    })))
}

pub async fn get_latest_sensor_data(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let device_id = params.get("device_id").cloned();
    let data = state
        .db
        .query_latest_sensor_data(device_id.as_deref())
        .await?;

    Ok(Json(serde_json::json!({
        "count": data.len(),
        "data": data,
    })))
}

pub async fn get_device_status(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let device_id = params
        .get("device_id")
        .ok_or_else(|| crate::errors::AppError::InvalidParameter("缺少 device_id 参数".to_string()))?;

    let status = state.db.get_device_status(device_id).await?;

    Ok(Json(serde_json::json!({
        "device_id": device_id,
        "status": status,
    })))
}

pub async fn get_all_devices(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    let devices = state.db.get_all_devices().await?;

    let devices: Vec<serde_json::Value> = devices
        .into_iter()
        .map(|(id, name)| {
            let latest = state.sensor_data_cache.read().get(&id).cloned();
            serde_json::json!({
                "device_id": id,
                "device_name": name,
                "latest_data": latest,
            })
        })
        .collect();

    Ok(Json(serde_json::json!({
        "count": devices.len(),
        "devices": devices,
    })))
}

pub async fn calculate_geomagnetic_field(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let lat: f64 = params
        .get("lat")
        .ok_or_else(|| crate::errors::AppError::InvalidParameter("缺少 lat 参数".to_string()))?
        .parse()
        .map_err(|_| crate::errors::AppError::InvalidParameter("lat 参数格式错误".to_string()))?;
    let lon: f64 = params
        .get("lon")
        .ok_or_else(|| crate::errors::AppError::InvalidParameter("缺少 lon 参数".to_string()))?
        .parse()
        .map_err(|_| crate::errors::AppError::InvalidParameter("lon 参数格式错误".to_string()))?;
    let target_year: f64 = params
        .get("year")
        .ok_or_else(|| crate::errors::AppError::InvalidParameter("缺少 year 参数".to_string()))?
        .parse()
        .map_err(|_| crate::errors::AppError::InvalidParameter("year 参数格式错误".to_string()))?;
    let altitude_km: f64 = params
        .get("altitude")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    let field_data = state
        .geomagnetic_model
        .read()
        .calculate_field_at_point(lat, lon, target_year, Some(altitude_km))?;

    state.db.insert_geomagnetic_data(&field_data).await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": field_data,
    })))
}

pub async fn generate_vector_field(
    State(state): State<AppState>,
    Json(request): Json<VectorFieldRequest>,
) -> Result<Json<serde_json::Value>> {
    let response = state
        .geomagnetic_model
        .read()
        .generate_vector_field(&request)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": response,
    })))
}

pub async fn run_pointing_simulation(
    State(state): State<AppState>,
    Json(params): Json<PointingSimulationParams>,
) -> Result<Json<serde_json::Value>> {
    let geo_field = state.geomagnetic_model.read().get_field_vector(
        params.location_lat,
        params.location_lon,
        params.target_year,
    )?;

    let result = state.simulator.simulate_pointing(&params, geo_field)?;

    state.db.insert_simulation_result(&result).await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "指向精度仿真完成",
        "data": result,
    })))
}

pub async fn get_simulation_results(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let device_id = params.get("device_id").cloned();
    let simulation_id = params.get("simulation_id").cloned();
    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(100);

    let results = state
        .db
        .query_simulation_results(device_id.as_deref(), simulation_id.as_deref(), limit)
        .await?;

    Ok(Json(serde_json::json!({
        "count": results.len(),
        "data": results,
    })))
}

pub async fn get_active_alerts(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let limit = params
        .get("limit")
        .and_then(|v| v.parse().ok())
        .unwrap_or(50);

    let alerts = state.db.get_active_alerts(limit).await?;

    Ok(Json(serde_json::json!({
        "count": alerts.len(),
        "data": alerts,
    })))
}

pub async fn acknowledge_alert(
    State(state): State<AppState>,
    Json(request): Json<AlertAcknowledgeRequest>,
) -> Result<Json<serde_json::Value>> {
    state.alert_service.acknowledge_alert(request.alert_id).await?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "message": "告警已确认",
        "alert_id": request.alert_id.to_string(),
        "acknowledged_by": request.acknowledged_by,
        "note": request.note,
    })))
}

pub async fn get_statistics(State(state): State<AppState>) -> Result<Json<serde_json::Value>> {
    let stats = state.db.get_statistics().await?;

    let thresholds = serde_json::json!({
        "warning_threshold": state.alert_service.get_warning_threshold(),
        "critical_threshold": state.alert_service.get_critical_threshold(),
    });

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": stats,
        "thresholds": thresholds,
    })))
}

pub async fn get_secular_variation(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let lat: f64 = params
        .get("lat")
        .ok_or_else(|| crate::errors::AppError::InvalidParameter("缺少 lat 参数".to_string()))?
        .parse()
        .map_err(|_| crate::errors::AppError::InvalidParameter("lat 参数格式错误".to_string()))?;
    let lon: f64 = params
        .get("lon")
        .ok_or_else(|| crate::errors::AppError::InvalidParameter("缺少 lon 参数".to_string()))?
        .parse()
        .map_err(|_| crate::errors::AppError::InvalidParameter("lon 参数格式错误".to_string()))?;
    let target_year: f64 = params
        .get("year")
        .ok_or_else(|| crate::errors::AppError::InvalidParameter("缺少 year 参数".to_string()))?
        .parse()
        .map_err(|_| crate::errors::AppError::InvalidParameter("year 参数格式错误".to_string()))?;

    let (d_intensity, d_declination, d_inclination) = state
        .geomagnetic_model
        .read()
        .calculate_secular_variation(lat, lon, target_year)?;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": {
            "target_year": target_year,
            "location": { "lat": lat, "lon": lon },
            "secular_variation": {
                "intensity_rate_nT_per_year": d_intensity,
                "declination_rate_deg_per_year": d_declination,
                "inclination_rate_deg_per_year": d_inclination,
            }
        }
    })))
}

pub async fn sensor_data_stream(
    State(state): State<AppState>,
) -> Sse<impl tokio_stream::Stream<Item = std::result::Result<Event, std::convert::Infallible>>> {
    let cache = state.sensor_data_cache.clone();

    let stream = tokio_stream::iter(()).then(move |_| {
        let cache = cache.clone();
        async move {
            let data: Vec<_> = cache.read().values().cloned().collect();
            Event::default()
                .json_data(&data)
                .unwrap_or_else(|_| Event::default().data("{}"))
        }
    })
    .throttle(std::time::Duration::from_secs(1))
    .map(Ok);

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}
