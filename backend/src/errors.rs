use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("数据库错误: {0}")]
    DatabaseError(#[from] clickhouse::error::Error),

    #[error("MQTT错误: {0}")]
    MqttError(#[from] rumqttc::Error),

    #[error("序列化错误: {0}")]
    SerializationError(#[from] serde_json::Error),

    #[error("IO错误: {0}")]
    IoError(#[from] std::io::Error),

    #[error("配置错误: {0}")]
    ConfigError(String),

    #[error("无效参数: {0}")]
    InvalidParameter(String),

    #[error("仿真计算错误: {0}")]
    SimulationError(String),

    #[error("地磁场模型错误: {0}")]
    GeomagneticError(String),

    #[error("HTTP错误: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("未找到资源: {0}")]
    NotFound(String),

    #[error("内部错误: {0}")]
    InternalError(String),
}

impl axum::response::IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = match &self {
            AppError::InvalidParameter(_) => axum::http::StatusCode::BAD_REQUEST,
            AppError::NotFound(_) => axum::http::StatusCode::NOT_FOUND,
            _ => axum::http::StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = serde_json::json!({
            "error": self.to_string(),
            "code": status.as_u16()
        });

        (status, axum::Json(body)).into_response()
    }
}

pub type Result<T> = std::result::Result<T, AppError>;
