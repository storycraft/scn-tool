use serde::{Deserialize, Deserializer, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Script {
    pub scenes: Vec<Scene>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub title: String,

    #[serde(default)]
    pub texts: Vec<Text>,

    #[serde(default, deserialize_with = "deserialize_selects")]
    pub selects: Vec<Select>,
}

fn deserialize_selects<'de, D>(de: D) -> Result<Vec<Select>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    struct Wrapper {
        #[serde(rename = "selectInfo")]
        #[serde(default)]
        select_info: Wrapper2,
    }
    #[derive(Deserialize, Default)]
    struct Wrapper2 {
        #[serde(default)]
        selects: Vec<Select>,
    }

    Ok(Wrapper::deserialize(de)?.select_info.selects)
}

#[derive(Debug, Clone, Serialize)]
pub struct Text {
    pub name: Option<String>,
    pub display_name: Option<String>,
    pub text: String,
}

impl<'de> Deserialize<'de> for Text {
    fn deserialize<D>(de: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct Map {
            pub name: Option<String>,
            pub display_name: Option<String>,
            pub text: String,
        }
        impl From<Map> for Text {
            fn from(v: Map) -> Self {
                Self {
                    name: v.name,
                    display_name: v.display_name,
                    text: v.text,
                }
            }
        }

        #[derive(serde_query::Deserialize)]
        struct Seq {
            #[query(".[0]")]
            pub name: Option<String>,
            #[query(".[1]")]
            pub display_name: Option<String>,
            #[query(".[2]")]
            pub text: String,
        }
        impl From<Seq> for Text {
            fn from(v: Seq) -> Self {
                Self {
                    name: v.name,
                    display_name: v.display_name,
                    text: v.text,
                }
            }
        }

        #[derive(Deserialize)]
        #[serde(untagged)]
        enum Variant {
            Seq(Seq),
            Map(Map),
        }

        Ok(match Variant::deserialize(de)? {
            Variant::Seq(v) => v.into(),
            Variant::Map(v) => v.into(),
        })
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct Select {
    pub text: String,
}
