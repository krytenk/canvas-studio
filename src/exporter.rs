use std::fs;
use std::path::Path;

use base64::Engine;
use image::{ImageBuffer, Rgba, RgbaImage};

use crate::magic_studio::apply_image_pipeline;
use crate::model::asset_store::AssetStore;
use crate::model::pdf_export::{sample_bilinear_rgba, sample_shape_polygon};
use crate::model::render_scene::{
    build_page_scene, mask_coverage, shape_signed_distance, ImagePlacement, LocalToPageTransform,
    SceneItem,
};
use crate::model::text_layout::rasterize_text_layout;
use crate::model::{
    hydrate_project_assets, CanvasElement, CanvasPage, ChartType, ColorRgba, ElementKind,
    FillStyle, ImageAdjustments, ImageFit, ImageMask, PageBackground, Project, ShapeType,
    TextEffect,
};

/// Generates a clean, standalone SVG document string for a given `CanvasPage`
/// using the unified `RenderScene` display list so transforms, flips, font families,
/// transparent fills, strokes, and image crops/masks match PNG, PDF, and preview.
pub fn export_page_to_svg(
    page: &CanvasPage,
    width: f32,
    height: f32,
    transparent_bg: bool,
) -> Result<String, String> {
    let mut project = Project::default();
    project.width = width;
    project.height = height;
    project.width_mm = width / 96. * 25.4;
    project.height_mm = height / 96. * 25.4;
    project.pages = vec![page.clone()];
    project.production.resolved = true;
    for element in &page.elements {
        if let ElementKind::Text { font_family, .. } = &element.kind {
            for family in std::iter::once(*font_family).chain(
                element
                    .text_format
                    .runs
                    .iter()
                    .filter_map(|r| r.font_family),
            ) {
                if let crate::model::FontFamily::Imported(id) = family {
                    if let Some(face) = crate::model::font_registry::FontRegistry::global()
                        .get(crate::model::font_registry::FontFaceId::Imported(id))
                    {
                        if !project.imported_fonts.iter().any(|f| f.id == id) {
                            project
                                .imported_fonts
                                .push(crate::imported_fonts::ImportedFont {
                                    id,
                                    name: face.id.postscript_name(),
                                    bold: face.bold,
                                    italic: face.italic,
                                    bytes: face.bytes.clone(),
                                });
                        }
                    }
                }
            }
        }
    }
    let report = crate::model::pdf_export::run_export_preflight(&project, 96);
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| i.severity == crate::model::pdf_export::PreflightSeverity::Error)
        .map(|i| format!("{}: {}", i.element_name, i.message))
        .collect();
    if !errors.is_empty() {
        return Err(errors.join("; "));
    }
    Ok(render_page_svg(page, width, height, transparent_bg))
}

pub fn export_project_page_to_svg(
    project: &Project,
    index: usize,
    transparent: bool,
) -> Result<String, String> {
    let resolved = crate::production::resolve(project, true)?;
    crate::imported_fonts::register_project(&resolved)?;
    let page = resolved
        .pages
        .get(index)
        .ok_or("Page is outside document")?;
    export_page_to_svg(page, resolved.width, resolved.height, transparent)
}

fn render_page_svg(page: &CanvasPage, width: f32, height: f32, transparent_bg: bool) -> String {
    let w = width.round() as i32;
    let h = height.round() as i32;
    let scene = build_page_scene(page, width, height);
    let mut svg = String::with_capacity(16384);

    svg.push_str(&format!(
        r#"<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 {w} {h}" width="{w}" height="{h}">
<defs>
"#
    ));

    if let PageBackground::LinearGradient { start, end, .. } = &page.background {
        svg.push_str(&format!(
            r#"  <linearGradient id="page_bg_grad" x1="0%" y1="0%" x2="100%" y2="100%">
    <stop offset="0%" stop-color="{}" stop-opacity="{:.2}"/>
    <stop offset="100%" stop-color="{}" stop-opacity="{:.2}"/>
  </linearGradient>
"#,
            hex_rgb(*start),
            start[3] as f32 / 255.0,
            hex_rgb(*end),
            end[3] as f32 / 255.0
        ));
    } else if let PageBackground::RadialGradient { center, outer } = &page.background {
        svg.push_str(&format!(
            r#"  <radialGradient id="page_bg_rad" cx="50%" cy="50%" r="70%">
    <stop offset="0%" stop-color="{}" stop-opacity="{:.2}"/>
    <stop offset="100%" stop-color="{}" stop-opacity="{:.2}"/>
  </radialGradient>
"#,
            hex_rgb(*center),
            center[3] as f32 / 255.0,
            hex_rgb(*outer),
            outer[3] as f32 / 255.0
        ));
    }

    for el in &page.elements {
        if let ElementKind::Shape {
            fill:
                FillStyle::LinearGradient {
                    start,
                    end,
                    vertical,
                },
            ..
        } = &el.kind
        {
            let (x2, y2) = if *vertical {
                ("0%", "100%")
            } else {
                ("100%", "0%")
            };
            svg.push_str(&format!(
                r#"  <linearGradient id="grad_{}" x1="0%" y1="0%" x2="{x2}" y2="{y2}">
    <stop offset="0%" stop-color="{}" stop-opacity="{:.2}"/>
    <stop offset="100%" stop-color="{}" stop-opacity="{:.2}"/>
  </linearGradient>
"#,
                el.id,
                hex_rgb(*start),
                start[3] as f32 / 255.0,
                hex_rgb(*end),
                end[3] as f32 / 255.0
            ));
        }
    }

    let mut used = std::collections::HashSet::new();
    svg.push_str("<style>\n");
    for item in &scene.items {
        if let SceneItem::Text { layout, .. } = item {
            for g in layout.lines.iter().flat_map(|l| &l.glyphs) {
                if used.insert(g.face_id) {
                    if let Some(face) =
                        crate::model::font_registry::FontRegistry::global().get(g.face_id)
                    {
                        svg.push_str(&format!("@font-face{{font-family:'{}';font-weight:{};font-style:{};src:url(data:font/ttf;base64,{}) format('truetype');}}\n",g.face_id.postscript_name(),if g.face_id.is_bold(){"bold"}else{"normal"},if g.face_id.is_italic(){"italic"}else{"normal"},base64::engine::general_purpose::STANDARD.encode(face.bytes.as_slice())));
                    }
                }
            }
        }
    }
    svg.push_str("</style>\n</defs>\n");

    // Page background (unless transparent export is requested)
    if !transparent_bg {
        match &page.background {
            PageBackground::Solid(c) => {
                if c[3] > 0 {
                    svg.push_str(&format!(
                        r#"<rect x="0" y="0" width="{w}" height="{h}" {}/>"#,
                        svg_fill_attr(*c)
                    ));
                    svg.push('\n');
                }
            }
            PageBackground::LinearGradient { .. } => {
                svg.push_str(&format!(
                    r#"<rect x="0" y="0" width="{w}" height="{h}" fill="url(#page_bg_grad)"/>"#
                ));
                svg.push('\n');
            }
            PageBackground::RadialGradient { .. } => {
                svg.push_str(&format!(
                    r#"<rect x="0" y="0" width="{w}" height="{h}" fill="url(#page_bg_rad)"/>"#
                ));
                svg.push('\n');
            }
            PageBackground::GridPattern { bg, .. } => {
                svg.push_str(&format!(
                    r#"<rect x="0" y="0" width="{w}" height="{h}" {}/>"#,
                    svg_fill_attr(*bg)
                ));
                svg.push('\n');
            }
        }
    }

    for item in &scene.items {
        match item {
            SceneItem::Shape {
                element_id,
                transform,
                shape_type,
                fill,
                stroke_color,
                stroke_width,
                corner_radius,
                opacity,
            } => {
                let tf_attr = transform.svg_transform_attr();
                svg.push_str(&format!(
                    r#"<g opacity="{opacity:.2}" data-canvas-id="{element_id}"{tf_attr}>"#
                ));
                svg.push('\n');

                let fill_attr = match fill {
                    FillStyle::Solid(c) => svg_fill_attr(*c),
                    FillStyle::LinearGradient { .. } => {
                        format!(r#"fill="url(#grad_{element_id})""#)
                    }
                };
                let stroke_attr = svg_stroke_attr(*stroke_color, *stroke_width);

                match shape_type {
                    ShapeType::Circle => {
                        svg.push_str(&format!(
                            r#"  <ellipse cx="{:.1}" cy="{:.1}" rx="{:.1}" ry="{:.1}" {fill_attr} {stroke_attr}/>"#,
                            transform.cx,
                            transform.cy,
                            transform.width * 0.5,
                            transform.height * 0.5
                        ));
                    }
                    ShapeType::Pill => {
                        let r = (transform.width.min(transform.height) * 0.5).max(1.0);
                        svg.push_str(&format!(
                            r#"  <rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{r:.1}" {fill_attr} {stroke_attr}/>"#,
                            transform.x, transform.y, transform.width, transform.height
                        ));
                    }
                    ShapeType::Rectangle | ShapeType::RoundedRect => {
                        let r = if *shape_type == ShapeType::RoundedRect {
                            *corner_radius
                        } else {
                            0.0
                        };
                        svg.push_str(&format!(
                            r#"  <rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{r:.1}" {fill_attr} {stroke_attr}/>"#,
                            transform.x, transform.y, transform.width, transform.height
                        ));
                    }
                    _ => {
                        let pts = compute_shape_polygon_points(
                            *shape_type,
                            transform.x,
                            transform.y,
                            transform.width,
                            transform.height,
                        );
                        let pts_str: Vec<String> = pts
                            .iter()
                            .map(|(px, py)| format!("{px:.1},{py:.1}"))
                            .collect();
                        svg.push_str(&format!(
                            r#"  <polygon points="{}" {fill_attr} {stroke_attr}/>"#,
                            pts_str.join(" ")
                        ));
                    }
                }
                svg.push('\n');
                svg.push_str("</g>\n");
            }
            SceneItem::Text {
                element_id,
                transform,
                layout,
                color,
                underline,
                opacity,
                ..
            } => {
                let tf_attr = transform.svg_transform_attr();
                svg.push_str(&format!(
                    r#"<g opacity="{opacity:.2}" data-canvas-id="{element_id}"{tf_attr}>"#
                ));
                svg.push('\n');

                if let Some(el) = page.elements.iter().find(|e| e.id == *element_id) {
                    if let ElementKind::Text {
                        effect:
                            TextEffect::BackgroundPill {
                                bg_color,
                                radius,
                                padding,
                            },
                        ..
                    } = &el.kind
                    {
                        svg.push_str(&format!(
                            r#"  <rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" rx="{radius:.1}" {}/>"#,
                            transform.x - padding,
                            transform.y - padding * 0.5,
                            transform.width + padding * 2.0,
                            transform.height + padding,
                            svg_fill_attr(*bg_color)
                        ));
                        svg.push('\n');
                    }
                }

                let fill_attr = svg_fill_attr(*color);
                for line in &layout.lines {
                    svg.push_str(&format!(
                        r#"  <text aria-label="{}" font-size="{:.4}" {fill_attr}>"#,
                        xml_escape(&line.text),
                        layout.font_size
                    ));
                    for glyph in &line.glyphs {
                        svg.push_str(&format!(r#"<tspan x="{:.4}" y="{:.4}" font-family="{}" font-weight="{}" font-style="{}">{}</tspan>"#,transform.x+glyph.x,transform.y+glyph.y_baseline,glyph.face_id.postscript_name(),if glyph.face_id.is_bold(){"bold"}else{"normal"},if glyph.face_id.is_italic(){"italic"}else{"normal"},xml_escape(&glyph.ch.to_string())));
                    }
                    svg.push_str("</text>\n");
                    for (start, end) in crate::model::text_layout::underline_spans(line, *underline)
                    {
                        let y = transform.y + line.y_baseline + layout.font_size * 0.12;
                        svg.push_str(&format!(
                            "<line x1=\"{}\" y1=\"{y}\" x2=\"{}\" y2=\"{y}\" {}/>\n",
                            transform.x + start,
                            transform.x + end,
                            svg_stroke_attr(*color, layout.font_size * 0.06)
                        ));
                    }
                }

                svg.push_str("</g>\n");
            }
            SceneItem::Image {
                element_id,
                transform,
                width_px,
                height_px,
                raw_rgba,
                mask,
                adjustments,
                placement,
                border_color,
                border_width,
                opacity,
                ..
            } => {
                let tf_attr = transform.svg_transform_attr();
                svg.push_str(&format!(
                    r#"<g opacity="{opacity:.2}" data-canvas-id="{element_id}"{tf_attr}>"#
                ));
                svg.push('\n');

                if let Some(cropped_img) = render_local_image_frame(
                    *width_px,
                    *height_px,
                    raw_rgba,
                    *mask,
                    adjustments,
                    placement,
                    transform.width.round().max(1.0) as u32,
                    transform.height.round().max(1.0) as u32,
                ) {
                    let mut png_bytes: Vec<u8> = Vec::new();
                    let mut cursor = std::io::Cursor::new(&mut png_bytes);
                    if cropped_img
                        .write_to(&mut cursor, image::ImageFormat::Png)
                        .is_ok()
                    {
                        let b64 = base64::engine::general_purpose::STANDARD.encode(&png_bytes);
                        let par = match adjustments.fit {
                            ImageFit::Contain => "xMidYMid meet",
                            ImageFit::Cover => "xMidYMid slice",
                            ImageFit::Stretch => "none",
                        };
                        svg.push_str(&format!(
                            r#"  <image x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" preserveAspectRatio="{par}" href="data:image/png;base64,{b64}"/>"#,
                            transform.x, transform.y, transform.width, transform.height
                        ));
                        svg.push('\n');
                    }
                }

                if *border_width > 0.1 && border_color[3] > 0 {
                    svg.push_str(&format!(
                        r#"  <rect x="{:.1}" y="{:.1}" width="{:.1}" height="{:.1}" fill="none" {}/>"#,
                        transform.x,
                        transform.y,
                        transform.width,
                        transform.height,
                        svg_stroke_attr(*border_color, *border_width)
                    ));
                    svg.push('\n');
                }

                svg.push_str("</g>\n");
            }
            SceneItem::FallbackElement(el) => {
                emit_fallback_svg_element(&mut svg, el);
            }
        }
    }

    svg.push_str("</svg>\n");
    svg
}

fn svg_fill_attr(c: ColorRgba) -> String {
    if c[3] == 0 {
        r#"fill="none" fill-opacity="0.00""#.to_string()
    } else if c[3] == 255 {
        format!(r#"fill="{}""#, hex_rgb(c))
    } else {
        format!(
            r#"fill="{}" fill-opacity="{:.3}""#,
            hex_rgb(c),
            c[3] as f32 / 255.0
        )
    }
}

fn svg_stroke_attr(c: ColorRgba, stroke_width: f32) -> String {
    if stroke_width <= 0.05 || c[3] == 0 {
        String::new()
    } else if c[3] == 255 {
        format!(
            r#"stroke="{}" stroke-width="{stroke_width:.2}""#,
            hex_rgb(c)
        )
    } else {
        format!(
            r#"stroke="{}" stroke-opacity="{:.3}" stroke-width="{stroke_width:.2}""#,
            hex_rgb(c),
            c[3] as f32 / 255.0
        )
    }
}

fn emit_fallback_svg_element(svg: &mut String, el: &CanvasElement) {
    let tf = LocalToPageTransform::from_element(el);
    let tf_attr = tf.svg_transform_attr();
    let opacity = el.opacity.clamp(0.0, 1.0);
    let element_id = el.id;
    svg.push_str(&format!(
        r#"<g opacity="{opacity:.2}" data-canvas-id="{element_id}"{tf_attr}>"#
    ));
    svg.push('\n');

    match &el.kind {
        ElementKind::FreehandPath {
            points,
            color,
            stroke_width,
            ..
        } => {
            if points.len() >= 2 {
                let mut d = String::new();
                for (idx, (nx, ny)) in points.iter().enumerate() {
                    let px = el.x + nx * el.width;
                    let py = el.y + ny * el.height;
                    if idx == 0 {
                        d.push_str(&format!("M {px:.1} {py:.1}"));
                    } else {
                        d.push_str(&format!(" L {px:.1} {py:.1}"));
                    }
                }
                svg.push_str(&format!(
                    r#"  <path d="{d}" fill="none" stroke="{}" stroke-width="{stroke_width:.1}" stroke-linecap="round" stroke-linejoin="round"/>"#,
                    hex_rgb(*color)
                ));
                svg.push('\n');
            }
        }
        ElementKind::Chart {
            chart_type,
            title,
            labels,
            values,
            palette,
            text_color,
            ..
        } => {
            svg.push_str(&format!(
                r#"  <text x="{:.1}" y="{:.1}" font-family="'Noto Sans', sans-serif" font-size="22" font-weight="bold" fill="{}">{}</text>"#,
                el.x + 16.0,
                el.y + 32.0,
                hex_rgb(*text_color),
                xml_escape(title)
            ));
            svg.push('\n');
            let max_val = values.iter().copied().fold(1.0_f32, f32::max);
            let n = values.len().max(1);

            match chart_type {
                ChartType::VerticalBar => {
                    let chart_h = (el.height - 90.0).max(40.0);
                    let bar_w = ((el.width - 40.0) / (n as f32) * 0.65).max(12.0);
                    let step_x = (el.width - 40.0) / (n as f32);
                    for (i, val) in values.iter().enumerate() {
                        let bh = (val / max_val) * chart_h;
                        let bx = el.x + 28.0 + (i as f32) * step_x;
                        let by = el.y + el.height - 36.0 - bh;
                        let col = palette
                            .get(i % palette.len().max(1))
                            .copied()
                            .unwrap_or([0, 196, 204, 255]);
                        svg.push_str(&format!(
                            r#"  <rect x="{bx:.1}" y="{by:.1}" width="{bar_w:.1}" height="{bh:.1}" rx="6" fill="{}"/>"#,
                            hex_rgb(col)
                        ));
                        svg.push('\n');
                        if let Some(lbl) = labels.get(i) {
                            svg.push_str(&format!(
                                r#"  <text x="{:.1}" y="{:.1}" font-family="'Noto Sans', sans-serif" font-size="14" fill="{}" text-anchor="middle">{}</text>"#,
                                bx + bar_w * 0.5,
                                el.y + el.height - 12.0,
                                hex_rgb(*text_color),
                                xml_escape(lbl)
                            ));
                            svg.push('\n');
                        }
                    }
                }
                ChartType::HorizontalBar => {
                    let chart_w = (el.width - 140.0).max(40.0);
                    let step_y = (el.height - 70.0) / (n as f32);
                    let bar_h = (step_y * 0.62).max(10.0);
                    for (i, val) in values.iter().enumerate() {
                        let bw = (val / max_val) * chart_w;
                        let bx = el.x + 110.0;
                        let by = el.y + 54.0 + (i as f32) * step_y;
                        let col = palette
                            .get(i % palette.len().max(1))
                            .copied()
                            .unwrap_or([0, 196, 204, 255]);
                        svg.push_str(&format!(
                            r#"  <rect x="{bx:.1}" y="{by:.1}" width="{bw:.1}" height="{bar_h:.1}" rx="5" fill="{}"/>"#,
                            hex_rgb(col)
                        ));
                        svg.push('\n');
                    }
                }
                ChartType::Pie | ChartType::Donut => {
                    let sum: f32 = values.iter().copied().sum::<f32>().max(1.0);
                    let r = (el.width.min(el.height - 50.0) * 0.38).max(20.0);
                    let center_x = el.x + el.width * 0.5;
                    let center_y = el.y + 40.0 + (el.height - 40.0) * 0.5;
                    let mut start_angle = -std::f32::consts::FRAC_PI_2;
                    for (i, val) in values.iter().enumerate() {
                        let sweep = (*val / sum) * std::f32::consts::TAU;
                        let steps = ((sweep * 12.0).ceil() as usize).max(3);
                        let col = palette
                            .get(i % palette.len().max(1))
                            .copied()
                            .unwrap_or([0, 196, 204, 255]);
                        let mut pts = vec![format!("{center_x:.1},{center_y:.1}")];
                        for s in 0..=steps {
                            let a = start_angle + (s as f32 / steps as f32) * sweep;
                            pts.push(format!(
                                "{:.1},{:.1}",
                                center_x + r * a.cos(),
                                center_y + r * a.sin()
                            ));
                        }
                        svg.push_str(&format!(
                            r#"  <polygon points="{}" fill="{}"/>"#,
                            pts.join(" "),
                            hex_rgb(col)
                        ));
                        svg.push('\n');
                        start_angle += sweep;
                    }
                    if *chart_type == ChartType::Donut {
                        svg.push_str(&format!(
                            r##"  <circle cx="{center_x:.1}" cy="{center_y:.1}" r="{:.1}" fill="#0F172A"/>"##,
                            r * 0.54
                        ));
                        svg.push('\n');
                    }
                }
                ChartType::LineArea => {
                    if values.len() >= 2 {
                        let plot_w = (el.width - 60.0).max(20.0);
                        let plot_h = (el.height - 90.0).max(20.0);
                        let mut pts = Vec::new();
                        for (i, val) in values.iter().enumerate() {
                            let px = el.x + 30.0 + (i as f32 / (values.len() - 1) as f32) * plot_w;
                            let py = el.y + el.height - 36.0 - (val / max_val) * plot_h;
                            pts.push(format!("{px:.1},{py:.1}"));
                        }
                        let col = palette.first().copied().unwrap_or([0, 196, 204, 255]);
                        svg.push_str(&format!(
                            r#"  <polyline points="{}" fill="none" stroke="{}" stroke-width="4"/>"#,
                            pts.join(" "),
                            hex_rgb(col)
                        ));
                        svg.push('\n');
                    }
                }
            }
        }
        ElementKind::Table {
            rows,
            cols,
            cells,
            header_bg,
            row_bg,
            border_color,
            text_color,
            font_size,
            ..
        } => {
            let r_cnt = (*rows).max(1);
            let c_cnt = (*cols).max(1);
            let cw = el.width / (c_cnt as f32);
            let rh = el.height / (r_cnt as f32);
            for r in 0..r_cnt {
                for c in 0..c_cnt {
                    let cx_pos = el.x + (c as f32) * cw;
                    let cy_pos = el.y + (r as f32) * rh;
                    let bg = if r == 0 { *header_bg } else { *row_bg };
                    svg.push_str(&format!(
                        r#"  <rect x="{cx_pos:.1}" y="{cy_pos:.1}" width="{cw:.1}" height="{rh:.1}" fill="{}" stroke="{}" stroke-width="1"/>"#,
                        hex_rgb(bg),
                        hex_rgb(*border_color)
                    ));
                    if let Some(txt) = cells.get(r).and_then(|row| row.get(c)) {
                        svg.push_str(&format!(
                            r#"  <text x="{:.1}" y="{:.1}" font-family="'Noto Sans', sans-serif" font-size="{font_size:.1}" fill="{}">{}</text>"#,
                            cx_pos + 10.0,
                            cy_pos + rh * 0.62,
                            hex_rgb(*text_color),
                            xml_escape(txt)
                        ));
                    }
                }
            }
        }
        _ => {}
    }

    svg.push_str("</g>\n");
}

#[allow(clippy::too_many_arguments)]
fn render_local_image_frame(
    width_px: u32,
    height_px: u32,
    raw_rgba: &[u8],
    mask: ImageMask,
    adjustments: &ImageAdjustments,
    placement: &ImagePlacement,
    out_w: u32,
    out_h: u32,
) -> Option<RgbaImage> {
    if width_px == 0
        || height_px == 0
        || raw_rgba.len() < (width_px as usize) * (height_px as usize) * 4
    {
        return None;
    }
    let processed = apply_image_pipeline(raw_rgba, width_px, height_px, adjustments);
    let mut frame = RgbaImage::new(out_w.max(1), out_h.max(1));
    let fw = placement.clip_w.max(out_w as f32);
    let fh = placement.clip_h.max(out_h as f32);
    let _ = (fw, fh);

    // Map local coordinates (0..frame_w, 0..frame_h)
    let frame_w = if adjustments.fit == ImageFit::Contain {
        (placement.dest_w + placement.dest_x * 2.0).max(1.0)
    } else {
        placement.clip_w.max(1.0)
    };
    let frame_h = if adjustments.fit == ImageFit::Contain {
        (placement.dest_h + placement.dest_y * 2.0).max(1.0)
    } else {
        placement.clip_h.max(1.0)
    };

    for py in 0..frame.height() {
        let ly = (py as f32 + 0.5) / (frame.height() as f32) * frame_h;
        for px in 0..frame.width() {
            let lx = (px as f32 + 0.5) / (frame.width() as f32) * frame_w;
            let cov = mask_coverage(mask, lx, ly, frame_w, frame_h);
            if cov <= 0.001 {
                continue;
            }
            if let Some((sx, sy)) = placement.local_to_source_px(lx, ly) {
                let mut col = sample_bilinear_rgba(&processed, width_px, height_px, sx, sy);
                col[3] = ((col[3] as f32) * cov).round().clamp(0.0, 255.0) as u8;
                frame.put_pixel(px, py, Rgba(col));
            }
        }
    }
    Some(frame)
}

/// Renders a project page at a specific print DPI (`96`, `150`, `300`, etc.) using physical dimensions.
pub fn render_project_page_at_dpi(
    project: &Project,
    page_idx: usize,
    dpi: u32,
    transparent_bg: bool,
) -> Result<RgbaImage, String> {
    let resolved = crate::production::resolve(project, true)?;
    let project = resolved.as_ref();
    let page = project
        .pages
        .get(page_idx)
        .ok_or_else(|| format!("Page index {page_idx} out of bounds"))?;
    let report = crate::model::pdf_export::run_export_preflight(project, dpi);
    let fatal: Vec<_> = report
        .issues
        .iter()
        .filter(|i| {
            i.severity == crate::model::pdf_export::PreflightSeverity::Error
                && (i.element_id == 0 || i.page_index == page_idx)
        })
        .map(|i| i.message.clone())
        .collect();
    if !fatal.is_empty() {
        return Err(fatal.join("\n"));
    }
    let (out_w, out_h) = project.validate_export_dimensions(dpi)?;
    Ok(render_page_to_rgba_image(
        page,
        project.width,
        project.height,
        out_w,
        out_h,
        transparent_bg,
    ))
}

/// High-resolution software rasterizer: renders a `CanvasPage` into an `RgbaImage`
/// at exact output pixel dimensions `(out_width, out_height)`, automatically scaling
/// from the logical canvas size `(canvas_w, canvas_h)`.
pub fn render_page_to_rgba_image(
    page: &CanvasPage,
    canvas_w: f32,
    canvas_h: f32,
    out_width: u32,
    out_height: u32,
    transparent_bg: bool,
) -> RgbaImage {
    let w = out_width.max(16);
    let h = out_height.max(16);
    let scale_x = (w as f32) / canvas_w.max(1.0);
    let scale_y = (h as f32) / canvas_h.max(1.0);
    let uniform_scale = scale_x.min(scale_y);

    let mut img: RgbaImage = ImageBuffer::new(w, h);

    // 1. Render Page Background
    if !transparent_bg {
        for y in 0..h {
            let ty = y as f32 / (h as f32);
            for x in 0..w {
                let tx = x as f32 / (w as f32);
                let c = match &page.background {
                    PageBackground::Solid(col) => *col,
                    PageBackground::LinearGradient { start, end, .. } => {
                        let t = ((tx + ty) * 0.5).clamp(0.0, 1.0);
                        lerp_color(*start, *end, t)
                    }
                    PageBackground::RadialGradient { center, outer } => {
                        let dx = tx - 0.5;
                        let dy = ty - 0.5;
                        let d = ((dx * dx + dy * dy).sqrt() * 1.414).clamp(0.0, 1.0);
                        lerp_color(*center, *outer, d)
                    }
                    PageBackground::GridPattern { bg, line, spacing } => {
                        let sp = ((*spacing * uniform_scale).round() as u32).max(8);
                        if x % sp == 0 || y % sp == 0 {
                            *line
                        } else {
                            *bg
                        }
                    }
                };
                img.put_pixel(x, y, Rgba(c));
            }
        }
    }

    // 2. Build unified display list at logical canvas resolution and rasterize each item at output scale
    let scene = build_page_scene(page, canvas_w, canvas_h);

    for item in &scene.items {
        match item {
            SceneItem::Shape {
                transform,
                shape_type,
                fill,
                stroke_color,
                stroke_width,
                corner_radius,
                opacity,
                ..
            } => {
                rasterize_scene_shape(
                    &mut img,
                    transform,
                    *shape_type,
                    *fill,
                    *stroke_color,
                    *stroke_width,
                    *corner_radius,
                    *opacity,
                    scale_x,
                    scale_y,
                );
            }
            SceneItem::Image {
                transform,
                width_px,
                height_px,
                raw_rgba,
                mask,
                adjustments,
                placement,
                border_color,
                border_width,
                opacity,
                ..
            } => {
                rasterize_scene_image(
                    &mut img,
                    transform,
                    *width_px,
                    *height_px,
                    raw_rgba,
                    *mask,
                    adjustments,
                    placement,
                    *border_color,
                    *border_width,
                    *opacity,
                    scale_x,
                    scale_y,
                );
            }
            SceneItem::Text {
                element_id,
                transform,
                layout,
                color,
                underline,
                opacity,
                ..
            } => {
                if let Some(el) = page.elements.iter().find(|e| e.id == *element_id) {
                    if let ElementKind::Text {
                        effect:
                            TextEffect::BackgroundPill {
                                bg_color,
                                radius,
                                padding,
                            },
                        ..
                    } = &el.kind
                    {
                        let mut bg_el = el.clone();
                        bg_el.x -= *padding;
                        bg_el.y -= *padding * 0.5;
                        bg_el.width += *padding * 2.0;
                        bg_el.height += *padding;
                        let bg_tf = LocalToPageTransform::from_element(&bg_el);
                        rasterize_scene_shape(
                            &mut img,
                            &bg_tf,
                            ShapeType::RoundedRect,
                            FillStyle::Solid(*bg_color),
                            [0, 0, 0, 0],
                            0.0,
                            *radius,
                            *opacity,
                            scale_x,
                            scale_y,
                        );
                    }
                }

                rasterize_text_layout(
                    &mut img,
                    layout,
                    transform.x * scale_x,
                    transform.y * scale_y,
                    scale_x,
                    scale_y,
                    transform.rotation_deg,
                    transform.flip_h,
                    transform.flip_v,
                    *underline,
                    *color,
                    *opacity,
                );
            }
            SceneItem::FallbackElement(el) => {
                let mut scaled_el = el.clone();
                scaled_el.x *= scale_x;
                scaled_el.y *= scale_y;
                scaled_el.width *= scale_x;
                scaled_el.height *= scale_y;
                match &mut scaled_el.kind {
                    ElementKind::FreehandPath { stroke_width, .. } => {
                        *stroke_width *= uniform_scale;
                    }
                    ElementKind::Table { font_size, .. } => {
                        *font_size *= uniform_scale;
                    }
                    _ => {}
                }
                rasterize_fallback_element(&mut img, &scaled_el);
            }
        }
    }

    img
}

#[allow(clippy::too_many_arguments)]
fn rasterize_scene_shape(
    img: &mut RgbaImage,
    tf: &LocalToPageTransform,
    shape_type: ShapeType,
    fill: FillStyle,
    stroke_color: ColorRgba,
    stroke_width: f32,
    corner_radius: f32,
    opacity: f32,
    scale_x: f32,
    scale_y: f32,
) {
    let img_w = img.width() as i32;
    let img_h = img.height() as i32;
    let pad = stroke_width.max(2.0) + 2.0;
    let (min_x, min_y, max_x, max_y) = tf.page_aabb(pad);

    let x0 = ((min_x * scale_x).floor() as i32).max(0);
    let y0 = ((min_y * scale_y).floor() as i32).max(0);
    let x1 = ((max_x * scale_x).ceil() as i32).min(img_w - 1);
    let y1 = ((max_y * scale_y).ceil() as i32).min(img_h - 1);
    if x1 < x0 || y1 < y0 {
        return;
    }

    let local_poly = sample_shape_polygon(shape_type, tf.width, tf.height, corner_radius);
    let op = opacity.clamp(0.0, 1.0);
    let has_stroke = stroke_width > 0.05 && stroke_color[3] > 0;
    let half_sw = stroke_width * 0.5;
    let aa_feather = (1.0 / scale_x.min(scale_y).max(0.25)).clamp(0.35, 1.25);

    for py in y0..=y1 {
        let page_y = (py as f32 + 0.5) / scale_y;
        for px in x0..=x1 {
            let page_x = (px as f32 + 0.5) / scale_x;
            let (lx, ly) = tf.page_to_local(page_x, page_y);

            let sd = shape_signed_distance(
                shape_type,
                corner_radius,
                lx,
                ly,
                tf.width,
                tf.height,
                &local_poly,
            );

            // Fill coverage
            let fill_cov = ((aa_feather * 0.5 - sd) / aa_feather).clamp(0.0, 1.0);
            if fill_cov > 0.001 {
                let tx = (lx / tf.width.max(1.0)).clamp(0.0, 1.0);
                let ty = (ly / tf.height.max(1.0)).clamp(0.0, 1.0);
                let mut col = match fill {
                    FillStyle::Solid(c) => c,
                    FillStyle::LinearGradient {
                        start,
                        end,
                        vertical,
                    } => {
                        let t = if vertical { ty } else { tx };
                        lerp_color(start, end, t)
                    }
                };
                if col[3] > 0 {
                    col[3] = ((col[3] as f32) * op * fill_cov).round().clamp(0.0, 255.0) as u8;
                    blend_pixel(img, px as u32, py as u32, col);
                }
            }

            // Stroke coverage
            if has_stroke {
                let stroke_dist = sd.abs() - half_sw;
                let stroke_cov = ((aa_feather * 0.5 - stroke_dist) / aa_feather).clamp(0.0, 1.0);
                if stroke_cov > 0.001 {
                    let mut scol = stroke_color;
                    scol[3] = ((scol[3] as f32) * op * stroke_cov)
                        .round()
                        .clamp(0.0, 255.0) as u8;
                    blend_pixel(img, px as u32, py as u32, scol);
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn rasterize_scene_image(
    img: &mut RgbaImage,
    tf: &LocalToPageTransform,
    width_px: u32,
    height_px: u32,
    raw_rgba: &[u8],
    mask: ImageMask,
    adjustments: &ImageAdjustments,
    placement: &ImagePlacement,
    border_color: ColorRgba,
    border_width: f32,
    opacity: f32,
    scale_x: f32,
    scale_y: f32,
) {
    if width_px == 0
        || height_px == 0
        || raw_rgba.len() < (width_px as usize) * (height_px as usize) * 4
    {
        return;
    }

    let processed = apply_image_pipeline(raw_rgba, width_px, height_px, adjustments);
    let img_w = img.width() as i32;
    let img_h = img.height() as i32;
    let pad = border_width.max(1.0) + 2.0;
    let (min_x, min_y, max_x, max_y) = tf.page_aabb(pad);

    let x0 = ((min_x * scale_x).floor() as i32).max(0);
    let y0 = ((min_y * scale_y).floor() as i32).max(0);
    let x1 = ((max_x * scale_x).ceil() as i32).min(img_w - 1);
    let y1 = ((max_y * scale_y).ceil() as i32).min(img_h - 1);
    if x1 < x0 || y1 < y0 {
        return;
    }

    let op = opacity.clamp(0.0, 1.0);

    for py in y0..=y1 {
        let page_y = (py as f32 + 0.5) / scale_y;
        for px in x0..=x1 {
            let page_x = (px as f32 + 0.5) / scale_x;
            let (lx, ly) = tf.page_to_local(page_x, page_y);

            let m_cov = mask_coverage(mask, lx, ly, tf.width, tf.height);
            if m_cov <= 0.001 {
                continue;
            }

            if let Some((sx, sy)) = placement.local_to_source_px(lx, ly) {
                let mut col = sample_bilinear_rgba(&processed, width_px, height_px, sx, sy);
                col[3] = ((col[3] as f32) * op * m_cov).round().clamp(0.0, 255.0) as u8;
                blend_pixel(img, px as u32, py as u32, col);
            }
        }
    }

    if border_width > 0.05 && border_color[3] > 0 {
        let shape_for_mask = match mask {
            ImageMask::None => ShapeType::Rectangle,
            ImageMask::Rounded(_) => ShapeType::RoundedRect,
            ImageMask::Circle => ShapeType::Circle,
            ImageMask::Diamond => ShapeType::Diamond,
            ImageMask::Hexagon => ShapeType::Hexagon,
            ImageMask::Pill => ShapeType::Pill,
        };
        let corner_r = match mask {
            ImageMask::Rounded(r) => r,
            _ => 0.0,
        };
        rasterize_scene_shape(
            img,
            tf,
            shape_for_mask,
            FillStyle::Solid([0, 0, 0, 0]),
            border_color,
            border_width,
            corner_r,
            opacity,
            scale_x,
            scale_y,
        );
    }
}

fn rasterize_fallback_element(img: &mut RgbaImage, el: &CanvasElement) {
    let img_w = img.width() as i32;
    let img_h = img.height() as i32;
    let x0 = (el.x.floor() as i32).max(0);
    let y0 = (el.y.floor() as i32).max(0);
    let x1 = ((el.x + el.width).ceil() as i32).min(img_w - 1);
    let y1 = ((el.y + el.height).ceil() as i32).min(img_h - 1);

    if x1 <= x0 || y1 <= y0 {
        return;
    }

    match &el.kind {
        ElementKind::Chart {
            values,
            palette,
            chart_type,
            ..
        } => match chart_type {
            ChartType::VerticalBar | ChartType::HorizontalBar | ChartType::LineArea => {
                let max_val = values.iter().copied().fold(1.0_f32, f32::max);
                let n = values.len().max(1);
                let chart_h = (el.height - 60.0).max(20.0);
                let step_x = (el.width - 32.0) / (n as f32);
                let bar_w = (step_x * 0.65).max(8.0);
                for (i, val) in values.iter().enumerate() {
                    let bh = (val / max_val) * chart_h;
                    let bx0 = (el.x + 20.0 + (i as f32) * step_x) as i32;
                    let by0 = (el.y + el.height - 24.0 - bh) as i32;
                    let bx1 = (bx0 + bar_w as i32).min(img_w - 1);
                    let by1 = (el.y + el.height - 24.0) as i32;
                    let col = palette
                        .get(i % palette.len().max(1))
                        .copied()
                        .unwrap_or([0, 196, 204, 255]);
                    for y in by0.max(0)..=by1.min(img_h - 1) {
                        for x in bx0.max(0)..=bx1 {
                            blend_pixel(img, x as u32, y as u32, col);
                        }
                    }
                }
            }
            ChartType::Pie | ChartType::Donut => {
                let (cx, cy) = el.center();
                let r = el.width.min(el.height) * 0.38;
                let inner_r = if *chart_type == ChartType::Donut {
                    r * 0.52
                } else {
                    0.0
                };
                let col = palette.first().copied().unwrap_or([0, 196, 204, 255]);
                for y in y0..=y1 {
                    for x in x0..=x1 {
                        let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
                        if d <= r && d >= inner_r {
                            blend_pixel(img, x as u32, y as u32, col);
                        }
                    }
                }
            }
        },
        ElementKind::Table {
            rows,
            cols,
            header_bg,
            row_bg,
            alt_row_bg,
            ..
        } => {
            let r_cnt = (*rows).max(1) as f32;
            let rh = (el.height / r_cnt).max(1.0);
            for y in y0..=y1 {
                let row_idx = ((y as f32 - el.y) / rh).floor() as usize;
                let col = if row_idx == 0 {
                    *header_bg
                } else if row_idx % 2 == 1 {
                    *row_bg
                } else {
                    *alt_row_bg
                };
                for x in x0..=x1 {
                    let _ = cols;
                    blend_pixel(img, x as u32, y as u32, col);
                }
            }
        }
        ElementKind::FreehandPath {
            points,
            color,
            stroke_width,
            ..
        } => {
            let rad = (*stroke_width * 0.5).max(1.5) as i32;
            for window in points.windows(2) {
                let p0 = (
                    el.x + window[0].0 * el.width,
                    el.y + window[0].1 * el.height,
                );
                let p1 = (
                    el.x + window[1].0 * el.width,
                    el.y + window[1].1 * el.height,
                );
                let steps = (((p1.0 - p0.0).hypot(p1.1 - p0.1)).ceil() as i32).max(1);
                for s in 0..=steps {
                    let t = s as f32 / steps as f32;
                    let px = (p0.0 + (p1.0 - p0.0) * t) as i32;
                    let py = (p0.1 + (p1.1 - p0.1) * t) as i32;
                    for dy in -rad..=rad {
                        for dx in -rad..=rad {
                            let nx = px + dx;
                            let ny = py + dy;
                            if nx >= 0 && nx < img_w && ny >= 0 && ny < img_h {
                                blend_pixel(img, nx as u32, ny as u32, *color);
                            }
                        }
                    }
                }
            }
        }
        _ => {}
    }
}

/// Computes polygon vertices in canvas space `(x, y)` for any `ShapeType`.
pub fn compute_shape_polygon_points(
    shape_type: ShapeType,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
) -> Vec<(f32, f32)> {
    let cx = x + w * 0.5;
    let cy = y + h * 0.5;
    let rx = w * 0.5;
    let ry = h * 0.5;

    match shape_type {
        ShapeType::Triangle => vec![(cx, y), (x + w, y + h), (x, y + h)],
        ShapeType::RightTriangle => vec![(x, y), (x + w, y + h), (x, y + h)],
        ShapeType::Diamond => vec![(cx, y), (x + w, cy), (cx, y + h), (x, cy)],
        ShapeType::Pentagon => regular_polygon(cx, cy, rx, ry, 5),
        ShapeType::Hexagon => regular_polygon(cx, cy, rx, ry, 6),
        ShapeType::Octagon => regular_polygon(cx, cy, rx, ry, 8),
        ShapeType::Star5 => star_polygon(cx, cy, rx, ry, 5, 0.42),
        ShapeType::Star8 => star_polygon(cx, cy, rx, ry, 8, 0.48),
        ShapeType::Burst12 => star_polygon(cx, cy, rx, ry, 12, 0.72),
        ShapeType::Heart => {
            let mut pts = Vec::with_capacity(32);
            for i in 0..32 {
                let t = (i as f32 / 32.0) * std::f32::consts::TAU;
                let hx = 16.0 * t.sin().powi(3);
                let hy = -(13.0 * t.cos()
                    - 5.0 * (2.0 * t).cos()
                    - 2.0 * (3.0 * t).cos()
                    - (4.0 * t).cos());
                pts.push((cx + (hx / 17.0) * rx, cy + (hy / 17.0) * ry));
            }
            pts
        }
        ShapeType::ArrowRight => vec![
            (x, y + h * 0.28),
            (x + w * 0.62, y + h * 0.28),
            (x + w * 0.62, y),
            (x + w, cy),
            (x + w * 0.62, y + h),
            (x + w * 0.62, y + h * 0.72),
            (x, y + h * 0.72),
        ],
        ShapeType::ArrowLeft => vec![
            (x + w, y + h * 0.28),
            (x + w * 0.38, y + h * 0.28),
            (x + w * 0.38, y),
            (x, cy),
            (x + w * 0.38, y + h),
            (x + w * 0.38, y + h * 0.72),
            (x + w, y + h * 0.72),
        ],
        ShapeType::SpeechBubble => vec![
            (x, y),
            (x + w, y),
            (x + w, y + h * 0.76),
            (x + w * 0.48, y + h * 0.76),
            (x + w * 0.22, y + h),
            (x + w * 0.28, y + h * 0.76),
            (x, y + h * 0.76),
        ],
        ShapeType::BadgeShield => vec![
            (cx, y),
            (x + w, y + h * 0.18),
            (x + w * 0.88, y + h * 0.68),
            (cx, y + h),
            (x + w * 0.12, y + h * 0.68),
            (x, y + h * 0.18),
        ],
        ShapeType::Cross => vec![
            (x + w * 0.35, y),
            (x + w * 0.65, y),
            (x + w * 0.65, y + h * 0.35),
            (x + w, y + h * 0.35),
            (x + w, y + h * 0.65),
            (x + w * 0.65, y + h * 0.65),
            (x + w * 0.65, y + h),
            (x + w * 0.35, y + h),
            (x + w * 0.35, y + h * 0.65),
            (x, y + h * 0.65),
            (x, y + h * 0.35),
            (x + w * 0.35, y + h * 0.35),
        ],
        ShapeType::LineHorizontal => vec![
            (x, cy - (h * 0.15).max(2.0)),
            (x + w, cy - (h * 0.15).max(2.0)),
            (x + w, cy + (h * 0.15).max(2.0)),
            (x, cy + (h * 0.15).max(2.0)),
        ],
        ShapeType::Parallelogram => vec![
            (x + w * 0.16, y),
            (x + w, y),
            (x + w * 0.84, y + h),
            (x, y + h),
        ],
        ShapeType::Trapezoid => vec![
            (x + w * 0.18, y),
            (x + w * 0.82, y),
            (x + w, y + h),
            (x, y + h),
        ],
        ShapeType::BannerRibbon => vec![
            (x, y),
            (x + w, y),
            (x + w * 0.88, cy),
            (x + w, y + h),
            (x, y + h),
            (x + w * 0.12, cy),
        ],
        _ => vec![(x, y), (x + w, y), (x + w, y + h), (x, y + h)],
    }
}

fn regular_polygon(cx: f32, cy: f32, rx: f32, ry: f32, sides: usize) -> Vec<(f32, f32)> {
    let mut pts = Vec::with_capacity(sides);
    for i in 0..sides {
        let angle = (i as f32 / sides as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        pts.push((cx + rx * angle.cos(), cy + ry * angle.sin()));
    }
    pts
}

fn star_polygon(
    cx: f32,
    cy: f32,
    rx: f32,
    ry: f32,
    points: usize,
    inner_ratio: f32,
) -> Vec<(f32, f32)> {
    let total = points * 2;
    let mut pts = Vec::with_capacity(total);
    for i in 0..total {
        let angle = (i as f32 / total as f32) * std::f32::consts::TAU - std::f32::consts::FRAC_PI_2;
        let r_scale = if i % 2 == 0 { 1.0 } else { inner_ratio };
        pts.push((
            cx + rx * r_scale * angle.cos(),
            cy + ry * r_scale * angle.sin(),
        ));
    }
    pts
}

fn lerp_color(a: ColorRgba, b: ColorRgba, t: f32) -> ColorRgba {
    let t = t.clamp(0.0, 1.0);
    [
        (a[0] as f32 * (1.0 - t) + b[0] as f32 * t).round() as u8,
        (a[1] as f32 * (1.0 - t) + b[1] as f32 * t).round() as u8,
        (a[2] as f32 * (1.0 - t) + b[2] as f32 * t).round() as u8,
        (a[3] as f32 * (1.0 - t) + b[3] as f32 * t).round() as u8,
    ]
}

fn blend_pixel(img: &mut RgbaImage, x: u32, y: u32, src: ColorRgba) {
    let sa = src[3] as f32 / 255.0;
    if sa <= 0.001 {
        return;
    }
    let dst = img.get_pixel(x, y).0;
    let out_r = (src[0] as f32 * sa + dst[0] as f32 * (1.0 - sa)).round() as u8;
    let out_g = (src[1] as f32 * sa + dst[1] as f32 * (1.0 - sa)).round() as u8;
    let out_b = (src[2] as f32 * sa + dst[2] as f32 * (1.0 - sa)).round() as u8;
    let out_a = (src[3] as f32 + dst[3] as f32 * (1.0 - sa))
        .round()
        .clamp(0.0, 255.0) as u8;
    img.put_pixel(x, y, Rgba([out_r, out_g, out_b, out_a]));
}

fn hex_rgb(c: ColorRgba) -> String {
    format!("#{:02X}{:02X}{:02X}", c[0], c[1], c[2])
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Exports the entire multi-page `Project` as an interactive standalone HTML5 Presentation deck.
pub fn export_project_to_html_deck(project: &Project) -> Result<String, String> {
    let resolved = crate::production::resolve(project, true)?;
    let project = resolved.as_ref();
    if project.pages.is_empty() {
        return Err("Project contains no pages to export".into());
    }
    let mut slides_html = String::new();
    for (idx, page) in project.pages.iter().enumerate() {
        let svg = export_page_to_svg(page, project.width, project.height, false)?;
        let active_class = if idx == 0 { "slide active" } else { "slide" };
        slides_html.push_str(&format!(
            r#"<div class="{active_class}" data-title="{}">{svg}</div>"#,
            xml_escape(&page.title)
        ));
    }

    Ok(format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<title>{}</title>
<style>
  body {{ margin: 0; background: #0e1318; color: #fff; font-family: system-ui, sans-serif; display: flex; flex-direction: column; height: 100vh; overflow: hidden; }}
  .stage {{ flex: 1; display: flex; align-items: center; justify-content: center; padding: 24px; }}
  .slide {{ display: none; max-width: 100%; max-height: 100%; box-shadow: 0 24px 60px rgba(0,0,0,0.6); border-radius: 8px; overflow: hidden; }}
  .slide.active {{ display: block; }}
  .slide svg {{ width: auto; height: 82vh; max-width: 94vw; display: block; }}
  .bar {{ height: 48px; background: #18191b; display: flex; align-items: center; justify-content: space-between; padding: 0 24px; border-top: 1px solid #2b2d31; }}
  button {{ background: linear-gradient(90deg, #00C4CC, #7D2AE8); color: #fff; border: none; padding: 8px 18px; border-radius: 6px; font-weight: 600; cursor: pointer; }}
</style>
</head>
<body>
  <div class="stage">{slides_html}</div>
  <div class="bar">
    <div><strong>{}</strong> • Studio Deck</div>
    <div>
      <button onclick="step(-1)">Prev</button>
      <span id="counter" style="margin: 0 14px;">1 / {}</span>
      <button onclick="step(1)">Next</button>
    </div>
  </div>
  <script>
    let cur = 0;
    const slides = document.querySelectorAll('.slide');
    function step(d) {{
      slides[cur].classList.remove('active');
      cur = (cur + d + slides.length) % slides.length;
      slides[cur].classList.add('active');
      document.getElementById('counter').textContent = (cur + 1) + ' / ' + slides.length;
    }}
    window.addEventListener('keydown', e => {{
      if (e.key === 'ArrowRight' || e.key === ' ') step(1);
      if (e.key === 'ArrowLeft') step(-1);
    }});
  </script>
</body>
</html>"#,
        xml_escape(&project.title),
        xml_escape(&project.title),
        project.pages.len()
    ))
}

struct ActivePackageDirGuard(Option<std::path::PathBuf>);

impl Drop for ActivePackageDirGuard {
    fn drop(&mut self) {
        AssetStore::global().set_active_package_dir(self.0.take());
    }
}

fn validate_and_stage_project_assets(project: &Project, parent_dir: &Path) -> Result<(), String> {
    let store = AssetStore::global();
    for (page_idx, (page_title, elements)) in project
        .pages
        .iter()
        .map(|p| (p.title.as_str(), p.elements.as_slice()))
        .chain(std::iter::once((
            "Book master",
            project.production.book.master_elements.as_slice(),
        )))
        .chain(crate::campaign::asset_sets(&project.production.studio))
        .enumerate()
    {
        for el in elements {
            if let ElementKind::Image {
                width_px,
                height_px,
                raw_rgba,
                ..
            } = &el.kind
            {
                let expected_len = (*width_px as usize)
                    .saturating_mul(*height_px as usize)
                    .saturating_mul(4);
                if *width_px == 0 || *height_px == 0 || expected_len == 0 {
                    return Err(format!(
                        "Page {} ('{}') image element '{}' (id {}) has invalid dimensions {}x{}",
                        page_idx + 1,
                        page_title,
                        el.name,
                        el.id,
                        width_px,
                        height_px
                    ));
                }
                if raw_rgba.len() != expected_len {
                    return Err(format!(
                        "Page {} ('{}') image element '{}' (id {}, asset '{}') has {} RGBA bytes, expected {} ({}x{}x4)",
                        page_idx + 1,
                        page_title,
                        el.name,
                        el.id,
                        "raw RGBA",
                        raw_rgba.len(),
                        expected_len,
                        width_px,
                        height_px
                    ));
                }
                let stored = store.register_shared(raw_rgba, *width_px, *height_px);
                store.write_asset_to_dir(&stored, parent_dir).map_err(|e| {
                    format!(
                        "Page {} ('{}') image element '{}' (id {}, asset '{}') failed to persist: {e}",
                        page_idx + 1,
                        page_title,
                        el.name,
                        el.id,
                        stored.asset_id
                    )
                })?;
            }
        }
    }
    Ok(())
}

fn validate_loaded_project_assets(project: &Project, source_path: &Path) -> Result<(), String> {
    for (page_idx, (page_title, elements)) in project
        .pages
        .iter()
        .map(|p| (p.title.as_str(), p.elements.as_slice()))
        .chain(std::iter::once((
            "Book master",
            project.production.book.master_elements.as_slice(),
        )))
        .chain(crate::campaign::asset_sets(&project.production.studio))
        .enumerate()
    {
        for el in elements {
            if let ElementKind::Image {
                width_px,
                height_px,
                raw_rgba,
                ..
            } = &el.kind
            {
                let expected_len = (*width_px as usize)
                    .saturating_mul(*height_px as usize)
                    .saturating_mul(4);
                if *width_px == 0 || *height_px == 0 || expected_len == 0 {
                    return Err(format!(
                        "Loaded project '{}' has invalid image dimensions ({}x{}) on page {} ('{}'), element '{}' (id {})",
                        source_path.display(),
                        width_px,
                        height_px,
                        page_idx + 1,
                        page_title,
                        el.name,
                        el.id
                    ));
                }
                if raw_rgba.len() != expected_len {
                    return Err(format!(
                        "Loaded project '{}' is missing or has corrupt pixel data for image element '{}' (id {}, asset '{}') on page {} ('{}'): got {} bytes, expected {} ({}x{}x4)",
                        source_path.display(),
                        el.name,
                        el.id,
                        "raw RGBA",
                        page_idx + 1,
                        page_title,
                        raw_rgba.len(),
                        expected_len,
                        width_px,
                        height_px
                    ));
                }
            }
        }
    }
    Ok(())
}

/// Saves a `Project` atomically via a temporary sibling file, deduplicating and verifying all raster
/// image assets in `<project_dir>/assets/<asset_id>.<ext>` BEFORE committing the `.canvas.json` manifest.
pub fn save_project_json(project: &Project, path: &Path) -> Result<(), String> {
    if project.production.studio.active() {
        return crate::project_package::save(project, path);
    }
    save_project_manifest(project, path)
}
pub(crate) fn save_project_manifest(project: &Project, path: &Path) -> Result<(), String> {
    if path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("canvas"))
    {
        return crate::project_package::save(project, path);
    }
    let parent_dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    fs::create_dir_all(&parent_dir).map_err(|e| {
        format!(
            "Failed creating project directory '{}': {e}",
            parent_dir.display()
        )
    })?;

    crate::campaign::validate(project)?;
    crate::imported_fonts::register_project(project)?;
    // 1. Validate and persist all required image assets on disk before touching the manifest.
    validate_and_stage_project_assets(project, &parent_dir)?;

    let _pkg_guard = ActivePackageDirGuard(AssetStore::global().active_package_dir());
    AssetStore::global().set_active_package_dir(Some(parent_dir.clone()));

    let json = serde_json::to_string_pretty(project)
        .map_err(|e| format!("Failed serializing project JSON: {e}"))?;

    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("project.canvas.json");
    let tmp_path = parent_dir.join(format!(".{file_name}.{}.tmp", std::process::id()));

    use std::io::Write;
    fs::File::create(&tmp_path)
        .and_then(|mut f| {
            f.write_all(json.as_bytes())?;
            f.sync_all()
        })
        .map_err(|e| {
            let _ = fs::remove_file(&tmp_path);
            format!(
                "Failed writing temporary save file '{}': {e}",
                tmp_path.display()
            )
        })?;

    if path.exists() {
        let bak_path = path.with_extension("canvas.json.bak");
        fs::copy(path, &bak_path).map_err(|e| {
            format!(
                "Cannot preserve previous save '{}': {e}",
                bak_path.display()
            )
        })?;
    }

    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!(
            "Failed finalizing project save to '{}': {e}",
            path.display()
        )
    })
}

/// Loads a `Project` from `.canvas.json` (transparently supporting both v2 content-addressed
/// `"asset://<hash>"` references and legacy v1 raw `[u8, ...]` arrays), strictly verifying
/// that every required image asset exists and decodes cleanly.
pub fn load_project_json(path: &Path) -> Result<Project, String> {
    if crate::project_package::is_package(path) {
        return crate::project_package::load(path);
    }
    let parent_dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_default());

    let _pkg_guard = ActivePackageDirGuard(AssetStore::global().active_package_dir());
    AssetStore::global().set_active_package_dir(Some(parent_dir));

    let data = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading project file '{}': {e}", path.display()))?;
    let mut project: Project = serde_json::from_str(&data)
        .map_err(|e| format!("Failed parsing project JSON '{}': {e}", path.display()))?;
    crate::campaign::validate(&project)?;
    if project.schema_version > 2 {
        return Err("This project requires a newer Canvas Studio document schema".into());
    }
    crate::imported_fonts::register_project(&project)?;
    hydrate_project_assets(&mut project);
    validate_loaded_project_assets(&project, path)?;
    Ok(project)
}

/// Migrates a legacy v1 `.canvas.json` file (containing inline raw_rgba integer arrays)
/// to a compact v2 `.canvas.json` + `assets/` directory at `v2_dest_path` without modifying `v1_source_path`.
#[allow(dead_code)]
pub fn migrate_v1_project_to_v2(
    v1_source_path: &Path,
    v2_dest_path: &Path,
) -> Result<Project, String> {
    let mut project = load_project_json(v1_source_path)?;
    project.schema_version = 2;
    save_project_json(&project, v2_dest_path)?;
    Ok(project)
}
