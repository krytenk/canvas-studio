use super::*;
use crate::model::pdf_export::{export_page_to_pdf, export_project_to_pdf_bytes};
use std::{fs, process::Command};

fn workspace(label: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "canvas-regression-{label}-{}",
        crate::model::generate_document_id()
    ));
    fs::create_dir_all(&dir).unwrap();
    dir
}
fn app(dir: &Path) -> CanvasStudioApp {
    let mut a = CanvasStudioApp::new_headless_with_recovery_dir(Some(dir.join("recovery")));
    a.replace_document(Project::default(), None);
    a.stock_assets.clear();
    a
}
fn element(kind: ElementKind) -> CanvasElement {
    CanvasElement {
        id: 1,
        name: "Regression fixture".into(),
        x: 25.,
        y: 35.,
        width: 420.,
        height: 130.,
        rotation: 0.,
        opacity: 1.,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: Default::default(),
        animation: ElementAnimation::None,
        group_id: None,
        kind,
    }
}
fn photo() -> CanvasElement {
    element(ElementKind::Image {
        source_name: "pattern.png".into(),
        width_px: 20,
        height_px: 12,
        raw_rgba: (0..240)
            .flat_map(|i| {
                [
                    (i % 256) as u8,
                    ((i * 7) % 256) as u8,
                    ((i * 13) % 256) as u8,
                    255,
                ]
            })
            .collect(),
        mask: ImageMask::None,
        adjustments: Default::default(),
        border_color: rgba(0, 0, 0, 0),
        border_width: 0.,
        revision: 1,
    })
}
fn text(s: &str) -> CanvasElement {
    element(ElementKind::Text {
        content: s.into(),
        font_family: FontFamily::DisplaySerif,
        font_size: 28.,
        color: rgb(60, 25, 40),
        bold: false,
        italic: false,
        underline: false,
        uppercase: false,
        align: TextAlign::Left,
        letter_spacing: 0.,
        line_height: 1.4,
        effect: TextEffect::None,
        list_style: ListStyle::None,
    })
}
fn document(e: CanvasElement) -> Project {
    let mut p = Project::default();
    p.width = 480.;
    p.height = 360.;
    p.width_mm = 127.;
    p.height_mm = 95.25;
    p.format = DesignFormat::Custom(480, 360);
    p.pages[0].elements = vec![e];
    p
}
fn bytes(p: &Project) -> Vec<u8> {
    export_project_to_pdf_bytes(
        p,
        &PdfExportOptions {
            target_image_dpi: 96,
            ..Default::default()
        },
    )
    .unwrap()
}
fn extract(p: &Path) -> String {
    let o = Command::new("pdftotext")
        .arg("-layout")
        .arg(p)
        .arg("-")
        .output()
        .expect("Poppler required for PDF regression verification");
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap()
}
fn compact(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

#[test]
fn asset_failure_preserves_document_and_recovery() {
    let dir = workspace("asset-failure");
    let mut a = app(&dir);
    a.current_project_path = Some(dir.join("current.canvas.json"));
    assert_eq!(
        a.write_project_to_path(&dir.join("current.canvas.json")),
        SaveOutcome::Saved
    );
    let original = fs::read(dir.join("current.canvas.json")).unwrap();
    a.record_mutation();
    a.project.pages[0].elements.push(photo());
    let recovery = a.write_recovery_snapshot().unwrap();
    fs::write(dir.join("assets"), "blocked directory").unwrap();
    a.pending_close = true;
    let replacement = Project::default();
    a.request_replace_document(replacement, None, "Next".into());
    assert_eq!(a.confirm_unsaved_save_and_continue(), SaveOutcome::Failed);
    assert!(
        a.dirty
            && a.pending_close
            && !a.allow_close_without_prompt
            && a.pending_replace_project.is_some()
    );
    assert!(recovery.exists());
    assert_eq!(fs::read(dir.join("current.canvas.json")).unwrap(), original);
}

#[test]
fn cold_load_child() {
    let Some(path) = std::env::var_os("CANVAS_REGRESSION_LOAD") else {
        return;
    };
    let result = load_project_json(Path::new(&path));
    if std::env::var_os("CANVAS_REGRESSION_EXPECT_FAILURE").is_some() {
        assert!(result.is_err());
        return;
    }
    let p = result.unwrap();
    assert_eq!(p.pages[0].elements.len(), 1);
    match &p.pages[0].elements[0].kind {
        ElementKind::Image {
            width_px,
            height_px,
            raw_rgba,
            ..
        } => {
            assert_eq!((*width_px, *height_px), (20, 12));
            if let ElementKind::Image {
                raw_rgba: expected, ..
            } = photo().kind
            {
                assert_eq!(*raw_rgba, expected)
            }
        }
        _ => panic!("image missing"),
    }
}
fn cold_load(path: &Path, fail: bool) {
    let mut cmd = Command::new(std::env::current_exe().unwrap());
    cmd.args([
        "--exact",
        "regression_tests::cold_load_child",
        "--nocapture",
    ])
    .env("CANVAS_REGRESSION_LOAD", path);
    if fail {
        cmd.env("CANVAS_REGRESSION_EXPECT_FAILURE", "1");
    }
    let out = cmd.output().unwrap();
    assert!(
        out.status.success(),
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
}
#[test]
fn cold_process_assets_and_genuine_legacy_migration() {
    let dir = workspace("cold");
    let p = document(photo());
    let path = dir.join("project.canvas.json");
    save_project_json(&p, &path).unwrap();
    cold_load(&path, false);
    let asset = fs::read_dir(dir.join("assets"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let image = image::open(&asset).unwrap();
    assert_eq!((image.width(), image.height()), (20, 12));
    let good = fs::read(&asset).unwrap();
    let _ = load_project_json(&path).unwrap();
    fs::remove_file(&asset).unwrap();
    assert!(load_project_json(&path).is_err());
    cold_load(&path, true);
    fs::write(&asset, "broken image").unwrap();
    cold_load(&path, true);
    // A valid PNG with changed pixels must also fail, even with a warm cache.
    image::RgbaImage::from_pixel(20, 12, image::Rgba([1, 2, 3, 255]))
        .save(&asset)
        .unwrap();
    assert!(load_project_json(&path).is_err());
    cold_load(&path, true);
    fs::write(&asset, good).unwrap();
    let mut legacy = serde_json::to_value(&p).unwrap();
    legacy.as_object_mut().unwrap().remove("document_id");
    legacy
        .as_object_mut()
        .unwrap()
        .remove("recovery_origin_path");
    legacy.as_object_mut().unwrap().remove("schema_version");
    if let ElementKind::Image { raw_rgba, .. } = photo().kind {
        legacy["pages"][0]["elements"][0]["kind"]["Image"]["raw_rgba"] =
            serde_json::json!(raw_rgba);
    }
    let v1 = dir.join("legacy.json");
    fs::write(&v1, serde_json::to_vec(&legacy).unwrap()).unwrap();
    let original = fs::read(&v1).unwrap();
    cold_load(&v1, false);
    let dest = dir.join("migration/project.canvas.json");
    crate::exporter::migrate_v1_project_to_v2(&v1, &dest).unwrap();
    cold_load(&dest, false);
    assert_eq!(fs::read(&v1).unwrap(), original);
}

#[test]
fn recovery_ownership_and_unsaved_state() {
    let dir = workspace("recovery");
    let mut b = app(&dir);
    b.project.title = "Recovered B".into();
    b.record_mutation();
    b.project.pages[0].elements.push(photo());
    let b_path = b.write_recovery_snapshot().unwrap();
    let mut a = app(&dir);
    a.project.title = "Document A".into();
    a.current_project_path = Some(dir.join("A.canvas.json"));
    a.record_mutation();
    a.request_recover_document_from_path(&b_path).unwrap();
    assert!(a.pending_replace_project.is_some());
    assert_eq!(a.confirm_unsaved_save_and_continue(), SaveOutcome::Saved);
    assert_eq!(a.project.title, "Recovered B");
    assert!(a.dirty && a.current_project_path.is_none() && b_path.exists());
    assert!(!a.request_close_document());
    a.confirm_unsaved_cancel();
    assert!(a.dirty && b_path.exists());
    let mut c = app(&dir);
    c.project.title = "C".into();
    c.record_mutation();
    let c_path = c.write_recovery_snapshot().unwrap();
    assert_eq!(
        a.write_project_to_path(&dir.join("B.canvas.json")),
        SaveOutcome::Saved
    );
    assert!(!b_path.exists());
    assert!(c_path.exists());
}

#[test]
fn revisions_presets_and_undo_branch() {
    let dir = workspace("revisions");
    let mut a = app(&dir);
    a.insert_text_preset(
        "First",
        "A",
        FontFamily::Sans,
        24.,
        rgb(0, 0, 0),
        false,
        TextEffect::None,
    );
    assert!(a.dirty);
    let saved = dir.join("saved.canvas.json");
    assert_eq!(a.write_project_to_path(&saved), SaveOutcome::Saved);
    let saved_rev = a.document_revision;
    assert!(a.perform_undo());
    assert!(a.dirty);
    assert!(a.perform_redo());
    assert!(!a.dirty);
    assert!(a.perform_undo());
    a.insert_text_preset(
        "Branch",
        "B",
        FontFamily::Sans,
        24.,
        rgb(0, 0, 0),
        false,
        TextEffect::None,
    );
    assert!(a.dirty);
    assert_ne!(saved_rev, a.document_revision);
    a.request_replace_document(Project::default(), None, "Next".into());
    assert!(a.pending_replace_project.is_some());
    a.confirm_unsaved_cancel();
    assert!(a.dirty);
}

#[test]
fn fractional_sizes_conversion_and_allocation_limits() {
    let dir = workspace("sizes");
    let mut a = app(&dir);
    a.set_custom_size_unit(2);
    assert!((a.custom_w_input - 12.5).abs() < 0.0001);
    assert!((a.custom_h_input - 9.375).abs() < 0.0001);
    for (unit, w, h, mw, mh) in [
        (2, 8.5, 11., 215.9, 279.4),
        (2, 5.5, 8.5, 139.7, 215.9),
        (1, 215.9, 279.4, 215.9, 279.4),
    ] {
        a.custom_size_unit = unit;
        a.custom_w_input = w;
        a.custom_h_input = h;
        a.apply_custom_size().unwrap();
        assert!(
            (a.project.width_mm - mw).abs() < 0.001 && (a.project.height_mm - mh).abs() < 0.001
        );
    }
    a.custom_size_unit = 2;
    a.custom_w_input = 5.;
    a.custom_h_input = 7.;
    a.apply_custom_size().unwrap();
    assert_eq!(a.project.pixel_dimensions_for_dpi(300), (1500, 2100));
    let old = a.project.width_mm;
    a.custom_w_input = 1200.;
    a.custom_h_input = 900.;
    assert!(a.apply_custom_size().is_err());
    assert_eq!(a.project.width_mm, old);
    a.project.width_mm = 30480.;
    a.project.height_mm = 22860.;
    assert!(render_project_page_at_dpi(&a.project, 0, 300, false).is_err());
}

#[test]
fn snapshot_cache_tracks_pixels_and_small_history() {
    let mut p = document(photo());
    let mut cache = crate::model::SnapshotCache::default();
    let before = cache.snapshot(&p);
    let same = cache.snapshot(&p);
    assert_eq!(before, same);
    if let ElementKind::Image {
        raw_rgba, revision, ..
    } = &mut p.pages[0].elements[0].kind
    {
        raw_rgba[0] ^= 1;
        *revision += 1;
    }
    assert_ne!(cache.snapshot(&p), before);
    let mut history = HistoryManager::default();
    for i in 0..20 {
        p.title = format!("Edit {i}");
        history.record_with_revision(&p, i);
    }
    assert!(history.approximate_heap_bytes() < 50000);
}

#[test]
fn pdf_rendering_and_unicode_content_regressions() {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qa/completion-2026-11-01/fixtures");
    fs::create_dir_all(&dir).unwrap();
    let a = document(text("AVATAR"));
    let mut b = a.clone();
    if let ElementKind::Text { letter_spacing, .. } = &mut b.pages[0].elements[0].kind {
        *letter_spacing = 6.;
    }
    assert_ne!(bytes(&a), bytes(&b));
    assert_ne!(
        export_page_to_svg(&a.pages[0], a.width, a.height, false),
        export_page_to_svg(&b.pages[0], b.width, b.height, false)
    );
    for (name, p) in [("spacing-zero", &a), ("spacing-six", &b)] {
        fs::write(dir.join(format!("{name}.pdf")), bytes(p)).unwrap();
        render_project_page_at_dpi(p, 0, 96, false)
            .unwrap()
            .save(dir.join(format!("{name}.png")))
            .unwrap();
    }
    let unicode = "АБВГДЕЁЖЗИЙКЛМНОПРСТУФХЦЧШЩЪЫЬЭЮЯ абвгдеёжзийклмнопрстуфхцчшщъыьэюя";
    let mut u = document(text(unicode));
    if let ElementKind::Text { font_size, .. } = &mut u.pages[0].elements[0].kind {
        *font_size = 17.;
    }
    let path = dir.join("unicode.pdf");
    fs::write(&path, bytes(&u)).unwrap();
    assert_eq!(compact(&extract(&path)), compact(unicode));
    render_project_page_at_dpi(&u, 0, 96, false)
        .unwrap()
        .save(dir.join("unicode.png"))
        .unwrap();
    for angle in [-30., 30.] {
        let mut p = document(text("Sample Studio"));
        let e = &mut p.pages[0].elements[0];
        e.x = 120.;
        e.y = 100.;
        e.width = 250.;
        e.height = 100.;
        e.rotation = angle;
        let name = if angle < 0. {
            "rotation-negative"
        } else {
            "rotation-positive"
        };
        fs::write(dir.join(format!("{name}.pdf")), bytes(&p)).unwrap();
        render_project_page_at_dpi(&p, 0, 96, false)
            .unwrap()
            .save(dir.join(format!("{name}.png")))
            .unwrap();
    }
    let table = document(element(ElementKind::Table {
        rows: 1,
        cols: 1,
        cells: vec![vec!["CRITICAL TABLE CONTENT".into()]],
        header_bg: rgb(220, 230, 240),
        row_bg: rgb(255, 255, 255),
        alt_row_bg: rgb(240, 240, 240),
        border_color: rgb(0, 0, 0),
        text_color: rgb(0, 0, 0),
        font_size: 18.,
    }));
    let path = dir.join("table.pdf");
    fs::write(&path, bytes(&table)).unwrap();
    assert_eq!(compact(&extract(&path)), "CRITICALTABLECONTENT");
    render_project_page_at_dpi(&table, 0, 96, false)
        .unwrap()
        .save(dir.join("table.png"))
        .unwrap();
    let grad = document(element(ElementKind::Shape {
        shape_type: ShapeType::Rectangle,
        fill: FillStyle::LinearGradient {
            start: rgb(255, 0, 0),
            end: rgb(0, 0, 255),
            vertical: false,
        },
        stroke_color: rgba(0, 0, 0, 0),
        stroke_width: 0.,
        corner_radius: 0.,
    }));
    fs::write(dir.join("gradient.pdf"), bytes(&grad)).unwrap();
    render_project_page_at_dpi(&grad, 0, 96, false)
        .unwrap()
        .save(dir.join("gradient.png"))
        .unwrap();
}

#[test]
fn preflight_errors_do_not_overwrite_output() {
    let dir = workspace("preflight");
    let path = dir.join("previous.pdf");
    fs::write(&path, "previous output").unwrap();
    let mut p = document(photo());
    if let ElementKind::Image { raw_rgba, .. } = &mut p.pages[0].elements[0].kind {
        raw_rgba.clear();
    }
    assert!(export_page_to_pdf(&p, 0, &path).is_err());
    assert_eq!(fs::read(&path).unwrap(), b"previous output");
    p = document(element(ElementKind::Chart {
        chart_type: ChartType::VerticalBar,
        title: "Chart".into(),
        labels: vec!["A".into()],
        values: vec![1.],
        palette: vec![rgb(0, 0, 255)],
        show_values: true,
        show_grid: true,
        text_color: rgb(0, 0, 0),
    }));
    assert!(export_project_to_pdf_bytes(&p, &Default::default()).is_err());
}

fn find_text(shape: &egui::Shape, needle: &str) -> Option<egui::Pos2> {
    match shape {
        egui::Shape::Text(t) if t.galley.job.text == needle => {
            Some(t.pos + t.galley.rect.size() / 2.)
        }
        egui::Shape::Vec(v) => v.iter().find_map(|s| find_text(s, needle)),
        _ => None,
    }
}
fn ui_frame(
    a: &mut CanvasStudioApp,
    ctx: &Context,
    events: Vec<egui::Event>,
    mods: egui::Modifiers,
) -> egui::FullOutput {
    ctx.run(
        egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                Vec2::new(1300., 900.),
            )),
            events,
            modifiers: mods,
            ..Default::default()
        },
        |ctx| a.update_ui(ctx),
    )
}
#[test]
fn headless_ui_typing_is_dirty_and_undoable() {
    let dir = workspace("ui-typing");
    let mut a = app(&dir);
    a.project = document(text("Before"));
    a.selected_id = Some(1);
    a.canvas_state.inline_edit_id = Some(1);
    a.dirty = false;
    a.saved_revision = Some(a.document_revision);
    let ctx = Context::default();
    for _ in 0..3 {
        ui_frame(&mut a, &ctx, vec![], egui::Modifiers::NONE);
    }
    let f = ui_frame(&mut a, &ctx, vec![], egui::Modifiers::NONE);
    let pos = f
        .shapes
        .iter()
        .find_map(|s| find_text(&s.shape, "Before"))
        .expect("actual inline edit field must be visible");
    ui_frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: egui::Modifiers::NONE,
            },
        ],
        egui::Modifiers::NONE,
    );
    ui_frame(
        &mut a,
        &ctx,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: egui::Modifiers::NONE,
        }],
        egui::Modifiers::NONE,
    );
    ui_frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            },
            egui::Event::Key {
                key: Key::A,
                physical_key: None,
                pressed: false,
                repeat: false,
                modifiers: egui::Modifiers {
                    ctrl: true,
                    command: true,
                    ..Default::default()
                },
            },
        ],
        egui::Modifiers {
            ctrl: true,
            command: true,
            ..Default::default()
        },
    );
    ui_frame(
        &mut a,
        &ctx,
        vec![egui::Event::Text("After".into())],
        egui::Modifiers::NONE,
    );
    assert!(a.dirty, "typing must activate dirty");
    match &a.project.pages[0].elements[0].kind {
        ElementKind::Text { content, .. } => assert_eq!(content, "After"),
        _ => panic!(),
    }
    assert!(a.perform_undo());
    match &a.project.pages[0].elements[0].kind {
        ElementKind::Text { content, .. } => assert_eq!(content, "Before"),
        _ => panic!(),
    }
    assert!(!a.dirty);
    assert!(a.perform_redo());
    a.current_project_path = Some(dir.join("keyboard-save.canvas.json"));
    let mods = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    ui_frame(
        &mut a,
        &ctx,
        vec![egui::Event::Key {
            key: Key::S,
            physical_key: None,
            pressed: true,
            repeat: false,
            modifiers: mods,
        }],
        mods,
    );
    assert!(
        !a.dirty && dir.join("keyboard-save.canvas.json").exists(),
        "Ctrl+S must work while the text field has focus"
    );
}

fn ink_bounds(img: &image::RgbaImage) -> (u32, u32, u32, u32) {
    let (mut x0, mut y0, mut x1, mut y1) = (img.width(), img.height(), 0, 0);
    for (x, y, p) in img.enumerate_pixels() {
        if p[0].min(p[1]).min(p[2]) < 160 && p[3] > 180 {
            x0 = x0.min(x);
            y0 = y0.min(y);
            x1 = x1.max(x);
            y1 = y1.max(y);
        }
    }
    (x0, y0, x1, y1)
}
#[test]
fn independently_rendered_pdf_has_matching_rotation_and_gradient() {
    let dir = workspace("pixel-proof");
    for (i, angle) in [-30., 0., 30., 90.].iter().enumerate() {
        let mut p = document(text("Sample Studio"));
        let e = &mut p.pages[0].elements[0];
        e.x = 120.;
        e.y = 100.;
        e.width = 250.;
        e.height = 100.;
        e.rotation = *angle;
        let path = dir.join(format!("rotation-{i}.pdf"));
        fs::write(&path, bytes(&p)).unwrap();
        let prefix = dir.join(format!("render-{i}"));
        let o = Command::new("pdftoppm")
            .args(["-r", "96", "-f", "1", "-singlefile", "-png"])
            .arg(&path)
            .arg(&prefix)
            .output()
            .unwrap();
        assert!(o.status.success());
        let proof = image::open(prefix.with_extension("png"))
            .unwrap()
            .to_rgba8();
        let expected = render_project_page_at_dpi(&p, 0, 96, false).unwrap();
        let a = ink_bounds(&expected);
        let b = ink_bounds(&proof);
        for (x, y) in [a.0, a.1, a.2, a.3].into_iter().zip([b.0, b.1, b.2, b.3]) {
            assert!(x.abs_diff(y) <= 3, "rotation {angle}: PNG {a:?}, PDF {b:?}");
        }
    }
    let p = document(element(ElementKind::Shape {
        shape_type: ShapeType::Rectangle,
        fill: FillStyle::LinearGradient {
            start: rgb(255, 0, 0),
            end: rgb(0, 0, 255),
            vertical: false,
        },
        stroke_color: rgba(0, 0, 0, 0),
        stroke_width: 0.,
        corner_radius: 0.,
    }));
    let path = dir.join("gradient.pdf");
    fs::write(&path, bytes(&p)).unwrap();
    let prefix = dir.join("gradient-proof");
    assert!(Command::new("pdftoppm")
        .args(["-r", "96", "-f", "1", "-singlefile", "-png"])
        .arg(path)
        .arg(&prefix)
        .status()
        .unwrap()
        .success());
    let proof = image::open(prefix.with_extension("png"))
        .unwrap()
        .to_rgba8();
    let left = proof.get_pixel(40, 80);
    let right = proof.get_pixel(430, 80);
    assert!(left[0] > 220 && left[2] < 40 && right[2] > 220 && right[0] < 40);
}

#[test]
fn pdf_photo_adjustments_match_png() {
    let dir = workspace("image-adjustments");
    let mut p = document(photo());
    if let ElementKind::Image { adjustments, .. } = &mut p.pages[0].elements[0].kind {
        adjustments.sepia = true;
        adjustments.brightness = 0.15;
    }
    let path = dir.join("adjusted.pdf");
    fs::write(&path, bytes(&p)).unwrap();
    let prefix = dir.join("adjusted");
    assert!(Command::new("pdftoppm")
        .args(["-r", "96", "-f", "1", "-singlefile", "-png"])
        .arg(path)
        .arg(&prefix)
        .status()
        .unwrap()
        .success());
    let proof = image::open(prefix.with_extension("png"))
        .unwrap()
        .to_rgba8();
    let expected = render_project_page_at_dpi(&p, 0, 96, false).unwrap();
    for (x, y) in [(80, 70), (180, 90), (300, 100)] {
        let a = proof.get_pixel(x, y);
        let b = expected.get_pixel(x, y);
        assert!(
            (0..3).all(|c| a[c].abs_diff(b[c]) < 15),
            "photo adjustment mismatch at {x},{y}: {a:?} {b:?}"
        );
    }
}

#[test]
fn headless_preview_renders_rotated_gradient_and_table_without_editing() {
    let dir = workspace("ui-preview");
    let mut a = app(&dir);
    let mut e = element(ElementKind::Shape {
        shape_type: ShapeType::RoundedRect,
        fill: FillStyle::LinearGradient {
            start: rgb(255, 0, 0),
            end: rgb(0, 0, 255),
            vertical: true,
        },
        stroke_color: rgb(0, 0, 0),
        stroke_width: 1.,
        corner_radius: 10.,
    });
    e.rotation = 30.;
    a.project = document(e);
    let mut table = element(ElementKind::Table {
        rows: 1,
        cols: 1,
        cells: vec![vec!["Preview text".into()]],
        header_bg: rgb(240, 240, 240),
        row_bg: rgb(255, 255, 255),
        alt_row_bg: rgb(255, 255, 255),
        border_color: rgb(0, 0, 0),
        text_color: rgb(0, 0, 0),
        font_size: 16.,
    });
    table.id = 2;
    table.y = 200.;
    a.project.pages[0].elements.push(table);
    let ctx = Context::default();
    for _ in 0..3 {
        ui_frame(&mut a, &ctx, vec![], egui::Modifiers::NONE);
    }
    assert!(!a.dirty);
    assert!(a.canvas_state.texture_cache.len() >= 2);
}

#[test]
fn recovery_continues_during_pending_close_and_presentation() {
    let dir = workspace("idle-recovery");
    let mut a = app(&dir);
    a.insert_text_preset(
        "First",
        "Unsaved",
        FontFamily::Sans,
        20.,
        rgb(0, 0, 0),
        false,
        TextEffect::None,
    );
    a.pending_close = true;
    let ctx = Context::default();
    let input = |time| egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            Vec2::new(1300., 900.),
        )),
        time: Some(time),
        ..Default::default()
    };
    let _ = ctx.run(input(31.), |ctx| a.update_ui(ctx));
    let path = a.recovery_path_for_document(&a.project.document_id);
    assert!(path.exists());
    assert!(a.dirty);
    a.confirm_unsaved_cancel();
    a.insert_text_preset(
        "Second",
        "Also unsaved",
        FontFamily::Sans,
        20.,
        rgb(0, 0, 0),
        false,
        TextEffect::None,
    );
    a.presentation_mode = true;
    let _ = ctx.run(input(62.), |ctx| a.update_ui(ctx));
    let restored = load_project_json(&path).unwrap();
    assert_eq!(restored.pages[0].elements.len(), 2);
    assert!(a.dirty);
}

#[test]
fn replacing_an_updated_source_file_refreshes_the_photo() {
    let dir = workspace("photo-refresh");
    let mut a = app(&dir);
    a.project = document(photo());
    let path = dir.join("same-name.png");
    image::RgbaImage::from_pixel(20, 12, image::Rgba([0, 255, 0, 255]))
        .save(&path)
        .unwrap();
    a.replace_image_element_from_path(1, &path).unwrap();
    image::RgbaImage::from_pixel(20, 12, image::Rgba([0, 0, 255, 255]))
        .save(&path)
        .unwrap();
    a.replace_image_element_from_path(1, &path).unwrap();
    match &a.project.pages[0].elements[0].kind {
        ElementKind::Image { raw_rgba, .. } => assert_eq!(&raw_rgba[..4], &[0, 0, 255, 255]),
        _ => panic!(),
    };
    assert!(a.dirty);
    a.perform_undo();
    match &a.project.pages[0].elements[0].kind {
        ElementKind::Image { raw_rgba, .. } => assert_eq!(&raw_rgba[..4], &[0, 255, 0, 255]),
        _ => panic!(),
    };
}

#[test]
fn same_frame_typing_save_undo_and_open_do_not_lose_edits() {
    let dir = workspace("same-frame");
    let mut a = app(&dir);
    a.project = document(text("Before"));
    a.selected_id = Some(1);
    a.canvas_state.inline_edit_id = Some(1);
    a.current_project_path = Some(dir.join("saved.canvas.json"));
    let ctx = Context::default();
    for _ in 0..3 {
        ui_frame(&mut a, &ctx, vec![], egui::Modifiers::NONE);
    }
    let f = ui_frame(&mut a, &ctx, vec![], egui::Modifiers::NONE);
    let pos = f
        .shapes
        .iter()
        .find_map(|s| find_text(&s.shape, "Before"))
        .unwrap();
    ui_frame(
        &mut a,
        &ctx,
        vec![
            egui::Event::PointerMoved(pos),
            egui::Event::PointerButton {
                pos,
                button: egui::PointerButton::Primary,
                pressed: true,
                modifiers: Default::default(),
            },
        ],
        Default::default(),
    );
    ui_frame(
        &mut a,
        &ctx,
        vec![egui::Event::PointerButton {
            pos,
            button: egui::PointerButton::Primary,
            pressed: false,
            modifiers: Default::default(),
        }],
        Default::default(),
    );
    let mods = egui::Modifiers {
        ctrl: true,
        command: true,
        ..Default::default()
    };
    let key = |k| egui::Event::Key {
        key: k,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: mods,
    };
    ui_frame(
        &mut a,
        &ctx,
        vec![egui::Event::Text("X".into()), key(Key::S)],
        mods,
    );
    let get = |p: &Project| match &p.pages[0].elements[0].kind {
        ElementKind::Text { content, .. } => content.clone(),
        _ => panic!(),
    };
    let saved = get(&a.project);
    assert_ne!(saved, "Before");
    assert_eq!(
        get(&load_project_json(&dir.join("saved.canvas.json")).unwrap()),
        saved
    );
    assert!(!a.dirty);
    ui_frame(
        &mut a,
        &ctx,
        vec![egui::Event::Text("Y".into()), key(Key::Z)],
        mods,
    );
    assert_eq!(get(&a.project), saved);
    assert!(!a.dirty);
    let mut next = Project::default();
    next.title = "Other document".into();
    a.request_replace_document(next, None, "Opened".into());
    ui_frame(
        &mut a,
        &ctx,
        vec![egui::Event::Text("Z".into())],
        Default::default(),
    );
    assert_ne!(a.project.title, "Other document");
    assert!(a.dirty && a.pending_replace_project.is_some());
    a.confirm_unsaved_cancel();
    assert!(a.perform_undo());
    assert_eq!(get(&a.project), saved);
    assert!(!a.dirty);
    assert!(!a.history.can_redo() || a.perform_redo());
}
