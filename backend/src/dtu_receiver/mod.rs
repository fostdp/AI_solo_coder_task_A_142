use crate::channels::{ChannelSenders, DtuCommand};
use crate::database::Database;
use crate::errors::Result;
use crate::models::SinanSensorData;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;

pub mod validation;

pub struct DtuReceiver {
    rx: mpsc::Receiver<DtuCommand>,
    senders: ChannelSenders,
    db: Database,
    sensor_cache: Arc<RwLock<HashMap<String, SinanSensorData>>>,
}

impl DtuReceiver {
    pub fn new(
        rx: mpsc::Receiver<DtuCommand>,
        senders: ChannelSenders,
        db: Database,
        sensor_cache: Arc<RwLock<HashMap<String, SinanSensorData>>>,
    ) -> Self {
        Self { rx, senders, db, sensor_cache }
    }

    pub async fn run(&mut self) {
        tracing::info!("DTU接收器启动");
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                DtuCommand::ReceiveSensor(mut data) => {
                    match self.process_sensor_data(&mut data).await {
                        Ok(()) => {}
                        Err(e) => {
                            tracing::error!("DTU处理传感器数据失败: {}", e);
                        }
                    }
                }
                DtuCommand::GetCachedData(reply) => {
                    let data: Vec<_> = self.sensor_cache.read().values().cloned().collect();
                    let _ = reply.send(data);
                }
            }
        }
        tracing::warn!("DTU接收器通道关闭");
    }

    async fn process_sensor_data(&mut self, data: &mut SinanSensorData) -> Result<()> {
        data.id = uuid::Uuid::new_v4();
        data.timestamp = chrono::Utc::now();

        if let Err(e) = validation::validate_sensor_data(data) {
            tracing::warn!("传感器数据校验失败: {}", e);
            return Err(crate::errors::AppError::InvalidParameter(format!("数据校验失败: {}", e)));
        }

        validation::enrich_sensor_data(data);

        if let Err(e) = self.db.insert_sensor_data(data).await {
            tracing::error!("存储传感器数据失败: {}", e);
        }

        self.sensor_cache.write().insert(data.device_id.clone(), data.clone());

        let _ = self.senders.broadcast_tx.send(data.clone());

        if let Err(e) = self.senders.sim_tx.send(crate::channels::SimulatorCommand::AnalyzeSensor(data.clone())).await {
            tracing::warn!("转发数据到仿真器失败: {}", e);
        }

        Ok(())
    }
}
