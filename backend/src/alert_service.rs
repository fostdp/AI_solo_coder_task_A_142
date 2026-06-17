use crate::config::Config;
use crate::database::Database;
use crate::errors::Result;
use crate::models::{AlertEvent, SinanSensorData};
use crate::mqtt_service::MqttService;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;

pub struct AlertService {
    config: Config,
    db: Database,
    mqtt: Arc<MqttService>,
}

impl AlertService {
    pub fn new(config: Config, db: Database, mqtt: Arc<MqttService>) -> Self {
        Self { config, db, mqtt }
    }

    pub async fn process_sensor_data(&self, data: &mut SinanSensorData) -> Result<Option<AlertEvent>> {
        let deviation = data.pointing_deviation;
        let threshold = self.config.pointing_deviation_threshold;
        let critical_threshold = self.config.critical_deviation_threshold;

        if deviation > threshold {
            let alert_level = if deviation > critical_threshold {
                "CRITICAL"
            } else {
                "WARNING"
            };

            let message = format!(
                "设备 {} 指向偏差超限: {:.2}° > {:.2}°",
                data.device_id, deviation, threshold
            );

            let alert = AlertEvent {
                id: Uuid::new_v4(),
                timestamp: Utc::now(),
                device_id: data.device_id.clone(),
                alert_type: "POINTING_DEVIATION".to_string(),
                alert_level: alert_level.to_string(),
                pointing_deviation: deviation,
                threshold,
                sensor_data_id: data.id,
                is_acknowledged: false,
                message: message.clone(),
                mqtt_topic: self.config.mqtt_topic.clone(),
                mqtt_published: false,
            };

            data.is_alert = true;

            self.db.insert_alert_event(&alert).await?;

            let mqtt_published = self.mqtt.publish_alert(&alert).await.unwrap_or(false);

            if mqtt_published {
                self.db.mark_alert_mqtt_published(alert.id).await.ok();
            }

            tracing::warn!(
                "指向偏差告警 - 设备: {}, 偏差: {:.2}°, 级别: {}",
                data.device_id,
                deviation,
                alert_level
            );

            Ok(Some(alert))
        } else {
            Ok(None)
        }
    }

    pub async fn check_and_send_pending_alerts(&self) -> Result<usize> {
        let alerts = self.db.get_active_alerts(100).await?;
        let mut published_count = 0;

        for alert in alerts {
            if !alert.mqtt_published {
                match self.mqtt.publish_alert(&alert).await {
                    Ok(true) => {
                        self.db.mark_alert_mqtt_published(alert.id).await.ok();
                        published_count += 1;
                    }
                    Ok(false) => {}
                    Err(e) => {
                        tracing::error!("重发告警失败: {}", e);
                    }
                }
            }
        }

        Ok(published_count)
    }

    pub fn get_warning_threshold(&self) -> f64 {
        self.config.pointing_deviation_threshold
    }

    pub fn get_critical_threshold(&self) -> f64 {
        self.config.critical_deviation_threshold
    }

    pub async fn acknowledge_alert(&self, alert_id: Uuid) -> Result<()> {
        self.db.acknowledge_alert(alert_id).await
    }

    pub async fn get_active_alerts(&self, limit: usize) -> Result<Vec<AlertEvent>> {
        self.db.get_active_alerts(limit).await
    }
}
