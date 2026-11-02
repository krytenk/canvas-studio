use crate::{layout_commands, mockups, model::Project, project_package};
use serde_json::json;
#[test]
fn product_scene_validation_and_identity() {
    let mut p = Project::default();
    let cover = p.pages[0].id;
    layout_commands::apply(
        &mut p,
        &json!({"action":"mockup_create","kind":"book","page":0}),
    )
    .unwrap();
    let id = p.production.mockups[0].id.clone();
    assert_eq!(p.production.mockups[0].page_id, cover.to_string());
    let before = p.clone();
    assert!(layout_commands::apply(
        &mut p,
        &json!({"action":"mockup_update","id":id,"depth":-1})
    )
    .is_err());
    assert_eq!(p.production.mockups, before.production.mockups);
    assert!(layout_commands::apply(
        &mut p,
        &json!({"action":"mockup_update","id":id,"background":"url(https://invalid)"})
    )
    .is_err());
    assert!(mockups::cover(&p, &p.production.mockups[0], false).is_ok());
    p.pages[0].id += 1;
    assert!(mockups::cover(&p, &p.production.mockups[0], false).is_err());
}
#[test]
fn portable_product_scene_roundtrip() {
    let mut p = Project::default();
    mockups::apply(
        &mut p,
        &json!({"action":"mockup_create","kind":"box","page":0}),
    )
    .unwrap();
    let path = std::env::temp_dir().join(format!(
        "mockup-{}.canvas",
        crate::model::generate_document_id()
    ));
    project_package::save(&p, &path).unwrap();
    let copy = project_package::load(&path).unwrap();
    assert_eq!(p.production.mockups, copy.production.mockups);
    std::fs::remove_file(path).unwrap();
}
#[test]
fn scenes_are_bounded() {
    let mut p = Project::default();
    for _ in 0..24 {
        mockups::apply(
            &mut p,
            &json!({"action":"mockup_create","kind":"cards","page":0}),
        )
        .unwrap();
    }
    assert!(mockups::apply(&mut p, &json!({"action":"mockup_create","page":0})).is_err());
    let id = &p.production.mockups[0].id.clone();
    assert!(mockups::apply(
        &mut p,
        &json!({"action":"mockup_update","id":id,"output_width":99999})
    )
    .is_err());
}
