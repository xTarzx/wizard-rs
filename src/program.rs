use crate::pilot::Pilot;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Action {
    Sleep(u64),
    SetPilot(Pilot),
}
