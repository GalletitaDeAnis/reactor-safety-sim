use controller::{Pid, PidConfig};
use eframe::egui;
use egui_plot::{Line, Plot, PlotPoints};
use safety::{SafetyConfig, SafetyState};
use serde::Deserialize;
use serde_json::Value;
use sim::{PlantParams, PlantState, Sensor, SensorFault};
use std::fs;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Scenario {
    Normal,
    Overheat,
    LossOfCooling,
    SensorDisagree,
}

impl Scenario {
    fn label(self) -> &'static str {
        match self {
            Scenario::Normal => "Normal",
            Scenario::Overheat => "Overheat (low cooling)",
            Scenario::LossOfCooling => "Loss of cooling (after 30%)",
            Scenario::SensorDisagree => "Sensor disagree (bias on sensor 2)",
        }
    }
}

#[derive(Clone, Debug)]
struct Sample {
    t: f64,
    true_temp: f64,
    s1: f64,
    s2: f64,
    s3: f64,
    power: f64,
    coolant: f64,
    scram: bool,
}

#[derive(Debug, Deserialize)]
struct CliLine {
    t_s: f64,
    true_temp_c: f64,
    s1_c: f64,
    s2_c: f64,
    s3_c: f64,
    power: f64,
    coolant: f64,
    scram: bool,
    // JSONL: null al inicio, luego puede ser string u objeto
    reason: Option<Value>,
}

struct App {
    // Settings
    scenario: Scenario,
    seconds: f64,
    dt_ms: u64,
    setpoint: f64,
    trip_temp: f64,
    seed: u64,

    // Live simulation state
    running: bool,
    t: f64,
    dt_s: f64,
    max_steps: u64,
    step_count: u64,

    plant_p: PlantParams,
    plant_x: PlantState,
    pid: Pid,
    safety_cfg: SafetyConfig,
    safety_state: SafetyState,
    s1: Sensor,
    s2: Sensor,
    s3: Sensor,

    // Data shown in plots
    samples: Vec<Sample>,

    // Replay
    replay_loaded: bool,
    replay_path: String,
    replay_all: Vec<Sample>,
    replay_pos: usize,
    replay_playing: bool,
    replay_speed: usize, // samples per frame
    replay_reason: Option<String>,
    last_error: Option<String>,
}

impl Default for App {
    fn default() -> Self {
        let scenario = Scenario::Normal;
        let seconds = 60.0;
        let dt_ms = 50;
        let setpoint = 350.0;
        let trip_temp = 420.0;
        let seed = 12345;

        let dt_s = dt_ms as f64 / 1000.0;
        let max_steps = (seconds / dt_s).ceil() as u64;

        let mut app = Self {
            scenario,
            seconds,
            dt_ms,
            setpoint,
            trip_temp,
            seed,

            running: false,
            t: 0.0,
            dt_s,
            max_steps,
            step_count: 0,

            plant_p: PlantParams::default(),
            plant_x: PlantState::default(),
            pid: Pid::new(PidConfig::default()),
            safety_cfg: SafetyConfig {
                trip_temp_c: trip_temp,
                ..Default::default()
            },
            safety_state: SafetyState::default(),
            s1: Sensor::new(seed ^ 0xA1),
            s2: Sensor::new(seed ^ 0xB2),
            s3: Sensor::new(seed ^ 0xC3),

            samples: Vec::new(),

            replay_loaded: false,
            replay_path: "out/demo_overheat.jsonl".to_string(),
            replay_all: Vec::new(),
            replay_pos: 0,
            replay_playing: false,
            replay_speed: 50,
            replay_reason: None,
            last_error: None,
        };

        app.apply_scenario();
        app
    }
}

impl App {
    fn clear_replay(&mut self) {
        self.replay_loaded = false;
        self.replay_all.clear();
        self.replay_pos = 0;
        self.replay_playing = false;
        self.replay_reason = None;
        self.last_error = None;
    }

    fn reset_live(&mut self) {
        self.running = false;
        self.t = 0.0;
        self.dt_s = self.dt_ms as f64 / 1000.0;
        self.max_steps = (self.seconds / self.dt_s).ceil() as u64;
        self.step_count = 0;

        self.plant_p = PlantParams::default();
        self.plant_x = PlantState::default();
        self.pid = Pid::new(PidConfig::default());
        self.safety_cfg = SafetyConfig {
            trip_temp_c: self.trip_temp,
            ..Default::default()
        };
        self.safety_state = SafetyState::default();

        self.s1 = Sensor::new(self.seed ^ 0xA1);
        self.s2 = Sensor::new(self.seed ^ 0xB2);
        self.s3 = Sensor::new(self.seed ^ 0xC3);

        self.samples.clear();
        self.apply_scenario();
    }

    fn reset(&mut self) {
        self.clear_replay();
        self.reset_live();
    }

    fn apply_scenario(&mut self) {
        match self.scenario {
            Scenario::Normal => {
                self.plant_x.coolant = 0.6;
                self.s2.fault = SensorFault::None;
            }
            Scenario::Overheat => {
                self.plant_x.coolant = 0.2;
                self.s2.fault = SensorFault::None;
            }
            Scenario::LossOfCooling => {
                self.plant_x.coolant = 0.7;
                self.s2.fault = SensorFault::None;
            }
            Scenario::SensorDisagree => {
                self.plant_x.coolant = 0.6;
                self.s2.fault = SensorFault::Bias { value: 20.0 };
            }
        }

        self.s1.noise_std = 0.15;
        self.s2.noise_std = 0.15;
        self.s3.noise_std = 0.15;
    }

    fn load_jsonl(&mut self, path: &str) {
        self.last_error = None;

        let text = match fs::read_to_string(path) {
            Ok(t) => t,
            Err(e) => {
                self.last_error = Some(format!("Failed to read {path}: {e}"));
                return;
            }
        };

        let mut loaded: Vec<Sample> = Vec::new();
        let mut first_reason: Option<String> = None;

        for (i, raw) in text.lines().enumerate() {
            let line = raw.trim();
            if line.is_empty() {
                continue;
            }

            let row: CliLine = match serde_json::from_str(line) {
                Ok(v) => v,
                Err(e) => {
                    self.last_error = Some(format!("JSON parse error at line {}: {}", i + 1, e));
                    return;
                }
            };

            if first_reason.is_none() {
                if let Some(r) = row.reason.as_ref() {
                    first_reason = Some(r.to_string());
                }
            }

            loaded.push(Sample {
                t: row.t_s,
                true_temp: row.true_temp_c,
                s1: row.s1_c,
                s2: row.s2_c,
                s3: row.s3_c,
                power: row.power,
                coolant: row.coolant,
                scram: row.scram,
            });
        }

        if loaded.is_empty() {
            self.last_error = Some(format!("No samples found in {path}"));
            return;
        }

        // Enter replay mode
        self.running = false;
        self.clear_replay();
        self.replay_loaded = true;
        self.replay_reason = first_reason;

        self.replay_all = loaded;
        self.replay_pos = 0;
        self.replay_playing = false;

        // Start with an initial chunk so the plot isn't empty
        self.samples.clear();
        let initial = self.replay_speed.min(self.replay_all.len()).max(1);
        self.samples.extend_from_slice(&self.replay_all[..initial]);
        self.replay_pos = initial;

        // Sync time marker
        self.t = self.samples.last().map(|s| s.t).unwrap_or(0.0);
    }

    fn replay_tick(&mut self) {
        if !(self.replay_loaded && self.replay_playing) {
            return;
        }

        if self.replay_pos >= self.replay_all.len() {
            self.replay_playing = false;
            return;
        }

        let n = self.replay_speed.max(1);
        let end = (self.replay_pos + n).min(self.replay_all.len());
        self.samples
            .extend_from_slice(&self.replay_all[self.replay_pos..end]);
        self.replay_pos = end;

        self.t = self.samples.last().map(|s| s.t).unwrap_or(self.t);

        if self.replay_pos >= self.replay_all.len() {
            self.replay_playing = false;
        }
    }

    fn scram_time_for_plot(&self) -> Option<f64> {
        if self.replay_loaded {
            self.replay_all.iter().find(|s| s.scram).map(|s| s.t)
        } else {
            self.samples.iter().find(|s| s.scram).map(|s| s.t)
        }
    }

    fn scram_now(&self) -> bool {
        if self.replay_loaded {
            self.samples.last().map(|s| s.scram).unwrap_or(false)
        } else {
            self.safety_state.scram
        }
    }

    fn reason_text(&self) -> String {
        if let Some(r) = self.replay_reason.as_ref() {
            return r.clone();
        }
        self.safety_state
            .reason
            .as_ref()
            .map(|r| format!("{r:?}"))
            .unwrap_or_else(|| "—".to_string())
    }

    fn step_once_live(&mut self) {
        if self.step_count >= self.max_steps {
            self.running = false;
            return;
        }

        let y1 = self.s1.read_temp(self.plant_x.temp_c, self.dt_s);
        let y2 = self.s2.read_temp(self.plant_x.temp_c, self.dt_s);
        let y3 = self.s3.read_temp(self.plant_x.temp_c, self.dt_s);

        safety::evaluate(&self.safety_cfg, &mut self.safety_state, [y1, y2, y3]);

        if self.safety_state.scram {
            self.plant_x.power = 0.0;
        } else {
            let mut sum = 0.0;
            let mut n = 0.0;
            for y in [y1, y2, y3] {
                if y.is_finite() {
                    sum += y;
                    n += 1.0;
                }
            }

            let meas = if n > 0.0 {
                sum / n
            } else {
                self.plant_x.temp_c
            };
            let u = self.pid.update(self.setpoint, meas, self.dt_s);
            self.plant_x.power = u.clamp(0.0, 1.0);
        }

        if self.scenario == Scenario::LossOfCooling && self.t > (self.seconds * 0.3) {
            self.plant_x.coolant = 0.05;
        }

        self.plant_x.step(&self.plant_p, self.dt_s);

        self.samples.push(Sample {
            t: self.t,
            true_temp: self.plant_x.temp_c,
            s1: y1,
            s2: y2,
            s3: y3,
            power: self.plant_x.power,
            coolant: self.plant_x.coolant,
            scram: self.safety_state.scram,
        });

        self.t += self.dt_s;
        self.step_count += 1;

        if self.safety_state.scram {
            self.running = false;
        }
    }

    fn do_step_replay(&mut self) {
        if !self.replay_loaded {
            return;
        }
        if self.replay_pos >= self.replay_all.len() {
            self.replay_playing = false;
            return;
        }
        let end = (self.replay_pos + 1).min(self.replay_all.len());
        self.samples
            .extend_from_slice(&self.replay_all[self.replay_pos..end]);
        self.replay_pos = end;
        self.t = self.samples.last().map(|s| s.t).unwrap_or(self.t);
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.replay_tick();
        if self.replay_playing {
            ctx.request_repaint();
        }

        let mode_txt = if self.replay_loaded { "REPLAY" } else { "LIVE" };
        let scram_now = self.scram_now();
        let scram_time = self.scram_time_for_plot();
        let reason_txt = self.reason_text();

        egui::TopBottomPanel::top("top").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Reactor Safety Sim (Portfolio GUI)");
                ui.separator();
                ui.label(format!("MODE: {mode_txt}"));
                ui.separator();

                let label = if scram_now { "SCRAM: ON" } else { "SCRAM: OFF" };
                let color = if scram_now {
                    egui::Color32::RED
                } else {
                    egui::Color32::GREEN
                };
                ui.colored_label(color, label);

                if let Some(t) = scram_time {
                    ui.separator();
                    ui.label(format!("t_scram = {:.2}s", t));
                    ui.separator();
                    ui.label(format!("reason = {reason_txt}"));
                }
            });
        });

        egui::SidePanel::left("left")
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("Scenario");

                if self.replay_loaded {
                    ui.add_enabled(false, egui::Label::new(self.scenario.label()));
                } else {
                    let mut scenario_new = self.scenario;
                    egui::ComboBox::from_id_salt("scenario")
                        .selected_text(self.scenario.label())
                        .show_ui(ui, |ui| {
                            for s in [
                                Scenario::Normal,
                                Scenario::Overheat,
                                Scenario::LossOfCooling,
                                Scenario::SensorDisagree,
                            ] {
                                ui.selectable_value(&mut scenario_new, s, s.label());
                            }
                        });

                    if scenario_new != self.scenario {
                        self.scenario = scenario_new;
                        self.reset_live();
                    }
                }

                ui.separator();
                ui.label("Simulation settings");

                let live_enabled = !self.replay_loaded;
                ui.add_enabled(
                    live_enabled,
                    egui::Slider::new(&mut self.seconds, 10.0..=300.0).text("seconds"),
                );
                ui.add_enabled(
                    live_enabled,
                    egui::Slider::new(&mut self.dt_ms, 10..=200).text("dt (ms)"),
                );
                ui.add_enabled(
                    live_enabled,
                    egui::Slider::new(&mut self.setpoint, 100.0..=600.0).text("setpoint (°C)"),
                );
                ui.add_enabled(
                    live_enabled,
                    egui::Slider::new(&mut self.trip_temp, 200.0..=900.0).text("trip temp (°C)"),
                );
                ui.add_enabled(
                    live_enabled,
                    egui::DragValue::new(&mut self.seed).prefix("seed: "),
                );

                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Reset").clicked() {
                        self.reset();
                    }

                    let run_label = if self.running { "Pause" } else { "Run" };
                    if ui
                        .add_enabled(live_enabled, egui::Button::new(run_label))
                        .clicked()
                    {
                        if !self.running {
                            self.clear_replay();
                            self.reset_live();
                            self.apply_scenario();
                            self.running = true;
                        } else {
                            self.running = false;
                        }
                    }

                    if ui
                        .add_enabled(live_enabled, egui::Button::new("Step"))
                        .clicked()
                    {
                        self.step_once_live();
                    }
                });

                ui.separator();
                ui.label("Replay (JSONL)");
                ui.horizontal(|ui| {
                    ui.label("path:");
                    ui.text_edit_singleline(&mut self.replay_path);
                });

                ui.horizontal(|ui| {
                    if ui.button("Load").clicked() {
                        let p = self.replay_path.clone();
                        self.load_jsonl(&p);
                    }

                    if ui
                        .button(if self.replay_playing {
                            "Pause replay"
                        } else {
                            "Play replay"
                        })
                        .clicked()
                        && self.replay_loaded
                    {
                        self.replay_playing = !self.replay_playing;
                        ctx.request_repaint();
                    }

                    if ui.button("Step replay").clicked() && self.replay_loaded {
                        self.do_step_replay();
                    }
                });

                ui.add(
                    egui::Slider::new(&mut self.replay_speed, 1..=500)
                        .text("replay speed (samples/frame)"),
                );

                if self.replay_loaded {
                    ui.small(format!(
                        "Loaded: {}/{} samples",
                        self.samples.len(),
                        self.replay_all.len()
                    ));
                } else {
                    ui.small("No replay loaded.");
                }

                if let Some(err) = &self.last_error {
                    ui.separator();
                    ui.colored_label(egui::Color32::RED, err);
                }

                ui.separator();
                ui.small("Tip: Use make_demo.ps1 then Load out/demo_overheat.jsonl.");
            });

        egui::CentralPanel::default().show(ctx, |ui| {
            if self.running && !self.replay_loaded {
                for _ in 0..5 {
                    if !self.running {
                        break;
                    }
                    self.step_once_live();
                }
                ctx.request_repaint();
            }

            if self.samples.is_empty() {
                ui.label("No data yet. Run LIVE or Load a REPLAY file.");
                return;
            }

            let t_end = self.samples.last().map(|s| s.t).unwrap_or(0.0);

            let temp_points: PlotPoints = self.samples.iter().map(|s| [s.t, s.true_temp]).collect();
            let power_points: PlotPoints = self.samples.iter().map(|s| [s.t, s.power]).collect();
            let cool_points: PlotPoints = self.samples.iter().map(|s| [s.t, s.coolant]).collect();

            let s1_points: PlotPoints = self.samples.iter().map(|s| [s.t, s.s1]).collect();
            let s2_points: PlotPoints = self.samples.iter().map(|s| [s.t, s.s2]).collect();
            let s3_points: PlotPoints = self.samples.iter().map(|s| [s.t, s.s3]).collect();

            let mut y_min = f64::INFINITY;
            let mut y_max = f64::NEG_INFINITY;
            for s in &self.samples {
                for y in [s.true_temp, s.s1, s.s2, s.s3] {
                    if y.is_finite() {
                        y_min = y_min.min(y);
                        y_max = y_max.max(y);
                    }
                }
            }
            if !y_min.is_finite() || !y_max.is_finite() {
                y_min = 0.0;
                y_max = 1.0;
            }
            let pad = ((y_max - y_min).abs() * 0.10).max(1.0);
            let y0 = y_min - pad;
            let y1 = y_max + pad;

            ui.heading("Traces");
            Plot::new("temp_plot").height(240.0).show(ui, |plot_ui| {
                plot_ui.line(Line::new(temp_points).name("Temp (°C)"));
                plot_ui.line(Line::new(s1_points).name("Sensor 1"));
                plot_ui.line(Line::new(s2_points).name("Sensor 2"));
                plot_ui.line(Line::new(s3_points).name("Sensor 3"));

                if !self.replay_loaded && t_end > 0.0 {
                    let setpoint_line: PlotPoints =
                        vec![[0.0, self.setpoint], [t_end, self.setpoint]].into();
                    let trip_line: PlotPoints =
                        vec![[0.0, self.trip_temp], [t_end, self.trip_temp]].into();
                    plot_ui.line(Line::new(setpoint_line).name("Setpoint"));
                    plot_ui.line(Line::new(trip_line).name("Trip temp"));
                }

                if let Some(t) = scram_time {
                    let vline: PlotPoints = vec![[t, y0], [t, y1]].into();
                    plot_ui.line(Line::new(vline).name("SCRAM"));
                }
            });

            Plot::new("act_plot").height(180.0).show(ui, |plot_ui| {
                plot_ui.line(Line::new(power_points).name("Power (0..1)"));
                plot_ui.line(Line::new(cool_points).name("Coolant (0..1)"));
            });

            ui.separator();
            let last = self.samples.last().unwrap();
            ui.label(format!(
                "t={:.2}s  temp={:.2}°C  power={:.2}  coolant={:.2}",
                last.t, last.true_temp, last.power, last.coolant
            ));
        });
    }
}

fn main() -> eframe::Result<()> {
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "Reactor Safety Sim (Portfolio GUI)",
        native_options,
        Box::new(|_cc| Ok(Box::new(App::default()))),
    )
}
