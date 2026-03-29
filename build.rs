use std::collections::HashSet;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

#[macro_export]
macro_rules! log {
    ($($arg:tt)*) => {{
        let message = format!($($arg)*);
        println!("cargo:warning={}", message);
    }};
}

fn dump_env() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set");
    let target_dir = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| {
        let default = Path::new(&manifest_dir).join("target");
        default.to_str().unwrap().to_owned()
    });
    println!("cargo:rustc-env=PROJECT_ROOT={}/", manifest_dir);
    println!("cargo:rustc-env=TARGET_DIR={}/", target_dir);
}

fn project_root() -> PathBuf {
    PathBuf::from(env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR is not set"))
}

fn kind_to_type(kind: &str, id: &str) -> &'static str {
    match kind {
        "float" => "crate::gui_adjustables::FloatParam",
        "int" => "crate::gui_adjustables::IntParam",
        "uint" => "crate::gui_adjustables::UintParam",
        "bool" => "crate::gui_adjustables::BoolParam",
        "color" => "crate::gui_adjustables::ColorParam",
        other => panic!(
            "GUI config generation failed: unsupported kind '{}' for param '{}'",
            other, id
        ),
    }
}

fn generate_gui_adjustables() {
    let root = project_root();
    let config_path = root.join("config").join("gui.toml");

    let content = fs::read_to_string(&config_path).unwrap_or_else(|e| {
        panic!(
            "GUI config generation failed: unable to read {}: {}",
            config_path.display(),
            e
        )
    });

    let parsed: toml::Value = content.parse().unwrap_or_else(|e| {
        panic!(
            "GUI config generation failed: unable to parse {}: {}",
            config_path.display(),
            e
        )
    });

    let schema_version = parsed
        .get("schema_version")
        .and_then(|v| v.as_integer())
        .unwrap_or_else(|| {
            panic!(
                "GUI config generation failed: missing or invalid integer schema_version in {}",
                config_path.display()
            )
        });

    let mut descriptors: Vec<(String, String, String, String)> = Vec::new();
    let mut seen_sections = HashSet::new();
    let mut seen_ids = HashSet::new();
    let sections = parsed
        .get("section")
        .and_then(|v| v.as_array())
        .unwrap_or_else(|| {
            panic!(
                "GUI config generation failed: missing or invalid [[section]] array in {}",
                config_path.display()
            )
        });

    for (section_idx, section) in sections.iter().enumerate() {
        let table = section.as_table().unwrap_or_else(|| {
            panic!(
                "GUI config generation failed: section at index {} is not a table",
                section_idx
            )
        });

        let section_name = table
            .get("name")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| {
                panic!(
                    "GUI config generation failed: section at index {} has missing/empty name",
                    section_idx
                )
            })
            .to_owned();

        if !seen_sections.insert(section_name.clone()) {
            panic!(
                "GUI config generation failed: duplicate section name '{}'",
                section_name
            );
        }

        let params = table
            .get("param")
            .and_then(|v| v.as_array())
            .unwrap_or_else(|| {
                panic!(
                    "GUI config generation failed: section '{}' has missing/invalid param array",
                    section_name
                )
            });

        for (param_idx, param) in params.iter().enumerate() {
            let param_tbl = param.as_table().unwrap_or_else(|| {
                panic!(
                    "GUI config generation failed: section '{}' param at index {} is not a table",
                    section_name, param_idx
                )
            });

            let id = param_tbl
                .get("id")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|id| !id.is_empty())
                .unwrap_or_else(|| {
                    panic!(
                        "GUI config generation failed: section '{}' param at index {} missing/empty id",
                        section_name, param_idx
                    )
                })
                .to_owned();

            if !seen_ids.insert(id.clone()) {
                panic!("GUI config generation failed: duplicate param id '{}'", id);
            }

            let kind = param_tbl
                .get("kind")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|kind| !kind.is_empty())
                .unwrap_or_else(|| {
                    panic!(
                        "GUI config generation failed: section '{}' param '{}' missing/empty kind",
                        section_name, id
                    )
                })
                .to_owned();
            let _ = kind_to_type(&kind, &id);

            let label = param_tbl
                .get("label")
                .and_then(|v| v.as_str())
                .map(str::trim)
                .filter(|label| !label.is_empty())
                .unwrap_or_else(|| {
                    panic!(
                        "GUI config generation failed: section '{}' param '{}' missing/empty label",
                        section_name, id
                    )
                })
                .to_owned();

            descriptors.push((section_name.clone(), id, kind, label));
        }
    }

    let generated_dir = root.join("src").join("app").join("generated");
    fs::create_dir_all(&generated_dir).unwrap_or_else(|e| {
        panic!(
            "GUI config generation failed: unable to create {}: {}",
            generated_dir.display(),
            e
        )
    });

    let out_path = generated_dir.join("gui_adjustables_gen.rs");

    let mut code = String::new();
    code.push_str(
        "// ============================================================================\n",
    );
    code.push_str("// !!! DO NOT EDIT THIS FILE BY HAND !!!\n");
    code.push_str("// This file is generated at build time.\n");
    code.push_str("//\n");
    code.push_str("// generator: build.rs::generate_gui_adjustables\n");
    code.push_str("// source: config/gui.toml\n");
    code.push_str("//\n");
    code.push_str("// To regenerate this file, run a Cargo build command, for example:\n");
    code.push_str("//   cargo check\n");
    code.push_str(
        "// ============================================================================\n",
    );
    code.push_str("// @generated by build.rs - do not edit.\n");
    code.push_str("// This file reflects config/gui.toml at build time.\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str("pub const GENERATED_SCHEMA_VERSION: u32 = ");
    code.push_str(&schema_version.to_string());
    code.push_str(";\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str("pub struct GeneratedGuiParamDescriptor {\n");
    code.push_str("    pub section: &'static str,\n");
    code.push_str("    pub id: &'static str,\n");
    code.push_str("    pub kind: &'static str,\n");
    code.push_str("    pub label: &'static str,\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str("pub static GENERATED_GUI_PARAMS: &[GeneratedGuiParamDescriptor] = &[\n");

    for (section, id, kind, label) in &descriptors {
        code.push_str("    GeneratedGuiParamDescriptor {\n");
        code.push_str(&format!(
            "        section: \"{}\",\n",
            section.replace('"', "\\\"")
        ));
        code.push_str(&format!("        id: \"{}\",\n", id.replace('"', "\\\"")));
        code.push_str(&format!(
            "        kind: \"{}\",\n",
            kind.replace('"', "\\\"")
        ));
        code.push_str(&format!(
            "        label: \"{}\",\n",
            label.replace('"', "\\\"")
        ));
        code.push_str("    },\n");
    }

    code.push_str("];\n\n");

    // generated struct with one field per GUI param
    code.push_str("#[allow(dead_code)]\n");
    code.push_str("pub struct GuiAdjustables {\n");
    for (_section, id, kind, _label) in &descriptors {
        let ty = kind_to_type(kind, id);

        code.push_str(&format!("    pub {}: {},\n", id, ty));
    }
    code.push_str("}\n\n");

    // Default implementation that loads the config file
    code.push_str("impl Default for GuiAdjustables {\n");
    code.push_str("    fn default() -> Self {\n");
    code.push_str("        let config = crate::app::gui_config_loader::GuiConfigLoader::load();\n");
    code.push_str("        Self::from_config(&config)\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // from_config constructor that materializes params from GuiConfigFile
    code.push_str("impl GuiAdjustables {\n");
    code.push_str(
        "    pub fn from_config(config: &crate::app::gui_config_model::GuiConfigFile) -> Self {\n",
    );
    code.push_str("        use crate::app::gui_config_model::{GuiParamKind, GuiParamValue};\n\n");

    // one local Option per field
    for (_section, id, kind, _label) in &descriptors {
        let ty = kind_to_type(kind, id);
        code.push_str(&format!(
            "        let mut {id}_field: Option<{ty}> = None;\n",
            id = id,
            ty = ty
        ));
    }

    code.push_str("\n        for section in &config.section {\n");
    code.push_str("            for param in &section.param {\n");
    code.push_str("                match param.id.as_str() {\n");

    for (_section, id, kind, _label) in &descriptors {
        code.push_str(&format!("                    \"{}\" => {{\n", id));
        match kind.as_str() {
            "float" => {
                code.push_str(
                    "                        if let (GuiParamKind::Float, GuiParamValue::Float { value, min, max }) = (&param.kind, &param.value) {\n",
                );
                code.push_str(
                    "                            let min = min.unwrap_or(0.0);\n                            let max = max.unwrap_or(1.0);\n",
                );
                code.push_str(&format!(
                    "                            {id}_field = Some(crate::gui_adjustables::FloatParam::new(*value, min..=max));\n",
                    id = id
                ));
                code.push_str("                        }\n");
            }
            "int" => {
                code.push_str(
                    "                        if let (GuiParamKind::Int, GuiParamValue::Int { value, min, max }) = (&param.kind, &param.value) {\n",
                );
                code.push_str(
                    "                            let min = min.unwrap_or(0);\n                            let max = max.unwrap_or(100);\n",
                );
                code.push_str(&format!(
                    "                            {id}_field = Some(crate::gui_adjustables::IntParam::new(*value, min..=max));\n",
                    id = id
                ));
                code.push_str("                        }\n");
            }
            "uint" => {
                code.push_str(
                    "                        if let (GuiParamKind::Uint, GuiParamValue::Uint { value, min, max }) = (&param.kind, &param.value) {\n",
                );
                code.push_str(
                    "                            let min = min.unwrap_or(0);\n                            let max = max.unwrap_or(100);\n",
                );
                code.push_str(&format!(
                    "                            {id}_field = Some(crate::gui_adjustables::UintParam::new(*value, min..=max));\n",
                    id = id
                ));
                code.push_str("                        }\n");
            }
            "bool" => {
                code.push_str(
                    "                        if let (GuiParamKind::Bool, GuiParamValue::Bool { value }) = (&param.kind, &param.value) {\n",
                );
                code.push_str(&format!(
                    "                            {id}_field = Some(crate::gui_adjustables::BoolParam::new(*value));\n",
                    id = id
                ));
                code.push_str("                        }\n");
            }
            "color" => {
                code.push_str(
                    "                        if let (GuiParamKind::Color, GuiParamValue::Color { value }) = (&param.kind, &param.value) {\n",
                );
                code.push_str(&format!(
                    "                            {id}_field = Some(crate::gui_adjustables::ColorParam::new(crate::app::gui_config::parse_color(value)));\n",
                    id = id
                ));
                code.push_str("                        }\n");
            }
            _ => {}
        }
        code.push_str("                    }\n");
    }

    code.push_str("                    _ => {}\n");
    code.push_str("                }\n");
    code.push_str("            }\n");
    code.push_str("        }\n\n");

    code.push_str("        GuiAdjustables {\n");
    for (_section, id, kind, _label) in &descriptors {
        let _ = kind_to_type(kind, id);
        code.push_str(&format!(
            "            {id}: {id}_field.expect(\"Missing parameter: {id}\"),\n",
            id = id
        ));
    }
    code.push_str("        }\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    // generated accessors operating on GuiAdjustables by id
    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_float_param<'a>(adjustables: &'a crate::app::GuiAdjustables, id: &str) -> Option<&'a crate::gui_adjustables::FloatParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "float" {
            code.push_str(&format!(
                "        \"{}\" => Some(&adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_int_param<'a>(adjustables: &'a crate::app::GuiAdjustables, id: &str) -> Option<&'a crate::gui_adjustables::IntParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "int" {
            code.push_str(&format!(
                "        \"{}\" => Some(&adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_uint_param<'a>(adjustables: &'a crate::app::GuiAdjustables, id: &str) -> Option<&'a crate::gui_adjustables::UintParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "uint" {
            code.push_str(&format!(
                "        \"{}\" => Some(&adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_bool_param<'a>(adjustables: &'a crate::app::GuiAdjustables, id: &str) -> Option<&'a crate::gui_adjustables::BoolParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "bool" {
            code.push_str(&format!(
                "        \"{}\" => Some(&adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_color_param<'a>(adjustables: &'a crate::app::GuiAdjustables, id: &str) -> Option<&'a crate::gui_adjustables::ColorParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "color" {
            code.push_str(&format!(
                "        \"{}\" => Some(&adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_float_param_mut<'a>(adjustables: &'a mut crate::app::GuiAdjustables, id: &str) -> Option<&'a mut crate::gui_adjustables::FloatParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "float" {
            code.push_str(&format!(
                "        \"{}\" => Some(&mut adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_int_param_mut<'a>(adjustables: &'a mut crate::app::GuiAdjustables, id: &str) -> Option<&'a mut crate::gui_adjustables::IntParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "int" {
            code.push_str(&format!(
                "        \"{}\" => Some(&mut adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_uint_param_mut<'a>(adjustables: &'a mut crate::app::GuiAdjustables, id: &str) -> Option<&'a mut crate::gui_adjustables::UintParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "uint" {
            code.push_str(&format!(
                "        \"{}\" => Some(&mut adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_bool_param_mut<'a>(adjustables: &'a mut crate::app::GuiAdjustables, id: &str) -> Option<&'a mut crate::gui_adjustables::BoolParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "bool" {
            code.push_str(&format!(
                "        \"{}\" => Some(&mut adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n\n");

    code.push_str("#[allow(dead_code)]\n");
    code.push_str(
        "pub fn get_color_param_mut<'a>(adjustables: &'a mut crate::app::GuiAdjustables, id: &str) -> Option<&'a mut crate::gui_adjustables::ColorParam> {\n",
    );
    code.push_str("    match id {\n");
    for (_section, id, kind, _label) in &descriptors {
        if kind == "color" {
            code.push_str(&format!(
                "        \"{}\" => Some(&mut adjustables.{}),\n",
                id, id
            ));
        }
    }
    code.push_str("        _ => None,\n");
    code.push_str("    }\n");
    code.push_str("}\n");

    fs::write(&out_path, code).unwrap_or_else(|e| {
        panic!(
            "GUI config generation failed: unable to write {}: {}",
            out_path.display(),
            e
        )
    });
    log!("wrote generated GUI descriptors to {}", out_path.display());
}

fn main() {
    dump_env();
    generate_gui_adjustables();
}
