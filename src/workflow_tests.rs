use super::*;
use crate::model::pdf_export::export_project_to_pdf_bytes;
use std::{fs, process::Command};

#[test]
fn every_production_template_round_trips_and_exports() {
    let root = std::env::current_dir().unwrap().join("qa/template-proof");
    fs::create_dir_all(&root).unwrap();
    for template in available_templates() {
        let mut project = build_template_project(template.id);
        project.title.push_str(" — edited proof");
        let path = root.join(format!("{}.canvas.json", template.id));
        save_project_json(&project, &path).unwrap();
        let project = load_project_json(&path).unwrap();
        let report = run_export_preflight(&project, 300);
        assert!(!report.has_errors(), "{}: {:?}", template.id, report.issues);
        let pdf = root.join(format!("{}.pdf", template.id));
        fs::write(
            &pdf,
            export_project_to_pdf_bytes(
                &project,
                &PdfExportOptions {
                    target_image_dpi: 96,
                    ..Default::default()
                },
            )
            .unwrap(),
        )
        .unwrap();
        let text = Command::new("pdftotext")
            .arg(&pdf)
            .arg("-")
            .output()
            .unwrap();
        assert!(text.status.success());
        assert!(
            !text.stdout.is_empty(),
            "template PDF must preserve real text"
        );
        for i in 0..project.pages.len() {
            render_project_page_at_dpi(&project, i, 96, false)
                .unwrap()
                .save(root.join(format!("{}-{i}.png", template.id)))
                .unwrap();
            let svg = crate::exporter::export_project_page_to_svg(&project, i, false).unwrap();
            fs::write(root.join(format!("{}-{i}.svg", template.id)), svg).unwrap();
        }
        let render = Command::new("pdftoppm")
            .args(["-scale-to", "900", "-png"])
            .arg(&pdf)
            .arg(root.join(format!("{}-pdf", template.id)))
            .output()
            .unwrap();
        assert!(
            render.status.success(),
            "{}",
            String::from_utf8_lossy(&render.stderr)
        );
        assert!(export_project_to_html_deck(&project)
            .unwrap()
            .contains("<svg"));
    }
}

#[test]
fn complete_groups_copy_transform_undo_and_reopen_independently() {
    let dir = std::env::temp_dir().join(model::generate_document_id());
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(dir.join("recovery")));
    app.project = build_template_project("brand_logo");
    app.project.pages[0].elements.truncate(3);
    app.canvas_state.selected_ids = app.project.pages[0].elements.iter().map(|e| e.id).collect();
    app.selected_id = app.canvas_state.selected_ids.first().copied();
    app.group_selected_elements();
    let original = app.project.pages[0].elements.clone();
    app.duplicate_selected();
    assert_eq!(app.project.pages[0].elements.len(), 6);
    assert_eq!(app.canvas_state.selected_ids.len(), 3);
    let copies = app.project.pages[0].elements[3..].to_vec();
    assert_ne!(copies[0].group_id, original[0].group_id);
    assert!(copies.iter().all(|e| e.group_id == copies[0].group_id));
    let from = document_ops::bounds(&copies).unwrap();
    let to = egui::Rect::from_center_size(from.center() + egui::vec2(40., 60.), from.size() * 1.5);
    app.record_mutation();
    document_ops::transform_selection(&mut app.project.pages[0], &copies, from, to, 90.);
    assert_eq!(app.project.pages[0].elements[..3], original);
    for (before, after) in copies.iter().zip(&app.project.pages[0].elements[3..]) {
        assert!((after.width - before.width * 1.5).abs() < 0.001);
        assert!((after.rotation - before.rotation - 90.).abs() < 0.001);
    }
    let transformed = app.project.clone();
    assert!(app.perform_undo());
    assert_eq!(app.project.pages[0].elements[3..], copies);
    assert!(app.perform_redo());
    assert_eq!(app.project, transformed);
    save_project_json(&app.project, &dir.join("copy.canvas.json")).unwrap();
    assert_eq!(
        load_project_json(&dir.join("copy.canvas.json")).unwrap(),
        transformed
    );
    app.toggle_selected_lock();
    assert!(app.project.pages[0].elements[3..].iter().all(|e| e.locked));
    let locked = app.project.pages[0].elements[3..].to_vec();
    document_ops::transform_selection(&mut app.project.pages[0], &locked, to, from, 30.);
    assert_eq!(app.project.pages[0].elements[3..], locked);
    app.insert_copies(copies, 28.);
    assert_eq!(app.project.pages[0].elements.len(), 9);
    assert_ne!(
        app.project.pages[0].elements[6].group_id,
        app.project.pages[0].elements[3].group_id
    );
    fs::remove_dir_all(dir).unwrap();
}

#[test]
fn every_inserted_page_fits_without_changing_the_source_or_master() {
    let source = build_template_project("pitch_deck");
    let saved = source.clone();
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(
        std::env::temp_dir().join(model::generate_document_id()),
    ));
    app.project.format = DesignFormat::DocumentA4;
    (app.project.width, app.project.height) = app.project.format.dimensions();
    let before = app.project.clone();
    app.insert_template_pages(&source);
    assert_eq!(app.project.pages.len(), 3);
    assert_eq!(source, saved);
    for (inserted, old) in app.project.pages[1..].iter().zip(&source.pages) {
        assert_eq!(inserted.elements.len(), old.elements.len());
        let scale = app.project.width / source.width;
        for (new, old) in inserted.elements.iter().zip(&old.elements) {
            assert!((new.width - old.width * scale).abs() < 0.001);
            assert!((new.x - old.x * scale).abs() < 0.001);
        }
    }
    let mut ids = std::collections::HashSet::new();
    for page in &app.project.pages {
        assert!(ids.insert(page.id));
        for element in &page.elements {
            assert!(ids.insert(element.id));
        }
    }
    assert!(app.perform_undo());
    assert_eq!(app.project, before);
}

#[test]
fn svg_and_html_refuse_unsupported_effects_instead_of_dropping_them() {
    let mut project = build_template_project("brand_logo");
    project.pages[0].elements[0].shadow.enabled = true;
    assert!(
        export_page_to_svg(&project.pages[0], project.width, project.height, false)
            .unwrap_err()
            .contains("shadow")
    );
    assert!(export_project_to_html_deck(&project).is_err());
    project.pages.clear();
    assert!(export_project_to_html_deck(&project).is_err());
}

#[test]
fn layer_selection_and_keyboard_operations_preserve_groups_and_locks() {
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(
        std::env::temp_dir().join(model::generate_document_id()),
    ));
    app.project = build_template_project("brand_logo");
    app.project.pages[0].elements.truncate(2);
    app.canvas_state.selected_ids = app.project.pages[0].elements.iter().map(|e| e.id).collect();
    app.selected_id = app.canvas_state.selected_ids.first().copied();
    app.group_selected_elements();
    let mut decoration = app.project.pages[0].elements[0].clone();
    decoration.id = app.project.alloc_id();
    decoration.group_id = None;
    app.project.pages[0].elements.push(decoration.clone());
    let original = app.project.pages[0].elements[..2].to_vec();
    app.select_layer(original[0].id);
    assert_eq!(app.canvas_state.selected_ids.len(), 2);
    app.nudge_selected(Vec2::new(2., 4.));
    for (after, before) in app.project.pages[0].elements[..2].iter().zip(&original) {
        assert_eq!((after.x, after.y), (before.x + 2., before.y + 4.));
    }
    app.move_selected_layer(true);
    assert_eq!(app.project.pages[0].elements[0].id, decoration.id);
    assert_eq!(app.project.pages[0].elements[1].id, original[0].id);
    app.project.pages[0].elements[1].locked = true;
    let locked = app.project.clone();
    app.nudge_selected(Vec2::new(10., 10.));
    app.move_selected_layer(false);
    app.delete_selected();
    assert_eq!(app.project, locked);
    app.project.pages[0].elements[1].locked = false;
    app.delete_selected();
    assert_eq!(app.project.pages[0].elements, vec![decoration]);
}
