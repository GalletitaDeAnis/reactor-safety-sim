#[derive(Clone, Copy, Debug)]
pub struct PidConfig {
    pub kp: f64,
    pub ki: f64,
    pub kd: f64,
    pub out_min: f64,
    pub out_max: f64,
}

impl Default for PidConfig {
    fn default() -> Self {
        Self {
            kp: 0.02,
            ki: 0.005,
            kd: 0.0,
            out_min: 0.0,
            out_max: 1.0,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Pid {
    cfg: PidConfig,
    integral: f64,
    prev_error: Option<f64>,
}

impl Pid {
    pub fn new(cfg: PidConfig) -> Self {
        Self {
            cfg,
            integral: 0.0,
            prev_error: None,
        }
    }

    pub fn reset(&mut self) {
        self.integral = 0.0;
        self.prev_error = None;
    }

    /// Update PID given setpoint and measurement. Returns a saturated output [out_min, out_max].
    pub fn update(&mut self, setpoint: f64, measurement: f64, dt_s: f64) -> f64 {
        let error = setpoint - measurement;

        // Integral
        self.integral += error * dt_s;

        // Derivative
        let deriv = match self.prev_error {
            Some(prev) if dt_s > 0.0 => (error - prev) / dt_s,
            _ => 0.0,
        };
        self.prev_error = Some(error);

        let mut out = self.cfg.kp * error + self.cfg.ki * self.integral + self.cfg.kd * deriv;

        // Saturate output + simple anti-windup by clamping integral if saturated
        if out > self.cfg.out_max {
            out = self.cfg.out_max;
            // prevent runaway integral
            if error > 0.0 {
                self.integral *= 0.98;
            }
        } else if out < self.cfg.out_min {
            out = self.cfg.out_min;
            if error < 0.0 {
                self.integral *= 0.98;
            }
        }

        out
    }
}
