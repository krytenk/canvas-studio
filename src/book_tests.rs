use super::*;
use std::{fs, process::Command};

fn fixture_art(root: &Path, count: usize) -> Vec<PathBuf> {
    fs::create_dir_all(root).unwrap();
    (1..=count)
        .map(|index| {
            let path = root.join(format!("Artwork {index:02}.png"));
            let mut image =
                image::RgbaImage::from_pixel(512, 640, image::Rgba([255, 255, 255, 255]));
            for y in 20..620 {
                for x in 20..492 {
                    if x == 20
                        || x == 491
                        || y == 20
                        || y == 619
                        || (x + index * 4) % 89 == 0 && y > 100 && y < 500
                    {
                        image.put_pixel(x as u32, y, image::Rgba([20, 30, 40, 255]));
                    }
                }
            }
            image.save(&path).unwrap();
            path
        })
        .collect()
}

#[test]
fn fifty_page_workbook_reorders_numbers_and_reopens_as_a_portable_project() {
    let started = std::time::Instant::now();
    let root = std::env::current_dir().unwrap().join("qa/book-proof");
    let paths = fixture_art(&root.join("artwork"), 50);
    let mut project = Project::default();
    project.title = "Synthetic 50-page workbook acceptance fixture".into();
    project.production.book.header = "Workbook · {{page_title}}".into();
    project.production.book.footer = "Print proof · {{pages}} pages".into();
    assert_eq!(
        book_production::append_artwork(&mut project, &paths).unwrap(),
        50
    );
    assert_eq!(project.pages.len(), 50);
    let recto = book_production::content_rect(&project, 0).unwrap();
    let verso = book_production::content_rect(&project, 1).unwrap();
    assert!(recto.min.x > verso.min.x);
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(root.join("recovery")));
    app.project = project;
    let first = app.project.pages[0].id;
    app.move_book_page(0, false);
    assert_eq!(app.project.pages[1].id, first);
    assert!(app.perform_undo());
    assert_eq!(app.project.pages[0].id, first);
    app.project.production.book.footer = "Updated master · {{pages}} pages".into();
    let package = root.join("workbook.canvas");
    save_project_json(&app.project, &package).unwrap();
    let reopened = load_project_json(&package).unwrap();
    let resolved = production::resolve(&reopened, true).unwrap();
    let mut ids = std::collections::HashSet::new();
    for (index, page) in resolved.pages.iter().enumerate() {
        for element in &page.elements {
            assert!(ids.insert(element.id), "Duplicate generated layer ID");
        }
        let number = page
            .elements
            .iter()
            .find(|e| e.name == "Automatic page number")
            .unwrap();
        if let ElementKind::Text { content, .. } = &number.kind {
            assert_eq!(content, &(index + 1).to_string());
        }
        assert!(page.elements.iter().any(
            |e| matches!(&e.kind,ElementKind::Text{content,..}if content.contains("Updated master"))
        ));
    }
    let pdf = root.join("workbook.pdf");
    export_project_to_pdf(
        &reopened,
        &PdfExportOptions {
            target_image_dpi: 96,
            ..Default::default()
        },
        &pdf,
    )
    .unwrap();
    let info = Command::new("pdfinfo").arg(&pdf).output().unwrap();
    assert!(info.status.success());
    let info = String::from_utf8(info.stdout).unwrap();
    assert!(info
        .lines()
        .any(|line| line.starts_with("Pages:") && line.split_whitespace().last() == Some("50")));
    let range = root.join("pages-2-to-3.pdf");
    export_project_to_pdf(
        &reopened,
        &PdfExportOptions {
            page_selection: PageSelection::Range(1, 2),
            target_image_dpi: 96,
            ..Default::default()
        },
        &range,
    )
    .unwrap();
    let text = Command::new("pdftotext")
        .arg(range)
        .arg("-")
        .output()
        .unwrap();
    let text = String::from_utf8(text.stdout).unwrap();
    assert!(text.contains("Artwork 02") && text.contains("Artwork 03"));
    let render = Command::new("pdftoppm")
        .args(["-f", "1", "-l", "2", "-scale-to", "900", "-png"])
        .arg(&pdf)
        .arg(root.join("workbook-page"))
        .output()
        .unwrap();
    assert!(render.status.success());
    fs::write(root.join("timing.txt"),format!("50 pages; synthetic 512×640 artwork; save/reopen/full PDF/range PDF/independent proof: {:.3} seconds\n",started.elapsed().as_secs_f64())).unwrap();
}

#[test]
fn blank_backs_and_front_matter_have_explicit_numbering_and_invalid_imports_do_not_mutate() {
    let root = std::env::temp_dir().join(model::generate_document_id());
    let paths = fixture_art(&root, 2);
    let mut project = Project::default();
    project.production.book.blank_backs = true;
    book_production::append_artwork(&mut project, &paths).unwrap();
    assert_eq!(project.pages.len(), 4);
    assert!(!project.pages[1].master_enabled && !project.pages[3].master_enabled);
    let pages = production::resolve(&project, true).unwrap();
    assert!(pages.pages[1].elements.is_empty());
    project.production.book.start_after_pages = 2;
    assert_eq!(book_production::page_number(&project, 0).unwrap(), None);
    assert_eq!(book_production::page_number(&project, 2).unwrap(), Some(1));
    let before = project.clone();
    let mut invalid = paths.clone();
    invalid.push(root.join("missing.png"));
    assert!(book_production::append_artwork(&mut project, &invalid).is_err());
    assert!(project == before);
    project.production.book.inner_mm = 500.;
    assert!(production::resolve(&project, true).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn master_edits_save_all_working_layers_and_can_be_cancelled() {
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(
        std::env::temp_dir().join(model::generate_document_id()),
    ));
    app.project = build_template_project("brand_logo");
    app.project.pages[0].elements.truncate(2);
    app.canvas_state.selected_ids = app.project.pages[0].elements.iter().map(|e| e.id).collect();
    app.selected_id = app.canvas_state.selected_ids.first().copied();
    app.move_selection_to_master();
    assert_eq!(app.project.production.book.master_elements.len(), 2);
    assert!(app.project.pages[0].elements.is_empty());
    app.begin_master_edit();
    assert_eq!(app.project.pages[0].elements.len(), 2);
    app.project.pages[0].elements[0].x += 30.;
    app.finish_master_edit(false);
    assert!(app.project.pages[0].elements.is_empty());
    let x = app.project.production.book.master_elements[0].x;
    app.begin_master_edit();
    app.project.pages[0].elements[0].x += 45.;
    app.canvas_state.selected_ids.truncate(1);
    app.finish_master_edit(true);
    assert_eq!(app.project.production.book.master_elements.len(), 2);
    assert_eq!(app.project.production.book.master_elements[0].x, x + 45.);
    assert!(app.project.pages[0].elements.is_empty());
}

#[test]
fn concurrent_artwork_import_preserves_newer_edits() {
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(
        std::env::temp_dir().join(model::generate_document_id()),
    ));
    let source = production::fingerprint(&app.project).unwrap();
    let mut imported = app.project.clone();
    let id = imported.alloc_id();
    imported.pages.push(CanvasPage::new(id, "Imported"));
    app.record_mutation();
    app.project.title = "Newer work".into();
    let newer = app.project.clone();
    let (sender, receiver) = std::sync::mpsc::channel();
    app.production_job = Some(receiver);
    sender
        .send(Ok(crate::production_ui::ProductionOutcome::Artwork {
            project: imported,
            source_fingerprint: source,
        }))
        .unwrap();
    app.poll_production_job();
    assert!(app.project == newer);
    assert!(app.completed_artwork.is_some());
}

#[test]
fn book_master_assets_reopen_in_a_fresh_process() {
    if let Some(path) = std::env::var_os("CANVAS_MASTER_LOAD") {
        let project = load_project_json(Path::new(&path)).unwrap();
        let resolved = production::resolve(&project, true).unwrap();
        assert!(
            matches!(&resolved.pages[0].elements[0].kind,ElementKind::Image{raw_rgba,..} if raw_rgba.len()==512*640*4)
        );
        return;
    }
    let root = std::env::temp_dir().join(model::generate_document_id());
    let paths = fixture_art(&root, 1);
    let mut project = Project::default();
    book_production::append_artwork(&mut project, &paths).unwrap();
    project.production.book.master_elements = std::mem::take(&mut project.pages[0].elements);
    let path = root.join("master.canvas");
    save_project_json(&project, &path).unwrap();
    let out = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "book_tests::book_master_assets_reopen_in_a_fresh_process",
        ])
        .env("CANVAS_MASTER_LOAD", &path)
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stdout)
    );
    // A separate JSON master must also stage its assets and reject a corrupt file,
    // even when the in-process cache already contains the healthy pixels.
    let json = root.join("master.json");
    save_project_json(&project, &json).unwrap();
    let asset = fs::read_dir(root.join("assets"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    fs::write(asset, b"damaged").unwrap();
    assert!(load_project_json(&json).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn copied_pixels_share_storage_until_one_copy_is_edited() {
    use model::asset_store::{AssetStore, SharedPixels};
    let original: SharedPixels = vec![25; 4_000_000].into();
    let mut copy = original.clone();
    assert_eq!(original.as_ptr(), copy.as_ptr());
    let stored = AssetStore::global().register_shared(&original, 1000, 1000);
    assert_eq!(stored.decoded_rgba.as_ptr(), original.as_ptr());
    copy[0] = 99;
    assert_ne!(original.as_ptr(), copy.as_ptr());
    assert_eq!(original[0], 25);
    assert_eq!(copy[0], 99);
}
