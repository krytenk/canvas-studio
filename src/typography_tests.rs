use super::*;
use std::{fs, process::Command};

fn font_project() -> Project {
    let mut project = Project::default();
    let font =
        imported_fonts::parse(include_bytes!("../assets/fonts/NotoSerif-Regular.ttf").to_vec())
            .unwrap();
    let id = font.id;
    project.imported_fonts.push(font);
    let mut text = build_template_project("festival_poster").pages[0]
        .elements
        .iter()
        .find(|e| matches!(e.kind, ElementKind::Text { .. }))
        .unwrap()
        .clone();
    text.x = 60.;
    text.y = 60.;
    text.width = 600.;
    text.height = 180.;
    if let ElementKind::Text {
        font_family,
        content,
        font_size,
        color,
        ..
    } = &mut text.kind
    {
        *font_family = FontFamily::Imported(id);
        *content = "Portable café — René & Zoë".into();
        *font_size = 30.;
        *color = rgb(20, 20, 20);
    }
    project.pages[0].elements = vec![text];
    project
}

#[test]
fn cold_imported_font_child() {
    let Ok(path) = std::env::var("CANVAS_FONT_PROJECT") else {
        return;
    };
    let project = load_project_json(Path::new(&path)).unwrap();
    let report = run_export_preflight(&project, 96);
    assert!(!report.has_errors(), "{:?}", report.issues);
    let pdf = Path::new(&path).with_extension("pdf");
    export_project_to_pdf(&project, &PdfExportOptions::default(), &pdf).unwrap();
    render_project_page_at_dpi(&project, 0, 96, false)
        .unwrap()
        .save(Path::new(&path).with_extension("png"))
        .unwrap();
}

#[test]
fn embedded_imported_faces_survive_cold_load_and_export_real_text() {
    let root = std::env::temp_dir().join(model::generate_document_id());
    fs::create_dir_all(&root).unwrap();
    let project = font_project();
    let path = root.join("portable.canvas.json");
    save_project_json(&project, &path).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "typography_tests::cold_imported_font_child",
            "--nocapture",
        ])
        .env("CANVAS_FONT_PROJECT", &path)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    let pdf = path.with_extension("pdf");
    let text = Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .unwrap();
    assert!(text.status.success());
    assert!(String::from_utf8_lossy(&text.stdout).contains("Portable café"));
    let fonts = Command::new("pdffonts").arg(&pdf).output().unwrap();
    assert!(fonts.status.success());
    assert!(String::from_utf8_lossy(&fonts.stdout).contains("CanvasImported"));
    let svg = export_page_to_svg(&project.pages[0], project.width, project.height, false).unwrap();
    assert!(svg.contains("data:font/ttf;base64,"));
    assert!(svg.contains("CanvasImported"));
    let mut target = Project::default();
    let pages = document_ops::fitted_pages(&mut target, &project);
    target.pages = pages;
    assert_eq!(target.imported_fonts, project.imported_fonts);
    assert!(!run_export_preflight(&target, 96).has_errors());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn broken_or_missing_fonts_are_not_silently_replaced_and_legacy_files_still_open() {
    assert!(imported_fonts::parse(b"not a font".to_vec()).is_err());
    let mut project = font_project();
    project.imported_fonts.clear();
    assert!(run_export_preflight(&project, 96).has_errors());
    let mut legacy = serde_json::to_value(Project::default()).unwrap();
    legacy.as_object_mut().unwrap().remove("imported_fonts");
    assert!(serde_json::from_value::<Project>(legacy)
        .unwrap()
        .imported_fonts
        .is_empty());
    let mut corrupt = font_project();
    corrupt.imported_fonts[0].id += 1;
    assert!(imported_fonts::register_project(&corrupt).is_err());
}
