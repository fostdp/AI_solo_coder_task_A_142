use crate::errors::{AppError, Result};
use crate::models::{PointingSimulationParams, PointingSimulationResult};
use nalgebra::{Vector3, Matrix3};
use rand_distr::{Normal, Distribution};
use std::f64::consts::PI;

#[derive(Debug, Clone)]
pub struct DemagnetizationTensor {
    pub n_xx: f64,
    pub n_yy: f64,
    pub n_zz: f64,
    pub n_xy: f64,
    pub n_xz: f64,
    pub n_yz: f64,
    pub corner_correction: f64,
    pub domain_wall_correction: f64,
}

impl DemagnetizationTensor {
    pub fn for_ellipsoid(a: f64, b: f64, c: f64) -> Self {
        let n = Self::ellipsoid_demagnetization_factors(a, b, c);
        Self {
            n_xx: n.0,
            n_yy: n.1,
            n_zz: n.2,
            n_xy: 0.0,
            n_xz: 0.0,
            n_yz: 0.0,
            corner_correction: 0.0,
            domain_wall_correction: 0.0,
        }
    }

    pub fn for_spoon_shape(length: f64, width: f64, thickness: f64) -> Self {
        let (n_body_x, n_body_y, n_body_z) = Self::ellipsoid_demagnetization_factors(
            length * 0.7, width * 0.5, thickness
        );

        let handle_ratio = 0.3;
        let (n_handle_x, n_handle_y, n_handle_z) = Self::ellipsoid_demagnetization_factors(
            length * handle_ratio, width * 0.3, thickness * 1.2
        );

        let w_body = 0.75;
        let w_handle = 0.25;

        let bowl_correction = Self::calculate_bowl_edge_correction(width, thickness);
        let handle_correction = Self::calculate_handle_tip_correction(length, thickness);
        let corner_correction = (bowl_correction + handle_correction) * 0.5;

        let domain_wall_correction = Self::calculate_domain_wall_demagnetization(
            length, width, thickness
        );

        Self {
            n_xx: w_body * n_body_x + w_handle * n_handle_x,
            n_yy: w_body * n_body_y + w_handle * n_handle_y,
            n_zz: w_body * n_body_z + w_handle * n_handle_z,
            n_xy: -0.02 * corner_correction,
            n_xz: 0.015 * corner_correction,
            n_yz: -0.01 * corner_correction,
            corner_correction,
            domain_wall_correction,
        }
    }

    fn ellipsoid_demagnetization_factors(a: f64, b: f64, c: f64) -> (f64, f64, f64) {
        let axes = [a, b, c];
        let mut sorted_indices: Vec<usize> = (0..3).collect();
        sorted_indices.sort_by(|&i, &j| axes[j].partial_cmp(&axes[i]).unwrap());

        let a_max = axes[sorted_indices[0]];
        let a_mid = axes[sorted_indices[1]];
        let a_min = axes[sorted_indices[2]];

        let n_max: f64;
        let n_mid: f64;
        let n_min: f64;

        if (a_max - a_mid).abs() < 1e-6 && (a_mid - a_min).abs() < 1e-6 {
            n_max = 1.0 / 3.0;
            n_mid = 1.0 / 3.0;
            n_min = 1.0 / 3.0;
        } else if (a_max - a_mid).abs() < 1e-6 {
            let e = (1.0 - (a_min / a_max).powi(2)).sqrt();
            let ln_term = ((1.0 + e) / (1.0 - e)).ln();
            n_min = (1.0 / e.powi(2)) * (1.0 - (1.0 - e.powi(2)).sqrt() / e * ln_term / 2.0);
            n_max = (1.0 - n_min) / 2.0;
            n_mid = n_max;
        } else if (a_mid - a_min).abs() < 1e-6 {
            let e = (1.0 - (a_min / a_max).powi(2)).sqrt();
            let asin_e = e.asin();
            n_max = (1.0 - e.powi(2)) / e.powi(2) * (asin_e / e - 1.0);
            n_min = (1.0 - n_max) / 2.0;
            n_mid = n_min;
        } else {
            let k_sq = (a_max.powi(2) - a_mid.powi(2)) / (a_max.powi(2) - a_min.powi(2));
            let alpha_sq = a_min.powi(2) / a_max.powi(2);
            let beta_sq = a_mid.powi(2) / a_max.powi(2);
            let k = k_sq.sqrt();

            let (k_complete, e_complete) = Self::elliptic_integrals(k);

            let a_factor = 2.0 / ((1.0 - alpha_sq).sqrt() * (1.0 - beta_sq));
            let b_factor = (beta_sq / (1.0 - beta_sq)).sqrt();
            let c_factor = (alpha_sq / (1.0 - alpha_sq)).sqrt();

            n_max = a_factor * (k_complete - e_complete);
            n_mid = a_factor * (e_complete / beta_sq - k_complete * (1.0 - beta_sq) / beta_sq
                - b_factor * (k_complete - e_complete) * (1.0 - k_sq) / k_sq);
            n_min = 1.0 - n_max - n_mid;
        }

        let mut result = [0.0f64; 3];
        result[sorted_indices[0]] = n_max;
        result[sorted_indices[1]] = n_mid;
        result[sorted_indices[2]] = n_min;

        (result[0], result[1], result[2])
    }

    fn elliptic_integrals(k: f64) -> (f64, f64) {
        let a0 = 1.0;
        let b0 = (1.0 - k * k).sqrt();

        let mut a = a0;
        let mut b = b0;
        let mut c = k;
        let mut sum_d = 0.0;
        let mut pow_2n = 1.0;

        for _ in 0..20 {
            let a_new = (a + b) / 2.0;
            let b_new = (a * b).sqrt();
            let c_new = (a - b) / 2.0;

            pow_2n *= 2.0;
            sum_d += pow_2n * c_new * c_new;

            a = a_new;
            b = b_new;
            c = c_new;

            if c.abs() < 1e-15 {
                break;
            }
        }

        let k_complete = PI / (2.0 * a);
        let e_complete = k_complete * (1.0 - sum_d / 2.0);

        (k_complete, e_complete)
    }

    fn calculate_bowl_edge_correction(bowl_width: f64, thickness: f64) -> f64 {
        let aspect_ratio = bowl_width / thickness;
        let edge_singularity = (aspect_ratio / 10.0).ln().max(0.5);
        let curvature_factor = 1.0 + 1.0 / (aspect_ratio * 0.5 + 1.0);

        0.08 * edge_singularity * curvature_factor
    }

    fn calculate_handle_tip_correction(handle_length: f64, thickness: f64) -> f64 {
        let taper_ratio = handle_length / thickness;
        let tip_field_enhancement = (taper_ratio / 8.0).tanh() * 2.0;

        0.06 * tip_field_enhancement
    }

    fn calculate_domain_wall_demagnetization(length: f64, width: f64, thickness: f64) -> f64 {
        let volume = length * width * thickness;
        let domain_width = 1e-6;
        let wall_thickness = 1e-8;
        let num_domains = (volume / (domain_width * width * thickness)).max(1.0);

        let wall_energy_factor = 1e-3 * num_domains.sqrt();
        let shape_factor = (length / (width + thickness)).tanh();

        wall_energy_factor * shape_factor
    }

    pub fn as_matrix(&self) -> Matrix3<f64> {
        Matrix3::new(
            self.n_xx, self.n_xy, self.n_xz,
            self.n_xy, self.n_yy, self.n_yz,
            self.n_xz, self.n_yz, self.n_zz,
        )
    }

    pub fn effective_trace(&self) -> f64 {
        (self.n_xx + self.n_yy + self.n_zz) / 3.0 + self.corner_correction * 0.3
    }
}

pub struct MicromagneticSimulator {
    pub boltzmann_constant: f64,
    pub vacuum_permeability: f64,
    pub gyromagnetic_ratio: f64,
    pub spoon_length_m: f64,
    pub spoon_width_m: f64,
    pub spoon_thickness_m: f64,
}

impl Default for MicromagneticSimulator {
    fn default() -> Self {
        Self {
            boltzmann_constant: 1.380649e-23,
            vacuum_permeability: 4.0 * PI * 1e-7,
            gyromagnetic_ratio: 1.760859644e11,
            spoon_length_m: 0.17,
            spoon_width_m: 0.08,
            spoon_thickness_m: 0.015,
        }
    }
}

impl MicromagneticSimulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_spoon_dimensions(length_m: f64, width_m: f64, thickness_m: f64) -> Self {
        let mut s = Self::default();
        s.spoon_length_m = length_m;
        s.spoon_width_m = width_m;
        s.spoon_thickness_m = thickness_m;
        s
    }

    pub fn get_demagnetization_tensor(&self) -> DemagnetizationTensor {
        DemagnetizationTensor::for_spoon_shape(
            self.spoon_length_m,
            self.spoon_width_m,
            self.spoon_thickness_m,
        )
    }

    pub fn simulate_pointing(
        &self,
        params: &PointingSimulationParams,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<PointingSimulationResult> {
        let demag_tensor = self.get_demagnetization_tensor();

        let magnetic_moment_vec = self.calculate_equilibrium_magnetization(
            params.magnetic_moment_magnitude,
            params.remanence,
            geomagnetic_field,
            params.anisotropy_constant,
            params.temperature,
        )?;

        let effective_moment = self.apply_demagnetization_tensor(
            magnetic_moment_vec,
            &demag_tensor,
            params.remanence,
            geomagnetic_field,
        )?;

        let torque = self.calculate_magnetic_torque(effective_moment, geomagnetic_field);

        let (simulated_azimuth, pointing_accuracy) = self.calculate_equilibrium_orientation(
            effective_moment,
            geomagnetic_field,
            torque,
            params.friction_coefficient,
            params.temperature,
            params.expected_azimuth,
            &demag_tensor,
        )?;

        let model_params = serde_json::json!({
            "boltzmann_constant": self.boltzmann_constant,
            "vacuum_permeability": self.vacuum_permeability,
            "gyromagnetic_ratio": self.gyromagnetic_ratio,
            "effective_moment_magnitude": effective_moment.magnitude(),
            "torque_magnitude": torque.magnitude(),
            "thermal_energy": self.boltzmann_constant * (params.temperature + 273.15),
            "magnetic_potential_energy": -effective_moment.dot(&geomagnetic_field),
            "demagnetization_tensor": {
                "n_xx": demag_tensor.n_xx,
                "n_yy": demag_tensor.n_yy,
                "n_zz": demag_tensor.n_zz,
                "n_xy": demag_tensor.n_xy,
                "n_xz": demag_tensor.n_xz,
                "n_yz": demag_tensor.n_yz,
                "corner_correction": demag_tensor.corner_correction,
                "domain_wall_correction": demag_tensor.domain_wall_correction,
            },
            "spoon_dimensions": {
                "length_m": self.spoon_length_m,
                "width_m": self.spoon_width_m,
                "thickness_m": self.spoon_thickness_m,
            },
        });

        Ok(PointingSimulationResult {
            id: uuid::Uuid::new_v4(),
            timestamp: chrono::Utc::now(),
            device_id: params.device_id.clone(),
            simulation_id: params.simulation_id.clone(),
            target_year: params.target_year,
            location_lat: params.location_lat,
            location_lon: params.location_lon,
            expected_azimuth: params.expected_azimuth,
            simulated_azimuth,
            pointing_accuracy,
            magnetic_moment_magnitude: params.magnetic_moment_magnitude,
            remanence: params.remanence,
            temperature: params.temperature,
            friction_coefficient: params.friction_coefficient,
            demagnetization_factor: params.demagnetization_factor,
            anisotropy_constant: params.anisotropy_constant,
            model_parameters: model_params.to_string(),
        })
    }

    fn calculate_equilibrium_magnetization(
        &self,
        moment_magnitude: f64,
        remanence: f64,
        external_field: Vector3<f64>,
        anisotropy_constant: f64,
        temperature: f64,
    ) -> Result<Vector3<f64>> {
        let field_magnitude = external_field.magnitude();
        if field_magnitude < 1e-12 {
            return Err(AppError::SimulationError(
                "地磁场强度过小，无法进行有效仿真".to_string(),
            ));
        }

        let temp_kelvin = temperature + 273.15;
        let saturation_magnetization = remanence / self.vacuum_permeability;

        let field_unit = external_field / field_magnitude;

        let thermal_energy = self.boltzmann_constant * temp_kelvin;
        let anisotropy_energy = anisotropy_constant;
        let zeeman_energy = moment_magnitude * field_magnitude;

        let effective_field_ratio = zeeman_energy / thermal_energy;

        let langevin_argument = if effective_field_ratio > 100.0 {
            1.0 - 1.0 / effective_field_ratio
        } else if effective_field_ratio < 0.01 {
            effective_field_ratio / 3.0
        } else {
            (effective_field_ratio).cosh() / (effective_field_ratio).sinh()
                - 1.0 / effective_field_ratio
        };

        let anisotropy_factor = (anisotropy_energy / thermal_energy).tanh();
        let total_magnetization_ratio = langevin_argument * (1.0 + 0.3 * anisotropy_factor);

        let effective_magnitude = saturation_magnetization * total_magnetization_ratio.max(0.1);

        Ok(field_unit * effective_magnitude)
    }

    #[allow(dead_code)]
    fn apply_demagnetization(
        &self,
        magnetization: Vector3<f64>,
        demagnetization_factor: f64,
        remanence: f64,
    ) -> Result<Vector3<f64>> {
        let n = demagnetization_factor.clamp(0.0, 1.0 / 3.0);
        let demagnetizing_field = -magnetization * n;
        let internal_field = demagnetizing_field;

        let saturation_magnetization = remanence / self.vacuum_permeability;
        let susceptibility = 100.0;

        let effective_magnetization = magnetization + internal_field * susceptibility;

        let scale = if effective_magnetization.magnitude() > saturation_magnetization {
            saturation_magnetization / effective_magnetization.magnitude()
        } else {
            1.0
        };

        Ok(effective_magnetization * scale)
    }

    fn apply_demagnetization_tensor(
        &self,
        magnetization: Vector3<f64>,
        demag_tensor: &DemagnetizationTensor,
        remanence: f64,
        external_field: Vector3<f64>,
    ) -> Result<Vector3<f64>> {
        let n_matrix = demag_tensor.as_matrix();

        let demagnetizing_field_vec = -n_matrix * magnetization;

        let corner_field = self.calculate_corner_demagnetizing_field(
            &magnetization,
            demag_tensor.corner_correction,
        );

        let domain_wall_field = self.calculate_domain_wall_field(
            &magnetization,
            demag_tensor.domain_wall_correction,
        );

        let total_demag_field = demagnetizing_field_vec + corner_field + domain_wall_field;

        let saturation_magnetization = remanence / self.vacuum_permeability;
        let susceptibility = self.calculate_field_dependent_susceptibility(
            &external_field,
            &total_demag_field,
        );

        let internal_field = external_field * self.vacuum_permeability + total_demag_field;
        let mut effective_magnetization = magnetization + internal_field * susceptibility;

        let mag_magnitude = effective_magnetization.magnitude();
        if mag_magnitude > saturation_magnetization && mag_magnitude > 1e-20 {
            effective_magnetization = effective_magnetization * (saturation_magnetization / mag_magnitude);
        }

        if mag_magnitude < saturation_magnetization * 0.1 && saturation_magnetization > 1e-20 {
            let boost_factor = 0.15 * demag_tensor.corner_correction;
            effective_magnetization = effective_magnetization * (1.0 + boost_factor);
        }

        Ok(effective_magnetization)
    }

    fn calculate_corner_demagnetizing_field(
        &self,
        magnetization: &Vector3<f64>,
        corner_correction: f64,
    ) -> Vector3<f64> {
        let length_ratio = self.spoon_length_m / self.spoon_width_m.max(1e-6);
        let thickness_ratio = self.spoon_width_m / self.spoon_thickness_m.max(1e-6);

        let bowl_edge_factor = 0.6 * corner_correction;
        let handle_tip_factor = 0.4 * corner_correction;

        let mag_normalized = if magnetization.magnitude() > 1e-20 {
            magnetization / magnetization.magnitude()
        } else {
            Vector3::new(1.0, 0.0, 0.0)
        };

        let bowl_field = -mag_normalized * bowl_edge_factor * (thickness_ratio / 5.0).tanh();

        let handle_dir = Vector3::new(1.0, 0.0, 0.0);
        let handle_component = mag_normalized.dot(&handle_dir);
        let handle_field = -handle_dir * handle_component * handle_tip_factor
            * (length_ratio / 3.0).tanh();

        let edge_shear = Vector3::new(
            0.0,
            corner_correction * 0.15 * mag_normalized.x,
            -corner_correction * 0.1 * mag_normalized.y,
        );

        bowl_field + handle_field + edge_shear
    }

    fn calculate_domain_wall_field(
        &self,
        magnetization: &Vector3<f64>,
        domain_wall_correction: f64,
    ) -> Vector3<f64> {
        let mag_xy = Vector3::new(magnetization.x, magnetization.y, 0.0);
        let xy_magnitude = mag_xy.magnitude();

        if xy_magnitude < 1e-20 {
            return Vector3::zeros();
        }

        let wall_count_factor = (self.spoon_length_m / 1e-6).sqrt();
        let pinning_strength = domain_wall_correction * wall_count_factor * 0.01;

        let domain_avg = -magnetization * pinning_strength * 0.5;

        let wall_roughness = Vector3::new(
            (magnetization.y * pinning_strength * 0.3).tanh(),
            (-magnetization.x * pinning_strength * 0.3).tanh(),
            0.0,
        );

        domain_avg + wall_roughness
    }

    fn calculate_field_dependent_susceptibility(
        &self,
        external_field: &Vector3<f64>,
        demag_field: &Vector3<f64>,
    ) -> f64 {
        let total_field_mag = (external_field * self.vacuum_permeability + demag_field).magnitude();

        let base_susceptibility = 100.0;

        if total_field_mag < 1e-12 {
            return base_susceptibility;
        }

        let saturation_field = 50e-3;
        let field_ratio = total_field_mag / saturation_field;

        base_susceptibility / (1.0 + field_ratio * 0.5)
    }

    fn calculate_magnetic_torque(
        &self,
        magnetic_moment: Vector3<f64>,
        magnetic_field: Vector3<f64>,
    ) -> Vector3<f64> {
        magnetic_moment.cross(&magnetic_field) * self.vacuum_permeability
    }

    fn calculate_equilibrium_orientation(
        &self,
        magnetic_moment: Vector3<f64>,
        magnetic_field: Vector3<f64>,
        torque: Vector3<f64>,
        friction_coefficient: f64,
        temperature: f64,
        expected_azimuth: f64,
        demag_tensor: &DemagnetizationTensor,
    ) -> Result<(f64, f64)> {
        let field_xy = Vector3::new(magnetic_field.x, magnetic_field.y, 0.0);
        let theoretical_azimuth = if field_xy.magnitude() > 1e-12 {
            let angle = field_xy.y.atan2(field_xy.x);
            (angle * 180.0 / PI + 360.0) % 360.0
        } else {
            expected_azimuth
        };

        let torque_magnitude = torque.magnitude();
        let moment_magnitude = magnetic_moment.magnitude();
        let field_magnitude = magnetic_field.magnitude();

        let max_torque = self.vacuum_permeability * moment_magnitude * field_magnitude;

        let alignment_angle = if max_torque > 1e-20 {
            (torque_magnitude / max_torque).asin() * 180.0 / PI
        } else {
            0.0
        };

        let temp_kelvin = temperature + 273.15;
        let thermal_energy = self.boltzmann_constant * temp_kelvin;
        let magnetic_energy = self.vacuum_permeability * moment_magnitude * field_magnitude;

        let stability_ratio = magnetic_energy / thermal_energy;

        let shape_anisotropy_bias = self.calculate_shape_anisotropy_bias(
            demag_tensor,
            &magnetic_moment,
        );

        let corner_noise_amplitude = demag_tensor.corner_correction * 2.5;
        let domain_wall_drift = demag_tensor.domain_wall_correction * 1.5;

        let thermal_fluctuation = if stability_ratio > 1.0 {
            (1.0 / stability_ratio).sqrt() * (5.0 + corner_noise_amplitude + domain_wall_drift)
        } else {
            15.0 + corner_noise_amplitude * 2.0 + domain_wall_drift
        };

        let friction_damping = 1.0 - friction_coefficient.clamp(0.0, 0.9) * 0.5;

        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0).unwrap();

        let random_variation = normal.sample(&mut rng) * thermal_fluctuation * friction_damping;

        let simulated_azimuth = theoretical_azimuth
            + alignment_angle * 0.3
            + shape_anisotropy_bias
            + random_variation;

        let mut deviation = (simulated_azimuth - expected_azimuth).abs();
        if deviation > 180.0 {
            deviation = 360.0 - deviation;
        }

        let finite_element_correction = 1.0 + demag_tensor.corner_correction * 0.4
            + demag_tensor.domain_wall_correction * 0.3;
        let corrected_deviation = deviation * finite_element_correction;

        let accuracy = if corrected_deviation < 0.5 {
            0.1
        } else if corrected_deviation < 2.0 {
            0.5
        } else if corrected_deviation < 5.0 {
            1.0
        } else {
            2.0
        };

        Ok((simulated_azimuth, accuracy))
    }

    fn calculate_shape_anisotropy_bias(
        &self,
        demag_tensor: &DemagnetizationTensor,
        magnetic_moment: &Vector3<f64>,
    ) -> f64 {
        let n_diff = demag_tensor.n_yy - demag_tensor.n_xx;

        let moment_xy_mag = (magnetic_moment.x.powi(2) + magnetic_moment.y.powi(2)).sqrt();
        if moment_xy_mag < 1e-20 {
            return 0.0;
        }

        let moment_angle = magnetic_moment.y.atan2(magnetic_moment.x);
        let sin_2theta = (2.0 * moment_angle).sin();

        let anisotropy_field = n_diff * moment_xy_mag;
        let bias_angle_deg = (anisotropy_field * sin_2theta * 1e3).clamp(-3.0, 3.0);

        bias_angle_deg + demag_tensor.n_xy * 5.0
    }

    pub fn calculate_pointing_deviation(
        &self,
        measured_moment: Vector3<f64>,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<f64> {
        let moment_xy = Vector3::new(measured_moment.x, measured_moment.y, 0.0);
        let field_xy = Vector3::new(geomagnetic_field.x, geomagnetic_field.y, 0.0);

        if moment_xy.magnitude() < 1e-12 || field_xy.magnitude() < 1e-12 {
            return Err(AppError::SimulationError(
                "磁矩或地磁场在水平面分量过小，无法计算指向偏差".to_string(),
            ));
        }

        let moment_azimuth = (moment_xy.y.atan2(moment_xy.x) * 180.0 / PI + 360.0) % 360.0;
        let field_azimuth = (field_xy.y.atan2(field_xy.x) * 180.0 / PI + 360.0) % 360.0;

        let mut deviation = (moment_azimuth - field_azimuth).abs();
        if deviation > 180.0 {
            deviation = 360.0 - deviation;
        }

        Ok(deviation)
    }

    pub fn calculate_magnetic_moment_from_sensor(
        &self,
        mx: f64,
        my: f64,
        mz: f64,
    ) -> Vector3<f64> {
        Vector3::new(mx, my, mz)
    }

    pub fn calculate_field_components(
        &self,
        intensity: f64,
        declination: f64,
        inclination: f64,
    ) -> Vector3<f64> {
        let dec_rad = declination * PI / 180.0;
        let inc_rad = inclination * PI / 180.0;

        let h = intensity * inc_rad.cos();
        let bx = h * dec_rad.cos();
        let by = h * dec_rad.sin();
        let bz = intensity * inc_rad.sin();

        Vector3::new(bx, by, bz)
    }

    pub fn stoner_wohlfarth_switching(
        &self,
        anisotropy_constant: f64,
        saturation_magnetization: f64,
        applied_field: f64,
        field_angle: f64,
    ) -> Result<f64> {
        let anisotropy_field = 2.0 * anisotropy_constant / (self.vacuum_permeability * saturation_magnetization);

        if anisotropy_field < 1e-12 {
            return Err(AppError::SimulationError(
                "各向异性场过小，无法计算开关场".to_string(),
            ));
        }

        let theta = field_angle * PI / 180.0;
        let reduced_field = applied_field / anisotropy_field;

        let switching_field = if theta.abs() < 1e-6 {
            1.0
        } else {
            let sin_theta = theta.sin();
            let cos_theta = theta.cos();
            (sin_theta.powf(2.0/3.0) + cos_theta.powf(2.0/3.0)).powf(-1.5)
        };

        Ok(switching_field * anisotropy_field)
    }
}
