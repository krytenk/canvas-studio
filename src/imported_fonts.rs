use crate::model::{ElementKind, FontFamily, Project};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImportedFont {
    pub id: u64,
    pub name: String,
    pub bold: bool,
    pub italic: bool,
    #[serde(with = "font_bytes")]
    pub bytes: Arc<Vec<u8>>,
}

mod font_bytes {
    use super::*;
    pub fn serialize<S: serde::Serializer>(
        bytes: &Arc<Vec<u8>>,
        serializer: S,
    ) -> Result<S::Ok, S::Error> {
        serializer
            .serialize_str(&base64::engine::general_purpose::STANDARD.encode(bytes.as_slice()))
    }
    pub fn deserialize<'de, D: serde::Deserializer<'de>>(
        deserializer: D,
    ) -> Result<Arc<Vec<u8>>, D::Error> {
        let data = String::deserialize(deserializer)?;
        if data.len() > 28_000_000 {
            return Err(serde::de::Error::custom("Embedded font exceeds size limit"));
        }
        base64::engine::general_purpose::STANDARD
            .decode(data)
            .map(Arc::new)
            .map_err(serde::de::Error::custom)
    }
}

pub fn parse(bytes: Vec<u8>) -> Result<ImportedFont, String> {
    if bytes.len() > 20_000_000 || bytes.len() < 12 {
        return Err("Choose a static TrueType font smaller than 20 MB".into());
    }
    let face =
        ttf_parser::Face::parse(&bytes, 0).map_err(|e| format!("Invalid TrueType font: {e:?}"))?;
    if bytes.starts_with(b"ttcf") || face.is_variable() || face.tables().glyf.is_none() {
        return Err("Use an individual static .ttf face; collections, variable fonts and CFF outlines are not supported yet".into());
    }
    if !face.is_outline_embedding_allowed()
        || !matches!(
            face.permissions(),
            Some(ttf_parser::Permissions::Installable | ttf_parser::Permissions::Editable)
        )
    {
        return Err("This font does not permit embedding in editable documents".into());
    }
    let name = face
        .names()
        .into_iter()
        .find(|n| n.name_id == ttf_parser::name_id::FULL_NAME && n.is_unicode())
        .and_then(|n| n.to_string())
        .unwrap_or_else(|| "Imported TrueType face".into());
    let (bold, italic) = (face.is_bold(), face.is_italic());
    let id = bytes.iter().fold(0xcbf29ce484222325u64, |hash, b| {
        (hash ^ *b as u64).wrapping_mul(0x100000001b3)
    });
    ab_glyph::FontArc::try_from_vec(bytes.clone())
        .map_err(|_| "Font outlines could not be loaded")?;
    Ok(ImportedFont {
        id,
        name,
        bold,
        italic,
        bytes: Arc::new(bytes),
    })
}

pub fn import(project: &mut Project, path: &Path) -> Result<u64, String> {
    let size = std::fs::metadata(path).map_err(|e| e.to_string())?.len();
    if size > 20_000_000 {
        return Err("Font exceeds the 20 MB limit".into());
    }
    let font = parse(std::fs::read(path).map_err(|e| e.to_string())?)?;
    crate::model::font_registry::FontRegistry::global().register_imported(&font)?;
    let id = font.id;
    if !project.imported_fonts.iter().any(|f| f.id == id) {
        project.imported_fonts.push(font);
    }
    Ok(id)
}

pub fn register_project(project: &Project) -> Result<(), String> {
    let registry = crate::model::font_registry::FontRegistry::global();
    for font in &project.imported_fonts {
        registry.register_imported(font)?;
    }
    for element in project.pages.iter().flat_map(|p| &p.elements) {
        if let ElementKind::Text {
            font_family,
            content,
            ..
        } = &element.kind
        {
            element.text_format.validate(content)?;
            for family in std::iter::once(*font_family).chain(
                element
                    .text_format
                    .runs
                    .iter()
                    .filter_map(|r| r.font_family),
            ) {
                if let FontFamily::Imported(id) = family {
                    if !project.imported_fonts.iter().any(|f| f.id == id) {
                        return Err(format!(
                            "Text '{}' references a missing imported font",
                            element.name
                        ));
                    }
                }
            }
        }
    }
    Ok(())
}
