use super::*;
use std::{fs, process::Command};

#[test]
fn cold_portable_project_child() {
    let Ok(path) = std::env::var("CANVAS_PORTABLE_TEST") else {
        return;
    };
    let project = load_project_json(Path::new(&path)).unwrap();
    assert_eq!(project.imported_fonts.len(), 1);
    assert_eq!(project.print_settings.guides_x_mm, vec![25.4]);
    assert_eq!(project.print_settings.presets[0].dpi, 150);
    assert_eq!(project.pages[0].elements.len(), 2);
    render_project_page_at_dpi(&project, 0, 96, false).unwrap();
    let pdf = Path::new(&path).with_extension("pdf");
    export_project_to_pdf(&project, &PdfExportOptions::default(), &pdf).unwrap();
    let text = Command::new("pdftotext")
        .arg(pdf)
        .arg("-")
        .output()
        .unwrap();
    assert!(text.status.success());
    assert!(String::from_utf8_lossy(&text.stdout).contains("Portable proof"));
}

#[test]
fn single_file_projects_move_with_images_fonts_styles_and_output_settings() {
    let root = std::env::temp_dir().join(model::generate_document_id());
    fs::create_dir_all(&root).unwrap();
    let mut project = Project::default();
    let font =
        imported_fonts::parse(include_bytes!("../assets/fonts/NotoSerif-Regular.ttf").to_vec())
            .unwrap();
    let id = font.id;
    project.imported_fonts.push(font);
    let mut text = build_template_project("festival_poster")
        .pages
        .remove(0)
        .elements
        .remove(2);
    text.x = 60.;
    text.y = 60.;
    text.width = 600.;
    text.height = 120.;
    text.text_format.inset_left = 12.;
    if let ElementKind::Text {
        content,
        font_size,
        font_family,
        color,
        ..
    } = &mut text.kind
    {
        *content = "Portable proof".into();
        *font_size = 30.;
        *font_family = FontFamily::Imported(id);
        *color = rgb(20, 20, 20);
    }
    let assets = generate_builtin_stock_assets();
    let photo = make_stock_image_element(project.alloc_id(), &assets[0], 60., 240.);
    project.pages[0].elements = vec![text, photo];
    project.print_settings.guides_x_mm = vec![25.4];
    project
        .print_settings
        .presets
        .push(crate::model::OutputPreset {
            name: "Proof".into(),
            dpi: 150,
            bleed_mm: 0.,
            crop_marks: false,
            transparent: false,
        });
    let package = root.join("source/design.canvas");
    save_project_json(&project, &package).unwrap();
    assert!(project_package::is_package(&package));
    let moved = root.join("moved.canvas");
    fs::rename(&package, &moved).unwrap();
    fs::remove_dir_all(root.join("source")).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "print_studio_tests::cold_portable_project_child",
            "--nocapture",
        ])
        .env("CANVAS_PORTABLE_TEST", &moved)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}{}",
        String::from_utf8_lossy(&child.stdout),
        String::from_utf8_lossy(&child.stderr)
    );
    // Image texture revisions are intentionally transient and reset on loading.
    assert!(
        serde_json::to_value(load_project_json(&moved).unwrap()).unwrap()
            == serde_json::to_value(&project).unwrap(),
        "Portable document content changed during round trip"
    );
    let original = fs::read(&moved).unwrap();
    let mut invalid = project.clone();
    if let ElementKind::Image { raw_rgba, .. } = &mut invalid.pages[0].elements[1].kind {
        raw_rgba.clear();
    }
    assert!(save_project_json(&invalid, &moved).is_err());
    assert_eq!(fs::read(&moved).unwrap(), original);
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn print_guides_are_persistent_editing_aids_and_safe_area_warnings_identify_text() {
    let mut project = build_template_project("festival_poster");
    let before = render_project_page_at_dpi(&project, 0, 96, false).unwrap();
    project.print_settings.show_guides = true;
    project.print_settings.guides_x_mm = vec![40.];
    project.print_settings.guides_y_mm = vec![60.];
    assert_eq!(
        render_project_page_at_dpi(&project, 0, 96, false).unwrap(),
        before
    );
    let index = project.pages[0]
        .elements
        .iter()
        .position(|e| matches!(e.kind, ElementKind::Text { .. }))
        .unwrap();
    project.pages[0].elements[index].x = 0.;
    assert!(run_export_preflight(&project, 96)
        .issues
        .iter()
        .any(|i| i.category == "safe_area"));
    project.print_settings.safe_margin_mm = f32::NAN;
    assert!(run_export_preflight(&project, 96).has_errors());
}
