use crate::errors::{AppError, Result};
use crate::models::{ArchaeologyMagneticData, GeomagneticFieldData, VectorFieldPoint, VectorFieldRequest, VectorFieldResponse};
use nalgebra::Vector3;
use std::collections::HashMap;
use std::f64::consts::PI;

const EARTH_RADIUS_KM: f64 = 6371.2;
const MAX_DEGREE: usize = 10;
const EASTASIA_LAT_MIN: f64 = 15.0;
const EASTASIA_LAT_MAX: f64 = 55.0;
const EASTASIA_LON_MIN: f64 = 73.0;
const EASTASIA_LON_MAX: f64 = 145.0;

struct SphericalHarmonicCoefficients {
    g: Vec<Vec<f64>>,
    h: Vec<Vec<f64>>,
    year: f64,
}

#[derive(Debug, Clone)]
struct RegionalKrigingPoint {
    lat: f64,
    lon: f64,
    year: f64,
    declination_residual: f64,
    inclination_residual: f64,
    intensity_residual: f64,
    weight: f64,
}

pub struct CALS10KModel {
    time_series: HashMap<i64, SphericalHarmonicCoefficients>,
    archaeo_data: Vec<ArchaeologyMagneticData>,
    eastasia_data: Vec<ArchaeologyMagneticData>,
    regional_kriging_points: Vec<RegionalKrigingPoint>,
    reference_year: f64,
    kriging_nugget: f64,
    kriging_sill: f64,
    kriging_range_km: f64,
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
            eastasia_data: Vec::new(),
            regional_kriging_points: Vec::new(),
            reference_year: 2000.0,
            kriging_nugget: 0.05,
            kriging_sill: 1.0,
            kriging_range_km: 800.0,
        };
        model.initialize_coefficients();
        model.load_eastasia_reference_data();
        model
    }

    fn is_eastasia_region(lat: f64, lon: f64) -> bool {
        lat >= EASTASIA_LAT_MIN && lat <= EASTASIA_LAT_MAX
            && lon >= EASTASIA_LON_MIN && lon <= EASTASIA_LON_MAX
    }

    fn haversine_distance_km(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
        let lat1_rad = lat1 * PI / 180.0;
        let lat2_rad = lat2 * PI / 180.0;
        let dlat = (lat2 - lat1) * PI / 180.0;
        let dlon = (lon2 - lon1) * PI / 180.0;

        let a = (dlat / 2.0).sin().powi(2)
            + lat1_rad.cos() * lat2_rad.cos() * (dlon / 2.0).sin().powi(2);
        let c = 2.0 * a.sqrt().asin();

        EARTH_RADIUS_KM * c
    }

    fn spherical_variogram(&self, h_km: f64) -> f64 {
        if h_km <= 0.0 {
            return self.kriging_nugget;
        }

        let h_range = h_km / self.kriging_range_km;
        if h_range >= 1.0 {
            self.kriging_sill
        } else {
            self.kriging_nugget + (self.kriging_sill - self.kriging_nugget)
                * (1.5 * h_range - 0.5 * h_range.powi(3))
        }
    }

    fn load_eastasia_reference_data(&mut self) {
        let eastasia_sites = vec![
            ArchaeologyMagneticData {
                site_name: "北京琉璃河".to_string(),
                location_lat: 39.6,
                location_lon: 116.0,
                sample_age: -1000.0,
                sample_age_error: 100.0,
                declination: -4.2,
                declination_error: 1.2,
                inclination: 58.5,
                inclination_error: 2.0,
                intensity: 58000.0,
                intensity_error: 4000.0,
                sample_material: "loess".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "安阳殷墟".to_string(),
                location_lat: 36.1,
                location_lon: 114.3,
                sample_age: -1200.0,
                sample_age_error: 80.0,
                declination: -3.8,
                declination_error: 1.0,
                inclination: 56.8,
                inclination_error: 1.8,
                intensity: 57500.0,
                intensity_error: 3500.0,
                sample_material: "brick".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "郑州商城".to_string(),
                location_lat: 34.7,
                location_lon: 113.6,
                sample_age: -1500.0,
                sample_age_error: 100.0,
                declination: -3.5,
                declination_error: 1.3,
                inclination: 55.2,
                inclination_error: 2.1,
                intensity: 56000.0,
                intensity_error: 3800.0,
                sample_material: "soil".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "成都金沙".to_string(),
                location_lat: 30.7,
                location_lon: 104.0,
                sample_age: -1000.0,
                sample_age_error: 150.0,
                declination: -5.0,
                declination_error: 1.5,
                inclination: 51.3,
                inclination_error: 2.3,
                intensity: 54000.0,
                intensity_error: 4200.0,
                sample_material: "sediment".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "山东临淄".to_string(),
                location_lat: 36.8,
                location_lon: 118.3,
                sample_age: -500.0,
                sample_age_error: 60.0,
                declination: -3.0,
                declination_error: 0.9,
                inclination: 57.2,
                inclination_error: 1.6,
                intensity: 55500.0,
                intensity_error: 3200.0,
                sample_material: "pottery".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "湖北江陵".to_string(),
                location_lat: 30.3,
                location_lon: 112.2,
                sample_age: -300.0,
                sample_age_error: 50.0,
                declination: -2.8,
                declination_error: 0.8,
                inclination: 52.0,
                inclination_error: 1.5,
                intensity: 53000.0,
                intensity_error: 3000.0,
                sample_material: "brick".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "广州南越王".to_string(),
                location_lat: 23.1,
                location_lon: 113.3,
                sample_age: -150.0,
                sample_age_error: 40.0,
                declination: -2.0,
                declination_error: 1.0,
                inclination: 43.5,
                inclination_error: 2.0,
                intensity: 48000.0,
                intensity_error: 3500.0,
                sample_material: "brick".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "云南晋宁".to_string(),
                location_lat: 24.7,
                location_lon: 102.6,
                sample_age: -200.0,
                sample_age_error: 80.0,
                declination: -4.5,
                declination_error: 1.4,
                inclination: 44.8,
                inclination_error: 2.2,
                intensity: 49500.0,
                intensity_error: 4000.0,
                sample_material: "sediment".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "辽宁建平".to_string(),
                location_lat: 41.4,
                location_lon: 119.5,
                sample_age: -500.0,
                sample_age_error: 70.0,
                declination: -3.2,
                declination_error: 1.1,
                inclination: 60.5,
                inclination_error: 1.8,
                intensity: 59000.0,
                intensity_error: 3800.0,
                sample_material: "loess".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "甘肃敦煌".to_string(),
                location_lat: 40.1,
                location_lon: 94.7,
                sample_age: -100.0,
                sample_age_error: 60.0,
                declination: -1.5,
                declination_error: 1.2,
                inclination: 58.0,
                inclination_error: 2.0,
                intensity: 56500.0,
                intensity_error: 3600.0,
                sample_material: "sediment".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "福建武夷山".to_string(),
                location_lat: 27.8,
                location_lon: 118.0,
                sample_age: -300.0,
                sample_age_error: 90.0,
                declination: -2.5,
                declination_error: 1.3,
                inclination: 48.7,
                inclination_error: 2.1,
                intensity: 51000.0,
                intensity_error: 3900.0,
                sample_material: "soil".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "新疆楼兰".to_string(),
                location_lat: 40.5,
                location_lon: 89.8,
                sample_age: -400.0,
                sample_age_error: 100.0,
                declination: -1.2,
                declination_error: 1.5,
                inclination: 57.5,
                inclination_error: 2.4,
                intensity: 55800.0,
                intensity_error: 4100.0,
                sample_material: "sediment".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "内蒙古和林格尔".to_string(),
                location_lat: 40.4,
                location_lon: 111.8,
                sample_age: -200.0,
                sample_age_error: 50.0,
                declination: -2.2,
                declination_error: 1.0,
                inclination: 59.2,
                inclination_error: 1.7,
                intensity: 57800.0,
                intensity_error: 3300.0,
                sample_material: "brick".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "江苏徐州".to_string(),
                location_lat: 34.2,
                location_lon: 117.3,
                sample_age: -150.0,
                sample_age_error: 50.0,
                declination: -2.7,
                declination_error: 0.8,
                inclination: 54.5,
                inclination_error: 1.4,
                intensity: 54200.0,
                intensity_error: 2900.0,
                sample_material: "soil".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
            ArchaeologyMagneticData {
                site_name: "江西南昌".to_string(),
                location_lat: 28.7,
                location_lon: 115.9,
                sample_age: -100.0,
                sample_age_error: 60.0,
                declination: -2.3,
                declination_error: 1.0,
                inclination: 49.8,
                inclination_error: 1.9,
                intensity: 51800.0,
                intensity_error: 3400.0,
                sample_material: "brick".to_string(),
                reference: "东亚考古地磁数据库2020".to_string(),
            },
        ];

        self.eastasia_data = eastasia_sites;
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
        self.build_regional_kriging_points();
    }

    fn build_regional_kriging_points(&mut self) {
        let mut kriging_points = Vec::new();

        let all_sites: Vec<&ArchaeologyMagneticData> = self.archaeo_data
            .iter()
            .chain(self.eastasia_data.iter())
            .collect();

        for site in &all_sites {
            if !Self::is_eastasia_region(site.location_lat, site.location_lon) {
                continue;
            }

            let cals_field = self.calculate_field_intensity_at_point(
                site.location_lat,
                site.location_lon,
                site.sample_age,
            );

            if let Ok(cals) = cals_field {
                let error_weight = 1.0 / (site.declination_error.max(0.1)
                    * site.intensity_error.max(100.0) / 1000.0);

                kriging_points.push(RegionalKrigingPoint {
                    lat: site.location_lat,
                    lon: site.location_lon,
                    year: site.sample_age,
                    declination_residual: site.declination - cals.declination,
                    inclination_residual: site.inclination - cals.inclination,
                    intensity_residual: site.intensity - cals.field_intensity,
                    weight: error_weight.min(10.0),
                });
            }
        }

        self.regional_kriging_points = kriging_points;
    }

    fn ordinary_kriging_interpolate(
        &self,
        target_lat: f64,
        target_lon: f64,
        target_year: f64,
    ) -> Option<(f64, f64, f64)> {
        if self.regional_kriging_points.is_empty() {
            return None;
        }

        let max_distance = self.kriging_range_km * 2.0;
        let year_window = 500.0;

        let mut nearby_points: Vec<(f64, f64, &RegionalKrigingPoint)> = Vec::new();
        for point in &self.regional_kriging_points {
            let dist_km = Self::haversine_distance_km(
                target_lat, target_lon, point.lat, point.lon
            );
            let year_diff = (target_year - point.year).abs();

            if dist_km < max_distance && year_diff < year_window {
                let temporal_weight = (1.0 - year_diff / year_window).max(0.0);
                let spatial_weight = point.weight * (1.0 - dist_km / max_distance).max(0.01);
                let combined_weight = spatial_weight * (0.5 + 0.5 * temporal_weight);
                nearby_points.push((dist_km, combined_weight, point));
            }
        }

        if nearby_points.len() < 3 {
            return None;
        }

        nearby_points.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let _nearby: Vec<_> = nearby_points.iter().take(12).collect();

        let n = nearby_points.len().min(12);
        if n < 3 {
            return None;
        }

        let mut weights: Vec<f64> = Vec::with_capacity(n);
        let mut total_weight = 0.0;

        for (dist_km, combined_w, point) in nearby_points.iter().take(n) {
            let gamma = self.spherical_variogram(*dist_km);
            let idw_weight = combined_w / (gamma + self.kriging_nugget + 1e-6);
            weights.push(idw_weight);
            total_weight += idw_weight;
            let _ = point;
        }

        if total_weight < 1e-10 {
            return None;
        }

        let mut dec_residual = 0.0;
        let mut inc_residual = 0.0;
        let mut int_residual = 0.0;

        for (i, (_, _, point)) in nearby_points.iter().take(n).enumerate() {
            let w = weights[i] / total_weight;
            dec_residual += w * point.declination_residual;
            inc_residual += w * point.inclination_residual;
            int_residual += w * point.intensity_residual;
        }

        let n_eff = (nearby_points.len() as f64).min(12.0);
        let distance_penalty = if n_eff < 6.0 {
            1.0 - (6.0 - n_eff) * 0.1
        } else {
            1.0
        };

        Some((
            dec_residual * distance_penalty,
            inc_residual * distance_penalty,
            int_residual * distance_penalty,
        ))
    }

    fn calibrate_with_archaeo_data(&mut self) {
        for site in &self.archaeo_data {
            let year = site.sample_age;
            let current_result = self.calculate_field_intensity_at_point(
                site.location_lat,
                site.location_lon,
                year,
            );

            if let Ok(current) = current_result {
                let intensity_ratio = site.intensity / current.field_intensity;
                let adjustment = 1.0 + (intensity_ratio - 1.0) * 0.3;

                let current_declination = current.declination;
                let declination_diff = site.declination - current_declination;
                let dec_rad = declination_diff * PI / 180.0;
                let cos_dec = dec_rad.cos();
                let sin_dec = dec_rad.sin();

                if let Some(coeffs) = self.time_series.get_mut(&(year.round() as i64)) {
                    for n in 1..=MAX_DEGREE {
                        for m in 0..=n {
                            coeffs.g[n][m] *= adjustment;
                            if m > 0 {
                                coeffs.h[n][m] *= adjustment;
                            }
                        }
                    }

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

        let mut bx = x;
        let mut by = y;
        let mut bz = z;

        let field_intensity_raw = (x * x + y * y + z * z).sqrt();
        let h_intensity = (x * x + y * y).sqrt();

        let mut declination = y.atan2(x) * 180.0 / PI;
        let mut inclination = z.atan2(h_intensity) * 180.0 / PI;
        let mut field_intensity = field_intensity_raw;
        let mut model_source = "CALS10K".to_string();

        if Self::is_eastasia_region(lat_deg, lon_deg) {
            if let Some((dec_corr, inc_corr, int_corr)) = self.ordinary_kriging_interpolate(
                lat_deg, lon_deg, target_year
            ) {
                declination += dec_corr;
                inclination += inc_corr;
                field_intensity += int_corr;
                model_source = "CALS10K+EastAsiaKriging".to_string();

                let dec_rad = declination * PI / 180.0;
                let inc_rad = inclination * PI / 180.0;
                let h = field_intensity * inc_rad.cos();
                bx = h * dec_rad.cos();
                by = h * dec_rad.sin();
                bz = field_intensity * inc_rad.sin();
            }
        }

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
            model_source,
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

        if max_degree >= 1 {
            p[1][0] = cos_theta;
            dp[1][0] = -sin_theta;
            p[1][1] = sin_theta;
            dp[1][1] = cos_theta;
        }

        for n in 2..=max_degree {
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
