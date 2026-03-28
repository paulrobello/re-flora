use crate::tracer::palette_remap::{
    collect_used_colors, detect_png_color_mode, infer_role_order_5, remap_palette, PaletteColor,
};
use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum ButterflyPaletteRole {
    Transparent,
    Border,
    DarkShade,
    MidShade,
    LightShade,
}

impl ButterflyPaletteRole {
    #[allow(dead_code)]
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
    pub border: PaletteColor,
    pub dark_shade: PaletteColor,
    pub mid_shade: PaletteColor,
    pub light_shade: PaletteColor,
}

impl ButterflyPaletteConfig {
    pub fn into_role_array(self) -> [PaletteColor; 5] {
        [
            [0, 0, 0, 0],
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
    Orange = 2,
    Blue = 3,
    Green = 4,
    Pink = 5,
    Purple = 6,
    Cyan = 7,
    Monarch = 8,
    Midnight = 9,
}

impl ButterflyPalettePreset {
    pub const COUNT: u32 = 10;

    pub fn from_index(index: u32) -> Self {
        match index {
            0 => Self::Grayscale,
            1 => Self::Yellow,
            2 => Self::Orange,
            3 => Self::Blue,
            4 => Self::Green,
            5 => Self::Pink,
            6 => Self::Purple,
            7 => Self::Cyan,
            8 => Self::Monarch,
            9 => Self::Midnight,
            _ => Self::Grayscale,
        }
    }

    pub fn config(&self) -> ButterflyPaletteConfig {
        match self {
            Self::Grayscale => ButterflyPaletteConfig::default_current(),
            Self::Yellow => ButterflyPaletteConfig::yellow(),
            Self::Orange => ButterflyPaletteConfig::orange(),
            Self::Blue => ButterflyPaletteConfig::blue(),
            Self::Green => ButterflyPaletteConfig::green(),
            Self::Pink => ButterflyPaletteConfig::pink(),
            Self::Purple => ButterflyPaletteConfig::purple(),
            Self::Cyan => ButterflyPaletteConfig::cyan(),
            Self::Monarch => ButterflyPaletteConfig::monarch(),
            Self::Midnight => ButterflyPaletteConfig::midnight(),
        }
    }

    pub fn name(&self) -> &'static str {
        match self {
            Self::Grayscale => "grayscale",
            Self::Yellow => "yellow",
            Self::Orange => "orange",
            Self::Blue => "blue",
            Self::Green => "green",
            Self::Pink => "pink",
            Self::Purple => "purple",
            Self::Cyan => "cyan",
            Self::Monarch => "monarch",
            Self::Midnight => "midnight",
        }
    }
}

impl ButterflyPaletteConfig {
    pub fn default_current() -> Self {
        Self {
            border: [0, 0, 0, 255],
            dark_shade: [148, 148, 148, 255],
            mid_shade: [179, 179, 179, 255],
            light_shade: [255, 255, 255, 255],
        }
    }

    pub fn yellow() -> Self {
        Self {
            border: [58, 36, 8, 255],
            dark_shade: [176, 122, 22, 255],
            mid_shade: [232, 185, 48, 255],
            light_shade: [255, 233, 140, 255],
        }
    }

    pub fn orange() -> Self {
        Self {
            border: [70, 25, 0, 255],
            dark_shade: [170, 70, 10, 255],
            mid_shade: [230, 120, 40, 255],
            light_shade: [255, 196, 120, 255],
        }
    }

    pub fn blue() -> Self {
        Self {
            border: [10, 30, 80, 255],
            dark_shade: [30, 70, 140, 255],
            mid_shade: [70, 120, 200, 255],
            light_shade: [170, 210, 255, 255],
        }
    }

    pub fn green() -> Self {
        Self {
            border: [10, 55, 10, 255],
            dark_shade: [40, 100, 30, 255],
            mid_shade: [90, 160, 70, 255],
            light_shade: [190, 230, 150, 255],
        }
    }

    pub fn pink() -> Self {
        Self {
            border: [80, 10, 40, 255],
            dark_shade: [150, 40, 90, 255],
            mid_shade: [210, 90, 150, 255],
            light_shade: [250, 190, 230, 255],
        }
    }

    pub fn purple() -> Self {
        Self {
            border: [40, 10, 70, 255],
            dark_shade: [80, 40, 130, 255],
            mid_shade: [130, 80, 190, 255],
            light_shade: [210, 170, 255, 255],
        }
    }

    pub fn cyan() -> Self {
        Self {
            border: [0, 60, 70, 255],
            dark_shade: [0, 110, 130, 255],
            mid_shade: [40, 170, 190, 255],
            light_shade: [160, 230, 240, 255],
        }
    }

    pub fn monarch() -> Self {
        Self {
            border: [15, 15, 15, 255],
            dark_shade: [120, 60, 10, 255],
            mid_shade: [210, 110, 20, 255],
            light_shade: [250, 190, 110, 255],
        }
    }

    pub fn midnight() -> Self {
        Self {
            border: [5, 5, 15, 255],
            dark_shade: [20, 20, 60, 255],
            mid_shade: [40, 40, 110, 255],
            light_shade: [120, 140, 210, 255],
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
    println!("  transparent: {:02X?}", [0, 0, 0, 0]);
    println!("  border:      {:02X?}", target_config.border);
    println!("  dark_shade: {:02X?}", target_config.dark_shade);
    println!("  mid_shade:  {:02X?}", target_config.mid_shade);
    println!("  light_shade: {:02X?}", target_config.light_shade);

    let target_roles = target_config.into_role_array();
    let remapped = remap_palette(&source_roles, &target_roles, &rgba);

    println!("palette remapped for {}", path_str);

    remapped
}
