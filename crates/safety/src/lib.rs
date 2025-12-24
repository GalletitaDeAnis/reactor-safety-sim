#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TripReason {
    OverTemp,
    SensorInvalid,
    SensorDisagree,
}

#[derive(Clone, Copy, Debug)]
pub struct SafetyConfig {
    pub trip_temp_c: f64,
    pub max_sensor_delta_c: f64,
    pub valid_range_c: (f64, f64),
}

impl Default for SafetyConfig {
    fn default() -> Self {
        Self {
            trip_temp_c: 420.0,
            max_sensor_delta_c: 10.0,
            valid_range_c: (0.0, 2000.0),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SafetyState {
    pub scram: bool,
    pub reason: Option<TripReason>,
}

impl Default for SafetyState {
    fn default() -> Self {
        Self {
            scram: false,
            reason: None,
        }
    }
}

fn is_valid(cfg: &SafetyConfig, v: f64) -> bool {
    v.is_finite() && !v.is_nan() && v >= cfg.valid_range_c.0 && v <= cfg.valid_range_c.1
}

fn two_out_of_three(flags: [bool; 3]) -> bool {
    let mut c = 0;
    for f in flags {
        if f { c += 1; }
    }
    c >= 2
}

/// Evaluate safety conditions using three redundant sensor measurements.
/// Returns updated state. Once SCRAM is asserted, it remains latched.
pub fn evaluate(cfg: &SafetyConfig, state: &mut SafetyState, temps: [f64; 3]) {
    if state.scram {
        return;
    }

    // Validity
    let valids = [is_valid(cfg, temps[0]), is_valid(cfg, temps[1]), is_valid(cfg, temps[2])];
    if !two_out_of_three(valids) {
        state.scram = true;
        state.reason = Some(TripReason::SensorInvalid);
        return;
    }

    // Disagreement check among valid sensors
    let mut min_v = f64::INFINITY;
    let mut max_v = f64::NEG_INFINITY;
    for (i, v) in temps.iter().enumerate() {
        if valids[i] {
            min_v = min_v.min(*v);
            max_v = max_v.max(*v);
        }
    }
    if (max_v - min_v) > cfg.max_sensor_delta_c {
        state.scram = true;
        state.reason = Some(TripReason::SensorDisagree);
        return;
    }

    // Over-temp vote
    let over = [
        valids[0] && temps[0] >= cfg.trip_temp_c,
        valids[1] && temps[1] >= cfg.trip_temp_c,
        valids[2] && temps[2] >= cfg.trip_temp_c,
    ];
    if two_out_of_three(over) {
        state.scram = true;
        state.reason = Some(TripReason::OverTemp);
    }
}
