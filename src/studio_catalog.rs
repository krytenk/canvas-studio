use crate::*;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
static PREVIEWS: OnceLock<Mutex<HashMap<String, Vec<u8>>>> = OnceLock::new();
pub(crate) fn catalog() -> Value {
    json!({"templates":templates::available_templates().iter().map(|t|{let p=templates::build_template_project(t.id);json!({"id":t.id,"name":t.name,"category":t.category,"description":t.subtitle,"width":p.width,"height":p.height,"pages":p.pages.len(),"accent":format!("#{:02x}{:02x}{:02x}",t.accent[0],t.accent[1],t.accent[2])})}).collect::<Vec<_>>()})
}
pub(crate) fn thumbnail(project: &Project, index: usize, edge: u32) -> Result<Vec<u8>, String> {
    let proof = production::resolve(project, false)?;
    let p = &*proof;
    let page = p.pages.get(index).ok_or("Page outside document")?;
    let edge = edge.clamp(64, 1600);
    let ratio = edge as f32 / p.width.max(p.height);
    let w = (p.width * ratio).round().max(1.) as u32;
    let h = (p.height * ratio).round().max(1.) as u32;
    let image = exporter::render_page_to_rgba_image(page, p.width, p.height, w, h, false);
    let mut data = Vec::new();
    image
        .write_to(
            &mut std::io::Cursor::new(&mut data),
            image::ImageFormat::Png,
        )
        .map_err(|e| e.to_string())?;
    Ok(data)
}
pub(crate) fn template_preview(id: &str) -> Result<Vec<u8>, String> {
    if !templates::available_templates().iter().any(|t| t.id == id) {
        return Err("Unknown template".into());
    }
    let cache = PREVIEWS.get_or_init(Default::default);
    if let Ok(cache) = cache.lock() {
        if let Some(data) = cache.get(id) {
            return Ok(data.clone());
        }
    }
    let data = thumbnail(&templates::build_template_project(id), 0, 400)?;
    if let Ok(mut cache) = cache.lock() {
        cache.insert(id.into(), data.clone());
    }
    Ok(data)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn campaign_catalog_has_distinct_purposes_and_exportable_readable_defaults() {
        let catalog = templates::available_templates();
        let mut ids = std::collections::HashSet::new();
        let mut pages = 0;
        for template in &catalog {
            assert!(ids.insert(template.id));
            let p = templates::build_template_project(template.id);
            pages += p.pages.len();
            let report = run_export_preflight(&p, 300);
            assert!(
                !report.has_errors() && !report.has_overflows(),
                "{}: {:?}",
                template.id,
                report.issues
            );
            let png = template_preview(template.id).unwrap();
            assert_eq!(
                image::load_from_memory(&png)
                    .unwrap()
                    .width()
                    .max(image::load_from_memory(&png).unwrap().height()),
                400
            );
        }
        assert_eq!(catalog.len(), 18);
        assert_eq!(pages, 27);
        assert_eq!(
            templates::build_template_project("ig_carousel").pages.len(),
            5
        );
        assert_eq!(
            templates::build_template_project("ig_story_sequence")
                .pages
                .len(),
            3
        );
        let banner = templates::build_template_project("yt_channel_art");
        assert_eq!((banner.width, banner.height), (2560., 1440.));
        let xs: Vec<_> = banner
            .print_settings
            .guides_x_mm
            .iter()
            .map(|v| v / 25.4 * 96.)
            .collect();
        let ys: Vec<_> = banner
            .print_settings
            .guides_y_mm
            .iter()
            .map(|v| v / 25.4 * 96.)
            .collect();
        for e in &banner.pages[0].elements {
            if matches!(e.kind, ElementKind::Text { .. }) {
                assert!(
                    e.x >= xs[0]
                        && e.x + e.width <= xs[1]
                        && e.y >= ys[0]
                        && e.y + e.height <= ys[1]
                );
            }
        }
    }
}
