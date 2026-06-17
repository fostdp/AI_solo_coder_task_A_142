use metrics::{counter, describe_counter, describe_gauge, describe_histogram, gauge, histogram};
use metrics_exporter_prometheus::{Matcher, PrometheusBuilder, PrometheusHandle};
use std::time::Instant;

const LATENCY_HISTOGRAM_BUCKETS: &[f64] = &[
    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
];

pub fn init_metrics() -> PrometheusHandle {
    let exporter = PrometheusBuilder::new()
        .set_buckets_for_metric(
            Matcher::Suffix("latency_seconds".to_string()),
            LATENCY_HISTOGRAM_BUCKETS,
        )
        .expect("Failed to configure metrics buckets")
        .install_recorder()
        .expect("Failed to install metrics recorder");

    describe_counter!(
        "dtu_sensor_data_received_total",
        "Total number of sensor data packets received by DTU receiver"
    );
    describe_counter!(
        "dtu_sensor_data_valid_total",
        "Total number of valid sensor data packets"
    );
    describe_counter!(
        "dtu_sensor_data_invalid_total",
        "Total number of invalid sensor data packets"
    );
    describe_counter!(
        "simulation_run_total",
        "Total number of magnetic pointing simulations run"
    );
    describe_counter!(
        "geomagnetic_calculation_total",
        "Total number of geomagnetic field calculations"
    );
    describe_counter!(
        "alerts_generated_total",
        "Total number of alerts generated"
    );
    describe_counter!(
        "alerts_mqtt_published_total",
        "Total number of alerts published via MQTT"
    );
    describe_counter!(
        "alerts_mqtt_failed_total",
        "Total number of failed MQTT alert publications"
    );
    describe_counter!(
        "clickhouse_insert_total",
        "Total number of ClickHouse insert operations"
    );
    describe_counter!(
        "clickhouse_insert_failed_total",
        "Total number of failed ClickHouse insert operations"
    );
    describe_counter!(
        "http_requests_total",
        "Total number of HTTP requests"
    );

    describe_gauge!(
        "dtu_connected_devices",
        "Number of currently connected DTU devices"
    );
    describe_gauge!(
        "alerts_active",
        "Number of active, unacknowledged alerts"
    );
    describe_gauge!(
        "simulation_pointing_accuracy_degrees",
        "Current pointing accuracy in degrees"
    );

    describe_histogram!(
        "dtu_processing_latency_seconds",
        "Latency of DTU sensor data processing in seconds"
    );
    describe_histogram!(
        "simulation_latency_seconds",
        "Latency of magnetic simulation in seconds"
    );
    describe_histogram!(
        "geomagnetic_calculation_latency_seconds",
        "Latency of geomagnetic field calculation in seconds"
    );
    describe_histogram!(
        "http_request_latency_seconds",
        "Latency of HTTP requests in seconds"
    );

    exporter
}

pub fn inc_dtu_received() {
    counter!("dtu_sensor_data_received_total").increment(1);
}

pub fn inc_dtu_valid() {
    counter!("dtu_sensor_data_valid_total").increment(1);
}

pub fn inc_dtu_invalid() {
    counter!("dtu_sensor_data_invalid_total").increment(1);
}

pub fn inc_simulation_run() {
    counter!("simulation_run_total").increment(1);
}

pub fn inc_geomagnetic_calc() {
    counter!("geomagnetic_calculation_total").increment(1);
}

pub fn inc_alert_generated() {
    counter!("alerts_generated_total").increment(1);
}

pub fn inc_alert_mqtt_published() {
    counter!("alerts_mqtt_published_total").increment(1);
}

pub fn inc_alert_mqtt_failed() {
    counter!("alerts_mqtt_failed_total").increment(1);
}

pub fn inc_clickhouse_insert() {
    counter!("clickhouse_insert_total").increment(1);
}

pub fn inc_clickhouse_insert_failed() {
    counter!("clickhouse_insert_failed_total").increment(1);
}

pub fn inc_http_request(method: &'static str, endpoint: &'static str, status: &'static str) {
    counter!(
        "http_requests_total",
        "method" => method,
        "endpoint" => endpoint,
        "status" => status
    )
    .increment(1);
}

pub fn set_connected_devices(count: usize) {
    gauge!("dtu_connected_devices").set(count as f64);
}

pub fn set_active_alerts(count: usize) {
    gauge!("alerts_active").set(count as f64);
}

pub fn set_pointing_accuracy(degrees: f64) {
    gauge!("simulation_pointing_accuracy_degrees").set(degrees);
}

pub fn record_dtu_latency(duration: std::time::Duration) {
    histogram!("dtu_processing_latency_seconds").record(duration.as_secs_f64());
}

pub fn record_simulation_latency(duration: std::time::Duration) {
    histogram!("simulation_latency_seconds").record(duration.as_secs_f64());
}

pub fn record_geomagnetic_calc_latency(duration: std::time::Duration) {
    histogram!("geomagnetic_calculation_latency_seconds").record(duration.as_secs_f64());
}

pub fn record_http_latency(method: &'static str, endpoint: &'static str, duration: std::time::Duration) {
    histogram!(
        "http_request_latency_seconds",
        "method" => method,
        "endpoint" => endpoint
    )
    .record(duration.as_secs_f64());
}

pub struct HttpRequestTimer {
    start: Instant,
    method: &'static str,
    endpoint: &'static str,
}

impl HttpRequestTimer {
    pub fn new(method: &'static str, endpoint: &'static str) -> Self {
        Self {
            start: Instant::now(),
            method,
            endpoint,
        }
    }

    pub fn finish(self, status: &'static str) {
        let duration = self.start.elapsed();
        record_http_latency(self.method, self.endpoint, duration);
        inc_http_request(self.method, self.endpoint, status);
    }
}
