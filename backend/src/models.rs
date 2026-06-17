use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SinanSensorData {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    pub device_id: String,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub magnetic_moment_x: f64,
    pub magnetic_moment_y: f64,
    pub magnetic_moment_z: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub pointing_deviation: f64,
    pub environment_temp: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    #[serde(default)]
    pub is_alert: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeomagneticFieldData {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub field_intensity: f64,
    pub declination: f64,
    pub inclination: f64,
    pub bx: f64,
    pub by: f64,
    pub bz: f64,
    #[serde(default = "default_model_source")]
    pub model_source: String,
}

fn default_model_source() -> String {
    "CALS10K".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointingSimulationParams {
    pub device_id: String,
    pub simulation_id: String,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub temperature: f64,
    pub friction_coefficient: f64,
    pub demagnetization_factor: f64,
    pub anisotropy_constant: f64,
    pub expected_azimuth: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PointingSimulationResult {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub device_id: String,
    pub simulation_id: String,
    pub target_year: f64,
    pub location_lat: f64,
    pub location_lon: f64,
    pub expected_azimuth: f64,
    pub simulated_azimuth: f64,
    pub pointing_accuracy: f64,
    pub magnetic_moment_magnitude: f64,
    pub remanence: f64,
    pub temperature: f64,
    pub friction_coefficient: f64,
    pub demagnetization_factor: f64,
    pub anisotropy_constant: f64,
    pub model_parameters: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertEvent {
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
    pub device_id: String,
    pub alert_type: String,
    pub alert_level: String,
    pub pointing_deviation: f64,
    pub threshold: f64,
    pub sensor_data_id: Uuid,
    #[serde(default)]
    pub is_acknowledged: bool,
    pub message: String,
    pub mqtt_topic: String,
    #[serde(default)]
    pub mqtt_published: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArchaeologyMagneticData {
    pub site_name: String,
    pub location_lat: f64,
    pub location_lon: f64,
    pub sample_age: f64,
    pub sample_age_error: f64,
    pub declination: f64,
    pub declination_error: f64,
    pub inclination: f64,
    pub inclination_error: f64,
    pub intensity: f64,
    pub intensity_error: f64,
    pub sample_material: String,
    pub reference: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFieldPoint {
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub bx: f64,
    pub by: f64,
    pub bz: f64,
    pub magnitude: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFieldRequest {
    pub target_year: f64,
    pub center_lat: f64,
    pub center_lon: f64,
    pub radius_km: f64,
    pub grid_size: usize,
    pub altitude_km: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorFieldResponse {
    pub target_year: f64,
    pub center_lat: f64,
    pub center_lon: f64,
    pub grid_size: usize,
    pub points: Vec<VectorFieldPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertAcknowledgeRequest {
    pub alert_id: Uuid,
    pub acknowledged_by: String,
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueryRange {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SensorDataQuery {
    pub device_id: Option<String>,
    pub time_range: Option<QueryRange>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}
