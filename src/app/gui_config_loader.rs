use std::collections::HashSet;
use std::path::Path;

use crate::app::gui_config_model::{GuiConfigFile, GuiParamKind};

const SUPPORTED_SCHEMA_VERSION: u32 = 1;
const CONFIG_FILE_NAME: &str = "gui.toml";

pub struct GuiConfigLoader;

impl GuiConfigLoader {
    pub fn load() -> GuiConfigFile {
        let config_path = Self::config_path();

        if !config_path.exists() {
            panic!(
                "GUI config file not found: {}\n\
                 Please ensure {} exists in the config directory.",
                config_path.display(),
                CONFIG_FILE_NAME
            );
        }

        let content = std::fs::read_to_string(&config_path).unwrap_or_else(|e| {
            panic!(
                "Failed to read GUI config file at {}: {}",
                config_path.display(),
                e
            );
        });

        let config: GuiConfigFile = toml::from_str(&content).unwrap_or_else(|e| {
            panic!(
                "Failed to parse GUI config at {}:\n{}",
                config_path.display(),
                e
            );
        });

        Self::validate(&config, &config_path);

        log::info!(
            "Loaded GUI config: {} (schema v{}, {} sections, {} params)",
            config_path.display(),
            config.schema_version,
            config.section.len(),
            config.section.iter().map(|s| s.param.len()).sum::<usize>()
        );

        config
    }

    fn config_path() -> std::path::PathBuf {
        let project_root = env!("PROJECT_ROOT");
        Path::new(project_root)
            .join("config")
            .join(CONFIG_FILE_NAME)
    }

    fn validate(config: &GuiConfigFile, path: &Path) {
        let mut errors = Vec::new();

        if config.schema_version != SUPPORTED_SCHEMA_VERSION {
            errors.push(format!(
                "Unsupported schema version: {} (supported: {})",
                config.schema_version, SUPPORTED_SCHEMA_VERSION
            ));
        }

        let mut section_names = HashSet::new();
        let mut param_ids = HashSet::new();

        for (section_idx, section) in config.section.iter().enumerate() {
            if section.name.is_empty() {
                errors.push(format!("Section at index {} has empty name", section_idx));
            }

            if !section_names.insert(section.name.clone()) {
                errors.push(format!("Duplicate section name: '{}'", section.name));
            }

            for (param_idx, param) in section.param.iter().enumerate() {
                if param.id.is_empty() {
                    errors.push(format!(
                        "Section '{}' param at index {} has empty id",
                        section.name, param_idx
                    ));
                }

                if !param_ids.insert(param.id.clone()) {
                    errors.push(format!(
                        "Duplicate param id: '{}' in section '{}'",
                        param.id, section.name
                    ));
                }

                Self::validate_param(&mut errors, &section.name, &param, param_idx);
            }
        }

        if !errors.is_empty() {
            let mut msg = format!("GUI config validation failed for {}:\n", path.display());
            for error in errors {
                msg.push_str(&format!("  - {}\n", error));
            }
            panic!("{}", msg);
        }
    }

    fn validate_param(
        errors: &mut Vec<String>,
        section_name: &str,
        param: &crate::app::gui_config_model::GuiParam,
        _param_idx: usize,
    ) {
        use crate::app::gui_config_model::GuiParamValue;

        match (&param.kind, &param.value) {
            (GuiParamKind::Float, GuiParamValue::Float { value, min, max }) => {
                if let (Some(min), Some(max)) = (min, max) {
                    if min > max {
                        errors.push(format!(
                            "Section '{}' param '{}': min ({}) > max ({})",
                            section_name, param.id, min, max
                        ));
                    }
                }
                if let Some(min) = min {
                    if value < min {
                        errors.push(format!(
                            "Section '{}' param '{}': value ({}) < min ({})",
                            section_name, param.id, value, min
                        ));
                    }
                }
                if let Some(max) = max {
                    if value > max {
                        errors.push(format!(
                            "Section '{}' param '{}': value ({}) > max ({})",
                            section_name, param.id, value, max
                        ));
                    }
                }
            }
            (GuiParamKind::Int, GuiParamValue::Int { value, min, max }) => {
                if let (Some(min), Some(max)) = (min, max) {
                    if min > max {
                        errors.push(format!(
                            "Section '{}' param '{}': min ({}) > max ({})",
                            section_name, param.id, min, max
                        ));
                    }
                }
                if let Some(min) = min {
                    if value < min {
                        errors.push(format!(
                            "Section '{}' param '{}': value ({}) < min ({})",
                            section_name, param.id, value, min
                        ));
                    }
                }
                if let Some(max) = max {
                    if value > max {
                        errors.push(format!(
                            "Section '{}' param '{}': value ({}) > max ({})",
                            section_name, param.id, value, max
                        ));
                    }
                }
            }
            (GuiParamKind::Uint, GuiParamValue::Uint { value, min, max }) => {
                if let (Some(min), Some(max)) = (min, max) {
                    if min > max {
                        errors.push(format!(
                            "Section '{}' param '{}': min ({}) > max ({})",
                            section_name, param.id, min, max
                        ));
                    }
                }
                if let Some(min) = min {
                    if value < min {
                        errors.push(format!(
                            "Section '{}' param '{}': value ({}) < min ({})",
                            section_name, param.id, value, min
                        ));
                    }
                }
                if let Some(max) = max {
                    if value > max {
                        errors.push(format!(
                            "Section '{}' param '{}': value ({}) > max ({})",
                            section_name, param.id, value, max
                        ));
                    }
                }
            }
            (GuiParamKind::Bool, GuiParamValue::Bool { .. }) => {}
            (GuiParamKind::Color, GuiParamValue::Color { value }) => {
                if !Self::is_valid_color(value) {
                    errors.push(format!(
                        "Section '{}' param '{}': invalid color format '{}' (expected #RRGGBB or #RRGGBBAA)",
                        section_name, param.id, value
                    ));
                }
            }
            (kind, _value) => {
                let expected = match kind {
                    GuiParamKind::Float => "float { value, min, max }",
                    GuiParamKind::Int => "int { value, min, max }",
                    GuiParamKind::Uint => "uint { value, min, max }",
                    GuiParamKind::Bool => "bool { value }",
                    GuiParamKind::Color => "color { value }",
                };
                errors.push(format!(
                    "Section '{}' param '{}': wrong value type for kind '{}', expected {}",
                    section_name,
                    param.id,
                    match kind {
                        GuiParamKind::Float => "float",
                        GuiParamKind::Int => "int",
                        GuiParamKind::Uint => "uint",
                        GuiParamKind::Bool => "bool",
                        GuiParamKind::Color => "color",
                    },
                    expected
                ));
            }
        }
    }

    fn is_valid_color(s: &str) -> bool {
        if s.len() != 7 && s.len() != 9 {
            return false;
        }
        if !s.starts_with('#') {
            return false;
        }
        s[1..].chars().all(|c| c.is_ascii_hexdigit())
    }
}
