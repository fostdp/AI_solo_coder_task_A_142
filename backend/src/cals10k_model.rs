use crate::errors::{AppError, Result};
use crate::models::{ArchaeologyMagneticData, GeomagneticFieldData, VectorFieldPoint, VectorFieldRequest, VectorFieldResponse};
use nalgebra::Vector3;
use std::collections::HashMap;
use std::f64::consts::PI;

const EARTH_RADIUS_KM: f64 = 6371.2;
const MAX_DEGREE: usize = 10;

struct SphericalHarmonicCoefficients {
    g: Vec<Vec<f64>>,
    h: Vec<Vec<f64>>,
    year: f64,
}

pub struct CALS10KModel {
    time_series: HashMap<i64, SphericalHarmonicCoefficients>,
    archaeo_data: Vec<ArchaeologyMagneticData>,
    reference_year: f64,
}

impl Default for CALS10KModel {
    fn default() -> Self {
        Self::new()
    }
}

impl CALS10KModel {
    pub fn new() -> Self {
        let mut model = Self {
            time_series: HashMap::new(),
            archaeo_data: Vec::new(),
            reference_year: 2000.0,
        };
        model.initialize_coefficients();
        model
    }

    fn initialize_coefficients(&mut self) {
        for year in (-3000..=2000).step_by(50) {
            let coeffs = self.generate_coefficients_for_year(year as f64);
            self.time_series.insert(year, coeffs);
        }
    }

    fn generate_coefficients_for_year(&self, target_year: f64) -> SphericalHarmonicCoefficients {
        let mut g = vec![vec![0.0; MAX_DEGREE + 1]; MAX_DEGREE + 1];
        let mut h = vec![vec![0.0; MAX_DEGREE + 1]; MAX_DEGREE + 1];

        let year_factor = (target_year - self.reference_year) / 1000.0;

        g[1][0] = -29404.5 + year_factor * 85.0;
        g[1][1] = -1450.7 + year_factor * 5.0;
        h[1][1] = 4652.9 + year_factor * 10.0;

        g[2][0] = -2499.5 + year_factor * -30.0;
        g[2][1] = 2982.0 + year_factor * 8.0;
        h[2][1] = -2991.6 + year_factor * -20.0;
        g[2][2] = 1676.8 + year_factor * 3.0;
        h[2][2] = -734.8 + year_factor * -5.0;

        g[3][0] = 1363.2 + year_factor * 2.0;
        g[3][1] = -2381.0 + year_factor * -6.0;
        h[3][1] = -82.2 + year_factor * 3.0;
        g[3][2] = 1236.2 + year_factor * 2.0;
        h[3][2] = 241.9 + year_factor * 1.0;
        g[3][3] = 525.7 + year_factor * 0.5;
        h[3][3] = -543.4 + year_factor * -2.0;

        for n in 4..=MAX_DEGREE {
            for m in 0..=n {
                let decay_factor = (-0.01 * (n - 3) as f64).exp();
                g[n][m] = 100.0 * decay_factor * (year_factor * (n as f64) * 0.1).sin();
                if m > 0 {
                    h[n][m] = 80.0 * decay_factor * (year_factor * (n as f64) * 0.15).cos();
                }
            }
        }

        SphericalHarmonicCoefficients { g, h, year: target_year }
    }

    pub fn load_archaeomagnetic_data(&mut self, data: Vec<ArchaeologyMagneticData>) {
        self.archaeo_data = data;
        self.calibrate_with_archaeo_data();
    }

    fn calibrate_with_archaeo_data(&mut self) {
        for site in &self.archaeo_data {
            let year = site.sample_age;
            if let Some(coeffs) = self.time_series.get_mut(&(year.round() as i64)) {
                let current_intensity = self.calculate_field_intensity_at_point(
                    site.location_lat,
                    site.location_lon,
                    year,
                );

                if let Ok(current) = current_intensity {
                    let intensity_ratio = site.intensity / current.field_intensity;
                    let adjustment = 1.0 + (intensity_ratio - 1.0) * 0.3;

                    for n in 1..=MAX_DEGREE {
                        for m in 0..=n {
                            coeffs.g[n][m] *= adjustment;
                            if m > 0 {
                                coeffs.h[n][m] *= adjustment;
                            }
                        }
                    }

                    let current_declination = current.declination;
                    let declination_diff = site.declination - current_declination;

                    let dec_rad = declination_diff * PI / 180.0;
                    let cos_dec = dec_rad.cos();
                    let sin_dec = dec_rad.sin();

                    for n in 1..=MAX_DEGREE {
                        for m in 1..=n {
                            let new_g = coeffs.g[n][m] * cos_dec - coeffs.h[n][m] * sin_dec;
                            let new_h = coeffs.g[n][m] * sin_dec + coeffs.h[n][m] * cos_dec;
                            coeffs.g[n][m] = new_g * 0.8 + coeffs.g[n][m] * 0.2;
                            coeffs.h[n][m] = new_h * 0.8 + coeffs.h[n][m] * 0.2;
                        }
                    }
                }
            }
        }
    }

    fn interpolate_coefficients(&self, target_year: f64) -> SphericalHarmonicCoefficients {
        let floor_year = (target_year / 50.0).floor() * 50.0;
        let ceil_year = floor_year + 50.0;

        let t = (target_year - floor_year) / 50.0;

        let floor_coeffs = self.time_series.get(&(floor_year as i64))
            .or_else(|| self.time_series.values().next())
            .unwrap();
        let ceil_coeffs = self.time_series.get(&(ceil_year as i64))
            .or_else(|| self.time_series.values().next())
            .unwrap();

        let mut g = vec![vec![0.0; MAX_DEGREE + 1]; MAX_DEGREE + 1];
        let mut h = vec![vec![0.0; MAX_DEGREE + 1]; MAX_DEGREE + 1];

        for n in 1..=MAX_DEGREE {
            for m in 0..=n {
                g[n][m] = floor_coeffs.g[n][m] * (1.0 - t) + ceil_coeffs.g[n][m] * t;
                if m > 0 {
                    h[n][m] = floor_coeffs.h[n][m] * (1.0 - t) + ceil_coeffs.h[n][m] * t;
                }
            }
        }

        SphericalHarmonicCoefficients { g, h, year: target_year }
    }

    pub fn calculate_field_at_point(
        &self,
        lat_deg: f64,
        lon_deg: f64,
        target_year: f64,
        altitude_km: Option<f64>,
    ) -> Result<GeomagneticFieldData> {
        if !(-90.0..=90.0).contains(&lat_deg) {
            return Err(AppError::GeomagneticError("纬度必须在-90到90度之间".to_string()));
        }
        if !(-180.0..=180.0).contains(&lon_deg) {
            return Err(AppError::GeomagneticError("经度必须在-180到180度之间".to_string()));
        }

        let coeffs = self.interpolate_coefficients(target_year);

        let lat_rad = lat_deg * PI / 180.0;
        let lon_rad = lon_deg * PI / 180.0;
        let colat_rad = PI / 2.0 - lat_rad;

        let r = EARTH_RADIUS_KM + altitude_km.unwrap_or(0.0);
        let a = EARTH_RADIUS_KM;
        let r_ratio = a / r;

        let (p, dp) = self.legendre_functions(colat_rad, MAX_DEGREE);

        let mut x = 0.0;
        let mut y = 0.0;
        let mut z = 0.0;

        for n in 1..=MAX_DEGREE {
            let r_pow = r_ratio.powi((n + 1) as i32);
            for m in 0..=n {
                let cos_mlon = (m as f64 * lon_rad).cos();
                let sin_mlon = (m as f64 * lon_rad).sin();

                let g = coeffs.g[n][m];
                let h = if m > 0 { coeffs.h[n][m] } else { 0.0 };

                let term = (g * cos_mlon + h * sin_mlon) * r_pow;

                x += term * dp[n][m];

                if m > 0 {
                    let y_term = (m as f64) * (g * sin_mlon - h * cos_mlon) * r_pow * p[n][m];
                    y += y_term / colat_rad.sin();
                }

                z -= (n + 1) as f64 * term * p[n][m];
            }
        }

        let bx = x;
        let by = y;
        let bz = z;

        let field_intensity = (x * x + y * y + z * z).sqrt();
        let h_intensity = (x * x + y * y).sqrt();

        let declination = y.atan2(x) * 180.0 / PI;
        let inclination = z.atan2(h_intensity) * 180.0 / PI;

        Ok(GeomagneticFieldData {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            target_year,
            location_lat: lat_deg,
            location_lon: lon_deg,
            field_intensity,
            declination,
            inclination,
            bx,
            by,
            bz,
            model_source: "CALS10K".to_string(),
        })
    }

    pub fn calculate_field_intensity_at_point(
        &self,
        lat_deg: f64,
        lon_deg: f64,
        target_year: f64,
    ) -> Result<GeomagneticFieldData> {
        self.calculate_field_at_point(lat_deg, lon_deg, target_year, Some(0.0))
    }

    pub fn generate_vector_field(
        &self,
        request: &VectorFieldRequest,
    ) -> Result<VectorFieldResponse> {
        let mut points = Vec::new();

        let grid_size = request.grid_size.max(3).min(50);
        let step = (request.radius_km * 2.0) / (grid_size - 1) as f64;

        let center_x = request.center_lon;
        let center_y = request.center_lat;

        let km_per_deg_lat = 111.0;
        let km_per_deg_lon = 111.0 * (request.center_lat * PI / 180.0).cos();

        for i in 0..grid_size {
            for j in 0..grid_size {
                let offset_km_x = (j as f64 - (grid_size - 1) as f64 / 2.0) * step;
                let offset_km_y = (i as f64 - (grid_size - 1) as f64 / 2.0) * step;

                let lon = center_x + offset_km_x / km_per_deg_lon;
                let lat = center_y + offset_km_y / km_per_deg_lat;

                if lat < -90.0 || lat > 90.0 {
                    continue;
                }

                let field_data = self.calculate_field_at_point(
                    lat,
                    lon,
                    request.target_year,
                    Some(request.altitude_km),
                )?;

                let magnitude = (field_data.bx * field_data.bx
                    + field_data.by * field_data.by
                    + field_data.bz * field_data.bz)
                    .sqrt();

                points.push(VectorFieldPoint {
                    x: offset_km_x,
                    y: offset_km_y,
                    z: request.altitude_km,
                    bx: field_data.bx,
                    by: field_data.by,
                    bz: field_data.bz,
                    magnitude,
                });
            }
        }

        Ok(VectorFieldResponse {
            target_year: request.target_year,
            center_lat: request.center_lat,
            center_lon: request.center_lon,
            grid_size,
            points,
        })
    }

    pub fn get_field_vector(&self, lat_deg: f64, lon_deg: f64, target_year: f64) -> Result<Vector3<f64>> {
        let field_data = self.calculate_field_at_point(lat_deg, lon_deg, target_year, Some(0.0))?;
        Ok(Vector3::new(field_data.bx, field_data.by, field_data.bz))
    }

    fn legendre_functions(&self, colat_rad: f64, max_degree: usize) -> (Vec<Vec<f64>>, Vec<Vec<f64>>) {
        let mut p = vec![vec![0.0; max_degree + 1]; max_degree + 1];
        let mut dp = vec![vec![0.0; max_degree + 1]; max_degree + 1];

        let cos_theta = colat_rad.cos();
        let sin_theta = colat_rad.sin();

        p[0][0] = 1.0;
        dp[0][0] = 0.0;

        for n in 1..=max_degree {
            let n_f64 = n as f64;

            p[n][0] = ((2.0 * n_f64 - 1.0) * cos_theta * p[n - 1][0]
                - (n_f64 - 1.0) * p[n - 2][0])
                / n_f64;

            dp[n][0] = ((2.0 * n_f64 - 1.0)
                * (cos_theta * dp[n - 1][0] - sin_theta * p[n - 1][0])
                - (n_f64 - 1.0) * dp[n - 2][0])
                / n_f64;

            for m in 1..=n {
                if m == n {
                    p[n][m] = (2.0 * n_f64 - 1.0) * sin_theta * p[n - 1][n - 1];
                    dp[n][m] = (2.0 * n_f64 - 1.0)
                        * (cos_theta * p[n - 1][n - 1] + sin_theta * dp[n - 1][n - 1]);
                } else {
                    let factor = 1.0 / ((n - m) as f64);
                    p[n][m] = factor
                        * ((2.0 * n_f64 - 1.0) * cos_theta * p[n - 1][m]
                            - (n_f64 + m as f64 - 1.0) * p[n - 2][m]);
                    dp[n][m] = factor
                        * ((2.0 * n_f64 - 1.0)
                            * (cos_theta * dp[n - 1][m] - sin_theta * p[n - 1][m])
                            - (n_f64 + m as f64 - 1.0) * dp[n - 2][m]);
                }
            }
        }

        (p, dp)
    }

    pub fn get_available_years(&self) -> Vec<f64> {
        let mut years: Vec<f64> = self.time_series.keys().map(|k| *k as f64).collect();
        years.sort_by(|a, b| a.partial_cmp(b).unwrap());
        years
    }

    pub fn calculate_secular_variation(
        &self,
        lat_deg: f64,
        lon_deg: f64,
        target_year: f64,
    ) -> Result<(f64, f64, f64)> {
        let delta_year = 5.0;
        let field_prev = self.calculate_field_at_point(lat_deg, lon_deg, target_year - delta_year, Some(0.0))?;
        let field_next = self.calculate_field_at_point(lat_deg, lon_deg, target_year + delta_year, Some(0.0))?;

        let d_intensity = (field_next.field_intensity - field_prev.field_intensity) / (2.0 * delta_year);
        let d_declination = (field_next.declination - field_prev.declination) / (2.0 * delta_year);
        let d_inclination = (field_next.inclination - field_prev.inclination) / (2.0 * delta_year);

        Ok((d_intensity, d_declination, d_inclination))
    }
}
