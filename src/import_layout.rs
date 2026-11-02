use std::fs;
use std::path::Path;

use serde::Deserialize;

use crate::model::asset_store::AssetStore;
use crate::model::text_layout::layout_text_block;
use crate::model::{
    rgb, rgba, BrandKit, CanvasElement, CanvasPage, ColorRgba, DesignFormat, DropShadow,
    ElementAnimation, ElementKind, FillStyle, FontFamily, ImageAdjustments, ImageCrop, ImageFit,
    ImageMask, ListStyle, PageBackground, PageTransition, Project, ShapeType, TextAlign,
    TextEffect,
};

#[derive(Clone, Debug, PartialEq)]
pub struct LayoutCollectionSummary {
    pub index: usize,
    pub name: String,
    pub stem: String,
    pub width: f32,
    pub height: f32,
    pub page_count: usize,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LayoutCollection {
    pub name: String,
    pub stem: String,
    pub w: f32,
    pub h: f32,
    pub pages: Vec<LayoutPage>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct LayoutPage {
    pub title: String,
    pub w: f32,
    pub h: f32,
    #[serde(default = "default_bg_hex")]
    pub bg: String,
    #[serde(default)]
    pub ops: Vec<LayoutOp>,
}

fn default_bg_hex() -> String {
    "#FFFFFF".to_string()
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
pub struct LayoutCrop {
    #[serde(default)]
    pub left: f32,
    #[serde(default)]
    pub top: f32,
    #[serde(default)]
    pub right: f32,
    #[serde(default)]
    pub bottom: f32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LayoutOp {
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        #[serde(default = "default_none_str")]
        fill: String,
        #[serde(default = "default_none_str")]
        stroke: String,
        #[serde(default)]
        sw: f32,
        #[serde(default)]
        radius: f32,
    },
    Text {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        text: String,
        size: f32,
        #[serde(default = "default_font_str")]
        font: String,
        #[serde(default = "default_text_color")]
        color: String,
        #[serde(default = "default_align_str")]
        align: String,
        #[serde(default)]
        leading: Option<f32>,
    },
    Image {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        path: String,
        #[serde(default = "default_fit_str")]
        fit: String,
        #[serde(default)]
        crop: Option<LayoutCrop>,
    },
}

fn default_none_str() -> String {
    "none".to_string()
}

fn default_font_str() -> String {
    "sans".to_string()
}

fn default_text_color() -> String {
    "#1C1917".to_string()
}

fn default_align_str() -> String {
    "left".to_string()
}

fn default_fit_str() -> String {
    "cover".to_string()
}

pub fn list_layout_collections(path: &Path) -> Result<Vec<LayoutCollectionSummary>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading collections file '{}': {e}", path.display()))?;
    let collections: Vec<LayoutCollection> = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed parsing collections JSON '{}': {e}", path.display()))?;
    Ok(collections
        .into_iter()
        .enumerate()
        .map(|(index, c)| LayoutCollectionSummary {
            index,
            name: c.name,
            stem: c.stem,
            width: c.w,
            height: c.h,
            page_count: c.pages.len(),
        })
        .collect())
}

pub fn import_all_layout_collections(path: &Path) -> Result<Vec<Project>, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading collections file '{}': {e}", path.display()))?;
    let mut collections: Vec<LayoutCollection> = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed parsing collections JSON '{}': {e}", path.display()))?;
    resolve_image_paths(&mut collections, path.parent().unwrap_or(Path::new(".")));
    let mut projects = Vec::with_capacity(collections.len());
    for collection in &collections {
        projects.push(convert_layout_collection_to_project(collection)?);
    }
    Ok(projects)
}

pub fn import_layout_collection_by_stem_or_name(
    path: &Path,
    query: &str,
) -> Result<Project, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading collections file '{}': {e}", path.display()))?;
    let mut collections: Vec<LayoutCollection> = serde_json::from_str(&raw)
        .map_err(|e| format!("Failed parsing collections JSON '{}': {e}", path.display()))?;
    resolve_image_paths(&mut collections, path.parent().unwrap_or(Path::new(".")));
    let q = query.trim().to_ascii_lowercase();
    let matched = collections
        .iter()
        .find(|c| c.stem.to_ascii_lowercase() == q || c.name.to_ascii_lowercase() == q)
        .or_else(|| {
            collections.iter().find(|c| {
                c.stem.to_ascii_lowercase().contains(&q) || c.name.to_ascii_lowercase().contains(&q)
            })
        })
        .ok_or_else(|| format!("No layout collection matched '{query}'"))?;
    convert_layout_collection_to_project(matched)
}

pub fn import_layout_json(path: &Path) -> Result<Project, String> {
    let raw = fs::read_to_string(path)
        .map_err(|e| format!("Failed reading layout file '{}': {e}", path.display()))?;
    if let Ok(collection) = serde_json::from_str::<LayoutCollection>(&raw) {
        let mut collections = vec![collection];
        resolve_image_paths(&mut collections, path.parent().unwrap_or(Path::new(".")));
        return convert_layout_collection_to_project(&collections[0]);
    }
    if let Ok(page) = serde_json::from_str::<LayoutPage>(&raw) {
        let collection = LayoutCollection {
            name: page.title.clone(),
            stem: path
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("layout-single-page")
                .to_string(),
            w: page.w,
            h: page.h,
            pages: vec![page],
        };
        let mut collections = vec![collection];
        resolve_image_paths(&mut collections, path.parent().unwrap_or(Path::new(".")));
        return convert_layout_collection_to_project(&collections[0]);
    }
    let mut collections: Vec<LayoutCollection> = serde_json::from_str(&raw).map_err(|e| {
        format!(
            "Unrecognized layout JSON structure in '{}': {e}",
            path.display()
        )
    })?;
    resolve_image_paths(&mut collections, path.parent().unwrap_or(Path::new(".")));
    let first = collections
        .first()
        .ok_or_else(|| "Empty collections array".to_string())?;
    convert_layout_collection_to_project(first)
}

fn resolve_image_paths(collections: &mut [LayoutCollection], base_dir: &Path) {
    for collection in collections {
        for page in &mut collection.pages {
            for op in &mut page.ops {
                if let LayoutOp::Image { path, .. } = op {
                    let image_path = Path::new(path);
                    if image_path.is_relative() {
                        *path = base_dir.join(image_path).to_string_lossy().into_owned();
                    }
                }
            }
        }
    }
}

pub fn convert_layout_collection_to_project(
    collection: &LayoutCollection,
) -> Result<Project, String> {
    if collection.pages.is_empty() {
        return Err(format!("Collection '{}' has zero pages", collection.name));
    }

    let (format, width_mm, height_mm) = infer_physical_format(collection.w, collection.h);
    let mut pages = Vec::with_capacity(collection.pages.len());
    let mut next_id = 1u64;

    for (page_idx, layout_page) in collection.pages.iter().enumerate() {
        let bg = parse_hex_color(&layout_page.bg).unwrap_or(rgb(255, 255, 255));
        let mut elements = Vec::with_capacity(layout_page.ops.len());

        for (op_idx, op) in layout_page.ops.iter().enumerate() {
            let elem_id = next_id;
            next_id += 1;
            let element = convert_layout_op(elem_id, op_idx, op)?;
            elements.push(element);
        }

        pages.push(CanvasPage {
            id: (page_idx as u64) + 1,
            title: layout_page.title.clone(),
            notes: format!("Imported from layout collection '{}'", collection.name),
            background: PageBackground::Solid(bg),
            transition: PageTransition::None,
            elements,
        });
    }

    Ok(Project {
        schema_version: 2,
        document_id: crate::model::generate_document_id(),
        recovery_origin_path: None,
        title: collection.name.clone(),
        format,
        width: collection.w,
        height: collection.h,
        width_mm,
        height_mm,
        units_per_inch: 96.0,
        bleed_mm: 0.0,
        crop_marks: false,
        pages,
        brand_kit: BrandKit::default(),
        imported_fonts: Vec::new(),
        next_id,
    })
}

fn infer_physical_format(w: f32, h: f32) -> (DesignFormat, f32, f32) {
    // A4 @ 96 dpi: 793.7008 x 1122.5197 -> 210 x 297 mm
    if (w - 793.7).abs() < 4.0 && (h - 1122.5).abs() < 4.0 {
        return (DesignFormat::DocumentA4, 210.0, 297.0);
    }
    // US-Letter @ 96 dpi: 816 x 1056 -> 215.9 x 279.4 mm
    if (w - 816.0).abs() < 4.0 && (h - 1056.0).abs() < 4.0 {
        return (DesignFormat::DocumentLetter, 215.9, 279.4);
    }
    // 5x7 in @ 96 dpi: 480 x 672 -> 127.0 x 177.8 mm
    if (w - 480.0).abs() < 4.0 && (h - 672.0).abs() < 4.0 {
        return (DesignFormat::Stationery5x7, 127.0, 177.8);
    }
    let width_mm = (w / 96.0) * 25.4;
    let height_mm = (h / 96.0) * 25.4;
    (
        DesignFormat::Custom(w.round().max(1.0) as u32, h.round().max(1.0) as u32),
        width_mm,
        height_mm,
    )
}

fn convert_layout_op(id: u64, op_idx: usize, op: &LayoutOp) -> Result<CanvasElement, String> {
    match op {
        LayoutOp::Rect {
            x,
            y,
            w,
            h,
            fill,
            stroke,
            sw,
            radius,
        } => {
            let fill_color = if fill.trim().eq_ignore_ascii_case("none") || fill.trim().is_empty() {
                rgba(0, 0, 0, 0)
            } else {
                parse_hex_color(fill)
                    .ok_or_else(|| format!("Invalid rect fill color '{fill}' at op #{op_idx}"))?
            };
            let (stroke_color, stroke_width) = if stroke.trim().eq_ignore_ascii_case("none")
                || stroke.trim().is_empty()
                || *sw <= 0.0
            {
                (rgba(0, 0, 0, 0), 0.0)
            } else {
                let sc = parse_hex_color(stroke).ok_or_else(|| {
                    format!("Invalid rect stroke color '{stroke}' at op #{op_idx}")
                })?;
                (sc, *sw)
            };
            let shape_type = if *radius > 0.5 {
                ShapeType::RoundedRect
            } else {
                ShapeType::Rectangle
            };
            Ok(CanvasElement {
                id,
                name: format!("Rect {}", op_idx + 1),
                x: *x,
                y: *y,
                width: w.max(0.5),
                height: h.max(0.5),
                rotation: 0.0,
                opacity: 1.0,
                locked: false,
                visible: true,
                flip_h: false,
                flip_v: false,
                shadow: DropShadow::default(),
                animation: ElementAnimation::None,
                group_id: None,
                text_format: Default::default(),
                kind: ElementKind::Shape {
                    shape_type,
                    fill: FillStyle::Solid(fill_color),
                    stroke_color,
                    stroke_width,
                    corner_radius: *radius,
                },
            })
        }
        LayoutOp::Text {
            x,
            y,
            w,
            h,
            text,
            size,
            font,
            color,
            align,
            leading,
        } => {
            let text_color = parse_hex_color(color)
                .ok_or_else(|| format!("Invalid text color '{color}' at op #{op_idx}"))?;
            let (font_family, bold, italic) = map_layout_font(font);
            let text_align = match align.trim().to_ascii_lowercase().as_str() {
                "center" => TextAlign::Center,
                "right" => TextAlign::Right,
                _ => TextAlign::Left,
            };
            let base_font_size = size.max(4.0);
            let line_height = leading
                .map(|ld| {
                    if ld <= 4.0 {
                        ld.clamp(0.9, 2.5)
                    } else {
                        (ld / base_font_size).clamp(0.9, 2.5)
                    }
                })
                .unwrap_or(1.22);

            let (calibrated_size, calibrated_x, calibrated_w, calibrated_h) =
                calibrate_layout_text_box(
                    text,
                    font_family,
                    bold,
                    italic,
                    text_align,
                    base_font_size,
                    line_height,
                    *x,
                    *w,
                    *h,
                );

            let preview_label: String = text.chars().take(24).collect();
            Ok(CanvasElement {
                id,
                name: if preview_label.trim().is_empty() {
                    format!("Text {}", op_idx + 1)
                } else {
                    preview_label.replace('\n', " ")
                },
                x: calibrated_x,
                y: *y,
                width: calibrated_w,
                height: calibrated_h,
                rotation: 0.0,
                opacity: 1.0,
                locked: false,
                visible: true,
                flip_h: false,
                flip_v: false,
                shadow: DropShadow::default(),
                animation: ElementAnimation::None,
                group_id: None,
                text_format: Default::default(),
                kind: ElementKind::Text {
                    content: text.clone(),
                    font_family,
                    font_size: calibrated_size,
                    color: text_color,
                    bold,
                    italic,
                    underline: false,
                    uppercase: false,
                    align: text_align,
                    letter_spacing: 0.0,
                    line_height,
                    effect: TextEffect::None,
                    list_style: ListStyle::None,
                },
            })
        }
        LayoutOp::Image {
            x,
            y,
            w,
            h,
            path,
            fit,
            crop,
        } => {
            let img_path = Path::new(path);
            let stored = AssetStore::global().import_image_file(img_path)?;
            let image_fit = match fit.trim().to_ascii_lowercase().as_str() {
                "contain" => ImageFit::Contain,
                "stretch" => ImageFit::Stretch,
                _ => ImageFit::Cover,
            };
            let image_crop = crop
                .map(|c| ImageCrop {
                    left: c.left,
                    top: c.top,
                    right: c.right,
                    bottom: c.bottom,
                })
                .unwrap_or_default()
                .sanitized();

            let file_name = img_path
                .file_name()
                .and_then(|s| s.to_str())
                .unwrap_or("image.png")
                .to_string();

            let adjustments = ImageAdjustments {
                fit: image_fit,
                crop: image_crop,
                focal_point: (0.5, 0.5),
                ..ImageAdjustments::default()
            };

            Ok(CanvasElement {
                id,
                name: file_name.clone(),
                x: *x,
                y: *y,
                width: w.max(1.0),
                height: h.max(1.0),
                rotation: 0.0,
                opacity: 1.0,
                locked: false,
                visible: true,
                flip_h: false,
                flip_v: false,
                shadow: DropShadow::default(),
                animation: ElementAnimation::None,
                group_id: None,
                text_format: Default::default(),
                kind: ElementKind::Image {
                    source_name: file_name,
                    width_px: stored.width_px,
                    height_px: stored.height_px,
                    raw_rgba: (*stored.decoded_rgba).clone(),
                    mask: ImageMask::None,
                    adjustments,
                    border_color: rgba(0, 0, 0, 0),
                    border_width: 0.0,
                    revision: 1,
                },
            })
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn calibrate_layout_text_box(
    content: &str,
    font_family: FontFamily,
    bold: bool,
    italic: bool,
    align: TextAlign,
    base_size: f32,
    line_height: f32,
    x: f32,
    w: f32,
    h: f32,
) -> (f32, f32, f32, f32) {
    let explicit_lines = content.split('\n').count().max(1);
    let unwrapped = layout_text_block(
        content,
        font_family,
        base_size,
        bold,
        italic,
        false,
        align,
        0.0,
        line_height,
        ListStyle::None,
        10_000.0,
        10_000.0,
    );

    let mut size = base_size;
    let mut adj_x = x;
    let mut adj_w = w.max(8.0);

    if unwrapped.measured_width > adj_w {
        let ratio = adj_w / unwrapped.measured_width.max(1.0);
        if ratio >= 0.84 {
            size = (base_size * ratio * 0.985).max(4.5);
        } else {
            let extra = (unwrapped.measured_width - adj_w + 4.0).min(48.0);
            match align {
                TextAlign::Center => {
                    adj_x -= extra * 0.5;
                    adj_w += extra;
                }
                TextAlign::Right => {
                    adj_x -= extra;
                    adj_w += extra;
                }
                TextAlign::Left => {
                    adj_w += extra;
                }
            }
        }
    }

    let mut final_layout = layout_text_block(
        content,
        font_family,
        size,
        bold,
        italic,
        false,
        align,
        0.0,
        line_height,
        ListStyle::None,
        adj_w,
        h.max(size * 1.25),
    );

    if final_layout.lines.len() > explicit_lines && final_layout.overflows_height {
        let shrink = (h / final_layout.measured_height.max(1.0)).clamp(0.82, 0.98);
        size = (size * shrink).max(4.5);
        final_layout = layout_text_block(
            content,
            font_family,
            size,
            bold,
            italic,
            false,
            align,
            0.0,
            line_height,
            ListStyle::None,
            adj_w,
            h.max(size * 1.25),
        );
    }

    let adj_h = h.max(final_layout.measured_height + 2.0);
    (size, adj_x, adj_w, adj_h)
}

pub fn replace_element_image_from_path(
    element: &mut CanvasElement,
    path: &Path,
) -> Result<(), String> {
    let stored = AssetStore::global().import_image_file(path)?;
    let file_name = path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("replaced_image.png")
        .to_string();

    if let ElementKind::Image {
        source_name,
        width_px,
        height_px,
        raw_rgba,
        revision,
        ..
    } = &mut element.kind
    {
        *source_name = file_name.clone();
        *width_px = stored.width_px;
        *height_px = stored.height_px;
        *raw_rgba = (*stored.decoded_rgba).clone();
        *revision = revision.wrapping_add(1);
        element.name = file_name;
        Ok(())
    } else {
        Err("Selected element is not an image".to_string())
    }
}

fn map_layout_font(font_code: &str) -> (FontFamily, bool, bool) {
    match font_code.trim().to_ascii_lowercase().as_str() {
        "serif" => (FontFamily::DisplaySerif, false, false),
        "serifb" | "serif-bold" => (FontFamily::DisplaySerif, true, false),
        "serifi" | "serif-italic" => (FontFamily::DisplaySerif, false, true),
        "serifbi" => (FontFamily::DisplaySerif, true, true),
        "sansb" | "sans-bold" => (FontFamily::Sans, true, false),
        "sansi" | "sans-italic" => (FontFamily::Sans, false, true),
        "sansbi" => (FontFamily::Sans, true, true),
        "mono" => (FontFamily::MonoCode, false, false),
        "monob" | "mono-bold" => (FontFamily::MonoCode, true, false),
        _ => (FontFamily::Sans, false, false),
    }
}

pub fn parse_hex_color(s: &str) -> Option<ColorRgba> {
    let trimmed = s.trim().trim_start_matches('#');
    match trimmed.len() {
        6 => {
            let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
            let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
            let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
            Some(rgb(r, g, b))
        }
        8 => {
            let r = u8::from_str_radix(&trimmed[0..2], 16).ok()?;
            let g = u8::from_str_radix(&trimmed[2..4], 16).ok()?;
            let b = u8::from_str_radix(&trimmed[4..6], 16).ok()?;
            let a = u8::from_str_radix(&trimmed[6..8], 16).ok()?;
            Some(rgba(r, g, b, a))
        }
        _ => None,
    }
}
