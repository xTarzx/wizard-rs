use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Bulb {
    pub ip: String,
    pub name: String,
    pub mac: String,
}

impl Bulb {
    pub fn new(ip: String, name: String, mac: String) -> Bulb {
        Bulb { ip, name, mac }
    }

    pub fn parse(ip: String, data: &str) -> Bulb {
        // example of data: {"method":"getDevInfo","env":"pro","result":{"mac":"a8bb50ec140e","devMac":"a8bb50ec140e","moduleName":"ESP03_SHRGB1C_01"}}
        // parse data as json and extract
        // mac
        let data = data.trim_start_matches(char::from(0));
        let data = data.trim_end_matches(char::from(0));
        let v: Value = serde_json::from_str(data).unwrap();
        let mac = v["result"]["mac"].to_string();
        // clean up mac
        let mac = mac.trim_start_matches('"');
        let mac = mac.trim_end_matches('"');
        let mac = mac.to_string();

        Bulb {
            ip: ip,
            name: mac.clone(),
            mac: mac.clone(),
        }
    }
}
