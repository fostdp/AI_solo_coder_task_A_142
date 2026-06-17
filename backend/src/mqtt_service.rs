use crate::config::Config;
use crate::errors::{AppError, Result};
use crate::models::AlertEvent;
use rumqttc::{AsyncClient, MqttOptions, QoS};
use serde_json;
use std::time::Duration;
use uuid::Uuid;

pub struct MqttService {
    client: Option<AsyncClient>,
    topic: String,
    enabled: bool,
}

impl MqttService {
    pub async fn new(config: &Config) -> Self {
        let mut options = MqttOptions::new(
            &config.mqtt_client_id,
            &config.mqtt_host,
            config.mqtt_port,
        );

        options.set_keep_alive(Duration::from_secs(30));
        options.set_pending_throttle(Duration::from_millis(100));

        if !config.mqtt_username.is_empty() {
            options.set_credentials(&config.mqtt_username, &config.mqtt_password);
        }

        let (client, mut eventloop) = AsyncClient::new(options, 100);

        let topic = config.mqtt_topic.clone();

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(_) => {}
                    Err(e) => {
                        tracing::warn!("MQTT事件循环错误: {}", e);
                        tokio::time::sleep(Duration::from_secs(5)).await;
                    }
                }
            }
        });

        Self {
            client: Some(client),
            topic,
            enabled: true,
        }
    }

    pub async fn publish_alert(&self, alert: &AlertEvent) -> Result<bool> {
        if !self.enabled || self.client.is_none() {
            tracing::warn!("MQTT未启用，跳过告警推送");
            return Ok(false);
        }

        let client = self.client.as_ref().unwrap();

        let payload = serde_json::json!({
            "alert_id": alert.id.to_string(),
            "timestamp": alert.timestamp.to_rfc3339(),
            "device_id": alert.device_id,
            "alert_type": alert.alert_type,
            "alert_level": alert.alert_level,
            "pointing_deviation": alert.pointing_deviation,
            "threshold": alert.threshold,
            "message": alert.message,
            "sensor_data_id": alert.sensor_data_id.to_string(),
        });

        let topic = format!("{}/{}", self.topic, alert.alert_type.to_lowercase());

        match client
            .publish(
                topic.clone(),
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&payload)?.as_bytes(),
            )
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "告警已通过MQTT推送 - 设备: {}, 偏差: {:.2}°, 主题: {}",
                    alert.device_id,
                    alert.pointing_deviation,
                    topic
                );
                Ok(true)
            }
            Err(e) => {
                tracing::error!("MQTT告警推送失败: {}", e);
                Err(AppError::InternalError(format!("MQTT推送失败: {}", e)))
            }
        }
    }

    pub async fn publish_custom_alert(
        &self,
        device_id: &str,
        alert_type: &str,
        alert_level: &str,
        message: &str,
        extra_data: Option<serde_json::Value>,
    ) -> Result<bool> {
        if !self.enabled || self.client.is_none() {
            return Ok(false);
        }

        let client = self.client.as_ref().unwrap();

        let mut payload = serde_json::json!({
            "alert_id": Uuid::new_v4().to_string(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "device_id": device_id,
            "alert_type": alert_type,
            "alert_level": alert_level,
            "message": message,
        });

        if let Some(extra) = extra_data {
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("extra_data".to_string(), extra);
            }
        }

        let topic = format!("{}/{}", self.topic, alert_type.to_lowercase());

        match client
            .publish(
                topic.clone(),
                QoS::AtLeastOnce,
                false,
                serde_json::to_string(&payload)?.as_bytes(),
            )
            .await
        {
            Ok(_) => {
                tracing::info!(
                    "自定义告警已通过MQTT推送 - 设备: {}, 级别: {}, 主题: {}",
                    device_id,
                    alert_level,
                    topic
                );
                Ok(true)
            }
            Err(e) => {
                tracing::error!("MQTT自定义告警推送失败: {}", e);
                Err(AppError::InternalError(format!("MQTT状态更新推送失败: {}", e)))
            }
        }
    }

    pub async fn publish_status_update(
        &self,
        device_id: &str,
        status: &str,
        data: serde_json::Value,
    ) -> Result<bool> {
        if !self.enabled || self.client.is_none() {
            return Ok(false);
        }

        let client = self.client.as_ref().unwrap();

        let payload = serde_json::json!({
            "device_id": device_id,
            "status": status,
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "data": data,
        });

        let topic = format!("{}/status/{}", self.topic, device_id);

        match client
            .publish(
                topic.clone(),
                QoS::AtMostOnce,
                false,
                serde_json::to_string(&payload)?.as_bytes(),
            )
            .await
        {
            Ok(_) => Ok(true),
            Err(e) => {
                tracing::warn!("MQTT状态更新推送失败: {}", e);
                Err(AppError::InternalError(format!("MQTT状态更新推送失败: {}", e)))
            }
        }
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }
}
