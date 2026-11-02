use std::collections::{BTreeMap, HashMap};
use std::fmt::Write as _;
use std::fs;
use std::io::Write as _;
use std::path::Path;

use flate2::write::ZlibEncoder;
use flate2::Compression;

use crate::model::font_registry::{FontFaceId, FontRegistry};
use crate::model::render_scene::{
    build_page_scene, mask_coverage, ImagePlacement, LocalToPageTransform, SceneItem,
};
use crate::model::text_layout::{layout_text_block, TextLayoutResult};
use crate::model::{
    rgba, CanvasPage, ColorRgba, ElementKind, FillStyle, ImageAdjustments, ImageMask,
    PageBackground, Project, ShapeType,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PageSelection {
    CurrentPage(usize),
    Range(usize, usize),
    AllPages,
}

#[derive(Clone, Debug)]
pub struct PdfExportOptions {
    pub page_selection: PageSelection,
    pub units_per_inch: f32,
    pub bleed_mm: f32,
    pub include_crop_marks: bool,
    pub target_image_dpi: u32,
}

impl Default for PdfExportOptions {
    fn default() -> Self {
        Self {
            page_selection: PageSelection::AllPages,
            units_per_inch: 96.0,
            bleed_mm: 0.0,
            include_crop_marks: false,
            target_image_dpi: 300,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PreflightSeverity {
    Warning,
    Error,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreflightIssue {
    pub page_index: usize,
    pub element_id: u64,
    pub element_name: String,
    pub severity: PreflightSeverity,
    pub category: &'static str,
    pub message: String,
}

#[derive(Clone, Debug, Default)]
pub struct PreflightReport {
    pub issues: Vec<PreflightIssue>,
}

impl PreflightReport {
    pub fn has_errors(&self) -> bool {
        self.issues
            .iter()
            .any(|i| i.severity == PreflightSeverity::Error)
    }

    pub fn has_overflows(&self) -> bool {
        self.issues.iter().any(|i| i.category == "text_overflow")
    }
}

pub fn run_export_preflight(project: &Project, target_dpi: u32) -> PreflightReport {
    let mut issues = Vec::new();
    let registry = FontRegistry::global();

    for warning in registry.load_warnings() {
        issues.push(PreflightIssue {
            page_index: 0,
            element_id: 0,
            element_name: "FontRegistry".to_string(),
            severity: PreflightSeverity::Error,
            category: "missing_font",
            message: warning.clone(),
        });
    }

    if let Err(message) = project.validate_export_dimensions(target_dpi) {
        issues.push(PreflightIssue {
            page_index: 0,
            element_id: 0,
            element_name: "Document".into(),
            severity: PreflightSeverity::Error,
            category: "dimensions",
            message,
        });
    }

    for (page_index, page) in project.pages.iter().enumerate() {
        for element in page.elements.iter().filter(|e| e.visible) {
            let unsupported=match &element.kind {
                ElementKind::FreehandPath {brush,..} if *brush!=crate::model::BrushType::Pen => Some("Use the Pen brush for vector PDF; decorative drawing brushes are not supported"),
                ElementKind::Image {mask,border_width,..} if *mask!=ImageMask::None && *border_width>0. => Some("Shaped photo borders are not supported in print PDF; set border width to zero"),
                _=>None,
            };
            if let Some(message) = unsupported {
                issues.push(PreflightIssue {
                    page_index,
                    element_id: element.id,
                    element_name: element.name.clone(),
                    severity: PreflightSeverity::Error,
                    category: "unsupported_effect",
                    message: message.into(),
                });
            }
        }
        let expanded = crate::model::render_scene::expand_page_elements(page);
        for element in &expanded {
            if !element.visible {
                continue;
            }
            if element.shadow.enabled {
                issues.push(PreflightIssue {page_index,element_id:element.id,element_name:element.name.clone(),severity:PreflightSeverity::Error,category:"unsupported_effect",message:"Drop shadows are not supported in print PDF; disable the shadow before exporting.".into()});
            }
            match &element.kind {
                ElementKind::Text {
                    content,
                    font_family,
                    font_size,
                    bold,
                    italic,
                    uppercase,
                    align,
                    letter_spacing,
                    line_height,
                    list_style,
                    ..
                } => {
                    let layout = layout_text_block(
                        content,
                        *font_family,
                        *font_size,
                        *bold,
                        *italic,
                        *uppercase,
                        *align,
                        *letter_spacing,
                        *line_height,
                        *list_style,
                        element.width,
                        element.height,
                    );
                    if layout
                        .lines
                        .iter()
                        .flat_map(|l| &l.glyphs)
                        .any(|g| g.glyph_id == 0)
                    {
                        issues.push(PreflightIssue {page_index,element_id:element.id,element_name:element.name.clone(),severity:PreflightSeverity::Error,category:"missing_glyph",message:format!("Page {} text '{}' contains characters not available in the bundled fonts",page_index+1,element.name)});
                    }
                    if let ElementKind::Text { effect, .. } = &element.kind {
                        if !matches!(effect, crate::model::TextEffect::None) {
                            issues.push(PreflightIssue {page_index,element_id:element.id,element_name:element.name.clone(),severity:PreflightSeverity::Error,category:"unsupported_effect",message:format!("Text effect '{}' is not supported in print PDF; set it to None",effect.label())});
                        }
                    }
                    if layout.overflows_width || layout.overflows_height {
                        issues.push(PreflightIssue {
                            page_index,
                            element_id: element.id,
                            element_name: element.name.clone(),
                            severity: PreflightSeverity::Warning,
                            category: "text_overflow",
                            message: format!(
                                "Page {} text '{}' ({:.1}x{:.1}) overflows box ({:.1}x{:.1})",
                                page_index + 1,
                                element.name,
                                layout.measured_width,
                                layout.measured_height,
                                element.width,
                                element.height
                            ),
                        });
                    }
                }
                ElementKind::Image {
                    source_name,
                    width_px,
                    height_px,
                    raw_rgba,
                    adjustments,
                    ..
                } => {
                    if raw_rgba.len()
                        != (*width_px as usize)
                            .saturating_mul(*height_px as usize)
                            .saturating_mul(4)
                        || *width_px == 0
                        || *height_px == 0
                    {
                        issues.push(PreflightIssue {
                            page_index,
                            element_id: element.id,
                            element_name: element.name.clone(),
                            severity: PreflightSeverity::Error,
                            category: "missing_asset",
                            message: format!(
                                "Page {} image '{}' ({}) has no loaded pixel data",
                                page_index + 1,
                                element.name,
                                source_name
                            ),
                        });
                    } else if target_dpi >= 150 {
                        let placement = crate::model::render_scene::compute_image_placement(
                            *width_px,
                            *height_px,
                            element.width,
                            element.height,
                            adjustments.fit,
                            adjustments.crop,
                            adjustments.focal_point,
                        );
                        let inch_x = project.effective_width_mm() / 25.4 / project.width.max(1.0);
                        let inch_y = project.effective_height_mm() / 25.4 / project.height.max(1.0);
                        let eff_dpi = (placement.src_w / (placement.dest_w * inch_x).max(0.001))
                            .min(placement.src_h / (placement.dest_h * inch_y).max(0.001));
                        if eff_dpi < target_dpi as f32 * 0.95 {
                            issues.push(PreflightIssue {
                                page_index,
                                element_id: element.id,
                                element_name: element.name.clone(),
                                severity: PreflightSeverity::Warning,
                                category: "low_dpi",
                                message: format!(
                                    "Page {} image '{}' effective resolution is {:.0} DPI (target {} DPI)",
                                    page_index + 1,
                                    element.name,
                                    eff_dpi,
                                    target_dpi
                                ),
                            });
                        }
                    }
                }
                ElementKind::Chart { .. } | ElementKind::Table { .. } => {
                    issues.push(PreflightIssue {page_index,element_id:element.id,element_name:element.name.clone(),severity:PreflightSeverity::Error,category:"unsupported_content",message:format!("Page {} layer '{}' has unsupported chart or invalid/oversized table content",page_index+1,element.name)});
                }
                _ => {}
            }
        }
    }

    PreflightReport { issues }
}

pub fn export_page_to_pdf(
    project: &Project,
    page_idx: usize,
    path: &Path,
) -> Result<PreflightReport, String> {
    let options = PdfExportOptions {
        page_selection: PageSelection::CurrentPage(page_idx),
        units_per_inch: project.effective_units_per_inch(),
        bleed_mm: project.bleed_mm,
        include_crop_marks: project.crop_marks,
        target_image_dpi: 300,
    };
    export_project_to_pdf(project, &options, path)
}

pub fn export_project_to_pdf(
    project: &Project,
    options: &PdfExportOptions,
    path: &Path,
) -> Result<PreflightReport, String> {
    let report = run_export_preflight(project, options.target_image_dpi);
    let pdf_bytes = export_project_to_pdf_bytes(project, options)?;
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            fs::create_dir_all(parent).map_err(|e| format!("Failed creating directory: {e}"))?;
        }
    }
    let tmp_path = path.with_extension("pdf.tmp");
    fs::write(&tmp_path, &pdf_bytes).map_err(|e| format!("Failed writing temporary PDF: {e}"))?;
    fs::rename(&tmp_path, path).map_err(|e| format!("Failed finalizing PDF export: {e}"))?;
    Ok(report)
}

struct PdfFontSlot {
    resource_name: String,
    face_id: FontFaceId,
    char_to_code: HashMap<char, u16>,
    code_to_char: BTreeMap<u16, char>,
    glyph_ids: BTreeMap<u16, u16>,
}

impl PdfFontSlot {
    fn new(resource_name: String, face_id: FontFaceId) -> Self {
        Self {
            resource_name,
            face_id,
            char_to_code: HashMap::new(),
            code_to_char: BTreeMap::new(),
            glyph_ids: BTreeMap::new(),
        }
    }
    fn encode_glyph(&mut self, ch: char, glyph: u16) -> u16 {
        if let Some(code) = self.char_to_code.get(&ch) {
            return *code;
        }
        let code = (self.char_to_code.len() + 1) as u16;
        self.char_to_code.insert(ch, code);
        self.code_to_char.insert(code, ch);
        self.glyph_ids.insert(code, glyph);
        code
    }
}

struct PdfImageObject {
    resource_name: String,
    width: u32,
    height: u32,
    rgb_compressed: Vec<u8>,
    alpha_compressed: Option<Vec<u8>>,
}

pub fn export_project_to_pdf_bytes(
    project: &Project,
    options: &PdfExportOptions,
) -> Result<Vec<u8>, String> {
    if project.pages.is_empty() {
        return Err("Project contains no pages to export".to_string());
    }

    let selected_indices: Vec<usize> = match options.page_selection {
        PageSelection::CurrentPage(i) if i < project.pages.len() => vec![i],
        PageSelection::Range(a, b) if a <= b && b < project.pages.len() => (a..=b).collect(),
        PageSelection::AllPages => (0..project.pages.len()).collect(),
        _ => return Err("PDF page selection is outside this document".into()),
    };
    let report = run_export_preflight(project, options.target_image_dpi);
    let errors: Vec<_> = report
        .issues
        .iter()
        .filter(|i| {
            i.severity == PreflightSeverity::Error
                && (i.element_id == 0 || selected_indices.contains(&i.page_index))
        })
        .collect();
    if !errors.is_empty() {
        return Err(errors
            .iter()
            .map(|i| {
                format!(
                    "Page {} / {}: {}",
                    i.page_index + 1,
                    i.element_name,
                    i.message
                )
            })
            .collect::<Vec<_>>()
            .join("\n"));
    }

    let bleed_pt = (options.bleed_mm.max(0.0) / 25.4) * 72.0;
    let crop_margin_pt = if options.include_crop_marks {
        (bleed_pt + 18.0).max(18.0)
    } else {
        bleed_pt
    };

    let (trim_w_pt, trim_h_pt) = project.pdf_points_dimensions();
    let media_w_pt = trim_w_pt + 2.0 * crop_margin_pt;
    let media_h_pt = trim_h_pt + 2.0 * crop_margin_pt;

    let mut font_slots: Vec<PdfFontSlot> = Vec::new();
    let mut face_to_slot: HashMap<FontFaceId, usize> = HashMap::new();
    let mut gs_alphas: BTreeMap<(u8, u8), String> = BTreeMap::new();
    let mut page_images: Vec<Vec<PdfImageObject>> = Vec::new();
    let mut page_streams: Vec<String> = Vec::new();

    for &page_idx in &selected_indices {
        let page = &project.pages[page_idx];
        let mut images_for_page = Vec::new();
        let stream = build_pdf_page_stream(
            page,
            project.width,
            project.height,
            trim_w_pt,
            trim_h_pt,
            crop_margin_pt,
            bleed_pt,
            options.include_crop_marks,
            options.target_image_dpi,
            &mut font_slots,
            &mut face_to_slot,
            &mut gs_alphas,
            &mut images_for_page,
        );
        page_images.push(images_for_page);
        page_streams.push(stream);
    }

    assemble_pdf_document(
        trim_w_pt,
        trim_h_pt,
        media_w_pt,
        media_h_pt,
        crop_margin_pt,
        bleed_pt,
        &font_slots,
        &gs_alphas,
        &page_images,
        &page_streams,
    )
}

fn background_primary_color(bg: &PageBackground) -> ColorRgba {
    match bg {
        PageBackground::Solid(c) => *c,
        PageBackground::LinearGradient { start, .. } => *start,
        PageBackground::RadialGradient { center, .. } => *center,
        PageBackground::GridPattern { bg, .. } => *bg,
    }
}

#[allow(clippy::too_many_arguments)]
fn build_pdf_page_stream(
    page: &CanvasPage,
    canvas_w: f32,
    canvas_h: f32,
    trim_w_pt: f32,
    trim_h_pt: f32,
    crop_margin_pt: f32,
    bleed_pt: f32,
    include_crop_marks: bool,
    target_image_dpi: u32,
    font_slots: &mut Vec<PdfFontSlot>,
    face_to_slot: &mut HashMap<FontFaceId, usize>,
    gs_alphas: &mut BTreeMap<(u8, u8), String>,
    images_for_page: &mut Vec<PdfImageObject>,
) -> String {
    let sx = trim_w_pt / canvas_w.max(1.0);
    let sy = trim_h_pt / canvas_h.max(1.0);
    let ox = crop_margin_pt;
    let oy = crop_margin_pt;

    let to_pdf_x = |lx: f32| -> f32 { ox + lx * sx };
    let to_pdf_y = |ly: f32| -> f32 { oy + (canvas_h - ly) * sy };

    let scene = build_page_scene(page, canvas_w, canvas_h);
    let mut out = String::with_capacity(8192);

    // Background rectangle (covers trim + bleed)
    let bg = background_primary_color(&scene.background);
    let bg_gs = ensure_gs(gs_alphas, bg[3], 255);
    let _ = writeln!(
        out,
        "q /{bg_gs} gs {:.4} {:.4} {:.4} rg {:.3} {:.3} {:.3} {:.3} re f Q",
        bg[0] as f32 / 255.0,
        bg[1] as f32 / 255.0,
        bg[2] as f32 / 255.0,
        ox - bleed_pt,
        oy - bleed_pt,
        trim_w_pt + 2.0 * bleed_pt,
        trim_h_pt + 2.0 * bleed_pt
    );

    if !matches!(scene.background, PageBackground::Solid(_)) {
        let mut background = page.clone();
        background.elements.clear();
        let rw = (trim_w_pt / 72. * target_image_dpi as f32).ceil().max(16.) as u32;
        let rh = (trim_h_pt / 72. * target_image_dpi as f32).ceil().max(16.) as u32;
        let image = crate::exporter::render_page_to_rgba_image(
            &background,
            canvas_w,
            canvas_h,
            rw,
            rh,
            false,
        );
        let object = pdf_image_from_rgba(&image, images_for_page.len());
        let name = object.resource_name.clone();
        images_for_page.push(object);
        let dummy = crate::model::CanvasElement {
            id: 0,
            name: String::new(),
            x: 0.,
            y: 0.,
            width: canvas_w,
            height: canvas_h,
            rotation: 0.,
            opacity: 1.,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: Default::default(),
            animation: crate::model::ElementAnimation::None,
            group_id: None,
            kind: ElementKind::Shape {
                shape_type: ShapeType::Rectangle,
                fill: FillStyle::Solid(bg),
                stroke_color: bg,
                stroke_width: 0.,
                corner_radius: 0.,
            },
        };
        emit_pdf_image_placement(
            &mut out,
            &name,
            &LocalToPageTransform::from_element(&dummy),
            1.,
            [0, 0, 0, 0],
            0.,
            sx,
            sy,
            &to_pdf_x,
            &to_pdf_y,
            gs_alphas,
        );
    }
    if include_crop_marks && crop_margin_pt >= 10.0 {
        emit_crop_marks(&mut out, ox, oy, trim_w_pt, trim_h_pt, bleed_pt);
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
                ..
            } => {
                if matches!(fill, FillStyle::LinearGradient { .. }) {
                    if let Some(original) = page.elements.iter().find(|e| e.id == *element_id) {
                        let mut local = original.clone();
                        local.x = 0.;
                        local.y = 0.;
                        local.rotation = 0.;
                        local.flip_h = false;
                        local.flip_v = false;
                        local.opacity = 1.;
                        local.shadow.enabled = false;
                        let mut sample = CanvasPage::new(0, "Gradient");
                        sample.elements.push(local);
                        let rw = (transform.width * sx / 72. * target_image_dpi as f32)
                            .ceil()
                            .max(16.) as u32;
                        let rh = (transform.height * sy / 72. * target_image_dpi as f32)
                            .ceil()
                            .max(16.) as u32;
                        let image = crate::exporter::render_page_to_rgba_image(
                            &sample,
                            transform.width,
                            transform.height,
                            rw,
                            rh,
                            true,
                        );
                        let object = pdf_image_from_rgba(&image, images_for_page.len());
                        let name = object.resource_name.clone();
                        images_for_page.push(object);
                        emit_pdf_image_placement(
                            &mut out,
                            &name,
                            transform,
                            *opacity,
                            [0, 0, 0, 0],
                            0.,
                            sx,
                            sy,
                            &to_pdf_x,
                            &to_pdf_y,
                            gs_alphas,
                        );
                    }
                } else {
                    emit_pdf_shape(
                        &mut out,
                        transform,
                        *shape_type,
                        *fill,
                        *stroke_color,
                        *stroke_width,
                        *corner_radius,
                        *opacity,
                        sx,
                        sy,
                        &to_pdf_x,
                        &to_pdf_y,
                        gs_alphas,
                    );
                }
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
                if let Some(img_obj) = rasterize_cropped_masked_pdf_image(
                    transform,
                    *width_px,
                    *height_px,
                    raw_rgba,
                    *mask,
                    adjustments,
                    placement,
                    images_for_page.len(),
                    trim_w_pt / canvas_w.max(1.0),
                    target_image_dpi,
                ) {
                    let res_name = img_obj.resource_name.clone();
                    images_for_page.push(img_obj);
                    emit_pdf_image_placement(
                        &mut out,
                        &res_name,
                        transform,
                        *opacity,
                        *border_color,
                        *border_width,
                        sx,
                        sy,
                        &to_pdf_x,
                        &to_pdf_y,
                        gs_alphas,
                    );
                }
            }
            SceneItem::Text {
                transform,
                layout,
                color,
                underline,
                opacity,
                ..
            } => {
                emit_pdf_text(
                    &mut out,
                    transform,
                    layout,
                    *color,
                    *underline,
                    *opacity,
                    sx,
                    sy,
                    &to_pdf_x,
                    &to_pdf_y,
                    font_slots,
                    face_to_slot,
                    gs_alphas,
                );
            }
            SceneItem::FallbackElement(_) => {
                unreachable!("preflight must reject unsupported items")
            }
        }
    }

    out
}

fn ensure_gs(gs_alphas: &mut BTreeMap<(u8, u8), String>, fill_a: u8, stroke_a: u8) -> String {
    let key = (fill_a, stroke_a);
    if let Some(existing) = gs_alphas.get(&key) {
        return existing.clone();
    }
    let name = format!("GS{}", gs_alphas.len() + 1);
    gs_alphas.insert(key, name.clone());
    name
}

fn emit_crop_marks(
    out: &mut String,
    ox: f32,
    oy: f32,
    trim_w_pt: f32,
    trim_h_pt: f32,
    bleed_pt: f32,
) {
    let gap = bleed_pt.max(3.0);
    let len = 12.0;
    let left = ox;
    let right = ox + trim_w_pt;
    let bottom = oy;
    let top = oy + trim_h_pt;
    let _ = writeln!(out, "q 0.15 0.15 0.15 RG 0.5 w");
    let _ = writeln!(
        out,
        "{:.3} {:.3} m {:.3} {:.3} l S {:.3} {:.3} m {:.3} {:.3} l S",
        left - gap - len,
        bottom,
        left - gap,
        bottom,
        left,
        bottom - gap - len,
        left,
        bottom - gap
    );
    let _ = writeln!(
        out,
        "{:.3} {:.3} m {:.3} {:.3} l S {:.3} {:.3} m {:.3} {:.3} l S",
        right + gap,
        bottom,
        right + gap + len,
        bottom,
        right,
        bottom - gap - len,
        right,
        bottom - gap
    );
    let _ = writeln!(
        out,
        "{:.3} {:.3} m {:.3} {:.3} l S {:.3} {:.3} m {:.3} {:.3} l S",
        left - gap - len,
        top,
        left - gap,
        top,
        left,
        top + gap,
        left,
        top + gap + len
    );
    let _ = writeln!(
        out,
        "{:.3} {:.3} m {:.3} {:.3} l S {:.3} {:.3} m {:.3} {:.3} l S",
        right + gap,
        top,
        right + gap + len,
        top,
        right,
        top + gap,
        right,
        top + gap + len
    );
    let _ = writeln!(out, "Q");
}

#[allow(clippy::too_many_arguments)]
fn emit_pdf_shape<FX, FY>(
    out: &mut String,
    tf: &LocalToPageTransform,
    shape_type: ShapeType,
    fill: FillStyle,
    stroke_color: ColorRgba,
    stroke_width: f32,
    corner_radius: f32,
    opacity: f32,
    sx: f32,
    sy: f32,
    to_pdf_x: &FX,
    to_pdf_y: &FY,
    gs_alphas: &mut BTreeMap<(u8, u8), String>,
) where
    FX: Fn(f32) -> f32,
    FY: Fn(f32) -> f32,
{
    let op = opacity.clamp(0.0, 1.0);
    let fill_color = fill.primary_color();
    let fill_a = ((fill_color[3] as f32) * op).round().clamp(0.0, 255.0) as u8;
    let stroke_a = if stroke_width > 0.0 {
        ((stroke_color[3] as f32) * op).round().clamp(0.0, 255.0) as u8
    } else {
        0
    };
    if fill_a == 0 && stroke_a == 0 {
        return;
    }

    let gs_name = ensure_gs(gs_alphas, fill_a, stroke_a);
    let _ = writeln!(out, "q /{gs_name} gs");
    if fill_a > 0 {
        let _ = writeln!(
            out,
            "{:.4} {:.4} {:.4} rg",
            fill_color[0] as f32 / 255.0,
            fill_color[1] as f32 / 255.0,
            fill_color[2] as f32 / 255.0
        );
    }
    if stroke_a > 0 {
        let sw_pt = (stroke_width * 0.5 * (sx + sy)).max(0.25);
        let _ = writeln!(
            out,
            "{:.4} {:.4} {:.4} RG {:.3} w",
            stroke_color[0] as f32 / 255.0,
            stroke_color[1] as f32 / 255.0,
            stroke_color[2] as f32 / 255.0,
            sw_pt
        );
    }

    let w = tf.width;
    let h = tf.height;

    let map_pt = |lx: f32, ly: f32| -> (f32, f32) {
        let (px, py) = tf.local_to_page(lx, ly);
        (to_pdf_x(px), to_pdf_y(py))
    };

    if shape_type == ShapeType::LineHorizontal {
        let line_color = if stroke_a > 0 {
            stroke_color
        } else {
            fill_color
        };
        let line_w = (h.max(stroke_width).max(1.0) * sy).max(0.5);
        let (x0, y0) = map_pt(0.0, h * 0.5);
        let (x1, y1) = map_pt(w, h * 0.5);
        let _ = writeln!(
            out,
            "{:.4} {:.4} {:.4} RG {:.3} w {:.3} {:.3} m {:.3} {:.3} l S Q",
            line_color[0] as f32 / 255.0,
            line_color[1] as f32 / 255.0,
            line_color[2] as f32 / 255.0,
            line_w,
            x0,
            y0,
            x1,
            y1
        );
        return;
    }

    let vertices = sample_shape_polygon(shape_type, w, h, corner_radius);
    if let Some((first, rest)) = vertices.split_first() {
        let (x0, y0) = map_pt(first.0, first.1);
        let _ = write!(out, "{:.3} {:.3} m ", x0, y0);
        for &(vx, vy) in rest {
            let (px, py) = map_pt(vx, vy);
            let _ = write!(out, "{:.3} {:.3} l ", px, py);
        }
        let paint_op = match (fill_a > 0, stroke_a > 0) {
            (true, true) => "h B",
            (true, false) => "h f",
            (false, true) => "h S",
            (false, false) => "n",
        };
        let _ = writeln!(out, "{paint_op}");
    }
    let _ = writeln!(out, "Q");
}

pub fn sample_shape_polygon(
    shape: ShapeType,
    w: f32,
    h: f32,
    corner_radius: f32,
) -> Vec<(f32, f32)> {
    match shape {
        ShapeType::Rectangle | ShapeType::LineHorizontal => {
            vec![(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)]
        }
        ShapeType::RoundedRect | ShapeType::Pill | ShapeType::SpeechBubble => {
            let r = if shape == ShapeType::Pill {
                w.min(h) * 0.5
            } else {
                corner_radius.clamp(0.0, w.min(h) * 0.5)
            };
            if r <= 0.5 {
                return vec![(0.0, 0.0), (w, 0.0), (w, h), (0.0, h)];
            }
            let mut pts = Vec::with_capacity(36);
            let steps = 8usize;
            let centers = [
                (w - r, r, -std::f32::consts::FRAC_PI_2, 0.0),
                (w - r, h - r, 0.0, std::f32::consts::FRAC_PI_2),
                (r, h - r, std::f32::consts::FRAC_PI_2, std::f32::consts::PI),
                (r, r, std::f32::consts::PI, std::f32::consts::PI * 1.5),
            ];
            for &(cx, cy, a0, a1) in &centers {
                for i in 0..=steps {
                    let t = i as f32 / steps as f32;
                    let a = a0 + (a1 - a0) * t;
                    pts.push((cx + r * a.cos(), cy + r * a.sin()));
                }
            }
            pts
        }
        ShapeType::Circle | ShapeType::Heart => {
            let rx = w * 0.5;
            let ry = h * 0.5;
            let steps = 48usize;
            (0..steps)
                .map(|i| {
                    let a = (i as f32 / steps as f32) * std::f32::consts::TAU;
                    (rx + rx * a.cos(), ry + ry * a.sin())
                })
                .collect()
        }
        ShapeType::Triangle => vec![(w * 0.5, 0.0), (w, h), (0.0, h)],
        ShapeType::RightTriangle => vec![(0.0, 0.0), (w, h), (0.0, h)],
        ShapeType::Diamond => vec![(w * 0.5, 0.0), (w, h * 0.5), (w * 0.5, h), (0.0, h * 0.5)],
        ShapeType::Pentagon => regular_polygon(5, w, h),
        ShapeType::Hexagon => vec![
            (w * 0.25, 0.0),
            (w * 0.75, 0.0),
            (w, h * 0.5),
            (w * 0.75, h),
            (w * 0.25, h),
            (0.0, h * 0.5),
        ],
        ShapeType::Octagon => regular_polygon(8, w, h),
        ShapeType::Star5 => star_polygon(5, 0.382, w, h),
        ShapeType::Star8 => star_polygon(8, 0.45, w, h),
        ShapeType::Burst12 => star_polygon(12, 0.72, w, h),
        ShapeType::ArrowRight => vec![
            (0.0, h * 0.28),
            (w * 0.62, h * 0.28),
            (w * 0.62, 0.0),
            (w, h * 0.5),
            (w * 0.62, h),
            (w * 0.62, h * 0.72),
            (0.0, h * 0.72),
        ],
        ShapeType::ArrowLeft => vec![
            (w, h * 0.28),
            (w * 0.38, h * 0.28),
            (w * 0.38, 0.0),
            (0.0, h * 0.5),
            (w * 0.38, h),
            (w * 0.38, h * 0.72),
            (w, h * 0.72),
        ],
        ShapeType::BadgeShield => vec![
            (w * 0.5, 0.0),
            (w, h * 0.2),
            (w * 0.88, h * 0.72),
            (w * 0.5, h),
            (w * 0.12, h * 0.72),
            (0.0, h * 0.2),
        ],
        ShapeType::Cross => vec![
            (w * 0.33, 0.0),
            (w * 0.67, 0.0),
            (w * 0.67, h * 0.33),
            (w, h * 0.33),
            (w, h * 0.67),
            (w * 0.67, h * 0.67),
            (w * 0.67, h),
            (w * 0.33, h),
            (w * 0.33, h * 0.67),
            (0.0, h * 0.67),
            (0.0, h * 0.33),
            (w * 0.33, h * 0.33),
        ],
        ShapeType::Parallelogram => vec![(w * 0.18, 0.0), (w, 0.0), (w * 0.82, h), (0.0, h)],
        ShapeType::Trapezoid => vec![(w * 0.18, 0.0), (w * 0.82, 0.0), (w, h), (0.0, h)],
        ShapeType::BannerRibbon => vec![
            (0.0, 0.0),
            (w, 0.0),
            (w * 0.88, h * 0.5),
            (w, h),
            (0.0, h),
            (w * 0.12, h * 0.5),
        ],
    }
}

fn regular_polygon(sides: usize, w: f32, h: f32) -> Vec<(f32, f32)> {
    let cx = w * 0.5;
    let cy = h * 0.5;
    let rx = w * 0.5;
    let ry = h * 0.5;
    (0..sides)
        .map(|i| {
            let a =
                -std::f32::consts::FRAC_PI_2 + (i as f32 / sides as f32) * std::f32::consts::TAU;
            (cx + rx * a.cos(), cy + ry * a.sin())
        })
        .collect()
}

fn star_polygon(points: usize, inner_ratio: f32, w: f32, h: f32) -> Vec<(f32, f32)> {
    let cx = w * 0.5;
    let cy = h * 0.5;
    let orx = w * 0.5;
    let ory = h * 0.5;
    let irx = orx * inner_ratio;
    let iry = ory * inner_ratio;
    let total = points * 2;
    (0..total)
        .map(|i| {
            let a =
                -std::f32::consts::FRAC_PI_2 + (i as f32 / total as f32) * std::f32::consts::TAU;
            if i % 2 == 0 {
                (cx + orx * a.cos(), cy + ory * a.sin())
            } else {
                (cx + irx * a.cos(), cy + iry * a.sin())
            }
        })
        .collect()
}

#[allow(clippy::too_many_arguments)]
fn rasterize_cropped_masked_pdf_image(
    tf: &LocalToPageTransform,
    width_px: u32,
    height_px: u32,
    raw_rgba: &[u8],
    mask: ImageMask,
    adjustments: &ImageAdjustments,
    placement: &ImagePlacement,
    index: usize,
    pt_per_unit: f32,
    target_dpi: u32,
) -> Option<PdfImageObject> {
    if raw_rgba.len() < (width_px as usize) * (height_px as usize) * 4
        || width_px == 0
        || height_px == 0
    {
        return None;
    }

    let processed =
        crate::magic_studio::apply_image_pipeline(raw_rgba, width_px, height_px, adjustments);
    let w_in = (tf.width * pt_per_unit / 72.0).max(0.05);
    let h_in = (tf.height * pt_per_unit / 72.0).max(0.05);
    let target_w =
        ((w_in * target_dpi as f32).round() as u32).clamp(32, width_px.max(256).min(2400));
    let target_h =
        ((h_in * target_dpi as f32).round() as u32).clamp(32, height_px.max(256).min(2400));

    let mut rgb_bytes = Vec::with_capacity((target_w * target_h * 3) as usize);
    let mut alpha_bytes = Vec::with_capacity((target_w * target_h) as usize);
    let mut has_transparency = mask != ImageMask::None;

    for py in 0..target_h {
        let ly = (py as f32 + 0.5) / (target_h as f32) * tf.height;
        for px in 0..target_w {
            let lx = (px as f32 + 0.5) / (target_w as f32) * tf.width;
            let mask_cov = mask_coverage(mask, lx, ly, tf.width, tf.height);
            if mask_cov <= 0.001 {
                has_transparency = true;
                rgb_bytes.extend_from_slice(&[255, 255, 255]);
                alpha_bytes.push(0);
                continue;
            }
            if let Some((sx, sy)) = placement.local_to_source_px(lx, ly) {
                let adj = sample_bilinear_rgba(&processed, width_px, height_px, sx, sy);
                let a = ((adj[3] as f32) * mask_cov).round().clamp(0.0, 255.0) as u8;
                if a < 255 {
                    has_transparency = true;
                }
                rgb_bytes.extend_from_slice(&[adj[0], adj[1], adj[2]]);
                alpha_bytes.push(a);
            } else {
                has_transparency = true;
                rgb_bytes.extend_from_slice(&[255, 255, 255]);
                alpha_bytes.push(0);
            }
        }
    }

    let rgb_compressed = zlib_compress(&rgb_bytes);
    let alpha_compressed = if has_transparency {
        Some(zlib_compress(&alpha_bytes))
    } else {
        None
    };

    Some(PdfImageObject {
        resource_name: format!("Im{}", index + 1),
        width: target_w,
        height: target_h,
        rgb_compressed,
        alpha_compressed,
    })
}

pub fn sample_bilinear_rgba(raw: &[u8], w: u32, h: u32, sx: f32, sy: f32) -> ColorRgba {
    let x = sx.clamp(0.0, (w.saturating_sub(1)) as f32);
    let y = sy.clamp(0.0, (h.saturating_sub(1)) as f32);
    let x0 = x.floor() as u32;
    let y0 = y.floor() as u32;
    let x1 = (x0 + 1).min(w.saturating_sub(1));
    let y1 = (y0 + 1).min(h.saturating_sub(1));
    let fx = x - x0 as f32;
    let fy = y - y0 as f32;

    let get = |px: u32, py: u32| -> [f32; 4] {
        let idx = ((py * w + px) * 4) as usize;
        [
            raw[idx] as f32,
            raw[idx + 1] as f32,
            raw[idx + 2] as f32,
            raw[idx + 3] as f32,
        ]
    };
    let c00 = get(x0, y0);
    let c10 = get(x1, y0);
    let c01 = get(x0, y1);
    let c11 = get(x1, y1);

    let mut out = [0u8; 4];
    for i in 0..4 {
        let top = c00[i] * (1.0 - fx) + c10[i] * fx;
        let bot = c01[i] * (1.0 - fx) + c11[i] * fx;
        out[i] = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
    }
    out
}

#[allow(clippy::too_many_arguments)]
fn emit_pdf_image_placement<FX, FY>(
    out: &mut String,
    res_name: &str,
    tf: &LocalToPageTransform,
    opacity: f32,
    border_color: ColorRgba,
    border_width: f32,
    sx: f32,
    sy: f32,
    to_pdf_x: &FX,
    to_pdf_y: &FY,
    gs_alphas: &mut BTreeMap<(u8, u8), String>,
) where
    FX: Fn(f32) -> f32,
    FY: Fn(f32) -> f32,
{
    let alpha_u8 = (opacity.clamp(0.0, 1.0) * 255.0).round() as u8;
    if alpha_u8 == 0 {
        return;
    }
    let gs_name = ensure_gs(gs_alphas, alpha_u8, alpha_u8);

    let (p00x, p00y) = tf.local_to_page(0.0, tf.height);
    let (p10x, p10y) = tf.local_to_page(tf.width, tf.height);
    let (p01x, p01y) = tf.local_to_page(0.0, 0.0);

    let x00 = to_pdf_x(p00x);
    let y00 = to_pdf_y(p00y);
    let x10 = to_pdf_x(p10x);
    let y10 = to_pdf_y(p10y);
    let x01 = to_pdf_x(p01x);
    let y01 = to_pdf_y(p01y);

    let a = x10 - x00;
    let b = y10 - y00;
    let c = x01 - x00;
    let d = y01 - y00;

    let _ = writeln!(
        out,
        "q /{gs_name} gs {:.4} {:.4} {:.4} {:.4} {:.3} {:.3} cm /{res_name} Do Q",
        a, b, c, d, x00, y00
    );

    if border_width > 0.0 && border_color[3] > 0 {
        emit_pdf_shape(
            out,
            tf,
            ShapeType::Rectangle,
            FillStyle::Solid(rgba(0, 0, 0, 0)),
            border_color,
            border_width,
            0.0,
            opacity,
            sx,
            sy,
            to_pdf_x,
            to_pdf_y,
            gs_alphas,
        );
    }
}

#[allow(clippy::too_many_arguments)]
fn emit_pdf_text<FX, FY>(
    out: &mut String,
    tf: &LocalToPageTransform,
    layout: &TextLayoutResult,
    color: ColorRgba,
    underline: bool,
    opacity: f32,
    _sx: f32,
    sy: f32,
    to_pdf_x: &FX,
    to_pdf_y: &FY,
    font_slots: &mut Vec<PdfFontSlot>,
    face_to_slot: &mut HashMap<FontFaceId, usize>,
    gs_alphas: &mut BTreeMap<(u8, u8), String>,
) where
    FX: Fn(f32) -> f32,
    FY: Fn(f32) -> f32,
{
    let op = opacity.clamp(0.0, 1.0);
    let fill_a = ((color[3] as f32) * op).round().clamp(0.0, 255.0) as u8;
    if fill_a == 0 {
        return;
    }

    let gs_name = ensure_gs(gs_alphas, fill_a, fill_a);
    let font_size_pt = (layout.font_size * sy).max(0.1);
    let (ox, oy) = tf.local_to_page(0., 0.);
    let (ux, uy) = tf.local_to_page(1., 0.);
    let (vx, vy) = tf.local_to_page(0., -1.);
    let a = (to_pdf_x(ux) - to_pdf_x(ox)) / sy;
    let b = (to_pdf_y(uy) - to_pdf_y(oy)) / sy;
    let c = (to_pdf_x(vx) - to_pdf_x(ox)) / sy;
    let d = (to_pdf_y(vy) - to_pdf_y(oy)) / sy;
    let _ = writeln!(
        out,
        "q /{gs_name} gs {:.4} {:.4} {:.4} rg",
        color[0] as f32 / 255.,
        color[1] as f32 / 255.,
        color[2] as f32 / 255.
    );
    for line in &layout.lines {
        for glyph in &line.glyphs {
            let slot_idx = *face_to_slot.entry(glyph.face_id).or_insert_with(|| {
                let idx = font_slots.len();
                font_slots.push(PdfFontSlot::new(format!("F{}", idx + 1), glyph.face_id));
                idx
            });
            let code = font_slots[slot_idx].encode_glyph(glyph.ch, glyph.glyph_id);
            let name = &font_slots[slot_idx].resource_name;
            let (x, y) = tf.local_to_page(glyph.x, glyph.y_baseline);
            let _=writeln!(out,"BT /{name} {font_size_pt:.5} Tf {a:.6} {b:.6} {c:.6} {d:.6} {:.5} {:.5} Tm <{code:04X}> Tj ET",to_pdf_x(x),to_pdf_y(y));
        }
        if underline && line.width > 0.5 {
            let uy = line.y_baseline + (layout.font_size * 0.11).max(1.2);
            let (ux0, uy0) = tf.local_to_page(line.x_offset, uy);
            let (ux1, uy1) = tf.local_to_page(line.x_offset + line.width, uy);
            let sw = (layout.font_size * 0.06 * sy).max(0.5);
            let _ = writeln!(
                out,
                "{:.4} {:.4} {:.4} RG {:.3} w {:.3} {:.3} m {:.3} {:.3} l S",
                color[0] as f32 / 255.0,
                color[1] as f32 / 255.0,
                color[2] as f32 / 255.0,
                sw,
                to_pdf_x(ux0),
                to_pdf_y(uy0),
                to_pdf_x(ux1),
                to_pdf_y(uy1)
            );
        }
    }

    let _ = writeln!(out, "Q");
}

#[allow(clippy::too_many_arguments)]
fn assemble_pdf_document(
    trim_w_pt: f32,
    trim_h_pt: f32,
    media_w_pt: f32,
    media_h_pt: f32,
    crop_margin_pt: f32,
    bleed_pt: f32,
    font_slots: &[PdfFontSlot],
    gs_alphas: &BTreeMap<(u8, u8), String>,
    page_images: &[Vec<PdfImageObject>],
    page_streams: &[String],
) -> Result<Vec<u8>, String> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let mut add_obj = |data: Vec<u8>| -> usize {
        objects.push(data);
        objects.len()
    };

    // Reserve 1 = Catalog, 2 = Pages
    let catalog_id = add_obj(Vec::new());
    let pages_id = add_obj(Vec::new());

    // Build ExtGState objects
    let mut gs_obj_ids: Vec<(String, usize)> = Vec::new();
    for (&(fill_a, stroke_a), gs_name) in gs_alphas {
        let gs_str = format!(
            "<< /Type /ExtGState /ca {:.4} /CA {:.4} >>",
            fill_a as f32 / 255.0,
            stroke_a as f32 / 255.0
        );
        let id = add_obj(gs_str.into_bytes());
        gs_obj_ids.push((gs_name.clone(), id));
    }

    // Build embedded TrueType font objects + ToUnicode CMaps
    let registry = FontRegistry::global();
    let mut font_obj_ids: Vec<(String, usize)> = Vec::new();

    for slot in font_slots {
        let raw_ttf: &[u8] = registry
            .get(slot.face_id)
            .map(|f| f.bytes.as_slice())
            .unwrap_or(&[]);
        let compressed_ttf = zlib_compress(raw_ttf);
        let font_file_id = add_obj(build_stream_object(
            &format!(
                "<< /Length {} /Length1 {} /Filter /FlateDecode >>",
                compressed_ttf.len(),
                raw_ttf.len()
            ),
            &compressed_ttf,
        ));

        let ps_name = slot.face_id.postscript_name();
        let descriptor_str = format!(
            "<< /Type /FontDescriptor /FontName /{ps_name} /Flags 32 /FontBBox [-500 -300 1500 1100] /ItalicAngle 0 /Ascent 880 /Descent -220 /CapHeight 710 /StemV 80 /FontFile2 {font_file_id} 0 R >>"
        );
        let descriptor_id = add_obj(descriptor_str.into_bytes());

        let mut widths = String::new();
        let max_code = slot.code_to_char.keys().next_back().copied().unwrap_or(0);
        let mut glyph_map = vec![0u8; (max_code as usize + 1) * 2];
        for (&code, &ch) in &slot.code_to_char {
            let width = registry.pdf_glyph_width_1000(slot.face_id, ch);
            let _ = write!(widths, "{code} [{width}] ");
            let gid = slot.glyph_ids[&code].to_be_bytes();
            glyph_map[code as usize * 2..code as usize * 2 + 2].copy_from_slice(&gid);
        }
        let glyph_map_id = add_obj(build_stream_object(
            &format!("<< /Length {} >>", glyph_map.len()),
            &glyph_map,
        ));
        let descendant_id=add_obj(format!("<< /Type /Font /Subtype /CIDFontType2 /BaseFont /{ps_name} /CIDSystemInfo << /Registry (Adobe) /Ordering (Identity) /Supplement 0 >> /FontDescriptor {descriptor_id} 0 R /CIDToGIDMap {glyph_map_id} 0 R /DW 1000 /W [{widths}] >>").into_bytes());
        let cmap_bytes = build_tounicode_cmap(&slot.code_to_char);
        let cmap_id = add_obj(build_stream_object(
            &format!("<< /Length {} >>", cmap_bytes.len()),
            &cmap_bytes,
        ));
        let font_id=add_obj(format!("<< /Type /Font /Subtype /Type0 /BaseFont /{ps_name} /Encoding /Identity-H /DescendantFonts [{descendant_id} 0 R] /ToUnicode {cmap_id} 0 R >>").into_bytes());
        font_obj_ids.push((slot.resource_name.clone(), font_id));
    }

    // Build Page objects
    let mut page_obj_ids = Vec::with_capacity(page_streams.len());
    for (page_idx, stream_text) in page_streams.iter().enumerate() {
        let mut xobj_entries = String::new();
        for img_obj in &page_images[page_idx] {
            let smask_entry = if let Some(alpha_z) = &img_obj.alpha_compressed {
                let smask_id = add_obj(build_stream_object(
                    &format!(
                        "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceGray /BitsPerComponent 8 /Filter /FlateDecode /Length {} >>",
                        img_obj.width,
                        img_obj.height,
                        alpha_z.len()
                    ),
                    alpha_z,
                ));
                format!(" /SMask {smask_id} 0 R")
            } else {
                String::new()
            };

            let img_id = add_obj(build_stream_object(
                &format!(
                    "<< /Type /XObject /Subtype /Image /Width {} /Height {} /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode{smask_entry} /Length {} >>",
                    img_obj.width,
                    img_obj.height,
                    img_obj.rgb_compressed.len()
                ),
                &img_obj.rgb_compressed,
            ));
            let _ = write!(xobj_entries, "/{} {} 0 R ", img_obj.resource_name, img_id);
        }

        let stream_bytes = stream_text.as_bytes();
        let content_id = add_obj(build_stream_object(
            &format!("<< /Length {} >>", stream_bytes.len()),
            stream_bytes,
        ));

        let mut font_res = String::new();
        for (name, id) in &font_obj_ids {
            let _ = write!(font_res, "/{name} {id} 0 R ");
        }
        let mut gs_res = String::new();
        for (name, id) in &gs_obj_ids {
            let _ = write!(gs_res, "/{name} {id} 0 R ");
        }

        let trim_box = format!(
            "[{:.3} {:.3} {:.3} {:.3}]",
            crop_margin_pt,
            crop_margin_pt,
            crop_margin_pt + trim_w_pt,
            crop_margin_pt + trim_h_pt
        );
        let bleed_box = format!(
            "[{:.3} {:.3} {:.3} {:.3}]",
            crop_margin_pt - bleed_pt,
            crop_margin_pt - bleed_pt,
            crop_margin_pt + trim_w_pt + bleed_pt,
            crop_margin_pt + trim_h_pt + bleed_pt
        );

        let page_dict = format!(
            "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 {:.3} {:.3}] /TrimBox {trim_box} /BleedBox {bleed_box} /Resources << /Font << {font_res}>> /ExtGState << {gs_res}>> /XObject << {xobj_entries}>> >> /Contents {content_id} 0 R >>",
            media_w_pt, media_h_pt
        );
        let page_id = add_obj(page_dict.into_bytes());
        page_obj_ids.push(page_id);
    }

    let kids_str = page_obj_ids
        .iter()
        .map(|id| format!("{id} 0 R"))
        .collect::<Vec<_>>()
        .join(" ");
    objects[pages_id - 1] = format!(
        "<< /Type /Pages /Kids [ {kids_str} ] /Count {} >>",
        page_obj_ids.len()
    )
    .into_bytes();
    objects[catalog_id - 1] = format!("<< /Type /Catalog /Pages {pages_id} 0 R >>").into_bytes();

    // Serialize PDF with xref table
    let mut pdf = Vec::with_capacity(256 * 1024);
    pdf.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");

    let mut offsets = Vec::with_capacity(objects.len() + 1);
    offsets.push(0usize);
    for (idx, obj_bytes) in objects.iter().enumerate() {
        offsets.push(pdf.len());
        let obj_num = idx + 1;
        let _ = writeln!(pdf, "{obj_num} 0 obj");
        pdf.extend_from_slice(obj_bytes);
        pdf.extend_from_slice(b"\nendobj\n");
    }

    let xref_offset = pdf.len();
    let total_objs = objects.len() + 1;
    let _ = writeln!(pdf, "xref\n0 {total_objs}");
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for &off in &offsets[1..] {
        let _ = writeln!(pdf, "{off:010} 00000 n ");
    }
    let _ = writeln!(
        pdf,
        "trailer\n<< /Size {total_objs} /Root {catalog_id} 0 R >>\nstartxref\n{xref_offset}\n%%EOF"
    );

    Ok(pdf)
}

fn build_stream_object(dict: &str, data: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(dict.len() + data.len() + 24);
    out.extend_from_slice(dict.as_bytes());
    out.extend_from_slice(b"\nstream\n");
    out.extend_from_slice(data);
    out.extend_from_slice(b"\nendstream");
    out
}

fn build_tounicode_cmap(code_to_char: &BTreeMap<u16, char>) -> Vec<u8> {
    let mut s = String::new();
    let _ = writeln!(
        s,
        "/CIDInit /ProcSet findresource begin\n12 dict begin\nbegincmap\n/CIDSystemInfo << /Registry (Adobe) /Ordering (UCS) /Supplement 0 >> def\n/CMapName /Custom-UTF16 def\n/CMapType 2 def\n1 begincodespacerange\n<0000> <FFFF>\nendcodespacerange"
    );
    let entries: Vec<(&u16, &char)> = code_to_char.iter().collect();
    for chunk in entries.chunks(100) {
        let _ = writeln!(s, "{} beginbfchar", chunk.len());
        for &(&code, &ch) in chunk {
            let mut utf16_buf = [0u16; 2];
            let encoded = ch.encode_utf16(&mut utf16_buf);
            if encoded.len() == 1 {
                let _ = writeln!(s, "<{code:04X}> <{:04X}>", encoded[0]);
            } else {
                let _ = writeln!(s, "<{code:04X}> <{:04X}{:04X}>", encoded[0], encoded[1]);
            }
        }
        let _ = writeln!(s, "endbfchar");
    }
    let _ = write!(
        s,
        "endcmap\nCMapName currentdict /CMap defineresource pop\nend\nend"
    );
    s.into_bytes()
}

fn adobe_glyph_name(ch: char) -> String {
    match ch {
        '\u{2018}' => "quoteleft".to_string(),
        '\u{2019}' => "quoteright".to_string(),
        '\u{201C}' => "quotedblleft".to_string(),
        '\u{201D}' => "quotedblright".to_string(),
        '\u{2022}' => "bullet".to_string(),
        '\u{2013}' => "endash".to_string(),
        '\u{2014}' => "emdash".to_string(),
        '\u{2026}' => "ellipsis".to_string(),
        '\u{20AC}' => "Euro".to_string(),
        '\u{2122}' => "trademark".to_string(),
        other => format!("uni{:04X}", other as u32),
    }
}

fn zlib_compress(data: &[u8]) -> Vec<u8> {
    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    let _ = encoder.write_all(data);
    encoder.finish().unwrap_or_default()
}

fn pdf_image_from_rgba(image: &image::RgbaImage, index: usize) -> PdfImageObject {
    let mut rgb = Vec::with_capacity(image.len() / 4 * 3);
    let mut alpha = Vec::with_capacity(image.len() / 4);
    for px in image.pixels() {
        rgb.extend_from_slice(&px.0[..3]);
        alpha.push(px.0[3]);
    }
    let mask = if alpha.iter().any(|a| *a < 255) {
        Some(zlib_compress(&alpha))
    } else {
        None
    };
    PdfImageObject {
        resource_name: format!("Im{}", index + 1),
        width: image.width(),
        height: image.height(),
        rgb_compressed: zlib_compress(&rgb),
        alpha_compressed: mask,
    }
}
