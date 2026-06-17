use crate::errors::Result;
use crate::models::{
    GeomagneticFieldData, PointingSimulationParams, PointingSimulationResult,
    SinanSensorData, VectorFieldRequest, VectorFieldResponse,
};
use tokio::sync::oneshot;

#[derive(Debug)]
pub enum DtuCommand {
    ReceiveSensor(SinanSensorData),
    GetCachedData(oneshot::Sender<Vec<SinanSensorData>>),
}

#[derive(Debug)]
pub enum SimulatorCommand {
    AnalyzeSensor(SinanSensorData),
    RunSimulation(PointingSimulationParams, oneshot::Sender<Result<PointingSimulationResult>>),
}

#[derive(Debug)]
pub enum GeomagneticCommand {
    CalculateField {
        lat: f64,
        lon: f64,
        year: f64,
        altitude_km: Option<f64>,
        reply: oneshot::Sender<Result<GeomagneticFieldData>>,
    },
    GenerateVectorField {
        request: VectorFieldRequest,
        reply: oneshot::Sender<Result<VectorFieldResponse>>,
    },
    CalculateSecularVariation {
        lat: f64,
        lon: f64,
        year: f64,
        reply: oneshot::Sender<Result<(f64, f64, f64)>>,
    },
    GetFieldVector {
        lat: f64,
        lon: f64,
        year: f64,
        reply: oneshot::Sender<Result<nalgebra::Vector3<f64>>>,
    },
}

#[derive(Debug)]
pub enum AlarmCommand {
    EvaluateDeviation(SinanSensorData),
    CheckPendingAlerts,
    AcknowledgeAlert(uuid::Uuid, oneshot::Sender<Result<()>>),
}

#[derive(Debug, Clone)]
pub struct SensorAnalyzedData {
    pub sensor_data: SinanSensorData,
    pub simulated_azimuth: f64,
    pub pointing_accuracy: f64,
    pub model_parameters: String,
}

pub struct ChannelHub {
    dtu_tx: tokio::sync::mpsc::Sender<DtuCommand>,
    dtu_rx: tokio::sync::mpsc::Receiver<DtuCommand>,
    sim_tx: tokio::sync::mpsc::Sender<SimulatorCommand>,
    sim_rx: tokio::sync::mpsc::Receiver<SimulatorCommand>,
    geo_tx: tokio::sync::mpsc::Sender<GeomagneticCommand>,
    geo_rx: tokio::sync::mpsc::Receiver<GeomagneticCommand>,
    alarm_tx: tokio::sync::mpsc::Sender<AlarmCommand>,
    alarm_rx: tokio::sync::mpsc::Receiver<AlarmCommand>,
    broadcast_tx: tokio::sync::broadcast::Sender<SinanSensorData>,
    broadcast_rx: tokio::sync::broadcast::Receiver<SinanSensorData>,
}

impl ChannelHub {
    pub fn new() -> Self {
        let (dtu_tx, dtu_rx) = tokio::sync::mpsc::channel(256);
        let (sim_tx, sim_rx) = tokio::sync::mpsc::channel(256);
        let (geo_tx, geo_rx) = tokio::sync::mpsc::channel(256);
        let (alarm_tx, alarm_rx) = tokio::sync::mpsc::channel(256);
        let (broadcast_tx, broadcast_rx) = tokio::sync::broadcast::channel(256);

        Self {
            dtu_tx,
            dtu_rx,
            sim_tx,
            sim_rx,
            geo_tx,
            geo_rx,
            alarm_tx,
            alarm_rx,
            broadcast_tx,
            broadcast_rx,
        }
    }

    pub fn split(self) -> (ChannelSenders, ChannelReceivers) {
        let senders = ChannelSenders {
            dtu_tx: self.dtu_tx,
            sim_tx: self.sim_tx,
            geo_tx: self.geo_tx,
            alarm_tx: self.alarm_tx,
            broadcast_tx: self.broadcast_tx,
        };
        let receivers = ChannelReceivers {
            dtu_rx: self.dtu_rx,
            sim_rx: self.sim_rx,
            geo_rx: self.geo_rx,
            alarm_rx: self.alarm_rx,
            broadcast_rx: self.broadcast_rx,
        };
        (senders, receivers)
    }
}

#[derive(Clone)]
pub struct ChannelSenders {
    pub dtu_tx: tokio::sync::mpsc::Sender<DtuCommand>,
    pub sim_tx: tokio::sync::mpsc::Sender<SimulatorCommand>,
    pub geo_tx: tokio::sync::mpsc::Sender<GeomagneticCommand>,
    pub alarm_tx: tokio::sync::mpsc::Sender<AlarmCommand>,
    pub broadcast_tx: tokio::sync::broadcast::Sender<SinanSensorData>,
}

pub struct ChannelReceivers {
    pub dtu_rx: tokio::sync::mpsc::Receiver<DtuCommand>,
    pub sim_rx: tokio::sync::mpsc::Receiver<SimulatorCommand>,
    pub geo_rx: tokio::sync::mpsc::Receiver<GeomagneticCommand>,
    pub alarm_rx: tokio::sync::mpsc::Receiver<AlarmCommand>,
    pub broadcast_rx: tokio::sync::broadcast::Receiver<SinanSensorData>,
}
