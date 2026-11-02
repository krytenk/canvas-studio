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

#[test]
fn unicode_character_styles_wrap_and_follow_edits() {
    use crate::model::{TextFormatting, TextRun};
    let mut format = TextFormatting {
        space_before: 4.,
        space_after: 12.,
        first_line_indent: 14.,
        inset_left: 8.,
        inset_right: 8.,
        inset_top: 6.,
        inset_bottom: 6.,
        ..Default::default()
    };
    format.runs.push(TextRun {
        start: 3,
        end: 4,
        bold: Some(true),
        underline: Some(true),
        ..Default::default()
    });
    let layout = crate::paragraph_layout::layout_formatted(
        "groß café\nAnother paragraph",
        FontFamily::Sans,
        20.,
        false,
        false,
        true,
        TextAlign::Left,
        0.,
        1.4,
        ListStyle::None,
        160.,
        350.,
        &format,
    );
    let glyphs: Vec<_> = layout.lines.iter().flat_map(|l| &l.glyphs).collect();
    let bold: Vec<_> = glyphs
        .iter()
        .filter(|g| g.face_id.is_bold())
        .map(|g| g.ch)
        .collect();
    assert_eq!(bold, vec!['S', 'S']);
    assert!(glyphs
        .iter()
        .filter(|g| g.face_id.is_bold())
        .all(|g| g.underline_override == Some(true)));
    assert!(!layout.is_overflowing());
    assert!((layout.lines[0].x_offset - 22.).abs() < 0.01);
    let mut edited = TextFormatting::default();
    edited.runs.push(TextRun {
        start: 5,
        end: 9,
        bold: Some(true),
        ..Default::default()
    });
    edited.reconcile_edit("Café bold", "Dear Café bold");
    assert_eq!((edited.runs[0].start, edited.runs[0].end), (10, 14));
    edited.reconcile_edit("Dear Café bold", "Dear Café ");
    assert!(edited.runs.is_empty());
}

#[test]
fn rich_paragraph_proof_round_trips_and_matches_independent_pdf_layout() {
    use crate::model::{TextFormatting, TextRun};
    let root = std::env::current_dir().unwrap().join("qa/typography-proof");
    fs::create_dir_all(&root).unwrap();
    let mut project = font_project();
    let content="A thoughtfully designed gathering brings people together. The invitation sets the tone, while clear information helps every guest feel welcome.\nGood typography creates a comfortable rhythm. Paragraph spacing, a considered first-line indent and generous frame insets give the words room to breathe.\nKeep the editable master, make a fresh working copy, and proof the complete collection before printing.";
    let element = &mut project.pages[0].elements[0];
    element.y = 150.;
    element.width = 650.;
    element.height = 800.;
    if let ElementKind::Text {
        content: text,
        font_family,
        font_size,
        bold,
        align,
        line_height,
        ..
    } = &mut element.kind
    {
        *text = content.into();
        *font_family = FontFamily::Sans;
        *font_size = 23.;
        *bold = false;
        *align = TextAlign::Left;
        *line_height = 1.5;
    }
    element.text_format = TextFormatting {
        space_after: 18.,
        first_line_indent: 20.,
        inset_left: 16.,
        inset_right: 16.,
        inset_top: 12.,
        inset_bottom: 12.,
        ..Default::default()
    };
    let start = content[..content.find("thoughtfully").unwrap()]
        .chars()
        .count();
    element.text_format.runs.push(TextRun {
        start,
        end: start + 12,
        bold: Some(true),
        ..Default::default()
    });
    let start = content[..content.find("comfortable rhythm").unwrap()]
        .chars()
        .count();
    element.text_format.runs.push(TextRun {
        start,
        end: start + 18,
        italic: Some(true),
        underline: Some(true),
        ..Default::default()
    });
    let start = content[..content.find("editable master").unwrap()]
        .chars()
        .count();
    element.text_format.runs.push(TextRun {
        start,
        end: start + 15,
        font_family: Some(FontFamily::Imported(project.imported_fonts[0].id)),
        ..Default::default()
    });
    let mut heading = element.clone();
    heading.id = project.alloc_id();
    heading.y = 55.;
    heading.height = 80.;
    heading.text_format = Default::default();
    if let ElementKind::Text {
        content,
        font_family,
        font_size,
        ..
    } = &mut heading.kind
    {
        *content = "A thoughtful gathering".into();
        *font_family = FontFamily::DisplaySerif;
        *font_size = 42.;
    }
    project.pages[0].elements.push(heading);
    let path = root.join("paragraphs.canvas.json");
    save_project_json(&project, &path).unwrap();
    let project = load_project_json(&path).unwrap();
    assert!(!run_export_preflight(&project, 150).has_errors());
    let png = render_project_page_at_dpi(&project, 0, 150, false).unwrap();
    png.save(root.join("paragraphs.png")).unwrap();
    let pdf = root.join("paragraphs.pdf");
    export_project_to_pdf(
        &project,
        &PdfExportOptions {
            target_image_dpi: 150,
            ..Default::default()
        },
        &pdf,
    )
    .unwrap();
    fs::write(
        root.join("paragraphs.svg"),
        export_page_to_svg(&project.pages[0], project.width, project.height, false).unwrap(),
    )
    .unwrap();
    let text = Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .unwrap();
    let extracted = String::from_utf8(text.stdout).unwrap();
    for phrase in ["thoughtfully", "comfortable rhythm", "editable master"] {
        assert!(extracted.contains(phrase), "{extracted}");
    }
    let render = Command::new("pdftoppm")
        .args(["-r", "150", "-singlefile", "-png"])
        .arg(&pdf)
        .arg(root.join("paragraphs-pdf"))
        .output()
        .unwrap();
    assert!(render.status.success());
    let independent = image::open(root.join("paragraphs-pdf.png"))
        .unwrap()
        .to_rgba8();
    let ink_bounds = |img: &image::RgbaImage| {
        let mut bounds = (u32::MAX, u32::MAX, 0, 0);
        for (x, y, pixel) in img.enumerate_pixels() {
            if pixel.0[..3].iter().map(|c| *c as u32).sum::<u32>() < 500 {
                bounds.0 = bounds.0.min(x);
                bounds.1 = bounds.1.min(y);
                bounds.2 = bounds.2.max(x);
                bounds.3 = bounds.3.max(y);
            }
        }
        bounds
    };
    let a = ink_bounds(&png);
    let b = ink_bounds(&independent);
    for (a, b) in [a.0, a.1, a.2, a.3].into_iter().zip([b.0, b.1, b.2, b.3]) {
        assert!(
            a.abs_diff(b) <= 6,
            "independent PDF and raster text bounds differ: {a} vs {b}"
        );
    }
}
