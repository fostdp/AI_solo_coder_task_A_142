use crate::channels::{AlarmCommand, ChannelSenders, DtuCommand, GeomagneticCommand, SimulatorCommand};
use crate::database::Database;
use crate::errors::{AppError, Result};
use crate::models::{AlertAcknowledgeRequest, PointingSimulationParams, SinanSensorData, VectorFieldRequest};
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

#[derive(Clone)]
pub struct AppState {
    pub db: Database,
    pub senders: ChannelSenders,
    pub sensor_data_cache: Arc<RwLock<HashMap<String, SinanSensorData>>>,
}

pub async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "ok",
        "timestamp": Utc::now().to_rfc3339(),
        "service": "sinan-backend",
        "version": "2.0.0",
        "architecture": "actor-mpsc"
    }))
}

pub async fn receive_sensor_data(
    State(state): State<AppState>,
    Json(data): Json<SinanSensorData>,
) -> Result<(StatusCode, Json<serde_json::Value>)> {
    state
        .senders
        .dtu_tx
        .send(DtuCommand::ReceiveSensor(data))
        .await
        .map_err(|e| AppError::InternalError(format!("DTU通道发送失败: {}", e)))?;

    Ok((StatusCode::OK, Json(serde_json::json!({
        "status": "success",
        "message": "传感器数据已提交到处理管线",
        "architecture": "dtu_receiver → magnetic_simulator → alarm_mqtt"
    }))))
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
        .ok_or_else(|| AppError::InvalidParameter("缺少 device_id 参数".to_string()))?;

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
        .ok_or_else(|| AppError::InvalidParameter("缺少 lat 参数".to_string()))?
        .parse()
        .map_err(|_| AppError::InvalidParameter("lat 参数格式错误".to_string()))?;
    let lon: f64 = params
        .get("lon")
        .ok_or_else(|| AppError::InvalidParameter("缺少 lon 参数".to_string()))?
        .parse()
        .map_err(|_| AppError::InvalidParameter("lon 参数格式错误".to_string()))?;
    let target_year: f64 = params
        .get("year")
        .ok_or_else(|| AppError::InvalidParameter("缺少 year 参数".to_string()))?
        .parse()
        .map_err(|_| AppError::InvalidParameter("year 参数格式错误".to_string()))?;
    let altitude_km: f64 = params
        .get("altitude")
        .and_then(|v| v.parse().ok())
        .unwrap_or(0.0);

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    state
        .senders
        .geo_tx
        .send(GeomagneticCommand::CalculateField {
            lat,
            lon,
            year: target_year,
            altitude_km: Some(altitude_km),
            reply: reply_tx,
        })
        .await
        .map_err(|e| AppError::InternalError(format!("地磁通道发送失败: {}", e)))?;

    let field_data = reply_rx
        .await
        .map_err(|_| AppError::InternalError("地磁计算响应通道关闭".to_string()))??;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": field_data,
    })))
}

pub async fn generate_vector_field(
    State(state): State<AppState>,
    Json(request): Json<VectorFieldRequest>,
) -> Result<Json<serde_json::Value>> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    state
        .senders
        .geo_tx
        .send(GeomagneticCommand::GenerateVectorField {
            request,
            reply: reply_tx,
        })
        .await
        .map_err(|e| AppError::InternalError(format!("地磁通道发送失败: {}", e)))?;

    let response = reply_rx
        .await
        .map_err(|_| AppError::InternalError("矢量场计算响应通道关闭".to_string()))??;

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": response,
    })))
}

pub async fn run_pointing_simulation(
    State(state): State<AppState>,
    Json(params): Json<PointingSimulationParams>,
) -> Result<Json<serde_json::Value>> {
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    state
        .senders
        .sim_tx
        .send(SimulatorCommand::RunSimulation(params, reply_tx))
        .await
        .map_err(|e| AppError::InternalError(format!("仿真通道发送失败: {}", e)))?;

    let result = reply_rx
        .await
        .map_err(|_| AppError::InternalError("仿真响应通道关闭".to_string()))??;

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
    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    state
        .senders
        .alarm_tx
        .send(AlarmCommand::AcknowledgeAlert(request.alert_id, reply_tx))
        .await
        .map_err(|e| AppError::InternalError(format!("告警通道发送失败: {}", e)))?;

    reply_rx
        .await
        .map_err(|_| AppError::InternalError("告警确认响应通道关闭".to_string()))??;

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

    Ok(Json(serde_json::json!({
        "status": "success",
        "data": stats,
    })))
}

pub async fn get_secular_variation(
    State(state): State<AppState>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>> {
    let lat: f64 = params
        .get("lat")
        .ok_or_else(|| AppError::InvalidParameter("缺少 lat 参数".to_string()))?
        .parse()
        .map_err(|_| AppError::InvalidParameter("lat 参数格式错误".to_string()))?;
    let lon: f64 = params
        .get("lon")
        .ok_or_else(|| AppError::InvalidParameter("缺少 lon 参数".to_string()))?
        .parse()
        .map_err(|_| AppError::InvalidParameter("lon 参数格式错误".to_string()))?;
    let target_year: f64 = params
        .get("year")
        .ok_or_else(|| AppError::InvalidParameter("缺少 year 参数".to_string()))?
        .parse()
        .map_err(|_| AppError::InvalidParameter("year 参数格式错误".to_string()))?;

    let (reply_tx, reply_rx) = tokio::sync::oneshot::channel();

    state
        .senders
        .geo_tx
        .send(GeomagneticCommand::CalculateSecularVariation {
            lat,
            lon,
            year: target_year,
            reply: reply_tx,
        })
        .await
        .map_err(|e| AppError::InternalError(format!("地磁通道发送失败: {}", e)))?;

    let (d_intensity, d_declination, d_inclination) = reply_rx
        .await
        .map_err(|_| AppError::InternalError("长期变计算响应通道关闭".to_string()))??;

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
    let broadcast_rx = state.senders.broadcast_tx.subscribe();

    let stream = tokio_stream::wrappers::BroadcastStream::new(broadcast_rx)
        .filter_map(|result| {
            match result {
                Ok(data) => {
                    let event = Event::default()
                        .json_data(&data)
                        .unwrap_or_else(|_| Event::default().data("{}"));
                    Some(Ok(event))
                }
                Err(_) => None,
            }
        });

    Sse::new(stream).keep_alive(
        axum::response::sse::KeepAlive::new()
            .interval(std::time::Duration::from_secs(15))
            .text("keep-alive"),
    )
}
