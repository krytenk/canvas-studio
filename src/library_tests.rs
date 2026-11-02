use super::*;
use std::{fs, process::Command};

#[test]
fn cold_library_copy_child() {
    let Ok(root) = std::env::var("CANVAS_TEST_TEMPLATE_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let (entries, warnings) = template_library::list(&root).unwrap();
    assert!(warnings.is_empty());
    assert_eq!(entries.len(), 1);
    let copy = template_library::open_copy(&root, &entries[0].id).unwrap();
    let photo = copy.pages[0]
        .elements
        .iter()
        .find(|e| matches!(e.kind, ElementKind::Image { .. }))
        .unwrap();
    if let ElementKind::Image {
        width_px,
        height_px,
        raw_rgba,
        ..
    } = &photo.kind
    {
        assert_eq!(raw_rgba.len(), *width_px as usize * *height_px as usize * 4);
        assert!(raw_rgba.iter().any(|b| *b != 0));
    }
    render_project_page_at_dpi(&copy, 0, 96, false).unwrap();
}

#[test]
fn library_masters_are_portable_and_copies_cannot_overwrite_them_by_default() {
    let dir = std::env::temp_dir().join(model::generate_document_id());
    let root = dir.join("library");
    let mut project = Project::default();
    project.title = "Original design".into();
    let assets = generate_builtin_stock_assets();
    let photo = make_stock_image_element(project.alloc_id(), &assets[0], 40., 40.);
    project.pages[0].elements.push(photo);
    let original = project.clone();
    let entry = template_library::save(
        &root,
        "Editorial invitation",
        "Wedding, Editorial",
        &project,
    )
    .unwrap();
    assert_eq!(project, original);
    assert!(entry.matches("wEdDiNg"));
    assert!(!entry.matches("retirement"));
    assert!(root.join(&entry.id).join("preview.png").is_file());
    let master_path = root.join(&entry.id).join("project.canvas.json");
    let master = fs::read(&master_path).unwrap();
    let first = template_library::open_copy(&root, &entry.id).unwrap();
    let second = template_library::open_copy(&root, &entry.id).unwrap();
    assert_ne!(first.document_id, second.document_id);
    assert_ne!(first.document_id, project.document_id);
    assert!(first.recovery_origin_path.is_none());
    // Move the complete library and open it in a process with an empty asset cache.
    let moved = dir.join("moved-library");
    fs::rename(&root, &moved).unwrap();
    let child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "library_tests::cold_library_copy_child",
            "--nocapture",
        ])
        .env("CANVAS_TEST_TEMPLATE_ROOT", &moved)
        .output()
        .unwrap();
    assert!(
        child.status.success(),
        "{}",
        String::from_utf8_lossy(&child.stderr)
    );
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(dir.join("app-recovery")));
    let app_library = app.template_library_dir();
    fs::create_dir_all(app_library.parent().unwrap()).unwrap();
    fs::rename(&moved, &app_library).unwrap();
    app.refresh_template_library();
    assert_eq!(app.library_entries.len(), 1);
    app.open_library_copy(&entry.id);
    assert!(app.current_project_path.is_none());
    app.record_mutation();
    app.project.title = "Customer working copy".into();
    assert_eq!(
        app.write_project_to_path(&dir.join("working.canvas.json")),
        SaveOutcome::Saved
    );
    assert_eq!(
        fs::read(app_library.join(&entry.id).join("project.canvas.json")).unwrap(),
        master
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn broken_library_entries_are_reported_and_invalid_masters_do_not_leave_partial_templates() {
    let root = std::env::temp_dir().join(model::generate_document_id());
    let project = build_template_project("brand_logo");
    template_library::save(&root, "Valid master", "", &project).unwrap();
    fs::create_dir_all(root.join("broken-entry")).unwrap();
    fs::write(root.join("broken-entry/metadata.json"), "invalid JSON").unwrap();
    let (entries, warnings) = template_library::list(&root).unwrap();
    assert_eq!(entries.len(), 1);
    assert_eq!(warnings.len(), 1);
    assert!(template_library::open_copy(&root, "../outside").is_err());
    let mut unsupported = project.clone();
    unsupported.pages[0].elements[0].shadow.enabled = true;
    assert!(template_library::save(&root, "Unsupported", "", &unsupported).is_err());
    assert_eq!(template_library::list(&root).unwrap().0.len(), 1);
    assert!(!fs::read_dir(&root).unwrap().any(|e| e
        .unwrap()
        .file_name()
        .to_string_lossy()
        .starts_with(".staging")));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn inspector_geometry_preserves_physical_units_and_rejects_partial_invalid_edits() {
    let assets = generate_builtin_stock_assets();
    let mut element = make_stock_image_element(100, &assets[0], 0., 0.);
    element.width = 192.;
    element.height = 96.;
    document_ops::set_geometry(
        &mut element,
        [1., 2., 4., 1.],
        [true, true, true, false],
        1. / 96.,
        true,
    )
    .unwrap();
    assert_eq!(
        (element.x, element.y, element.width, element.height),
        (96., 192., 384., 192.)
    );
    let before = element.clone();
    assert!(document_ops::set_geometry(
        &mut element,
        [0., 0., f32::NAN, 50.],
        [false, false, true, true],
        25.4 / 96.,
        false
    )
    .is_err());
    assert_eq!(element, before);
    document_ops::set_geometry(
        &mut element,
        [25.4, 50.8, 101.6, 50.8],
        [true; 4],
        25.4 / 96.,
        false,
    )
    .unwrap();
    assert!((element.width - 384.).abs() < 0.001);
}

#[test]
fn print_panels_render_without_dirtying_the_document() {
    let dir = std::env::temp_dir().join(model::generate_document_id());
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(dir.join("recovery")));
    app.project = build_template_project("festival_poster");
    let id = app.project.pages[0]
        .elements
        .iter()
        .find(|e| matches!(e.kind, ElementKind::Text { .. }))
        .unwrap()
        .id;
    app.selected_id = Some(id);
    app.canvas_state.selected_ids = vec![id];
    let before = app.project.clone();
    let ctx = Context::default();
    for dock in [DockTab::Text, DockTab::Layers, DockTab::Templates] {
        app.active_dock = dock;
        for _ in 0..3 {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        Vec2::new(1440., 1000.),
                    )),
                    ..Default::default()
                },
                |ctx| app.update_ui(ctx),
            );
        }
        assert_eq!(app.project, before);
        assert!(!app.dirty);
    }
}
