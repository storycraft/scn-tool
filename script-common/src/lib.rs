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
    pub selects: Vec<Select>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Text {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Select {
    pub text: String,
}
