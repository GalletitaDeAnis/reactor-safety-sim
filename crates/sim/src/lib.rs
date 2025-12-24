use rand::SeedableRng;
use rand::rngs::StdRng;
use rand_distr::{Distribution, Normal};

#[derive(Clone, Copy, Debug)]
pub struct PlantParams {
    pub ambient_c: f64,
    pub thermal_mass: f64,
    pub k_power: f64,
    pub k_cool: f64,
}

impl Default for PlantParams {
    fn default() -> Self {
        Self {
            ambient_c: 25.0,
            thermal_mass: 100.0,
            k_power: 200.0,
            k_cool: 1.5,
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct PlantState {
    pub temp_c: f64,
    /// 0..=1 power fraction
    pub power: f64,
    /// 0..=1 cooling fraction
    pub coolant: f64,
}

impl Default for PlantState {
    fn default() -> Self {
        Self {
            temp_c: 300.0,
            power: 0.0,
            coolant: 0.5,
        }
    }
}

impl PlantState {
    /// Simple Euler integration of a generic thermal model:
    /// dT/dt = (k_power*power - k_cool*coolant*(T-ambient)) / thermal_mass
    pub fn step(&mut self, p: &PlantParams, dt_s: f64) {
        let heat_in = p.k_power * self.power;
        let heat_out = p.k_cool * self.coolant * (self.temp_c - p.ambient_c);
        let dtemp = (heat_in - heat_out) / p.thermal_mass;
        self.temp_c += dtemp * dt_s;

        // Keep within reasonable bounds for a demo
        if self.temp_c.is_nan() {
            self.temp_c = p.ambient_c;
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum SensorFault {
    None,
    Stuck { value: f64 },
    Bias { value: f64 },
    Drift { per_s: f64 },
    DropoutEvery { n: u64 },
}

#[derive(Clone, Debug)]
pub struct Sensor {
    pub noise_std: f64,
    pub fault: SensorFault,
    pub valid_range: (f64, f64),
    rng: StdRng,
    step_count: u64,
}

impl Sensor {
    pub fn new(seed: u64) -> Self {
        Self {
            noise_std: 0.25,
            fault: SensorFault::None,
            valid_range: (0.0, 2000.0),
            rng: StdRng::seed_from_u64(seed),
            step_count: 0,
        }
    }

    pub fn read_temp(&mut self, true_temp: f64, dt_s: f64) -> f64 {
        self.step_count += 1;

        let mut v = match self.fault {
            SensorFault::None => true_temp,
            SensorFault::Stuck { value } => value,
            SensorFault::Bias { value } => true_temp + value,
            SensorFault::Drift { per_s } => true_temp + per_s * (self.step_count as f64) * dt_s,
            SensorFault::DropoutEvery { n } => {
                if n > 0 && (self.step_count % n) == 0 {
                    return f64::NAN;
                }
                true_temp
            }
        };

        if self.noise_std > 0.0 {
            let normal = Normal::new(0.0, self.noise_std).unwrap();
            v += normal.sample(&mut self.rng);
        }

        v
    }

    pub fn is_valid(&self, value: f64) -> bool {
        if value.is_nan() || !value.is_finite() {
            return false;
        }
        value >= self.valid_range.0 && value <= self.valid_range.1
    }
}
