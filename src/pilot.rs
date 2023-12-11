use serde_json::{Map, Value};

use crate::scenes::Scene;

#[derive(Debug, Clone)]
pub enum Method {
    SetPilot,
    GetDevInfo,
}

impl Method {
    pub fn to_string(&self) -> String {
        match self {
            Method::SetPilot => String::from("setPilot"),
            Method::GetDevInfo => String::from("getDevInfo"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct Pilot {
    pub method: Method,
    pub state: bool,
    pub rgb: Option<[f32; 3]>,
    pub scene: Option<Scene>,
    pub brightness: f32,
}

impl Pilot {
    pub fn new(method: Method) -> Pilot {
        Pilot {
            method: method,
            state: true,
            rgb: None,
            scene: None,
            brightness: 100.0,
        }
    }

    pub fn set_method(&mut self, method: Method) {
        self.method = method;
    }

    pub fn set_state(&mut self, state: bool) {
        self.state = state;
    }

    pub fn set_rgb(&mut self, r: u8, g: u8, b: u8) {
        self.rgb = Some([r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0]);
    }

    pub fn set_scene(&mut self, scene: Scene) {
        self.scene = Some(scene);
    }

    pub fn set_brightness(&mut self, brightness: f32) {
        self.brightness = brightness;
    }

    pub fn build(&self) -> String {
        let mut map = Map::new();
        map.insert(
            String::from("method"),
            Value::String(self.method.to_string()),
        );

        match self.method {
            Method::GetDevInfo => {}
            Method::SetPilot => {
                let mut params = Map::new();
                params.insert(String::from("state"), Value::Bool(self.state));

                if self.state {
                    params.insert(
                        String::from("dimming"),
                        Value::Number(((self.brightness * 100.0) as u8).into()),
                    );
                }
                if let Some([r, g, b]) = self.rgb {
                    let r = (r * 255.0) as u8;
                    let g = (g * 255.0) as u8;
                    let b = (b * 255.0) as u8;

                    params.insert(String::from("r"), Value::Number(r.into()));
                    params.insert(String::from("g"), Value::Number(g.into()));
                    params.insert(String::from("b"), Value::Number(b.into()));
                }

                if let Some(scene) = self.scene {
                    params.insert(
                        String::from("sceneId"),
                        Value::Number((scene as i32).into()),
                    );
                }

                map.insert(String::from("params"), Value::Object(params));
            }
        }

        let json = Value::Object(map);
        json.to_string()
    }
}

impl Default for Pilot {
    fn default() -> Self {
        Pilot::new(Method::SetPilot)
    }
}
