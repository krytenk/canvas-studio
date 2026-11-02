use super::*;
use serde_json::json;
#[test]
fn campaign_variants_preserve_composition_shared_copy_colours_assets_and_package_guards() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("qa/campaign");
    std::fs::create_dir_all(&root).unwrap();
    let mut p = campaign::starter();
    campaign::validate(&p).unwrap();
    assert_eq!(p.production.studio.variants.len(), 3);
    assert_eq!(p.production.studio.components.len(), 1);
    layout_commands::apply(
        &mut p,
        &json!({"action":"brand_token","key":"accent","color":"#e8a892"}),
    )
    .unwrap();
    layout_commands::apply(
        &mut p,
        &json!({"action":"field_define","key":"brand","value":"KINDFOLD STUDIO"}),
    )
    .unwrap();
    let names = ["Instagram post", "Story", "YouTube thumbnail", "Print A4"];
    for (i, name) in names.iter().enumerate() {
        campaign::activate(&mut p, name).unwrap();
        let report = run_export_preflight(&p, 300);
        assert!(
            !report.has_errors() && !report.has_overflows(),
            "{name}: {:?}",
            report.issues
        );
        let resolved = production::resolve(&p, true).unwrap();
        assert!(resolved.pages[0].elements.iter().any(
            |e| matches!(&e.kind,ElementKind::Text{content,..} if content=="KINDFOLD STUDIO")
        ));
        let image = exporter::render_project_page_at_dpi(&p, 0, 96, false).unwrap();
        image.save(root.join(format!("variant-{i}.png"))).unwrap();
        let before = image.clone();
        project_package::save(&p, &root.join("campaign.canvas")).unwrap();
        let mut loaded = load_project_json(&root.join("campaign.canvas")).unwrap();
        assert_eq!(
            loaded.production.studio.active_variant.as_deref(),
            Some(*name)
        );
        assert_eq!(
            exporter::render_project_page_at_dpi(&loaded, 0, 96, false).unwrap(),
            before
        );
        campaign::activate(&mut loaded, names[(i + 1) % 4]).unwrap();
        campaign::activate(&mut loaded, name).unwrap();
        assert_eq!(
            exporter::render_project_page_at_dpi(&loaded, 0, 96, false).unwrap(),
            before
        );
        exporter::save_project_json(&p, &root.join("legacy-name.canvas.json")).unwrap();
        assert!(project_package::is_package(
            &root.join("legacy-name.canvas.json")
        ));
        let pdf = model::pdf_export::export_project_to_pdf_bytes(&p, &PdfExportOptions::default())
            .unwrap();
        std::fs::write(root.join(format!("variant-{i}.pdf")), pdf).unwrap();
    }
    // Images and imported fonts living only in an inactive layout/component survive cold loading.
    campaign::activate(&mut p, "Story").unwrap();
    let font =
        imported_fonts::parse(include_bytes!("../assets/fonts/NotoSerif-Regular.ttf").to_vec())
            .unwrap();
    p.imported_fonts.push(font);
    let assets = templates::generate_builtin_stock_assets();
    let e = templates::make_stock_image_element(p.alloc_id(), &assets[0], 80., 80.);
    let id = e.id;
    p.pages[0].elements.push(e);
    campaign::capture_component(&mut p, 0, &[id], "photo", "Photo group").unwrap();
    campaign::activate(&mut p, "Print A4").unwrap();
    project_package::save(&p, &root.join("campaign-assets.canvas")).unwrap();
    let mut loaded = load_project_json(&root.join("campaign-assets.canvas")).unwrap();
    campaign::activate(&mut loaded, "Story").unwrap();
    assert!(loaded.pages[0]
        .elements
        .iter()
        .any(|e| matches!(&e.kind,ElementKind::Image{raw_rgba,..} if !raw_rgba.is_empty())));
    campaign::insert_component(&mut loaded, 0, "photo", 40., 40., Some(100.)).unwrap();
    assert_eq!(loaded.imported_fonts.len(), 1);
}
#[test]
fn shared_selection_commands_preserve_groups_and_reject_locked_or_partial_updates() {
    let mut p = campaign::starter();
    let cta = p.pages[0]
        .elements
        .iter()
        .filter(|e| e.name.starts_with("CTA"))
        .map(|e| e.id)
        .collect::<Vec<_>>();
    let original = document_ops::selected_elements(&p.pages[0], &cta, None);
    let bounds = document_ops::bounds(&original).unwrap();
    layout_commands::apply(
        &mut p,
        &json!({"action":"selection_move","page":0,"ids":[cta[0].to_string()],"dx":-20,"dy":-30}),
    )
    .unwrap();
    for e in &original {
        let moved = p.pages[0].elements.iter().find(|a| a.id == e.id).unwrap();
        assert_eq!(moved.x, e.x - 20.);
        assert_eq!(moved.y, e.y - 30.);
    }
    let b = document_ops::selection_bounds(&p.pages[0], &cta).unwrap();
    layout_commands::apply(&mut p,&json!({"action":"selection_transform","page":0,"ids":cta.iter().map(u64::to_string).collect::<Vec<_>>(),"x":b.left(),"y":b.top(),"width":b.width()*0.8,"height":b.height()*0.8})).unwrap();
    assert!(
        (document_ops::selection_bounds(&p.pages[0], &cta)
            .unwrap()
            .width()
            - bounds.width() * 0.8)
            .abs()
            < 0.01
    );
    p.pages[0]
        .elements
        .iter_mut()
        .find(|e| e.id == cta[0])
        .unwrap()
        .locked = true;
    let before = p.clone();
    assert!(layout_commands::apply(
        &mut p,
        &json!({"action":"selection_move","page":0,"ids":[cta[1].to_string()],"dx":10,"dy":0})
    )
    .is_err());
    assert_eq!(p, before);
    assert!(layout_commands::apply(
        &mut p,
        &json!({"action":"selection_move","page":0,"ids":["999999"],"dx":10,"dy":0})
    )
    .is_err());
}

#[test]
fn reusable_groups_and_scaling_materialize_linked_type_without_changing_shared_copy() {
    let mut p = campaign::starter();
    let id = p.pages[0]
        .elements
        .iter()
        .find(|e| e.name == "Campaign headline")
        .unwrap()
        .id;
    let mut style =
        production::style_from(p.pages[0].elements.iter().find(|e| e.id == id).unwrap()).unwrap();
    style.size = 80.;
    p.production.styles.insert("headline".into(), style);
    p.pages[0]
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .unwrap()
        .style_id = Some("headline".into());
    campaign::capture_component(&mut p, 0, &[id], "headline_copy", "Headline copy").unwrap();
    let c = &p.production.studio.components["headline_copy"];
    assert!(c.elements[0].style_id.is_none());
    assert!(
        matches!(&c.elements[0].kind,ElementKind::Text{content,font_size,..} if content=="{{headline}}" && *font_size==80.)
    );
    let b = document_ops::selection_bounds(&p.pages[0], &[id]).unwrap();
    layout_commands::apply(&mut p,&json!({"action":"selection_transform","page":0,"ids":[id.to_string()],"x":b.left(),"y":b.top(),"width":b.width()*0.5,"height":b.height()*0.5})).unwrap();
    let e = p.pages[0].elements.iter().find(|e| e.id == id).unwrap();
    assert!(e.style_id.is_none());
    assert!(
        matches!(&e.kind,ElementKind::Text{content,font_size,..} if content=="{{headline}}" && *font_size==40.)
    );
    assert_eq!(p.production.styles["headline"].size, 80.);
}

#[test]
fn desktop_local_colour_overrides_detach_only_the_edited_target() {
    let before = campaign::starter();
    let mut after = before.clone();
    let id = after.pages[0]
        .elements
        .iter()
        .find(|e| e.name == "Campaign headline")
        .unwrap()
        .id;
    if let ElementKind::Text { color, .. } = &mut after.pages[0]
        .elements
        .iter_mut()
        .find(|e| e.id == id)
        .unwrap()
        .kind
    {
        *color = rgb(160, 40, 60);
    }
    campaign::reconcile_local_colors(&before, &mut after);
    assert!(!after.production.studio.colors[&id].contains_key("text"));
    assert_eq!(
        after.production.studio.tokens,
        before.production.studio.tokens
    );
    let mut shared = before.clone();
    shared
        .production
        .studio
        .tokens
        .insert("ink".into(), rgb(30, 50, 80));
    campaign::reconcile_local_colors(&before, &mut shared);
    assert_eq!(
        shared.production.studio.colors,
        before.production.studio.colors
    );
}
