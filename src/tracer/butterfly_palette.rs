use image::{ImageBuffer, Rgba};
use std::path::Path;

#[derive(Debug, Clone, Copy)]
pub struct ButterflyPaletteConfig {
    pub transparent: [u8; 4],
    pub border: [u8; 4],
    pub dark_shade: [u8; 4],
    pub mid_shade: [u8; 4],
    pub light_shade: [u8; 4],
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

    pub fn as_slice(&self) -> [&[u8; 4]; 5] {
        [
            &self.transparent,
            &self.border,
            &self.dark_shade,
            &self.mid_shade,
            &self.light_shade,
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButterflyPaletteRole {
    Transparent,
    Border,
    DarkShade,
    MidShade,
    LightShade,
}

impl ButterflyPaletteRole {
    pub fn all() -> [Self; 5] {
        [
            Self::Transparent,
            Self::Border,
            Self::DarkShade,
            Self::MidShade,
            Self::LightShade,
        ]
    }
}

fn detect_png_color_mode(path: &Path) -> Option<&'static str> {
    const PNG_SIGNATURE: [u8; 8] = [137, 80, 78, 71, 13, 10, 26, 10];

    let mut header = [0u8; 26];
    let mut file = std::fs::File::open(path).ok()?;
    use std::io::Read;
    file.read_exact(&mut header).ok()?;

    if header[0..8] != PNG_SIGNATURE || &header[12..16] != b"IHDR" {
        return None;
    }

    match header[25] {
        3 => Some("palette"),
        0 | 2 | 4 | 6 => Some("rgba"),
        _ => None,
    }
}

fn collect_used_colors(rgba: &image::RgbaImage) -> Vec<[u8; 4]> {
    use std::collections::BTreeSet;
    let mut palette = BTreeSet::new();
    for pixel in rgba.pixels() {
        palette.insert(pixel.0);
    }
    palette.into_iter().collect()
}

fn luminance(color: [u8; 4]) -> f32 {
    let r = color[0] as f32 / 255.0;
    let g = color[1] as f32 / 255.0;
    let b = color[2] as f32 / 255.0;
    0.299 * r + 0.587 * g + 0.114 * b
}

pub fn load_butterfly_rgba_with_palette_config(
    path: &Path,
    config: &ButterflyPaletteConfig,
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

    assert_eq!(
        used_colors.len(),
        5,
        "Butterfly atlas '{}' must have exactly 5 colors, got {}",
        path_str,
        used_colors.len()
    );

    let transparent_colors: Vec<_> = used_colors.iter().filter(|c| c[3] == 0).collect();
    assert_eq!(
        transparent_colors.len(),
        1,
        "Butterfly atlas '{}' must have exactly 1 transparent color, got {}",
        path_str,
        transparent_colors.len()
    );
    let source_transparent = *transparent_colors[0];

    let opaque_colors: Vec<_> = used_colors.iter().filter(|c| c[3] != 0).copied().collect();
    assert_eq!(
        opaque_colors.len(),
        4,
        "Butterfly atlas '{}' must have exactly 4 opaque colors, got {}",
        path_str,
        opaque_colors.len()
    );

    let mut sorted_opaque: Vec<_> = opaque_colors.clone();
    sorted_opaque.sort_by(|a, b| {
        luminance(*a)
            .partial_cmp(&luminance(*b))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let source_border = sorted_opaque[0];
    let source_dark_shade = sorted_opaque[1];
    let source_mid_shade = sorted_opaque[2];
    let source_light_shade = sorted_opaque[3];

    println!("source role mapping:");
    println!("  transparent: {:02X?}", source_transparent);
    println!("  border:      {:02X?}", source_border);
    println!("  dark_shade: {:02X?}", source_dark_shade);
    println!("  mid_shade:  {:02X?}", source_mid_shade);
    println!("  light_shade: {:02X?}", source_light_shade);

    println!("target config mapping:");
    println!("  transparent: {:02X?}", config.transparent);
    println!("  border:      {:02X?}", config.border);
    println!("  dark_shade: {:02X?}", config.dark_shade);
    println!("  mid_shade:  {:02X?}", config.mid_shade);
    println!("  light_shade: {:02X?}", config.light_shade);

    let mut remapped = ImageBuffer::new(rgba.width(), rgba.height());

    for (x, y, pixel) in rgba.enumerate_pixels() {
        let src = pixel.0;
        let target = if src == source_transparent {
            Rgba(config.transparent)
        } else if src == source_border {
            Rgba(config.border)
        } else if src == source_dark_shade {
            Rgba(config.dark_shade)
        } else if src == source_mid_shade {
            Rgba(config.mid_shade)
        } else if src == source_light_shade {
            Rgba(config.light_shade)
        } else {
            panic!(
                "Butterfly atlas '{}' has unexpected color {:02X?} at ({}, {})",
                path_str, src, x, y
            )
        };
        remapped.put_pixel(x, y, target);
    }

    println!("palette remapped for {}", path_str);

    remapped
}
