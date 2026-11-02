use crate::model::{
    rgb, rgba, CanvasElement, ColorRgba, DesignFormat, DropShadow, ElementAnimation, ElementKind,
    FillStyle, FontFamily, ImageAdjustments, ListStyle, Project, ShapeType, TextAlign, TextEffect,
};

/// Applies non-destructive adjustments (including One-Click Background Removal,
/// brightness, contrast, saturation, warmth, duotone, grayscale, sepia, invert,
/// posterize, blur, and sharpen) to raw RGBA pixels and returns a processed RGBA buffer.
pub fn apply_image_pipeline(
    raw_rgba: &[u8],
    width: u32,
    height: u32,
    adj: &ImageAdjustments,
) -> Vec<u8> {
    let total_px = (width as usize) * (height as usize);
    if raw_rgba.len() < total_px * 4 || width == 0 || height == 0 {
        return raw_rgba.to_vec();
    }

    let mut out = raw_rgba.to_vec();

    // 1. One-click Background Remover (samples perimeter corners & edges)
    if adj.bg_removed {
        let bg_color = estimate_border_bg_color(&out, width, height);
        let tol = (adj.bg_tolerance.clamp(0.05, 0.75) * 441.67).max(12.0);
        let feather = tol * 0.35;
        for px in out.chunks_exact_mut(4) {
            let dr = px[0] as f32 - bg_color[0] as f32;
            let dg = px[1] as f32 - bg_color[1] as f32;
            let db = px[2] as f32 - bg_color[2] as f32;
            let dist = (dr * dr + dg * dg + db * db).sqrt();
            if dist < tol - feather {
                px[3] = 0;
            } else if dist < tol + feather {
                let factor = ((dist - (tol - feather)) / (2.0 * feather)).clamp(0.0, 1.0);
                px[3] = ((px[3] as f32) * factor) as u8;
            }
        }
    }

    // 2. Per-pixel color pipeline
    let bright_offset = adj.brightness * 120.0;
    let contrast_factor = (1.0 + adj.contrast).max(0.05).powf(1.5);
    let sat_factor = (1.0 + adj.saturation).max(0.0);
    let warmth_shift = adj.warmth * 35.0;

    for px in out.chunks_exact_mut(4) {
        if px[3] == 0 {
            continue;
        }
        let mut r = px[0] as f32;
        let mut g = px[1] as f32;
        let mut b = px[2] as f32;

        // Brightness
        r += bright_offset;
        g += bright_offset;
        b += bright_offset;

        // Contrast around midpoint 128
        r = (r - 128.0) * contrast_factor + 128.0;
        g = (g - 128.0) * contrast_factor + 128.0;
        b = (b - 128.0) * contrast_factor + 128.0;

        // Warmth (R+/B-)
        r += warmth_shift;
        b -= warmth_shift;

        // Saturation
        let luma = 0.299 * r + 0.587 * g + 0.114 * b;
        r = luma + (r - luma) * sat_factor;
        g = luma + (g - luma) * sat_factor;
        b = luma + (b - luma) * sat_factor;

        // Grayscale
        if adj.grayscale {
            let l = 0.299 * r + 0.587 * g + 0.114 * b;
            r = l;
            g = l;
            b = l;
        }

        // Sepia
        if adj.sepia {
            let sr = (r * 0.393) + (g * 0.769) + (b * 0.189);
            let sg = (r * 0.349) + (g * 0.686) + (b * 0.168);
            let sb = (r * 0.272) + (g * 0.534) + (b * 0.131);
            r = sr;
            g = sg;
            b = sb;
        }

        // Duotone
        if let Some((shadow_c, highlight_c)) = adj.duotone {
            let t = ((0.299 * r + 0.587 * g + 0.114 * b) / 255.0).clamp(0.0, 1.0);
            r = shadow_c[0] as f32 * (1.0 - t) + highlight_c[0] as f32 * t;
            g = shadow_c[1] as f32 * (1.0 - t) + highlight_c[1] as f32 * t;
            b = shadow_c[2] as f32 * (1.0 - t) + highlight_c[2] as f32 * t;
        }

        // Posterize
        if adj.posterize {
            let levels = 4.0;
            r = ((r / 255.0 * levels).round() / levels) * 255.0;
            g = ((g / 255.0 * levels).round() / levels) * 255.0;
            b = ((b / 255.0 * levels).round() / levels) * 255.0;
        }

        // Invert
        if adj.invert {
            r = 255.0 - r;
            g = 255.0 - g;
            b = 255.0 - b;
        }

        px[0] = r.clamp(0.0, 255.0) as u8;
        px[1] = g.clamp(0.0, 255.0) as u8;
        px[2] = b.clamp(0.0, 255.0) as u8;
    }

    // 3. Optional Blur
    if adj.blur >= 0.5 {
        let radius = (adj.blur.round() as i32).clamp(1, 6);
        out = box_blur_rgba(&out, width as i32, height as i32, radius);
    }

    // 4. Optional Sharpen (Unsharp Mask)
    if adj.sharpen > 0.05 {
        let blurred = box_blur_rgba(&out, width as i32, height as i32, 1);
        let amount = adj.sharpen.clamp(0.0, 2.5);
        for (px, bpx) in out.chunks_exact_mut(4).zip(blurred.chunks_exact(4)) {
            for c in 0..3 {
                let orig = px[c] as f32;
                let diff = orig - bpx[c] as f32;
                px[c] = (orig + diff * amount).clamp(0.0, 255.0) as u8;
            }
        }
    }

    out
}

fn estimate_border_bg_color(rgba_buf: &[u8], width: u32, height: u32) -> [u8; 3] {
    let w = width as usize;
    let h = height as usize;
    let mut r_sum = 0u64;
    let mut g_sum = 0u64;
    let mut b_sum = 0u64;
    let mut count = 0u64;

    let sample_coords = [
        (0, 0),
        (w.saturating_sub(1), 0),
        (0, h.saturating_sub(1)),
        (w.saturating_sub(1), h.saturating_sub(1)),
        (w / 2, 0),
        (w / 2, h.saturating_sub(1)),
        (0, h / 2),
        (w.saturating_sub(1), h / 2),
    ];

    for (x, y) in sample_coords {
        let idx = (y * w + x) * 4;
        if idx + 3 < rgba_buf.len() {
            r_sum += rgba_buf[idx] as u64;
            g_sum += rgba_buf[idx + 1] as u64;
            b_sum += rgba_buf[idx + 2] as u64;
            count += 1;
        }
    }

    if count == 0 {
        [255, 255, 255]
    } else {
        [
            (r_sum / count) as u8,
            (g_sum / count) as u8,
            (b_sum / count) as u8,
        ]
    }
}

fn box_blur_rgba(src: &[u8], w: i32, h: i32, radius: i32) -> Vec<u8> {
    let mut dst = src.to_vec();
    for y in 0..h {
        for x in 0..w {
            let mut r = 0u32;
            let mut g = 0u32;
            let mut b = 0u32;
            let mut count = 0u32;
            for ky in -radius..=radius {
                let ny = (y + ky).clamp(0, h - 1);
                for kx in -radius..=radius {
                    let nx = (x + kx).clamp(0, w - 1);
                    let idx = ((ny * w + nx) * 4) as usize;
                    r += src[idx] as u32;
                    g += src[idx + 1] as u32;
                    b += src[idx + 2] as u32;
                    count += 1;
                }
            }
            let out_idx = ((y * w + x) * 4) as usize;
            dst[out_idx] = (r / count) as u8;
            dst[out_idx + 1] = (g / count) as u8;
            dst[out_idx + 2] = (b / count) as u8;
        }
    }
    dst
}

/// Image Upscaler: 2x bilinear upscale + crisp unsharp mask pass
pub fn upscale_image_2x(raw_rgba: &[u8], width: u32, height: u32) -> (u32, u32, Vec<u8>) {
    if width == 0 || height == 0 || width > 1024 || height > 1024 {
        return (width, height, raw_rgba.to_vec());
    }
    let new_w = width * 2;
    let new_h = height * 2;
    let mut out = vec![0u8; (new_w * new_h * 4) as usize];

    for ny in 0..new_h {
        let fy = (ny as f32) * 0.5;
        let y0 = (fy.floor() as u32).min(height - 1);
        let y1 = (y0 + 1).min(height - 1);
        let wy = fy - (y0 as f32);

        for nx in 0..new_w {
            let fx = (nx as f32) * 0.5;
            let x0 = (fx.floor() as u32).min(width - 1);
            let x1 = (x0 + 1).min(width - 1);
            let wx = fx - (x0 as f32);

            let idx00 = ((y0 * width + x0) * 4) as usize;
            let idx10 = ((y0 * width + x1) * 4) as usize;
            let idx01 = ((y1 * width + x0) * 4) as usize;
            let idx11 = ((y1 * width + x1) * 4) as usize;
            let out_idx = ((ny * new_w + nx) * 4) as usize;

            for c in 0..4 {
                let top = raw_rgba[idx00 + c] as f32 * (1.0 - wx) + raw_rgba[idx10 + c] as f32 * wx;
                let bot = raw_rgba[idx01 + c] as f32 * (1.0 - wx) + raw_rgba[idx11 + c] as f32 * wx;
                let val = top * (1.0 - wy) + bot * wy;
                out[out_idx + c] = val.round().clamp(0.0, 255.0) as u8;
            }
        }
    }

    // Apply subtle sharpen pass
    let blurred = box_blur_rgba(&out, new_w as i32, new_h as i32, 1);
    for (px, bpx) in out.chunks_exact_mut(4).zip(blurred.chunks_exact(4)) {
        for c in 0..3 {
            let orig = px[c] as f32;
            let diff = orig - bpx[c] as f32;
            px[c] = (orig + diff * 0.65).clamp(0.0, 255.0) as u8;
        }
    }

    (new_w, new_h, out)
}

/// Patch blending: Interpolates a normalized rectangular patch `(rx, ry, rw, rh)` in `0.0..1.0`
/// using boundary pixel interpolation without scene understanding.
pub fn magic_erase_patch(
    raw_rgba: &mut [u8],
    width: u32,
    height: u32,
    norm_x: f32,
    norm_y: f32,
    norm_w: f32,
    norm_h: f32,
) {
    if width < 4 || height < 4 {
        return;
    }
    let x0 = ((norm_x.clamp(0.0, 0.95) * width as f32) as i32).clamp(1, width as i32 - 2);
    let y0 = ((norm_y.clamp(0.0, 0.95) * height as f32) as i32).clamp(1, height as i32 - 2);
    let x1 = (((norm_x + norm_w).clamp(0.05, 1.0) * width as f32) as i32)
        .clamp(x0 + 1, width as i32 - 2);
    let y1 = (((norm_y + norm_h).clamp(0.05, 1.0) * height as f32) as i32)
        .clamp(y0 + 1, height as i32 - 2);

    let w = width as i32;
    let copy = raw_rgba.to_vec();

    for y in y0..=y1 {
        let ty = (y - y0) as f32 / ((y1 - y0).max(1) as f32);
        for x in x0..=x1 {
            let tx = (x - x0) as f32 / ((x1 - x0).max(1) as f32);
            let left_idx = ((y * w + (x0 - 1)) * 4) as usize;
            let right_idx = ((y * w + (x1 + 1)) * 4) as usize;
            let top_idx = (((y0 - 1) * w + x) * 4) as usize;
            let bot_idx = (((y1 + 1) * w + x) * 4) as usize;
            let out_idx = ((y * w + x) * 4) as usize;

            for c in 0..4 {
                let horiz =
                    copy[left_idx + c] as f32 * (1.0 - tx) + copy[right_idx + c] as f32 * tx;
                let vert = copy[top_idx + c] as f32 * (1.0 - ty) + copy[bot_idx + c] as f32 * ty;
                raw_rgba[out_idx + c] = ((horiz + vert) * 0.5).round().clamp(0.0, 255.0) as u8;
            }
        }
    }
}

/// Magic Layers: Samples four colour swatches and generates editable layered shapes & caption card
/// from an image element on the current page.
pub fn extract_magic_layers_from_image(
    raw_rgba: &[u8],
    width: u32,
    height: u32,
    base_x: f32,
    base_y: f32,
    base_w: f32,
    base_h: f32,
    next_id: &mut u64,
) -> Vec<CanvasElement> {
    let mut elements = Vec::new();
    if width == 0 || height == 0 || raw_rgba.len() < (width * height * 4) as usize {
        return elements;
    }

    // Sample 4 quadrants to extract a 4-color palette from the image
    let quadrants = [
        (width / 4, height / 4),
        ((width * 3) / 4, height / 4),
        (width / 4, (height * 3) / 4),
        ((width * 3) / 4, (height * 3) / 4),
    ];
    let mut colors: Vec<ColorRgba> = Vec::new();
    for (qx, qy) in quadrants {
        let idx = ((qy.min(height - 1) * width + qx.min(width - 1)) * 4) as usize;
        colors.push(rgb(raw_rgba[idx], raw_rgba[idx + 1], raw_rgba[idx + 2]));
    }

    // Backdrop accent frame
    let id1 = *next_id;
    *next_id += 1;
    elements.push(CanvasElement {
        id: id1,
        name: "Magic Layer Backdrop".to_string(),
        x: base_x + base_w + 24.0,
        y: base_y,
        width: base_w * 0.85,
        height: base_h,
        rotation: 0.0,
        opacity: 0.95,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow {
            enabled: true,
            offset_x: 0.0,
            offset_y: 10.0,
            blur: 20.0,
            color: rgba(0, 0, 0, 80),
        },
        animation: ElementAnimation::Rise,
        group_id: None,
        text_format: Default::default(),
        kind: ElementKind::Shape {
            shape_type: ShapeType::RoundedRect,
            fill: FillStyle::LinearGradient {
                start: colors[0],
                end: colors[3],
                vertical: true,
            },
            stroke_color: colors[1],
            stroke_width: 2.0,
            corner_radius: 20.0,
        },
    });

    // Extracted Palette Swatch Circles
    for (i, col) in colors.iter().enumerate() {
        let sid = *next_id;
        *next_id += 1;
        elements.push(CanvasElement {
            id: sid,
            name: format!("Extracted Swatch #{}", i + 1),
            x: base_x + base_w + 44.0 + (i as f32) * 54.0,
            y: base_y + base_h - 68.0,
            width: 42.0,
            height: 42.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::Pop,
            group_id: None,
            text_format: Default::default(),
            kind: ElementKind::Shape {
                shape_type: ShapeType::Circle,
                fill: FillStyle::Solid(*col),
                stroke_color: rgb(255, 255, 255),
                stroke_width: 3.0,
                corner_radius: 0.0,
            },
        });
    }

    // Editable Headline Layer
    let tid = *next_id;
    *next_id += 1;
    elements.push(CanvasElement {
        id: tid,
        name: "Magic Layer Headline".to_string(),
        x: base_x + base_w + 44.0,
        y: base_y + 28.0,
        width: (base_w * 0.85 - 40.0).max(160.0),
        height: 90.0,
        rotation: 0.0,
        opacity: 1.0,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow::default(),
        animation: ElementAnimation::Fade,
        group_id: None,
        text_format: Default::default(),
        kind: ElementKind::Text {
            content: "MAGIC LAYER\nEXTRACTED".to_string(),
            font_family: FontFamily::PosterHeavy,
            font_size: 32.0,
            color: rgb(255, 255, 255),
            bold: true,
            italic: false,
            underline: false,
            uppercase: true,
            align: TextAlign::Left,
            letter_spacing: 1.5,
            line_height: 1.15,
            effect: TextEffect::DropShadow {
                offset: 3.0,
                color: rgba(0, 0, 0, 140),
            },
            list_style: ListStyle::None,
        },
    });

    elements
}

/// Magic Switch: Resizes the entire project to a new `DesignFormat` and proportionally
/// rescales & centers all elements across every page.
pub fn magic_resize_project(project: &mut Project, new_format: DesignFormat) {
    let old_w = project.width.max(1.0);
    let old_h = project.height.max(1.0);
    let (new_w, new_h) = new_format.dimensions();

    let scale_x = new_w / old_w;
    let scale_y = new_h / old_h;
    let uniform_scale = scale_x.min(scale_y);

    let offset_x = (new_w - old_w * uniform_scale) * 0.5;
    let offset_y = (new_h - old_h * uniform_scale) * 0.5;

    for page in &mut project.pages {
        for el in &mut page.elements {
            // Full-bleed background shapes scale to fill the new canvas
            let is_full_bleed = el.x.abs() < 8.0
                && el.y.abs() < 8.0
                && (el.width - old_w).abs() < 24.0
                && (el.height - old_h).abs() < 24.0;

            if is_full_bleed {
                el.x = 0.0;
                el.y = 0.0;
                el.width = new_w;
                el.height = new_h;
            } else {
                el.x = el.x * uniform_scale + offset_x;
                el.y = el.y * uniform_scale + offset_y;
                el.width = (el.width * uniform_scale).max(12.0);
                el.height = (el.height * uniform_scale).max(12.0);

                if let ElementKind::Text { font_size, .. } = &mut el.kind {
                    *font_size = (*font_size * uniform_scale).clamp(10.0, 220.0);
                }
            }
        }
    }

    let (w_mm, h_mm) = new_format.physical_dimensions_mm();
    project.format = new_format;
    project.width = new_w;
    project.height = new_h;
    project.width_mm = w_mm;
    project.height_mm = h_mm;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MagicWriteTone {
    BoldLaunch,
    ExecutivePitch,
    SocialViralHook,
    LuxuryEditorial,
    EventInvitation,
    ActionChecklist,
}

impl MagicWriteTone {
    pub fn all() -> &'static [MagicWriteTone] {
        &[
            MagicWriteTone::BoldLaunch,
            MagicWriteTone::ExecutivePitch,
            MagicWriteTone::SocialViralHook,
            MagicWriteTone::LuxuryEditorial,
            MagicWriteTone::EventInvitation,
            MagicWriteTone::ActionChecklist,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            MagicWriteTone::BoldLaunch => "Bold Product Launch",
            MagicWriteTone::ExecutivePitch => "Executive Summary",
            MagicWriteTone::SocialViralHook => "Social Headline",
            MagicWriteTone::LuxuryEditorial => "Editorial Feature",
            MagicWriteTone::EventInvitation => "Event Invitation",
            MagicWriteTone::ActionChecklist => "Production Checklist",
        }
    }
}

/// Local Copy Templates: Generates structured placeholder text blocks by tone & topic.
pub fn magic_write_generate(topic: &str, tone: MagicWriteTone) -> (String, String) {
    let clean_topic = if topic.trim().is_empty() {
        "Creative Studio".to_string()
    } else {
        topic.trim().to_string()
    };

    match tone {
        MagicWriteTone::BoldLaunch => (
            format!("INTRODUCING {}", clean_topic.to_uppercase()),
            format!(
                "Built for local print and digital layout production. Clear typography, balanced grid hierarchy, and structured composition for {}.",
                clean_topic
            ),
        ),
        MagicWriteTone::ExecutivePitch => (
            format!("{} — Editorial Overview", clean_topic),
            format!(
                "• Structured editorial layout for print and digital pages\n• Local document storage with bundled Noto typography\n• Consistent page hierarchy for {}",
                clean_topic
            ),
        ),
        MagicWriteTone::SocialViralHook => (
            format!("Stop scrolling: {} changes everything.", clean_topic),
            format!(
                "Here are 3 secrets top designers use with {} to turn ordinary layouts into unforgettable visual stories ->",
                clean_topic
            ),
        ),
        MagicWriteTone::LuxuryEditorial => (
            format!("THE ART OF {}", clean_topic.to_uppercase()),
            format!(
                "Timeless craftsmanship meets modern minimalism. A curated exploration of form, balance, and bespoke typography in {}.",
                clean_topic
            ),
        ),
        MagicWriteTone::EventInvitation => (
            format!("YOU'RE INVITED: {} LIVE", clean_topic.to_uppercase()),
            format!(
                "Join us for an exclusive showcase celebrating {}.\nSaturday • 7:00 PM • Main Design Pavilion",
                clean_topic
            ),
        ),
        MagicWriteTone::ActionChecklist => (
            format!("{} Launch Checklist", clean_topic),
            "1. Define core visual hierarchy & brand palette\n2. Apply high-contrast headline typography\n3. Export crisp PNG & SVG assets for production"
                .to_string(),
        ),
    }
}
