use anyhow::Result;
use clap::{Parser, ValueEnum};
use controller::{Pid, PidConfig};
use safety::{SafetyConfig, SafetyState};
use sim::{PlantParams, PlantState, Sensor, SensorFault};

#[derive(Clone, Debug, ValueEnum)]
enum Scenario {
    Normal,
    Overheat,
    LossOfCooling,
    SensorDisagree,
}

#[derive(Parser, Debug)]
#[command(
    name = "reactor-safety-sim",
    version,
    about = "Generic safety-critical control simulation (portfolio)"
)]
struct Args {
    #[arg(value_enum, long, default_value = "normal")]
    scenario: Scenario,

    /// Total simulation time in seconds
    #[arg(long, default_value_t = 120.0)]
    seconds: f64,

    /// Fixed time step in milliseconds
    #[arg(long, default_value_t = 50)]
    dt_ms: u64,

    /// Control setpoint temperature (°C)
    #[arg(long, default_value_t = 350.0)]
    setpoint: f64,

    /// Trip temperature (°C) for SCRAM
    #[arg(long, default_value_t = 420.0)]
    trip_temp: f64,

    /// RNG seed for deterministic runs
    #[arg(long, default_value_t = 12345)]
    seed: u64,
}

#[derive(serde::Serialize)]
struct TraceRow {
    t_s: f64,
    true_temp_c: f64,
    s1_c: f64,
    s2_c: f64,
    s3_c: f64,
    power: f64,
    coolant: f64,
    scram: bool,
    reason: Option<String>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    let dt_s = (args.dt_ms as f64) / 1000.0;
    let steps = (args.seconds / dt_s).ceil() as u64;

    // Plant & controller
    let p = PlantParams::default();
    let mut x = PlantState::default();
    let mut pid = Pid::new(PidConfig::default());

    // Safety
    let s_cfg = SafetyConfig {
        trip_temp_c: args.trip_temp,
        ..Default::default()
    };
    let mut s_state = SafetyState::default();

    // Sensors
    let mut s1 = Sensor::new(args.seed ^ 0xA1);
    let mut s2 = Sensor::new(args.seed ^ 0xB2);
    let mut s3 = Sensor::new(args.seed ^ 0xC3);

    // Scenario setup
    apply_scenario(&args.scenario, &mut x, &mut s1, &mut s2, &mut s3);

    // Output JSONL trace to stdout (one object per line)
    for k in 0..steps {
        let t_s = (k as f64) * dt_s;

        // Read sensors
        let y1 = s1.read_temp(x.temp_c, dt_s);
        let y2 = s2.read_temp(x.temp_c, dt_s);
        let y3 = s3.read_temp(x.temp_c, dt_s);

        safety::evaluate(&s_cfg, &mut s_state, [y1, y2, y3]);

        if s_state.scram {
            x.power = 0.0;
        } else {
            // Use average of available sensor readings (simple demo)
            let mut sum = 0.0;
            let mut n = 0.0;
            for y in [y1, y2, y3] {
                if y.is_finite() && !y.is_nan() {
                    sum += y;
                    n += 1.0;
                }
            }
            let meas = if n > 0.0 { sum / n } else { x.temp_c };

            let u = pid.update(args.setpoint, meas, dt_s);
            x.power = u.clamp(0.0, 1.0);
        }

        // Scenario dynamics tweaks during run
        if matches!(args.scenario, Scenario::LossOfCooling) && t_s > (args.seconds * 0.3) {
            x.coolant = 0.05;
        }

        // Advance plant
        x.step(&p, dt_s);

        // Emit trace
        let row = TraceRow {
            t_s,
            true_temp_c: x.temp_c,
            s1_c: y1,
            s2_c: y2,
            s3_c: y3,
            power: x.power,
            coolant: x.coolant,
            scram: s_state.scram,
            reason: s_state.reason.map(|r| format!("{r:?}")),
        };
        println!("{}", serde_json::to_string(&row)?);

        if s_state.scram {
            // stop early for clarity
            break;
        }
    }

    Ok(())
}

fn apply_scenario(
    s: &Scenario,
    x: &mut PlantState,
    s1: &mut Sensor,
    s2: &mut Sensor,
    s3: &mut Sensor,
) {
    match s {
        Scenario::Normal => {
            x.coolant = 0.6;
        }
        Scenario::Overheat => {
            x.coolant = 0.2;
        }
        Scenario::LossOfCooling => {
            x.coolant = 0.7;
        }
        Scenario::SensorDisagree => {
            x.coolant = 0.6;
            s2.fault = SensorFault::Bias { value: 20.0 };
        }
    }

    // Slightly lower noise for clearer demos
    s1.noise_std = 0.15;
    s2.noise_std = 0.15;
    s3.noise_std = 0.15;
}
