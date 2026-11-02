//! Character ranges refer to Unicode scalar indices in the editable source text.
//! Wrapping retains the original byte offsets, including repeated whitespace.
use crate::model::font_registry::{FontFaceId, FontRegistry};
use crate::model::text_layout::{
    em_px_scale, layout_text_block, PositionedGlyph, TextLayoutResult, TextLineLayout,
};
use crate::model::{FontFamily, ListStyle, TextAlign, TextFormatting};
use ab_glyph::{Font, ScaleFont};

#[allow(clippy::too_many_arguments)]
pub fn layout_formatted(
    content: &str,
    family: FontFamily,
    size: f32,
    bold: bool,
    italic: bool,
    uppercase: bool,
    align: TextAlign,
    tracking: f32,
    leading: f32,
    list: ListStyle,
    width: f32,
    height: f32,
    format: &TextFormatting,
) -> TextLayoutResult {
    if format == &TextFormatting::default() {
        return layout_text_block(
            content, family, size, bold, italic, uppercase, align, tracking, leading, list, width,
            height,
        );
    }
    let registry = FontRegistry::global();
    let size = size.max(4.);
    let leading = leading.max(0.8);
    let line_height = size * leading;
    let available = (width - format.inset_left - format.inset_right).max(1.);
    let baseline = size * (0.88 + (leading - 1.).clamp(-0.2, 1.5) * 0.28);
    let mut y = format.inset_top;
    let mut char_base = 0;
    let mut lines = Vec::new();
    let mut measured_width = 0f32;
    let mut fallback = false;
    let mut overflow_width = false;
    for (paragraph, source) in content.split('\n').enumerate() {
        let prefix = match list {
            ListStyle::Bullet => "• ".to_string(),
            ListStyle::Numbered => format!("{}. ", paragraph + 1),
            ListStyle::None => String::new(),
        };
        let mut text = prefix;
        let mut mapping = std::collections::HashMap::new();
        for (index, ch) in source.trim_end_matches('\r').chars().enumerate() {
            let rendered = if uppercase {
                ch.to_uppercase().collect::<String>()
            } else {
                ch.to_string()
            };
            for rendered in rendered.chars() {
                mapping.insert(text.len(), char_base + index);
                text.push(rendered);
            }
        }
        let measure = |segment: &str, byte_start: usize| {
            let mut x = 0f32;
            let mut previous = None;
            let mut result = Vec::new();
            for (byte, ch) in segment.char_indices() {
                let mut weight = bold;
                let mut slant = italic;
                let mut face_family = family;
                let mut underline = None;
                if let Some(index) = mapping.get(&(byte_start + byte)) {
                    for run in format
                        .runs
                        .iter()
                        .filter(|r| r.start <= *index && *index < r.end)
                    {
                        if let Some(v) = run.bold {
                            weight = v;
                        }
                        if let Some(v) = run.italic {
                            slant = v;
                        }
                        if let Some(v) = run.font_family {
                            face_family = v;
                        }
                        if let Some(v) = run.underline {
                            underline = Some(v);
                        }
                    }
                }
                let primary = FontFaceId::resolve(face_family, weight, slant);
                if let Some((face_id, loaded, gid, used_fallback)) =
                    registry.resolve_glyph_for_char(primary, ch)
                {
                    let scaled = loaded.font.as_scaled(em_px_scale(&loaded.font, size));
                    if let Some((prev_face, prev_gid)) = previous {
                        if prev_face == face_id {
                            x += scaled.kern(prev_gid, gid);
                        }
                        x += tracking;
                    }
                    let advance = scaled.h_advance(gid);
                    result.push(PositionedGlyph {
                        ch,
                        byte_offset: byte_start + byte,
                        face_id,
                        glyph_id: gid.0,
                        x,
                        y_baseline: 0.,
                        advance,
                        used_fallback,
                        underline_override: underline,
                    });
                    x += advance;
                    previous = Some((face_id, gid));
                }
            }
            (x, result)
        };
        let (_, paragraph_glyphs) = measure(&text, 0);
        y += format.space_before;
        let mut start = 0;
        let mut line_number = 0;
        loop {
            let indent = if line_number == 0 {
                format.first_line_indent
            } else {
                0.
            };
            let room = (available - indent).max(1.);
            let mut end = start;
            let mut last_space = None;
            while end < paragraph_glyphs.len() {
                let candidate = paragraph_glyphs[end].x + paragraph_glyphs[end].advance
                    - paragraph_glyphs[start].x;
                if candidate > room + 0.01 && end > start {
                    break;
                }
                if paragraph_glyphs[end].ch.is_whitespace() {
                    last_space = Some(end);
                }
                end += 1;
                if candidate > room + 0.01 {
                    overflow_width = true;
                    break;
                }
            }
            if end < paragraph_glyphs.len() {
                if let Some(space) = last_space.filter(|s| *s > start) {
                    end = space;
                }
            }
            let byte_start = paragraph_glyphs
                .get(start)
                .map(|g| g.byte_offset)
                .unwrap_or(0);
            let byte_end = paragraph_glyphs
                .get(end.saturating_sub(1))
                .map(|g| g.byte_offset + g.ch.len_utf8())
                .unwrap_or(byte_start);
            let string = if start < end {
                &text[byte_start..byte_end]
            } else {
                ""
            };
            let (line_width, mut glyphs) = measure(string, byte_start);
            let x_offset = format.inset_left
                + indent
                + match align {
                    TextAlign::Left => 0.,
                    TextAlign::Center => ((room - line_width) / 2.).max(0.),
                    TextAlign::Right => (room - line_width).max(0.),
                };
            for glyph in &mut glyphs {
                glyph.x += x_offset;
                glyph.y_baseline = y + baseline;
                fallback |= glyph.used_fallback;
            }
            overflow_width |= line_width > room + 1.;
            measured_width = measured_width.max(x_offset + line_width + format.inset_right);
            lines.push(TextLineLayout {
                text: string.into(),
                line_index: lines.len(),
                x_offset,
                y_top: y,
                y_baseline: y + baseline,
                width: line_width,
                height: line_height,
                glyphs,
            });
            y += line_height;
            line_number += 1;
            start = end;
            while start < paragraph_glyphs.len() && paragraph_glyphs[start].ch.is_whitespace() {
                start += 1;
            }
            if start >= paragraph_glyphs.len() {
                break;
            }
        }
        y += format.space_after;
        char_base += source.chars().count() + 1;
    }
    let measured_height = (y - line_height + baseline + size * 0.26 + format.inset_bottom).max(0.);
    TextLayoutResult {
        primary_face_id: FontFaceId::resolve(family, bold, italic),
        font_size: size,
        line_height_px: line_height,
        box_width: width,
        box_height: height,
        measured_width,
        measured_height,
        overflows_width: overflow_width || measured_width > width + 1.,
        overflows_height: measured_height > height + 1.,
        has_fallback_glyphs: fallback,
        lines,
    }
}
