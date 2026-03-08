use image::Rgba;
use std::collections::BTreeSet;
use std::path::Path;

pub type PaletteColor = [u8; 4];

pub fn detect_png_color_mode(path: &Path) -> Option<&'static str> {
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

pub fn collect_used_colors(rgba: &image::RgbaImage) -> Vec<PaletteColor> {
    let mut palette = BTreeSet::new();
    for pixel in rgba.pixels() {
        palette.insert(pixel.0);
    }
    palette.into_iter().collect()
}

pub fn luminance(color: PaletteColor) -> f32 {
    let r = color[0] as f32 / 255.0;
    let g = color[1] as f32 / 255.0;
    let b = color[2] as f32 / 255.0;
    0.299 * r + 0.587 * g + 0.114 * b
}

pub fn remap_palette<const N: usize>(
    source_colors: &[PaletteColor; N],
    target_colors: &[PaletteColor; N],
    source_image: &image::RgbaImage,
) -> image::RgbaImage
where
    [PaletteColor; N]: Copy,
{
    use image::ImageBuffer;
    let mut remapped = ImageBuffer::new(source_image.width(), source_image.height());

    for (x, y, pixel) in source_image.enumerate_pixels() {
        let src = pixel.0;
        let target = find_matching_color(src, source_colors, target_colors);
        remapped.put_pixel(x, y, Rgba(target));
    }

    remapped
}

fn find_matching_color<const N: usize>(
    src: PaletteColor,
    source_colors: &[PaletteColor; N],
    target_colors: &[PaletteColor; N],
) -> PaletteColor
where
    PaletteColor: Copy,
{
    for i in 0..N {
        if src == source_colors[i] {
            return target_colors[i];
        }
    }
    src
}

pub fn infer_role_order_5(used_colors: &[PaletteColor], path_str: &str) -> [PaletteColor; 5] {
    assert_eq!(
        used_colors.len(),
        5,
        "'{}' must have exactly 5 colors, got {}",
        path_str,
        used_colors.len()
    );

    let transparent_colors: Vec<_> = used_colors.iter().filter(|c| c[3] == 0).collect();
    assert_eq!(
        transparent_colors.len(),
        1,
        "'{}' must have exactly 1 transparent color, got {}",
        path_str,
        transparent_colors.len()
    );
    let source_transparent = *transparent_colors[0];

    let opaque_colors: Vec<_> = used_colors.iter().filter(|c| c[3] != 0).copied().collect();
    assert_eq!(
        opaque_colors.len(),
        4,
        "'{}' must have exactly 4 opaque colors, got {}",
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

    let source_roles: [PaletteColor; 5] = [
        source_transparent,
        source_border,
        source_dark_shade,
        source_mid_shade,
        source_light_shade,
    ];

    source_roles
}
