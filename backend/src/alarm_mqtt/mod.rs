use crate::channels::AlarmCommand;
use crate::config::Config;
use crate::database::Database;
use crate::models::AlertEvent;
use crate::mqtt_service::MqttService;
use chrono::Utc;
use std::sync::Arc;
use tokio::sync::mpsc;
use uuid::Uuid;

pub struct AlarmMqttActor {
    rx: mpsc::Receiver<AlarmCommand>,
    db: Database,
    mqtt: Arc<MqttService>,
    config: Config,
    warning_threshold: f64,
    critical_threshold: f64,
}

impl AlarmMqttActor {
    pub fn new(
        rx: mpsc::Receiver<AlarmCommand>,
        db: Database,
        mqtt: Arc<MqttService>,
        config: Config,
    ) -> Self {
        let warning_threshold = config.pointing_deviation_threshold;
        let critical_threshold = config.critical_deviation_threshold;
        Self {
            rx,
            db,
            mqtt,
            config,
            warning_threshold,
            critical_threshold,
        }
    }

    pub async fn run(&mut self) {
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                AlarmCommand::EvaluateDeviation(data) => {
                    if data.pointing_deviation > self.warning_threshold {
                        let alert_level = if data.pointing_deviation > self.critical_threshold {
                            "CRITICAL"
                        } else {
                            "WARNING"
                        };

                        let alert = AlertEvent {
                            id: Uuid::new_v4(),
                            timestamp: Utc::now(),
                            device_id: data.device_id.clone(),
                            alert_type: "POINTING_DEVIATION".to_string(),
                            alert_level: alert_level.to_string(),
                            pointing_deviation: data.pointing_deviation,
                            threshold: self.warning_threshold,
                            sensor_data_id: data.id,
                            is_acknowledged: false,
                            message: format!(
                                "设备 {} 指向偏差超限: {:.2}° > {:.2}°",
                                data.device_id, data.pointing_deviation, self.warning_threshold
                            ),
                            mqtt_topic: self.config.mqtt_topic.clone(),
                            mqtt_published: false,
                        };

                        if let Err(e) = self.db.insert_alert_event(&alert).await {
                            tracing::error!("插入告警事件失败: {}", e);
                            continue;
                        }

                        match self.mqtt.publish_alert(&alert).await {
                            Ok(true) => {
                                if let Err(e) = self.db.mark_alert_mqtt_published(alert.id).await {
                                    tracing::error!("标记告警MQTT推送状态失败: {}", e);
                                }
                            }
                            Ok(false) => {}
                            Err(e) => {
                                tracing::error!("MQTT推送告警失败: {}", e);
                            }
                        }

                        tracing::warn!(
                            "指向偏差告警 - 设备: {}, 偏差: {:.2}°, 级别: {}",
                            alert.device_id,
                            alert.pointing_deviation,
                            alert_level
                        );
                    }
                }
                AlarmCommand::CheckPendingAlerts => {
                    match self.db.get_active_alerts(100).await {
                        Ok(alerts) => {
                            let mut republished = 0usize;
                            for alert in alerts {
                                if !alert.mqtt_published {
                                    match self.mqtt.publish_alert(&alert).await {
                                        Ok(true) => {
                                            if let Err(e) =
                                                self.db.mark_alert_mqtt_published(alert.id).await
                                            {
                                                tracing::error!(
                                                    "标记告警MQTT推送状态失败: {}",
                                                    e
                                                );
                                            }
                                            republished += 1;
                                        }
                                        Ok(false) => {}
                                        Err(e) => {
                                            tracing::error!("重发告警失败: {}", e);
                                        }
                                    }
                                }
                            }
                            tracing::info!("已重发 {} 条待推送告警", republished);
                        }
                        Err(e) => {
                            tracing::error!("获取活跃告警失败: {}", e);
                        }
                    }
                }
                AlarmCommand::AcknowledgeAlert(alert_id, reply) => {
                    let result = self.db.acknowledge_alert(alert_id).await;
                    let _ = reply.send(result);
                }
            }
        }

        tracing::warn!("AlarmMqttActor通道已关闭，退出事件循环");
    }
}
