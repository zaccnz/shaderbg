use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
pub enum Setting {
    Float {
        label: String,
        value: f32,
        min: f32,
        max: f32,
    },
    Colour3 {
        label: String,
        value: String,
    },
}
