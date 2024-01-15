use serde::{Deserialize, Serialize};

use crate::program::Action;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Msg {
    Stop,
    Run(Vec<Action>, String),
    Ignore,
}

pub const DAEMONNAME: &str = "wizarddaemon";
