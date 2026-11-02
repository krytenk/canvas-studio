use crate::model::*;
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::{borrow::Cow, collections::BTreeMap, path::Path};

#[derive(Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct DataField {
    pub value: String,
    pub required: bool,
    pub image: bool,
    #[serde(with = "photo_bytes")]
    pub image_png: Option<Arc<String>>,
}

impl std::fmt::Debug for DataField {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DataField")
            .field("value", &self.value)
            .field("required", &self.required)
            .field("photo_bytes", &self.image_png.as_ref().map(|v| v.len()))
            .finish()
    }
}
mod photo_bytes {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        value: &Option<Arc<String>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        match value {
            Some(v) => serializer.serialize_some(v.as_str()),
            None => serializer.serialize_none(),
        }
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Option<Arc<String>>, D::Error> {
        let value = Option::<String>::deserialize(deserializer)?;
        if value.as_ref().is_some_and(|v| v.len() > 34_000_000) {
            return Err(serde::de::Error::custom(
                "Embedded photo exceeds size limit",
            ));
        }
        Ok(value.map(Arc::new))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SharedStyle {
    pub family: FontFamily,
    pub size: f32,
    pub color: ColorRgba,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub leading: f32,
    pub tracking: f32,
    pub paragraph: TextFormatting,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct ProductionSettings {
    #[serde(skip)]
    pub resolved: bool,
    pub fields: BTreeMap<String, DataField>,
    pub styles: BTreeMap<String, SharedStyle>,
    pub book: crate::book_production::BookSettings,
    pub puzzles: Vec<crate::puzzle_production::WordSearchProduct>,
    pub studio: crate::campaign::StudioSettings,
}

impl ProductionSettings {
    pub fn active(&self) -> bool {
        !self.fields.is_empty()
            || !self.styles.is_empty()
            || self.book.enabled
            || self.studio.active()
    }
}

pub fn valid_key(key: &str) -> bool {
    !key.is_empty()
        && key.len() <= 64
        && key.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'_')
}

pub fn style_from(element: &CanvasElement) -> Option<SharedStyle> {
    if let ElementKind::Text {
        font_family,
        font_size,
        color,
        bold,
        italic,
        underline,
        line_height,
        letter_spacing,
        ..
    } = &element.kind
    {
        let mut paragraph = element.text_format.clone();
        paragraph.runs.clear();
        Some(SharedStyle {
            family: *font_family,
            size: *font_size,
            color: *color,
            bold: *bold,
            italic: *italic,
            underline: *underline,
            leading: *line_height,
            tracking: *letter_spacing,
            paragraph,
        })
    } else {
        None
    }
}

pub fn attach_image(field: &mut DataField, path: &Path) -> Result<(), String> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 25_000_000 {
        return Err("Field photo must be smaller than 25 MB".into());
    }
    let reader = image::ImageReader::open(path)
        .map_err(|e| e.to_string())?
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
    if w > 18_000 || h > 18_000 || w as u64 * h as u64 > 100_000_000 {
        return Err("Field photo exceeds supported dimensions".into());
    }
    let image = image::open(path).map_err(|e| e.to_string())?;
    let mut png = Vec::new();
    image
        .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    if png.len() > 25_000_000 {
        return Err("Encoded field photo exceeds 25 MB".into());
    }
    field.image = true;
    field.value = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .into_owned();
    field.image_png = Some(Arc::new(
        base64::engine::general_purpose::STANDARD.encode(png),
    ));
    Ok(())
}

pub fn apply_style(
    element: &mut CanvasElement,
    settings: &ProductionSettings,
    strict: bool,
) -> Result<(), String> {
    if let Some(key) = &element.style_id {
        if let Some(style) = settings.styles.get(key) {
            if let ElementKind::Text {
                font_family,
                font_size,
                color,
                bold,
                italic,
                underline,
                line_height,
                letter_spacing,
                ..
            } = &mut element.kind
            {
                *font_family = style.family;
                *font_size = style.size;
                *color = style.color;
                *bold = style.bold;
                *italic = style.italic;
                *underline = style.underline;
                *line_height = style.leading;
                *letter_spacing = style.tracking;
                let runs = std::mem::take(&mut element.text_format.runs);
                element.text_format = style.paragraph.clone();
                element.text_format.runs = runs;
            }
        } else if strict {
            return Err(format!(
                "Layer '{}' references missing shared style '{key}'",
                element.name
            ));
        }
    }
    Ok(())
}

fn resolve_element(
    element: &mut CanvasElement,
    settings: &ProductionSettings,
    strict: bool,
    page_number: &str,
    page_count: &str,
    page_title: &str,
) -> Result<(), String> {
    apply_style(element, settings, strict)?;
    if let ElementKind::Text { content, .. } = &mut element.kind {
        let source = content.clone();
        let mut edits = Vec::new();
        let mut cursor = 0;
        while let Some(relative) = source[cursor..].find("{{") {
            let start = cursor + relative;
            let Some(close) = source[start + 2..].find("}}") else {
                if strict {
                    return Err(format!(
                        "Layer '{}' has an unfinished field token",
                        element.name
                    ));
                }
                break;
            };
            let end = start + 2 + close + 2;
            let key = source[start + 2..end - 2].trim();
            let value = if key == "page" {
                page_number.into()
            } else if key == "pages" {
                page_count.into()
            } else if key == "page_title" {
                page_title.into()
            } else {
                match settings.fields.get(key) {
                    Some(field)
                        if !field.image && (!field.required || !field.value.trim().is_empty()) =>
                    {
                        field.value.clone()
                    }
                    Some(_) if strict => {
                        return Err(format!(
                            "Required text field '{key}' is empty or is a photo"
                        ))
                    }
                    None if strict => return Err(format!("Unknown text field '{key}'")),
                    _ => format!("«{key}»"),
                }
            };
            edits.push((start, end, value));
            cursor = end;
        }
        for (start, end, value) in edits.into_iter().rev() {
            let before = content.clone();
            content.replace_range(start..end, &value);
            element.text_format.reconcile_edit(&before, content);
        }
    }
    if let Some(key) = &element.binding {
        let field = settings.fields.get(key);
        if let Some(DataField {
            image_png: Some(encoded),
            value,
            ..
        }) = field
        {
            if encoded.len() > 34_000_000 {
                return Err("Embedded field photo exceeds supported size".into());
            }
            let data = base64::engine::general_purpose::STANDARD
                .decode(encoded.as_str())
                .map_err(|e| e.to_string())?;
            let reader = image::ImageReader::new(std::io::Cursor::new(&data))
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
            if w > 18_000 || h > 18_000 || w as u64 * h as u64 > 100_000_000 {
                return Err("Field photo exceeds supported dimensions".into());
            }
            let photo = image::load_from_memory(&data)
                .map_err(|e| e.to_string())?
                .to_rgba8();
            if let ElementKind::Image {
                width_px,
                height_px,
                raw_rgba,
                source_name,
                revision,
                ..
            } = &mut element.kind
            {
                *width_px = photo.width();
                *height_px = photo.height();
                *raw_rgba = photo.into_raw().into();
                *source_name = value.clone();
                *revision = crate::model::asset_store::AssetStore::compute_hash(raw_rgba)
                    .bytes()
                    .fold(0u64, |v, b| v.wrapping_mul(31).wrapping_add(b as u64));
            }
        } else if field.is_some_and(|f| !f.required) {
            element.visible = false;
        } else if strict {
            return Err(format!("Photo field '{key}' has no image"));
        }
    }
    crate::campaign::resolve_element(element, &settings.studio, strict)?;
    element.style_id = None;
    element.binding = None;
    Ok(())
}

pub fn resolve_page(
    project: &Project,
    index: usize,
    strict: bool,
) -> Result<Cow<'_, CanvasPage>, String> {
    let page = project
        .pages
        .get(index)
        .ok_or("Page is outside the document")?;
    if !project.production.active() {
        return Ok(Cow::Borrowed(page));
    }
    let mut page = crate::book_production::compose_page(project, index)?;
    if let Some(key) = project.production.studio.backgrounds.get(&page.id) {
        if let Some(color) = project.production.studio.tokens.get(key) {
            page.background = PageBackground::Solid(*color);
        } else if strict {
            return Err(format!("Missing page brand colour '{key}'"));
        }
    }
    let number = crate::book_production::page_number(project, index)?
        .map(|n| n.to_string())
        .unwrap_or_default();
    let count = project.pages.len().to_string();
    for element in &mut page.elements {
        resolve_element(
            element,
            &project.production,
            strict,
            &number,
            &count,
            &page.title,
        )?;
    }
    Ok(Cow::Owned(page))
}

pub fn resolve(project: &Project, strict: bool) -> Result<Cow<'_, Project>, String> {
    if project.production.resolved {
        return Ok(Cow::Borrowed(project));
    }
    if strict {
        crate::puzzle_production::validate_project(project)?;
    }
    if !project.production.active() {
        if strict
            && project.pages.iter().flat_map(|p| &p.elements).any(|e| {
                e.style_id.is_some()
                    || e.binding.is_some()
                    || matches!(&e.kind,ElementKind::Text {content,..} if content.contains("{{"))
            })
        {
            return Err(
                "This design has field/style bindings but no production definitions".into(),
            );
        }
        return Ok(Cow::Borrowed(project));
    }
    if strict {
        if project
            .production
            .fields
            .keys()
            .chain(project.production.styles.keys())
            .any(|key| !valid_key(key))
        {
            return Err("Production keys must contain letters, numbers and underscores".into());
        }
        for (key, field) in &project.production.fields {
            if field.required
                && ((field.image && field.image_png.is_none())
                    || (!field.image && field.value.trim().is_empty()))
            {
                return Err(format!("Required field '{key}' is empty"));
            }
        }
    }
    let mut resolved = project.clone();
    resolved.pages.clear();
    resolved.pages = (0..project.pages.len())
        .map(|i| resolve_page(project, i, strict).map(Cow::into_owned))
        .collect::<Result<_, _>>()?;
    resolved.production = ProductionSettings {
        resolved: true,
        ..Default::default()
    };
    Ok(Cow::Owned(resolved))
}

pub fn fingerprint(project: &Project) -> Result<String, String> {
    serde_json::to_vec(project)
        .map(|bytes| crate::model::asset_store::AssetStore::compute_hash(&bytes))
        .map_err(|e| e.to_string())
}
