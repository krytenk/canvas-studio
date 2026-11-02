use super::*;
use std::{fs, process::Command};

fn text_element(project: &mut Project, content: &str) -> CanvasElement {
    let mut text = build_template_project("festival_poster")
        .pages
        .remove(0)
        .elements
        .remove(2);
    text.id = project.alloc_id();
    text.x = 60.;
    text.y = 80.;
    text.width = 620.;
    text.height = 120.;
    if let ElementKind::Text {
        content: text_content,
        font_size,
        color,
        bold,
        align,
        ..
    } = &mut text.kind
    {
        *text_content = content.into();
        *font_size = 28.;
        *color = rgb(25, 30, 40);
        *bold = false;
        *align = TextAlign::Left;
    }
    text
}

fn collection() -> Project {
    let mut project = Project::default();
    project.title = "Gathering collection".into();
    project.pages.clear();
    project.production.fields.insert(
        "guest_name".into(),
        production::DataField {
            value: "Avery & Jordan".into(),
            required: true,
            ..Default::default()
        },
    );
    project.production.fields.insert(
        "event_date".into(),
        production::DataField {
            value: "14 June 2027".into(),
            required: true,
            ..Default::default()
        },
    );
    let sample = text_element(&mut project, "Sample");
    let mut style = production::style_from(&sample).unwrap();
    style.size = 26.;
    style.family = FontFamily::DisplaySerif;
    project
        .production
        .styles
        .insert("collection_heading".into(), style);
    for title in [
        "Invitation",
        "Programme",
        "Welcome",
        "Menu",
        "Keepsake newspaper",
    ] {
        let mut page = CanvasPage::new(project.alloc_id(), title);
        let mut text = text_element(
            &mut project,
            &format!("{title}\n{{{{guest_name}}}}\n{{{{event_date}}}}"),
        );
        text.height = 240.;
        text.style_id = Some("collection_heading".into());
        page.elements.push(text);
        project.pages.push(page);
    }
    project
}

#[test]
fn one_brief_and_shared_style_resolve_the_whole_collection_without_mutating_the_master() {
    let mut master = collection();
    let original = master.clone();
    let first = production::resolve(&master, true).unwrap();
    assert_eq!(first.pages.len(), 5);
    for page in &first.pages {
        if let ElementKind::Text {
            content,
            font_size,
            font_family,
            ..
        } = &page.elements[0].kind
        {
            assert!(content.contains("Avery & Jordan"));
            assert!(!content.contains("{{"));
            assert_eq!(*font_size, 26.);
            assert_eq!(*font_family, FontFamily::DisplaySerif);
        }
    }
    assert!(master == original);
    master
        .production
        .fields
        .get_mut("guest_name")
        .unwrap()
        .value = "René & Zoë".into();
    master
        .production
        .styles
        .get_mut("collection_heading")
        .unwrap()
        .color = rgb(110, 35, 70);
    let root = std::env::current_dir().unwrap().join("qa/collection-proof");
    fs::create_dir_all(&root).unwrap();
    let path = root.join("collection.canvas");
    save_project_json(&master, &path).unwrap();
    let master = load_project_json(&path).unwrap();
    let pdf = root.join("collection.pdf");
    export_project_to_pdf(&master, &Default::default(), &pdf).unwrap();
    let text = Command::new("pdftotext")
        .arg(&pdf)
        .arg("-")
        .output()
        .unwrap();
    let text = String::from_utf8(text.stdout).unwrap();
    assert_eq!(text.matches("René & Zoë").count(), 5);
    assert_eq!(text.matches("14 June 2027").count(), 5);
    let svg = crate::exporter::export_project_page_to_svg(&master, 0, false).unwrap();
    assert!(!svg.contains("{{guest_name}}"));
    render_project_page_at_dpi(&master, 0, 96, false)
        .unwrap()
        .save(root.join("invitation.png"))
        .unwrap();
    assert!(export_project_to_html_deck(&master)
        .unwrap()
        .contains("Ren"));
}

#[test]
fn unfinished_required_fields_can_be_saved_as_masters_but_cannot_be_delivered() {
    let mut master = collection();
    master
        .production
        .fields
        .get_mut("guest_name")
        .unwrap()
        .value
        .clear();
    assert!(production::resolve(&master, true).is_err());
    assert!(run_export_preflight(&master, 96).has_errors());
    let root = std::env::temp_dir().join(model::generate_document_id());
    let entry = template_library::save(&root, "Unfilled master", "collection", &master).unwrap();
    let copy = template_library::open_copy(&root, &entry.id).unwrap();
    assert!(production::resolve(&copy, true).is_err());
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn csv_quoting_unicode_and_batch_failures_are_reported_without_changing_the_master() {
    let parsed = batch_production::parse_csv(
        "\u{feff}guest_name,event_date\r\n\"René, \"\"RJ\"\"\",2027\r\n\"Two\nLines\",2028\r\n",
    )
    .unwrap();
    assert_eq!(parsed[1][0], "René, \"RJ\"");
    assert_eq!(parsed[2][0], "Two\nLines");
    assert!(batch_production::parse_csv("name\n\"unfinished").is_err());
    let mut master = collection();
    master.pages.truncate(1);
    let original = master.clone();
    let root = std::env::current_dir().unwrap().join("qa/csv-proof");
    fs::create_dir_all(&root).unwrap();
    let csv = root.join("guests.csv");
    let mut data = "guest_name,event_date\n".to_string();
    for index in 1..=50 {
        data.push_str(&format!("Guest {index},14 June 2027\n"));
    }
    data.push_str(",14 June 2027\n");
    data.push_str(&format!("{},14 June 2027\n", "Overlong".repeat(150)));
    fs::write(&csv, data).unwrap();
    let report = batch_production::export_csv(&master, &csv, &root, false).unwrap();
    assert_eq!(report.records.iter().filter(|r| r.success).count(), 50);
    assert_eq!(report.records.iter().filter(|r| !r.success).count(), 2);
    assert!(master == original);
    assert!(report.folder.join("report.json").is_file());
    let first = report.folder.join(format!(
        "{}.pdf",
        report.records[0].output.as_ref().unwrap()
    ));
    let text = Command::new("pdftotext")
        .arg(first)
        .arg("-")
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&text.stdout).contains("Guest 1"));
}

#[test]
fn a_photo_field_updates_every_bound_frame_and_remains_portable() {
    let root = std::env::temp_dir().join(model::generate_document_id());
    fs::create_dir_all(&root).unwrap();
    let path = root.join("portrait.png");
    image::RgbaImage::from_pixel(600, 600, image::Rgba([20, 90, 160, 255]))
        .save(&path)
        .unwrap();
    let mut master = Project::default();
    let stock = generate_builtin_stock_assets();
    let mut photo = make_stock_image_element(master.alloc_id(), &stock[0], 50., 100.);
    photo.width = 180.;
    photo.height = 180.;
    photo.binding = Some("portrait".into());
    let mut second = photo.clone();
    second.id = master.alloc_id();
    second.x = 300.;
    master.pages[0].elements = vec![photo, second];
    let mut field = production::DataField {
        required: true,
        ..Default::default()
    };
    production::attach_image(&mut field, &path).unwrap();
    master.production.fields.insert("portrait".into(), field);
    fs::remove_file(path).unwrap();
    let package = root.join("photo.canvas");
    save_project_json(&master, &package).unwrap();
    let reopened = load_project_json(&package).unwrap();
    let resolved = production::resolve(&reopened, true).unwrap();
    for element in &resolved.pages[0].elements {
        if let ElementKind::Image {
            width_px, raw_rgba, ..
        } = &element.kind
        {
            assert_eq!(*width_px, 600);
            assert_eq!(&raw_rgba[..4], &[20, 90, 160, 255]);
        }
    }
    assert!(!run_export_preflight(&reopened, 300).has_errors());
    let mut optional = master.clone();
    let field = optional.production.fields.get_mut("portrait").unwrap();
    field.required = false;
    field.image_png = None;
    assert!(production::resolve(&optional, true).unwrap().pages[0]
        .elements
        .iter()
        .all(|e| !e.visible));
    fs::remove_dir_all(root).unwrap();
}
