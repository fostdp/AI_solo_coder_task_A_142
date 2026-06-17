use crate::channels::GeomagneticCommand;
use crate::cals10k_model::CALS10KModel;
use crate::database::Database;
use parking_lot::RwLock;
use std::sync::Arc;
use tokio::sync::mpsc;

pub struct GeomagneticReconstructor {
    rx: mpsc::Receiver<GeomagneticCommand>,
    model: Arc<RwLock<CALS10KModel>>,
    db: Database,
}

impl GeomagneticReconstructor {
    pub fn new(
        rx: mpsc::Receiver<GeomagneticCommand>,
        model: Arc<RwLock<CALS10KModel>>,
        db: Database,
    ) -> Self {
        Self { rx, model, db }
    }

    pub async fn run(&mut self) {
        tracing::info!("地磁场重建Actor启动");
        while let Some(cmd) = self.rx.recv().await {
            match cmd {
                GeomagneticCommand::CalculateField {
                    lat,
                    lon,
                    year,
                    altitude_km,
                    reply,
                } => {
                    let result = {
                        let model = self.model.read();
                        model.calculate_field_at_point(lat, lon, year, altitude_km)
                    };
                    match result {
                        Ok(field_data) => {
                            if let Err(e) = self.db.insert_geomagnetic_data(&field_data).await {
                                tracing::error!("存储地磁数据失败: {}", e);
                            }
                            let _ = reply.send(Ok(field_data));
                        }
                        Err(e) => {
                            tracing::error!("CalculateField失败: {}", e);
                            let _ = reply.send(Err(e));
                        }
                    }
                }
                GeomagneticCommand::GenerateVectorField { request, reply } => {
                    let result = {
                        let model = self.model.read();
                        model.generate_vector_field(&request)
                    };
                    match result {
                        Ok(response) => {
                            let _ = reply.send(Ok(response));
                        }
                        Err(e) => {
                            tracing::error!("GenerateVectorField失败: {}", e);
                            let _ = reply.send(Err(e));
                        }
                    }
                }
                GeomagneticCommand::CalculateSecularVariation {
                    lat,
                    lon,
                    year,
                    reply,
                } => {
                    let result = {
                        let model = self.model.read();
                        model.calculate_secular_variation(lat, lon, year)
                    };
                    match result {
                        Ok(variation) => {
                            let _ = reply.send(Ok(variation));
                        }
                        Err(e) => {
                            tracing::error!("CalculateSecularVariation失败: {}", e);
                            let _ = reply.send(Err(e));
                        }
                    }
                }
                GeomagneticCommand::GetFieldVector {
                    lat,
                    lon,
                    year,
                    reply,
                } => {
                    let result = {
                        let model = self.model.read();
                        model.get_field_vector(lat, lon, year)
                    };
                    match result {
                        Ok(vector) => {
                            let _ = reply.send(Ok(vector));
                        }
                        Err(e) => {
                            tracing::error!("GetFieldVector失败: {}", e);
                            let _ = reply.send(Err(e));
                        }
                    }
                }
            }
        }
        tracing::warn!("地磁场重建Actor通道关闭");
    }
}
