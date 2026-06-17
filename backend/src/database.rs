use crate::config::Config;
use crate::errors::{AppError, Result};
use crate::models::{AlertEvent, GeomagneticFieldData, PointingSimulationResult, SinanSensorData};
use chrono::{DateTime, Utc};
use clickhouse::Client;
use parking_lot::RwLock;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Clone)]
pub struct Database {
    client: Arc<RwLock<Client>>,
    database: String,
}

impl Database {
    pub fn new(config: &Config) -> Result<Self> {
        let client = Client::default()
            .with_url(format!(
                "http://{}:{}",
                config.clickhouse_host, config.clickhouse_port
            ))
            .with_database(&config.clickhouse_database);

        let client = if !config.clickhouse_user.is_empty() {
            client
                .with_user(&config.clickhouse_user)
                .with_password(&config.clickhouse_password)
        } else {
            client
        };

        Ok(Self {
            client: Arc::new(RwLock::new(client)),
            database: config.clickhouse_database.clone(),
        })
    }

    pub async fn insert_sensor_data(&self, data: &SinanSensorData) -> Result<()> {
        let client = self.client.read();
        let mut insert = client.insert("sinan_sensor_data")?;

        insert
            .write((
                data.id,
                data.device_id.clone(),
                data.timestamp,
                data.magnetic_moment_x,
                data.magnetic_moment_y,
                data.magnetic_moment_z,
                data.magnetic_moment_magnitude,
                data.remanence,
                data.pointing_deviation,
                data.environment_temp,
                data.location_lat,
                data.location_lon,
                data.is_alert,
            ))
            .await?;

        insert.end().await?;
        Ok(())
    }

    pub async fn insert_geomagnetic_data(&self, data: &GeomagneticFieldData) -> Result<()> {
        let client = self.client.read();
        let mut insert = client.insert("geomagnetic_field_data")?;

        insert
            .write((
                data.id,
                data.timestamp,
                data.target_year,
                data.location_lat,
                data.location_lon,
                data.field_intensity,
                data.declination,
                data.inclination,
                data.bx,
                data.by,
                data.bz,
                data.model_source.clone(),
            ))
            .await?;

        insert.end().await?;
        Ok(())
    }

    pub async fn insert_simulation_result(&self, result: &PointingSimulationResult) -> Result<()> {
        let client = self.client.read();
        let mut insert = client.insert("pointing_simulation_results")?;

        insert
            .write((
                result.id,
                result.timestamp,
                result.device_id.clone(),
                result.simulation_id.clone(),
                result.target_year,
                result.location_lat,
                result.location_lon,
                result.expected_azimuth,
                result.simulated_azimuth,
                result.pointing_accuracy,
                result.magnetic_moment_magnitude,
                result.remanence,
                result.temperature,
                result.friction_coefficient,
                result.demagnetization_factor,
                result.anisotropy_constant,
                result.model_parameters.clone(),
            ))
            .await?;

        insert.end().await?;
        Ok(())
    }

    pub async fn insert_alert_event(&self, alert: &AlertEvent) -> Result<()> {
        let client = self.client.read();
        let mut insert = client.insert("alert_events")?;

        insert
            .write((
                alert.id,
                alert.timestamp,
                alert.device_id.clone(),
                alert.alert_type.clone(),
                alert.alert_level.clone(),
                alert.pointing_deviation,
                alert.threshold,
                alert.sensor_data_id,
                alert.is_acknowledged,
                alert.message.clone(),
                alert.mqtt_topic.clone(),
                alert.mqtt_published,
            ))
            .await?;

        insert.end().await?;
        Ok(())
    }

    pub async fn query_sensor_data(
        &self,
        device_id: Option<&str>,
        start_time: Option<DateTime<Utc>>,
        end_time: Option<DateTime<Utc>>,
        limit: usize,
        offset: usize,
    ) -> Result<Vec<SinanSensorData>> {
        let client = self.client.read();
        let mut query = String::from(
            "SELECT id, device_id, timestamp, magnetic_moment_x, magnetic_moment_y, magnetic_moment_z, \
             magnetic_moment_magnitude, remanence, pointing_deviation, environment_temp, \
             location_lat, location_lon, is_alert FROM sinan_sensor_data WHERE 1=1"
        );

        if let Some(device) = device_id {
            query.push_str(&format!(" AND device_id = '{}'", device));
        }
        if let Some(start) = start_time {
            query.push_str(&format!(" AND timestamp >= '{}'", start.to_rfc3339()));
        }
        if let Some(end) = end_time {
            query.push_str(&format!(" AND timestamp <= '{}'", end.to_rfc3339()));
        }

        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT {} OFFSET {}", limit, offset));

        let result = client.query(&query).fetch_all::<SinanSensorData>().await?;
        Ok(result)
    }

    pub async fn query_latest_sensor_data(&self, device_id: Option<&str>) -> Result<Vec<SinanSensorData>> {
        let client = self.client.read();
        let mut query = String::from(
            "SELECT id, device_id, timestamp, magnetic_moment_x, magnetic_moment_y, magnetic_moment_z, \
             magnetic_moment_magnitude, remanence, pointing_deviation, environment_temp, \
             location_lat, location_lon, is_alert FROM sinan_sensor_data "
        );

        if let Some(device) = device_id {
            query.push_str(&format!("WHERE device_id = '{}' ", device));
        }

        query.push_str("ORDER BY timestamp DESC LIMIT 100");

        let result = client.query(&query).fetch_all::<SinanSensorData>().await?;
        Ok(result)
    }

    pub async fn get_active_alerts(&self, limit: usize) -> Result<Vec<AlertEvent>> {
        let client = self.client.read();
        let query = format!(
            "SELECT id, timestamp, device_id, alert_type, alert_level, pointing_deviation, \
             threshold, sensor_data_id, is_acknowledged, message, mqtt_topic, mqtt_published \
             FROM alert_events WHERE is_acknowledged = false ORDER BY timestamp DESC LIMIT {}",
            limit
        );

        let result = client.query(&query).fetch_all::<AlertEvent>().await?;
        Ok(result)
    }

    pub async fn acknowledge_alert(&self, alert_id: Uuid) -> Result<()> {
        let client = self.client.read();
        let query = format!(
            "ALTER TABLE alert_events UPDATE is_acknowledged = true WHERE id = '{}'",
            alert_id
        );

        client.execute(&query).await?;
        Ok(())
    }

    pub async fn query_geomagnetic_data(
        &self,
        target_year: f64,
        lat: f64,
        lon: f64,
        tolerance: f64,
    ) -> Result<Vec<GeomagneticFieldData>> {
        let client = self.client.read();
        let query = format!(
            "SELECT id, timestamp, target_year, location_lat, location_lon, field_intensity, \
             declination, inclination, bx, by, bz, model_source FROM geomagnetic_field_data \
             WHERE abs(target_year - {}) <= {} \
             AND abs(location_lat - {}) <= {} \
             AND abs(location_lon - {}) <= {} \
             ORDER BY timestamp DESC LIMIT 100",
            target_year, tolerance, lat, 0.5, lon, 0.5
        );

        let result = client.query(&query).fetch_all::<GeomagneticFieldData>().await?;
        Ok(result)
    }

    pub async fn query_simulation_results(
        &self,
        device_id: Option<&str>,
        simulation_id: Option<&str>,
        limit: usize,
    ) -> Result<Vec<PointingSimulationResult>> {
        let client = self.client.read();
        let mut query = String::from(
            "SELECT id, timestamp, device_id, simulation_id, target_year, location_lat, location_lon, \
             expected_azimuth, simulated_azimuth, pointing_accuracy, magnetic_moment_magnitude, \
             remanence, temperature, friction_coefficient, demagnetization_factor, \
             anisotropy_constant, model_parameters FROM pointing_simulation_results WHERE 1=1"
        );

        if let Some(device) = device_id {
            query.push_str(&format!(" AND device_id = '{}'", device));
        }
        if let Some(sim_id) = simulation_id {
            query.push_str(&format!(" AND simulation_id = '{}'", sim_id));
        }

        query.push_str(&format!(" ORDER BY timestamp DESC LIMIT {}", limit));

        let result = client.query(&query).fetch_all::<PointingSimulationResult>().await?;
        Ok(result)
    }

    pub async fn get_device_status(&self, device_id: &str) -> Result<Option<SinanSensorData>> {
        let client = self.client.read();
        let query = format!(
            "SELECT id, device_id, timestamp, magnetic_moment_x, magnetic_moment_y, magnetic_moment_z, \
             magnetic_moment_magnitude, remanence, pointing_deviation, environment_temp, \
             location_lat, location_lon, is_alert FROM sinan_sensor_data \
             WHERE device_id = '{}' ORDER BY timestamp DESC LIMIT 1",
            device_id
        );

        let result = client.query(&query).fetch_one::<SinanSensorData>().await;
        match result {
            Ok(data) => Ok(Some(data)),
            Err(clickhouse::error::Error::RowNotFound) => Ok(None),
            Err(e) => Err(AppError::DatabaseError(e)),
        }
    }

    pub async fn get_all_devices(&self) -> Result<Vec<(String, String)>> {
        let client = self.client.read();
        let query = "SELECT DISTINCT device_id, device_name FROM sinan_devices WHERE is_active = true";

        let result = client.query(query).fetch_all::<(String, String)>().await?;
        Ok(result)
    }

    pub async fn mark_alert_mqtt_published(&self, alert_id: Uuid) -> Result<()> {
        let client = self.client.read();
        let query = format!(
            "ALTER TABLE alert_events UPDATE mqtt_published = true WHERE id = '{}'",
            alert_id
        );

        client.execute(&query).await?;
        Ok(())
    }

    pub async fn get_statistics(&self) -> Result<serde_json::Value> {
        let client = self.client.read();

        let total_sensors: u64 = client
            .query("SELECT count() FROM sinan_sensor_data")
            .fetch_one()
            .await?;

        let total_alerts: u64 = client
            .query("SELECT count() FROM alert_events")
            .fetch_one()
            .await?;

        let active_alerts: u64 = client
            .query("SELECT count() FROM alert_events WHERE is_acknowledged = false")
            .fetch_one()
            .await?;

        let avg_deviation: f64 = client
            .query("SELECT avg(pointing_deviation) FROM sinan_sensor_data WHERE timestamp >= now() - INTERVAL 1 HOUR")
            .fetch_one()
            .await?;

        Ok(serde_json::json!({
            "total_sensor_records": total_sensors,
            "total_alerts": total_alerts,
            "active_alerts": active_alerts,
            "average_deviation_last_hour": avg_deviation,
        }))
    }
}
