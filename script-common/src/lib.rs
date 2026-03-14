use emote_psb::value::PsbValue;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub title: String,

    #[serde(default)]
    pub texts: Vec<Text>,

    #[serde(default)]
    pub selects: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub name: Option<String>,

    #[serde(default)]
    pub dialogues: Vec<Dialogue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dialogue {
    pub display_name: Option<String>,
    pub values: Vec<PsbValue>,
}
