use serde::Deserialize;
use std::env;

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub clickhouse_host: String,
    pub clickhouse_port: u16,
    pub clickhouse_user: String,
    pub clickhouse_password: String,
    pub clickhouse_database: String,
    pub mqtt_host: String,
    pub mqtt_port: u16,
    pub mqtt_client_id: String,
    pub mqtt_topic: String,
    pub mqtt_username: String,
    pub mqtt_password: String,
    pub server_host: String,
    pub server_port: u16,
    pub pointing_deviation_threshold: f64,
    pub critical_deviation_threshold: f64,
}

impl Config {
    pub fn from_env() -> Result<Self, crate::errors::AppError> {
        dotenvy::dotenv().ok();

        Ok(Self {
            clickhouse_host: env::var("CLICKHOUSE_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            clickhouse_port: env::var("CLICKHOUSE_PORT")
                .unwrap_or_else(|_| "8123".to_string())
                .parse()
                .map_err(|e| crate::errors::AppError::ConfigError(format!("CLICKHOUSE_PORT 解析失败: {}", e))?,
            clickhouse_user: env::var("CLICKHOUSE_USER").unwrap_or_else(|_| "default".to_string()),
            clickhouse_password: env::var("CLICKHOUSE_PASSWORD").unwrap_or_default(),
            clickhouse_database: env::var("CLICKHOUSE_DATABASE").unwrap_or_else(|_| "sinan_db".to_string()),
            mqtt_host: env::var("MQTT_HOST").unwrap_or_else(|_| "127.0.0.1".to_string()),
            mqtt_port: env::var("MQTT_PORT")
                .unwrap_or_else(|_| "1883".to_string())
                .parse()
                .map_err(|e| crate::errors::AppError::ConfigError(format!("MQTT_PORT 解析失败: {}", e))?,
            mqtt_client_id: env::var("MQTT_CLIENT_ID").unwrap_or_else(|_| "sinan-backend".to_string()),
            mqtt_topic: env::var("MQTT_TOPIC").unwrap_or_else(|_| "sinan/alerts".to_string()),
            mqtt_username: env::var("MQTT_USERNAME").unwrap_or_default(),
            mqtt_password: env::var("MQTT_PASSWORD").unwrap_or_default(),
            server_host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            server_port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|e| crate::errors::AppError::ConfigError(format!("SERVER_PORT 解析失败: {}", e))?,
            pointing_deviation_threshold: env::var("POINTING_DEVIATION_THRESHOLD")
                .unwrap_or_else(|_| "5.0".to_string())
                .parse()
                .map_err(|e| crate::errors::AppError::ConfigError(format!("POINTING_DEVIATION_THRESHOLD 解析失败: {}", e))?,
            critical_deviation_threshold: env::var("CRITICAL_DEVIATION_THRESHOLD")
                .unwrap_or_else(|_| "10.0".to_string())
                .parse()
                .map_err(|e| crate::errors::AppError::ConfigError(format!("CRITICAL_DEVIATION_THRESHOLD 解析失败: {}", e))?,
        })
    }

    pub fn clickhouse_url(&self) -> String {
        if self.clickhouse_password.is_empty() {
            format!(
                "http://{}:{}/?database={}",
                self.clickhouse_host, self.clickhouse_port, self.clickhouse_database
            )
        } else {
            format!(
                "http://{}:{}@{}:{}/?database={}",
                self.clickhouse_user, self.clickhouse_password,
                self.clickhouse_host, self.clickhouse_port, self.clickhouse_database
            )
        }
    }
}
