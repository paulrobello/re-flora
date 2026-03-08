use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct GuiConfigFile {
    pub schema_version: u32,
    pub section: Vec<GuiSection>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GuiSection {
    pub name: String,
    pub param: Vec<GuiParam>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GuiParam {
    pub id: String,
    pub kind: GuiParamKind,
    pub label: String,
    #[serde(flatten)]
    pub value: GuiParamValue,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum GuiParamKind {
    Float,
    Int,
    Uint,
    Bool,
    Color,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum GuiParamValue {
    Float {
        value: f32,
        #[serde(default)]
        min: Option<f32>,
        #[serde(default)]
        max: Option<f32>,
    },
    Int {
        value: i32,
        #[serde(default)]
        min: Option<i32>,
        #[serde(default)]
        max: Option<i32>,
    },
    Uint {
        value: u32,
        #[serde(default)]
        min: Option<u32>,
        #[serde(default)]
        max: Option<u32>,
    },
    Bool {
        value: bool,
    },
    Color {
        value: String,
    },
}

impl GuiParamValue {
    pub fn get_float(&self) -> Option<(f32, Option<f32>, Option<f32>)> {
        match self {
            GuiParamValue::Float { value, min, max } => Some((*value, *min, *max)),
            _ => None,
        }
    }

    pub fn get_int(&self) -> Option<(i32, Option<i32>, Option<i32>)> {
        match self {
            GuiParamValue::Int { value, min, max } => Some((*value, *min, *max)),
            _ => None,
        }
    }

    pub fn get_uint(&self) -> Option<(u32, Option<u32>, Option<u32>)> {
        match self {
            GuiParamValue::Uint { value, min, max } => Some((*value, *min, *max)),
            _ => None,
        }
    }

    pub fn get_bool(&self) -> Option<bool> {
        match self {
            GuiParamValue::Bool { value } => Some(*value),
            _ => None,
        }
    }

    pub fn get_color(&self) -> Option<String> {
        match self {
            GuiParamValue::Color { value } => Some(value.clone()),
            _ => None,
        }
    }
}
