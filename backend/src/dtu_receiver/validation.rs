use crate::errors::{AppError, Result};
use crate::models::SinanSensorData;

pub fn validate_sensor_data(data: &SinanSensorData) -> Result<()> {
    if data.device_id.is_empty() {
        return Err(AppError::InvalidParameter("设备ID不能为空".to_string()));
    }
    if data.magnetic_moment_magnitude < 0.001 || data.magnetic_moment_magnitude > 100.0 {
        return Err(AppError::InvalidParameter(format!(
            "磁矩幅值超出范围: {}", data.magnetic_moment_magnitude
        )));
    }
    if data.remanence < 0.01 || data.remanence > 2.0 {
        return Err(AppError::InvalidParameter(format!(
            "剩磁强度超出范围: {}", data.remanence
        )));
    }
    if data.environment_temp < -50.0 || data.environment_temp > 150.0 {
        return Err(AppError::InvalidParameter(format!(
            "温度超出范围: {}", data.environment_temp
        )));
    }
    if data.location_lat < -90.0 || data.location_lat > 90.0 {
        return Err(AppError::InvalidParameter("纬度超出范围".to_string()));
    }
    if data.location_lon < -180.0 || data.location_lon > 180.0 {
        return Err(AppError::InvalidParameter("经度超出范围".to_string()));
    }
    if data.pointing_deviation < 0.0 || data.pointing_deviation > 360.0 {
        return Err(AppError::InvalidParameter(format!(
            "指向偏差超出范围: {}", data.pointing_deviation
        )));
    }
    Ok(())
}

pub fn enrich_sensor_data(data: &mut SinanSensorData) {
    let mxy = (data.magnetic_moment_x.powi(2) + data.magnetic_moment_y.powi(2)).sqrt();
    if mxy < 1e-10 && data.magnetic_moment_magnitude > 1e-10 {
        data.magnetic_moment_x = data.magnetic_moment_magnitude;
    }
    if data.magnetic_moment_magnitude < 1e-10 {
        let mx = data.magnetic_moment_x;
        let my = data.magnetic_moment_y;
        let mz = data.magnetic_moment_z;
        data.magnetic_moment_magnitude = (mx*mx + my*my + mz*mz).sqrt();
    }
}
