use crate::tracer::palette_remap::{
    collect_used_colors, detect_png_color_mode, infer_role_order_5, remap_palette, PaletteColor,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButterflyPaletteRole {
    Transparent,
    Border,
    DarkShade,
    MidShade,
    LightShade,
}

impl ButterflyPaletteRole {
    pub const ROLE_ORDER: [ButterflyPaletteRole; 5] = [
        ButterflyPaletteRole::Transparent,
        ButterflyPaletteRole::Border,
        ButterflyPaletteRole::DarkShade,
        ButterflyPaletteRole::MidShade,
        ButterflyPaletteRole::LightShade,
    ];
}

#[derive(Debug, Clone, Copy)]
pub struct ButterflyPaletteConfig {
    pub transparent: PaletteColor,
    pub border: PaletteColor,
    pub dark_shade: PaletteColor,
    pub mid_shade: PaletteColor,
    pub light_shade: PaletteColor,
}

impl ButterflyPaletteConfig {
    pub fn from_role_colors(role_colors: [PaletteColor; 5]) -> Self {
        Self {
            transparent: role_colors[0],
            border: role_colors[1],
            dark_shade: role_colors[2],
            mid_shade: role_colors[3],
            light_shade: role_colors[4],
        }
    }

    pub fn to_role_array(&self) -> [PaletteColor; 5] {
        [
            self.transparent,
            self.border,
            self.dark_shade,
            self.mid_shade,
            self.light_shade,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum ButterflyPalettePreset {
    Grayscale = 0,
    Yellow = 1,
}

impl ButterflyPalettePreset {
    pub const COUNT: u32 = 2;

    pub fn from_index(index: u32) -> Self {
        match index {
            0 => Self::Grayscale,
            1 => Self::Yellow,
            _ => Self::Grayscale,
        }
    }

    pub fn config(&self) -> ButterflyPaletteConfig {
        match self {
            Self::Grayscale => ButterflyPaletteConfig::default_current(),
            Self::Yellow => ButterflyPaletteConfig::yellow(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Grayscale => "grayscale",
            Self::Yellow => "yellow",
        }
    }
}

impl ButterflyPaletteConfig {
    pub fn default_current() -> Self {
        Self {
            transparent: [0, 0, 0, 0],
            border: [0, 0, 0, 255],
            dark_shade: [148, 148, 148, 255],
            mid_shade: [179, 179, 179, 255],
            light_shade: [255, 255, 255, 255],
        }
    }

    pub fn yellow() -> Self {
        Self {
            transparent: [0, 0, 0, 0],
            border: [58, 36, 8, 255],
            dark_shade: [176, 122, 22, 255],
            mid_shade: [232, 185, 48, 255],
            light_shade: [255, 233, 140, 255],
        }
    }
}

pub fn load_butterfly_and_remap(
    path: &Path,
    target_config: &ButterflyPaletteConfig,
) -> image::RgbaImage {
    let path_str = path.to_string_lossy().to_string();

    let color_mode = detect_png_color_mode(path);
    assert!(
        color_mode == Some("palette"),
        "Butterfly atlas '{}' must be in indexed palette mode, got: {:?}",
        path_str,
        color_mode
    );

    let img = image::open(path)
        .unwrap_or_else(|e| panic!("Failed to open butterfly atlas '{}': {}", path_str, e));

    let rgba = img.to_rgba8();
    let used_colors = collect_used_colors(&rgba);

    println!("detected color mode for {}: palette", path_str);
    println!("palette for {} ({} colors):", path_str, used_colors.len());
    for color in &used_colors {
        println!(
            "#{:02X}{:02X}{:02X}{:02X} (r={}, g={}, b={}, a={})",
            color[0], color[1], color[2], color[3], color[0], color[1], color[2], color[3]
        );
    }

    let source_roles = infer_role_order_5(&used_colors, &path_str);

    println!("source role mapping:");
    println!("  transparent: {:02X?}", source_roles[0]);
    println!("  border:      {:02X?}", source_roles[1]);
    println!("  dark_shade: {:02X?}", source_roles[2]);
    println!("  mid_shade:  {:02X?}", source_roles[3]);
    println!("  light_shade: {:02X?}", source_roles[4]);

    println!("target config mapping:");
    println!("  transparent: {:02X?}", target_config.transparent);
    println!("  border:      {:02X?}", target_config.border);
    println!("  dark_shade: {:02X?}", target_config.dark_shade);
    println!("  mid_shade:  {:02X?}", target_config.mid_shade);
    println!("  light_shade: {:02X?}", target_config.light_shade);

    let target_roles = target_config.to_role_array();
    let remapped = remap_palette(&source_roles, &target_roles, &rgba);

    println!("palette remapped for {}", path_str);

    remapped
}
