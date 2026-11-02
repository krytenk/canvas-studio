use ab_glyph::{point, Font, PxScale, ScaleFont};

use crate::model::font_registry::{FontFaceId, FontRegistry};
use crate::model::{ColorRgba, FontFamily, ListStyle, TextAlign};

#[derive(Clone, Debug)]
pub struct PositionedGlyph {
    pub ch: char,
    pub byte_offset: usize,
    pub face_id: FontFaceId,
    pub glyph_id: u16,
    pub x: f32,
    pub y_baseline: f32,
    pub advance: f32,
    pub used_fallback: bool,
    pub underline_override: Option<bool>,
}

#[derive(Clone, Debug)]
pub struct TextLineLayout {
    pub text: String,
    pub line_index: usize,
    pub x_offset: f32,
    pub y_top: f32,
    pub y_baseline: f32,
    pub width: f32,
    pub height: f32,
    pub glyphs: Vec<PositionedGlyph>,
}

pub fn underline_spans(line: &TextLineLayout, fallback: bool) -> Vec<(f32, f32)> {
    let mut spans = Vec::new();
    let mut current = None;
    for glyph in &line.glyphs {
        if glyph.underline_override.unwrap_or(fallback) {
            let start = current.map(|(start, _)| start).unwrap_or(glyph.x);
            current = Some((start, glyph.x + glyph.advance));
        } else if let Some(span) = current.take() {
            spans.push(span);
        }
    }
    if let Some(span) = current {
        spans.push(span);
    }
    spans
}

#[derive(Clone, Debug)]
pub struct TextLayoutResult {
    pub primary_face_id: FontFaceId,
    pub font_size: f32,
    pub line_height_px: f32,
    pub box_width: f32,
    pub box_height: f32,
    pub measured_width: f32,
    pub measured_height: f32,
    pub overflows_width: bool,
    pub overflows_height: bool,
    pub has_fallback_glyphs: bool,
    pub lines: Vec<TextLineLayout>,
}

impl TextLayoutResult {
    pub fn is_overflowing(&self) -> bool {
        self.overflows_width || self.overflows_height
    }
}

pub fn em_px_scale<F: Font>(font: &F, font_size: f32) -> PxScale {
    let upm = font.units_per_em().unwrap_or(1000.0).max(1.0);
    let height_unscaled = font.height_unscaled();
    PxScale::from(font_size.max(1.0) * (height_unscaled / upm))
}

#[allow(clippy::too_many_arguments)]
pub fn layout_text_block(
    content: &str,
    font_family: FontFamily,
    font_size: f32,
    bold: bool,
    italic: bool,
    uppercase: bool,
    align: TextAlign,
    letter_spacing: f32,
    line_height: f32,
    list_style: ListStyle,
    box_width: f32,
    box_height: f32,
) -> TextLayoutResult {
    let reg = FontRegistry::global();
    let primary_face_id = FontFaceId::resolve(font_family, bold, italic);

    // 1. Apply uppercase only when requested; preserve all Unicode characters and case otherwise
    let mut processed = if uppercase {
        content.to_uppercase()
    } else {
        content.to_string()
    };

    if list_style != ListStyle::None {
        processed = processed
            .split('\n')
            .enumerate()
            .map(|(i, line)| match list_style {
                ListStyle::Bullet => format!("• {line}"),
                ListStyle::Numbered => format!("{}. {line}", i + 1),
                ListStyle::None => line.to_string(),
            })
            .collect::<Vec<_>>()
            .join("\n");
    }

    let eff_size = font_size.max(4.0);
    let eff_leading = if line_height <= 0.1 { 1.4 } else { line_height };
    let line_height_px = eff_size * eff_leading;
    let wrap_width = box_width.max(eff_size);

    let mut has_fallback_glyphs = false;
    let mut overflows_width = false;

    // Helper to measure and shape a string into relative glyphs starting at x=0
    let measure_segment =
        |seg: &str, base_byte_offset: usize| -> (f32, Vec<PositionedGlyph>, bool) {
            let mut cursor_x = 0.0_f32;
            let mut prev_gid: Option<(FontFaceId, ab_glyph::GlyphId)> = None;
            let mut glyphs = Vec::with_capacity(seg.chars().count());
            let mut seg_fb = false;

            for (char_idx, (rel_byte, ch)) in seg.char_indices().enumerate() {
                if let Some((face_id, loaded, gid, used_fb)) =
                    reg.resolve_glyph_for_char(primary_face_id, ch)
                {
                    if used_fb {
                        seg_fb = true;
                    }
                    let scale = em_px_scale(&loaded.font, eff_size);
                    let scaled = loaded.font.as_scaled(scale);
                    if let Some((prev_face, pgid)) = prev_gid {
                        if prev_face == face_id {
                            cursor_x += scaled.kern(pgid, gid);
                        }
                    }
                    if char_idx > 0 {
                        cursor_x += letter_spacing;
                    }
                    let adv = scaled.h_advance(gid);
                    glyphs.push(PositionedGlyph {
                        ch,
                        byte_offset: base_byte_offset + rel_byte,
                        face_id,
                        glyph_id: gid.0,
                        x: cursor_x,
                        y_baseline: 0.0,
                        advance: adv,
                        used_fallback: used_fb,
                        underline_override: None,
                    });
                    cursor_x += adv;
                    prev_gid = Some((face_id, gid));
                }
            }
            (cursor_x, glyphs, seg_fb)
        };

    // 2. Break paragraphs by explicit '\n' and word-wrap within `wrap_width`
    let mut raw_lines: Vec<(String, usize)> = Vec::new();
    let mut running_offset = 0usize;

    for raw_para in processed.split('\n') {
        let para = raw_para.trim_end_matches('\r');
        if para.is_empty() {
            raw_lines.push((String::new(), running_offset));
            running_offset += raw_para.len() + 1;
            continue;
        }

        let (full_w, _, _) = measure_segment(para, running_offset);
        if full_w <= wrap_width + 1.5 {
            raw_lines.push((para.to_string(), running_offset));
        } else {
            // Word-wrap paragraph
            let mut current_line = String::new();
            let mut current_line_start = running_offset;

            for word in para.split_whitespace() {
                let candidate = if current_line.is_empty() {
                    word.to_string()
                } else {
                    format!("{current_line} {word}")
                };
                let (cand_w, _, _) = measure_segment(&candidate, current_line_start);

                if cand_w > wrap_width + 1.0 && !current_line.is_empty() {
                    raw_lines.push((current_line.clone(), current_line_start));
                    current_line_start += current_line.len() + 1;
                    current_line = word.to_string();

                    // Check if single word itself is wider than wrap_width
                    let (word_w, _, _) = measure_segment(&current_line, current_line_start);
                    if word_w > wrap_width + 1.0 {
                        overflows_width = true;
                        // Fallback character-level break for unusually long words
                        let mut chunk = String::new();
                        let mut chunk_start = current_line_start;
                        for ch in current_line.chars() {
                            let test = format!("{chunk}{ch}");
                            let (tw, _, _) = measure_segment(&test, chunk_start);
                            if tw > wrap_width && !chunk.is_empty() {
                                raw_lines.push((chunk.clone(), chunk_start));
                                chunk_start += chunk.len();
                                chunk.clear();
                            }
                            chunk.push(ch);
                        }
                        current_line = chunk;
                        current_line_start = chunk_start;
                    }
                } else {
                    current_line = candidate;
                }
            }
            if !current_line.is_empty() {
                let (lw, _, _) = measure_segment(&current_line, current_line_start);
                if lw > wrap_width + 2.0 {
                    overflows_width = true;
                }
                raw_lines.push((current_line, current_line_start));
            }
        }
        running_offset += raw_para.len() + 1;
    }

    // 3. Position lines & glyphs with alignment and baseline metrics
    let first_baseline = eff_size * (0.88 + (eff_leading - 1.0).clamp(-0.2, 1.5) * 0.28);
    let mut lines = Vec::with_capacity(raw_lines.len());
    let mut measured_width = 0.0_f32;

    for (line_idx, (line_str, byte_start)) in raw_lines.into_iter().enumerate() {
        let (line_w, mut glyphs, line_fb) = measure_segment(&line_str, byte_start);
        if line_fb {
            has_fallback_glyphs = true;
        }
        measured_width = measured_width.max(line_w);
        if line_w > box_width + 2.0 {
            overflows_width = true;
        }

        let x_offset = match align {
            TextAlign::Left => 0.0,
            TextAlign::Center => ((box_width - line_w) * 0.5).max(0.0),
            TextAlign::Right => (box_width - line_w).max(0.0),
        };
        let y_top = (line_idx as f32) * line_height_px;
        let y_baseline = first_baseline + (line_idx as f32) * line_height_px;

        for g in &mut glyphs {
            g.x += x_offset;
            g.y_baseline = y_baseline;
        }

        lines.push(TextLineLayout {
            text: line_str,
            line_index: line_idx,
            x_offset,
            y_top,
            y_baseline,
            width: line_w,
            height: line_height_px,
            glyphs,
        });
    }

    let measured_height = if lines.is_empty() {
        0.0
    } else {
        first_baseline + ((lines.len() - 1) as f32) * line_height_px + eff_size * 0.26
    };

    // Flag vertical overflow if the laid-out lines exceed the box height by more than a small descender tolerance
    let overflows_height =
        !lines.is_empty() && measured_height > box_height + (eff_size * 0.35).max(5.0);

    TextLayoutResult {
        primary_face_id,
        font_size: eff_size,
        line_height_px,
        box_width,
        box_height,
        measured_width,
        measured_height,
        overflows_width,
        overflows_height,
        has_fallback_glyphs,
        lines,
    }
}

/// Rasterizes a measured `TextLayoutResult` into an `image::RgbaImage` with subpixel-accurate
/// antialiased glyph coverage, optional rotation/flip around `(center_x, center_y)`, and optional underline.
#[allow(clippy::too_many_arguments)]
pub fn rasterize_text_layout(
    img: &mut image::RgbaImage,
    layout: &TextLayoutResult,
    origin_x: f32,
    origin_y: f32,
    scale_x: f32,
    scale_y: f32,
    rotation_deg: f32,
    flip_h: bool,
    flip_v: bool,
    underline: bool,
    color: ColorRgba,
    opacity: f32,
) {
    if rotation_deg.abs() > 0.05 || flip_h || flip_v {
        let pad = (layout.font_size * scale_x.max(scale_y) + 3.).ceil();
        let w = ((layout.box_width.max(layout.measured_width) * scale_x + 2. * pad).ceil() as u32)
            .min(img.width().saturating_add((2. * pad) as u32))
            .max(1);
        let h = ((layout.box_height.max(layout.measured_height) * scale_y + 2. * pad).ceil()
            as u32)
            .min(img.height().saturating_add((2. * pad) as u32))
            .max(1);
        let mut local = image::RgbaImage::new(w, h);
        rasterize_text_layout(
            &mut local, layout, pad, pad, scale_x, scale_y, 0., false, false, underline, color,
            opacity,
        );
        let cx = origin_x + layout.box_width * scale_x / 2.;
        let cy = origin_y + layout.box_height * scale_y / 2.;
        let (sin, cos) = rotation_deg.to_radians().sin_cos();
        let fx = if flip_h { -1. } else { 1. };
        let fy = if flip_v { -1. } else { 1. };
        let transform = |x: f32, y: f32| {
            let dx = (origin_x + x - pad - cx) * fx;
            let dy = (origin_y + y - pad - cy) * fy;
            (cx + dx * cos - dy * sin, cy + dx * sin + dy * cos)
        };
        let pts = [
            transform(0., 0.),
            transform(w as f32, 0.),
            transform(0., h as f32),
            transform(w as f32, h as f32),
        ];
        let x0 = pts
            .iter()
            .map(|p| p.0)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as u32;
        let x1 = pts
            .iter()
            .map(|p| p.0)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.)
            .min(img.width() as f32) as u32;
        let y0 = pts
            .iter()
            .map(|p| p.1)
            .fold(f32::INFINITY, f32::min)
            .floor()
            .max(0.) as u32;
        let y1 = pts
            .iter()
            .map(|p| p.1)
            .fold(f32::NEG_INFINITY, f32::max)
            .ceil()
            .max(0.)
            .min(img.height() as f32) as u32;
        for y in y0..y1 {
            for x in x0..x1 {
                let dx = x as f32 - cx;
                let dy = y as f32 - cy;
                let sx = (dx * cos + dy * sin) * fx + cx - origin_x + pad;
                let sy = (-dx * sin + dy * cos) * fy + cy - origin_y + pad;
                if sx >= 0. && sy >= 0. && sx < w as f32 && sy < h as f32 {
                    let px = crate::model::pdf_export::sample_bilinear_rgba(
                        local.as_raw(),
                        w,
                        h,
                        sx,
                        sy,
                    );
                    blend_rgba_pixel(img, x, y, px);
                }
            }
        }
        return;
    }
    let reg = FontRegistry::global();
    let eff_alpha = ((color[3] as f32 / 255.0) * opacity.clamp(0.0, 1.0)).clamp(0.0, 1.0);
    if eff_alpha <= 0.001 {
        return;
    }

    let box_cx = origin_x + layout.box_width * 0.5 * scale_x;
    let box_cy = origin_y + layout.box_height * 0.5 * scale_y;
    let has_rot = rotation_deg.abs() > 0.05 || flip_h || flip_v;
    let rad = rotation_deg.to_radians();
    let cos_r = rad.cos();
    let sin_r = rad.sin();
    let sx_sign = if flip_h { -1.0 } else { 1.0 };
    let sy_sign = if flip_v { -1.0 } else { 1.0 };

    let transform_pt = |px: f32, py: f32| -> (f32, f32) {
        if !has_rot {
            return (px, py);
        }
        let dx = (px - box_cx) * sx_sign;
        let dy = (py - box_cy) * sy_sign;
        (
            box_cx + dx * cos_r - dy * sin_r,
            box_cy + dx * sin_r + dy * cos_r,
        )
    };

    let (img_w, img_h) = img.dimensions();

    for line in &layout.lines {
        for pg in &line.glyphs {
            let Some(loaded) = reg.get(pg.face_id) else {
                continue;
            };
            let px_font_size = layout.font_size * ((scale_x + scale_y) * 0.5);
            let scale = em_px_scale(&loaded.font, px_font_size);
            let gx = origin_x + pg.x * scale_x;
            let gy = origin_y + pg.y_baseline * scale_y;

            let glyph =
                ab_glyph::GlyphId(pg.glyph_id).with_scale_and_position(scale, point(gx, gy));

            if let Some(outlined) = loaded.font.outline_glyph(glyph) {
                let bounds = outlined.px_bounds();
                outlined.draw(|rx, ry, cov| {
                    if cov <= 0.01 {
                        return;
                    }
                    let src_x = bounds.min.x + rx as f32;
                    let src_y = bounds.min.y + ry as f32;
                    let (tx, ty) = transform_pt(src_x, src_y);
                    let ix = tx.round() as i32;
                    let iy = ty.round() as i32;
                    if ix >= 0 && iy >= 0 && (ix as u32) < img_w && (iy as u32) < img_h {
                        let a = ((cov * eff_alpha) * 255.0).round().clamp(0.0, 255.0) as u8;
                        blend_rgba_pixel(
                            img,
                            ix as u32,
                            iy as u32,
                            [color[0], color[1], color[2], a],
                        );
                    }
                });
            }
        }

        for (start, end) in underline_spans(line, underline) {
            let uy = origin_y + (line.y_baseline + layout.font_size * 0.12) * scale_y;
            let x0 = origin_x + start * scale_x;
            let x1 = origin_x + end * scale_x;
            let thickness = (layout.font_size * 0.065 * scale_y).max(1.);
            let steps = ((x1 - x0).abs().ceil() as i32).max(1);
            for s in 0..=steps {
                for offset in 0..thickness.ceil() as i32 {
                    let (x, y) = transform_pt(
                        x0 + (s as f32 / steps as f32) * (x1 - x0),
                        uy + offset as f32,
                    );
                    if x >= 0. && y >= 0. && x < img_w as f32 && y < img_h as f32 {
                        blend_rgba_pixel(
                            img,
                            x as u32,
                            y as u32,
                            [color[0], color[1], color[2], (eff_alpha * 255.) as u8],
                        );
                    }
                }
            }
        }
    }
}

#[inline]
pub fn blend_rgba_pixel(img: &mut image::RgbaImage, x: u32, y: u32, src: ColorRgba) {
    let sa = (src[3] as f32) / 255.0;
    if sa <= 0.001 {
        return;
    }
    let dst = img.get_pixel_mut(x, y);
    let da = (dst.0[3] as f32) / 255.0;
    let out_a = sa + da * (1.0 - sa);
    if out_a <= 0.001 {
        return;
    }
    let r = ((src[0] as f32) * sa + (dst.0[0] as f32) * da * (1.0 - sa)) / out_a;
    let g = ((src[1] as f32) * sa + (dst.0[1] as f32) * da * (1.0 - sa)) / out_a;
    let b = ((src[2] as f32) * sa + (dst.0[2] as f32) * da * (1.0 - sa)) / out_a;
    *dst = image::Rgba([
        r.round().clamp(0.0, 255.0) as u8,
        g.round().clamp(0.0, 255.0) as u8,
        b.round().clamp(0.0, 255.0) as u8,
        (out_a * 255.0).round().clamp(0.0, 255.0) as u8,
    ]);
}
