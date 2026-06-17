use crate::channels::{AlarmCommand, ChannelSenders, SimulatorCommand};
use crate::cals10k_model::CALS10KModel;
use crate::database::Database;
use crate::micromagnetic_simulation::MicromagneticSimulator;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct MagneticSimulatorActor {
    rx: mpsc::Receiver<SimulatorCommand>,
    senders: ChannelSenders,
    db: Database,
    simulator: Arc<MicromagneticSimulator>,
    geomagnetic_model: Arc<RwLock<CALS10KModel>>,
}

impl MagneticSimulatorActor {
    pub fn new(
        rx: mpsc::Receiver<SimulatorCommand>,
        senders: ChannelSenders,
        db: Database,
        simulator: Arc<MicromagneticSimulator>,
        geomagnetic_model: Arc<RwLock<CALS10KModel>>,
    ) -> Self {
        Self {
            rx,
            senders,
            db,
            simulator,
            geomagnetic_model,
        }
    }

    pub async fn run(&mut self) {
        tracing::info!("微磁学仿真Actor启动");
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                SimulatorCommand::AnalyzeSensor(mut data) => {
                    let geo_field = {
                        let model = self.geomagnetic_model.read();
                        match model.get_field_vector(
                            data.location_lat,
                            data.location_lon,
                            -100.0,
                        ) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!("获取地磁场矢量失败: {}", e);
                                continue;
                            }
                        }
                    };

                    let moment_vec = self.simulator.calculate_magnetic_moment_from_sensor(
                        data.magnetic_moment_x,
                        data.magnetic_moment_y,
                        data.magnetic_moment_z,
                    );

                    match self.simulator.calculate_pointing_deviation(moment_vec, geo_field) {
                        Ok(deviation) => {
                            data.pointing_deviation = deviation;
                            data.is_alert = deviation > 5.0;

                            tracing::info!(
                                "传感器分析完成, 设备: {}, 偏差: {:.4}°, 告警: {}",
                                data.device_id,
                                deviation,
                                data.is_alert,
                            );

                            if let Err(e) = self.senders.alarm_tx.send(AlarmCommand::EvaluateDeviation(data)).await {
                                tracing::error!("转发数据到告警服务失败: {}", e);
                            }
                        }
                        Err(e) => {
                            tracing::error!("指向偏差计算失败: {}", e);
                        }
                    }
                }
                SimulatorCommand::RunSimulation(params, reply) => {
                    let geo_field = {
                        let model = self.geomagnetic_model.read();
                        match model.get_field_vector(
                            params.location_lat,
                            params.location_lon,
                            params.target_year,
                        ) {
                            Ok(v) => v,
                            Err(e) => {
                                tracing::error!("获取地磁场矢量失败: {}", e);
                                let _ = reply.send(Err(e));
                                continue;
                            }
                        }
                    };

                    match self.simulator.simulate_pointing(&params, geo_field) {
                        Ok(result) => {
                            if let Err(e) = self.db.insert_simulation_result(&result).await {
                                tracing::error!("存储仿真结果失败: {}", e);
                            }
                            tracing::info!(
                                "仿真完成, id: {}, 模拟方位: {:.4}°, 精度: {:.4}°",
                                result.simulation_id,
                                result.simulated_azimuth,
                                result.pointing_accuracy,
                            );
                            let _ = reply.send(Ok(result));
                        }
                        Err(e) => {
                            tracing::error!("仿真失败: {}", e);
                            let _ = reply.send(Err(e));
                        }
                    }
                }
            }
        }
        tracing::warn!("微磁学仿真Actor通道关闭");
    }
}
