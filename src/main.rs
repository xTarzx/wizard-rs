use strum::IntoEnumIterator;

use bulb::Bulb;
use eframe::egui::{self, Slider};
use pilot::{Method, Pilot};
use scenes::Scene;
use wizard::Wizard;

pub mod bulb;
pub mod pilot;
pub mod scenes;
pub mod wizard;

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
}

struct App {
    wiz: Wizard,
    selected: Option<usize>,
    pilot: Pilot,
    config_path: std::path::PathBuf,
}

impl App {
    fn load_config(&mut self) {
        let file = std::fs::File::open(&self.config_path);
        if let Ok(file) = file {
            let config: Config = serde_json::from_reader(file).unwrap();
            let bulbs: Vec<Bulb> = config.bulbs;
            self.selected = config.selected;
            *self.wiz.bulbs.lock().unwrap() = bulbs;
        }
    }

    fn save_config(&mut self) {
        let file = std::fs::File::create(&self.config_path);
        if let Ok(file) = file {
            let config = Config {
                bulbs: self.wiz.bulbs.lock().unwrap().clone(),
                selected: self.selected,
            };
            serde_json::to_writer(file, &config).unwrap();
        }
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
                    self.save_config();
                    ui.close_menu();
                }
            })
        });

        egui::Window::new("Bulbs").show(ctx, |ui| {
            ui.horizontal(|ui| {
                if ui.button("Discover").clicked() {
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

                let brightness = ui.add(Slider::new(&mut self.pilot.brightness, 0.1..=1.0));

                if brightness.changed() {
                    let mut pilot = Pilot::new(Method::SetPilot);
                    pilot.set_brightness(self.pilot.brightness);
                    self.wiz.set_pilot(bulb.clone(), pilot);
                }

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
                        self.wiz.set_pilot(bulb.clone(), pilot);
                    }
                }
            }
        });
    }
}
