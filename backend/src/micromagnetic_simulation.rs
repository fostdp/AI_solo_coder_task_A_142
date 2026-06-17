use crate::errors::{AppError, Result};
use crate::models::{PointingSimulationParams, PointingSimulationResult};
use nalgebra::{Vector3, Rotation3};
use rand::Rng;
use rand_distr::{Normal, Distribution};
use std::f64::consts::PI;

pub struct MicromagneticSimulator {
    pub boltzmann_constant: f64,
    pub vacuum_permeability: f64,
    pub gyromagnetic_ratio: f64,
}

impl Default for MicromagneticSimulator {
    fn default() -> Self {
        Self {
            boltzmann_constant: 1.380649e-23,
            vacuum_permeability: 4.0 * PI * 1e-7,
            gyromagnetic_ratio: 1.760859644e11,
        }
    }
}

impl MicromagneticSimulator {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn simulate_pointing(
        &self,
        params: &PointingSimulationParams,
        geomagnetic_field: Vector3<f64>,
    ) -> Result<PointingSimulationResult> {
        let magnetic_moment_vec = self.calculate_equilibrium_magnetization(
            params.magnetic_moment_magnitude,
            params.remanence,
            geomagnetic_field,
            params.anisotropy_constant,
            params.temperature,
        )?;

        let effective_moment = self.apply_demagnetization(
            magnetic_moment_vec,
            params.demagnetization_factor,
            params.remanence,
        )?;

        let torque = self.calculate_magnetic_torque(effective_moment, geomagnetic_field);

        let (simulated_azimuth, pointing_accuracy) = self.calculate_equilibrium_orientation(
            effective_moment,
            geomagnetic_field,
            torque,
            params.friction_coefficient,
            params.temperature,
            params.expected_azimuth,
        )?;

        let model_params = serde_json::json!({
            "boltzmann_constant": self.boltzmann_constant,
            "vacuum_permeability": self.vacuum_permeability,
            "gyromagnetic_ratio": self.gyromagnetic_ratio,
            "effective_moment_magnitude": effective_moment.magnitude(),
            "torque_magnitude": torque.magnitude(),
            "thermal_energy": self.boltzmann_constant * (params.temperature + 273.15),
            "magnetic_potential_energy": -effective_moment.dot(&geomagnetic_field),
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

        let thermal_fluctuation = if stability_ratio > 1.0 {
            (1.0 / stability_ratio).sqrt() * 5.0
        } else {
            15.0
        };

        let friction_damping = 1.0 - friction_coefficient.clamp(0.0, 0.9) * 0.5;

        let mut rng = rand::thread_rng();
        let normal = Normal::new(0.0, 1.0).unwrap();

        let random_variation = normal.sample(&mut rng) * thermal_fluctuation * friction_damping;

        let simulated_azimuth = theoretical_azimuth + alignment_angle * 0.3 + random_variation;

        let mut deviation = (simulated_azimuth - expected_azimuth).abs();
        if deviation > 180.0 {
            deviation = 360.0 - deviation;
        }

        let accuracy = if deviation < 0.5 {
            0.1
        } else if deviation < 2.0 {
            0.5
        } else if deviation < 5.0 {
            1.0
        } else {
            2.0
        };

        Ok((simulated_azimuth, accuracy))
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
