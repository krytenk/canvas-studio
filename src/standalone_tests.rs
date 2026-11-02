use super::*;
use crate::model::font_registry::{FontFaceId, FontRegistry};

#[test]
fn all_fonts_are_embedded_and_legacy_project_fonts_still_load() {
    let registry = FontRegistry::load_default();
    assert!(registry.is_healthy(), "{:?}", registry.load_warnings());
    for id in FontFaceId::all() {
        let face = registry.get(*id).unwrap();
        assert!(face.source_path.starts_with("embedded"));
        assert!(!face.used_fallback_dir);
        assert!(!face.bytes.is_empty());
    }
    let legacy: FontFamily = serde_json::from_str("\"CanvaSans\"").unwrap();
    assert_eq!(legacy, FontFamily::Sans);
    assert_eq!(serde_json::to_string(&legacy).unwrap(), "\"Sans\"");
}

#[test]
fn layout_assets_follow_the_json_when_its_folder_moves() {
    let root = std::env::temp_dir().join(crate::model::generate_document_id());
    let first = root.join("first");
    let moved = root.join("moved");
    std::fs::create_dir_all(first.join("assets")).unwrap();
    image::RgbaImage::from_pixel(32, 24, image::Rgba([20, 110, 160, 255]))
        .save(first.join("assets/example.png"))
        .unwrap();
    let fixture = serde_json::json!({
        "name": "Portable Sample", "stem": "portable-sample", "w": 480, "h": 672,
        "pages": [{"title": "Sample", "w": 480, "h": 672, "ops": [
            {"kind": "text", "x": 40, "y": 40, "w": 380, "h": 60,
             "text": "A portable layout", "size": 22},
            {"kind": "image", "x": 40, "y": 140, "w": 160, "h": 120,
             "path": "assets/example.png"}
        ]}]
    });
    std::fs::write(first.join("layout.json"), fixture.to_string()).unwrap();
    let original = import_layout_json(&first.join("layout.json")).unwrap();
    assert_eq!(original.pages[0].elements.len(), 2);
    std::fs::rename(&first, &moved).unwrap();
    let reopened = import_layout_json(&moved.join("layout.json")).unwrap();
    assert_eq!(reopened.title, original.title);
    assert_eq!(reopened.pages[0].elements, original.pages[0].elements);
    let saved = root.join("package/design.canvas.json");
    save_project_json(&reopened, &saved).unwrap();
    assert_eq!(load_project_json(&saved).unwrap().pages.len(), 1);
    std::fs::remove_file(moved.join("assets/example.png")).unwrap();
    assert!(import_layout_json(&moved.join("layout.json")).is_err());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn generic_collection_import_and_page_range_export_need_no_external_library() {
    use crate::model::import_layout::{import_all_layout_collections, list_layout_collections};
    let root = std::env::temp_dir().join(crate::model::generate_document_id());
    std::fs::create_dir_all(&root).unwrap();
    let mut collection: serde_json::Value =
        serde_json::from_str(include_str!("../examples/layout.json")).unwrap();
    let page = collection["pages"][0].clone();
    collection["pages"] = serde_json::json!([page.clone(), page.clone(), page]);
    let path = root.join("collections.json");
    std::fs::write(&path, serde_json::json!([collection]).to_string()).unwrap();
    let summaries = list_layout_collections(&path).unwrap();
    assert_eq!(summaries[0].page_count, 3);
    let projects = import_all_layout_collections(&path).unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(projects[0].pages.len(), 3);
    let output = root.join("range.pdf");
    export_project_to_pdf(
        &projects[0],
        &PdfExportOptions {
            page_selection: PageSelection::Range(1, 2),
            ..Default::default()
        },
        &output,
    )
    .unwrap();
    let pdf = std::fs::read(&output).unwrap();
    assert!(pdf.starts_with(b"%PDF-"));
    assert_eq!(
        pdf.windows(b"/Type /Page ".len())
            .filter(|w| *w == b"/Type /Page ")
            .count(),
        2
    );
    std::fs::remove_dir_all(root).unwrap();
}
