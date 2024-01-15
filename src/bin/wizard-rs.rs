#![windows_subsystem = "windows"]
use strum::IntoEnumIterator;

use eframe::egui::{self, DragValue, Slider};
use wizard_rs::bulb::Bulb;
use wizard_rs::pilot::{Method, Pilot};
use wizard_rs::program::Action;
use wizard_rs::scenes::Scene;
use wizard_rs::wizard::Wizard;

fn main() -> Result<(), eframe::Error> {
    // create eframe window
    let mut options = eframe::NativeOptions::default();
    options.persist_window = true;

    eframe::run_native("WiZard", options, Box::new(|_cc| Box::<App>::default()))
}

#[derive(serde::Serialize, serde::Deserialize)]
struct Config {
    bulbs: Vec<Bulb>,
    selected: Option<usize>,
    program: Vec<Action>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            bulbs: Vec::new(),
            selected: None,
            program: Vec::new(),
        }
    }
}

struct App {
    wiz: Wizard,
    selected: Option<usize>,
    pilot: Pilot,
    config_path: std::path::PathBuf,
    program: Vec<Action>,
}

impl App {
    fn load_config(&mut self) {
        let file = std::fs::File::open(&self.config_path);
        if let Ok(file) = file {
            let config: Config = serde_json::from_reader(file).unwrap_or_default();
            let bulbs: Vec<Bulb> = config.bulbs;
            self.selected = config.selected;
            *self.wiz.bulbs.lock().unwrap() = bulbs;
            self.program = config.program;
        }
    }

    fn save_config(&mut self) -> Result<(), ()> {
        let file = std::fs::File::create(&self.config_path);

        match file {
            Ok(file) => {
                let config = Config {
                    bulbs: self.wiz.bulbs.lock().unwrap().clone(),
                    selected: self.selected,
                    program: self.program.clone(),
                };

                if serde_json::to_writer(file, &config).is_err() {
                    return Err(());
                }
            }
            Err(_) => {
                return Err(());
            }
        }
        Ok(())
    }
}

impl Default for App {
    fn default() -> Self {
        // load bulbs from file

        let mut config_path = std::env::current_exe().unwrap();
        config_path.pop();
        config_path.push("bulbs.json");

        let mut app = Self {
            wiz: Wizard::new(),
            selected: None,
            pilot: Pilot::default(),
            config_path: config_path,
            program: Vec::new(),
        };

        app.load_config();

        app
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.menu_button("File", |ui| {
                if ui.button("Save").clicked() {
                    if self.save_config().is_err() {
                        // TODO: notify user of error somehow
                    }
                    ui.close_menu();
                }
            })
        });

        egui::Window::new("Bulbs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Discover").clicked() {
                    self.selected = None;
                    self.wiz.discover();
                }

                if let Some(idx) = self.selected {
                    let bulb = &self.wiz.bulbs.lock().unwrap()[idx];
                    ui.label(format!("{} {}", &bulb.name, &bulb.ip));
                }
            });

            for (idx, bulb) in self.wiz.bulbs.lock().unwrap().iter_mut().enumerate() {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(&mut bulb.name);
                    if ui.button("select").clicked() {
                        self.selected = Some(idx);
                    }
                });
            }
        });

        egui::Window::new("Control").show(ctx, |ui| {
            if let Some(idx) = self.selected {
                let bulb = &self.wiz.bulbs.lock().unwrap()[idx];
                ui.label(format!("{} {}", &bulb.name, &bulb.ip));

                ui.horizontal(|ui| {
                    ui.label("Dimming");
                    let brightness = ui.add(Slider::new(&mut self.pilot.brightness, 0.1..=1.0));
                    if brightness.changed() {
                        let mut pilot = Pilot::new(Method::SetPilot);
                        pilot.set_brightness(self.pilot.brightness);
                        self.wiz.set_pilot(bulb.clone(), pilot);
                    }
                });

                ui.horizontal(|ui| {
                    ui.label("Speed");
                    let _speed = ui.add(Slider::new(&mut self.pilot.speed, 0.2..=2.0));
                });

                ui.horizontal(|ui| {
                    if ui.button("on").clicked() {
                        let mut pilot = Pilot::new(Method::SetPilot);
                        pilot.set_state(true);
                        pilot.set_brightness(self.pilot.brightness);
                        self.wiz.set_pilot(bulb.clone(), pilot);
                    }
                    if ui.button("off").clicked() {
                        let mut pilot = Pilot::new(Method::SetPilot);
                        pilot.set_state(false);
                        self.wiz.set_pilot(bulb.clone(), pilot);
                    }
                });

                let mut rgb = self.pilot.rgb.unwrap_or([0.0, 0.0, 255.0]);
                let color_selector = ui.color_edit_button_rgb(&mut rgb);
                if color_selector.changed() {
                    self.pilot.rgb = Some(rgb);
                    self.wiz.set_pilot(bulb.clone(), self.pilot.clone());
                }
            }
        });

        egui::Window::new("Scenes").vscroll(true).show(ctx, |ui| {
            if let Some(idx) = self.selected {
                let bulb = &self.wiz.bulbs.lock().unwrap()[idx];
                ui.label(format!("{} {}", &bulb.name, &bulb.ip));

                for scene in Scene::iter() {
                    if ui.button(scene.to_string()).clicked() {
                        let mut pilot = Pilot::new(Method::SetPilot);
                        pilot.set_scene(scene);
                        pilot.set_brightness(self.pilot.brightness);
                        pilot.set_speed(self.pilot.speed);
                        self.wiz.set_pilot(bulb.clone(), pilot);
                    }
                }
            }
        });

        egui::Window::new("Daemon").vscroll(true).show(ctx, |ui| {
            let daemon = self.wiz.daemon.clone();

            ui.label(format!(
                "Daemon: {}",
                match daemon.lock().unwrap().is_some() {
                    true => "connected",
                    false => "not connected",
                }
            ));

            ui.horizontal(|ui| {
                if ui.button("connect").clicked() {
                    self.wiz.daemon_connect();
                };

                if ui.button("shutdown").clicked() {
                    self.wiz.daemon_shutdown();
                }
            });

            ui.separator();

            if ui.button("run").clicked() {
                if !self.program.is_empty() {
                    let bulb_ip = self.wiz.bulbs.lock().unwrap()[self.selected.unwrap()]
                        .ip
                        .clone();
                    self.wiz.daemon_run_program(self.program.clone(), bulb_ip);
                }
            }

            ui.separator();

            let mut to_delete: Option<usize> = None;
            let mut to_swap: Option<(usize, usize)> = None;
            let program_len = self.program.len();
            for (idx, action) in self.program.iter_mut().enumerate() {
                match action {
                    Action::Sleep(s) => {
                        ui.horizontal(|ui| {
                            ui.label(format!("sleep: "));

                            ui.add(DragValue::new(s).speed(0.1).clamp_range(0.0..=10.0));

                            if ui.button("remove").clicked() {
                                to_delete = Some(idx);
                            }
                        });

                        ui.horizontal(|ui| {
                            if ui.button("up").clicked() {
                                if idx > 0 {
                                    to_swap = Some((idx, idx - 1));
                                }
                            }

                            if ui.button("down").clicked() {
                                if idx < program_len - 1 {
                                    to_swap = Some((idx, idx + 1));
                                }
                            }
                        });
                    }
                    Action::SetPilot(p) => {
                        ui.label(format!("set pilot"));

                        ui.checkbox(&mut p.state, "state");

                        ui.horizontal(|ui| {
                            if ui.button("rgb").clicked() {
                                if p.rgb.is_none() {
                                    p.rgb = Some([0.0, 0.0, 255.0]);
                                } else {
                                    p.rgb = None;
                                }
                            }

                            if let Some(rgb) = &mut p.rgb {
                                ui.color_edit_button_rgb(rgb);
                            }
                        });

                        ui.horizontal(|ui| {
                            ui.label("brightness");
                            ui.add(Slider::new(&mut p.brightness, 0.1..=1.0));
                        });

                        ui.horizontal(|ui| {
                            if ui.button("up").clicked() {
                                if idx > 0 {
                                    to_swap = Some((idx, idx - 1));
                                }
                            }

                            if ui.button("down").clicked() {
                                if idx < program_len - 1 {
                                    to_swap = Some((idx, idx + 1));
                                }
                            }
                        });
                    }
                }
                ui.separator();
            }

            if let Some(idx) = to_delete {
                self.program.remove(idx);
            }

            if let Some((idx1, idx2)) = to_swap {
                self.program.swap(idx1, idx2);
            }

            ui.menu_button("add", |ui| {
                if ui.button("sleep").clicked() {
                    self.program.push(Action::Sleep(1));
                    ui.close_menu();
                }

                if ui.button("set pilot").clicked() {
                    self.program.push(Action::SetPilot(Pilot::default()));
                    ui.close_menu();
                }
            });
        });
    }
}
