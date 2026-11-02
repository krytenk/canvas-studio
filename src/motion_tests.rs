use super::*;
use serde_json::json;
fn project() -> Project {
    let mut p = Project::default();
    timeline::apply(
        &mut p,
        &json!({"action":"film_setup","width":1280,"height":720,"fps":30,"duration":900}),
    )
    .unwrap();
    timeline::apply(
        &mut p,
        &json!({"action":"film_caption","text":"Make it move.","start":30,"duration":180}),
    )
    .unwrap();
    p
}
#[test]
fn motion_split_move_and_trim_preserve_the_animation_clock() {
    let mut p = project();
    let id = p.production.timeline.clips[0].id.clone();
    timeline::apply(
        &mut p,
        &json!({"action":"film_motion","id":id,"preset":"rise"}),
    )
    .unwrap();
    let motion = p.production.timeline.clips[0].motion.clone().unwrap();
    timeline::apply(&mut p, &json!({"action":"film_split","id":id,"at":70})).unwrap();
    assert_eq!(
        p.production.timeline.clips[1].motion.as_ref(),
        Some(&motion)
    );
    let right = p.production.timeline.clips[1].id.clone();
    timeline::apply(&mut p, &json!({"action":"film_clip","id":right,"start":80})).unwrap();
    assert_eq!(
        p.production.timeline.clips[1]
            .motion
            .as_ref()
            .unwrap()
            .origin,
        40
    );
    timeline::apply(
        &mut p,
        &json!({"action":"film_clip","id":right,"start":85,"duration":135}),
    )
    .unwrap();
    assert_eq!(
        p.production.timeline.clips[1]
            .motion
            .as_ref()
            .unwrap()
            .origin,
        40
    );
}
#[test]
fn motion_keys_are_validated_and_inherited_timing_can_be_edited() {
    let mut p = project();
    let id = p.production.timeline.clips[0].id.clone();
    timeline::apply(
        &mut p,
        &json!({"action":"film_motion","id":id,"preset":"drift"}),
    )
    .unwrap();
    timeline::apply(&mut p, &json!({"action":"film_split","id":id,"at":90})).unwrap();
    let c = p.production.timeline.clips[1].clone();
    let m = c.motion.unwrap();
    timeline::apply(
        &mut p,
        &json!({"action":"film_motion","id":c.id,"keys":m.keys,"preserve_timing":true}),
    )
    .unwrap();
    assert_eq!(
        p.production.timeline.clips[1]
            .motion
            .as_ref()
            .unwrap()
            .origin,
        30
    );
    let mut candidate = p.clone();
    let mut keys = m.keys.clone();
    keys[1].frame = keys[0].frame;
    assert!(timeline::apply(
        &mut candidate,
        &json!({"action":"film_motion","id":c.id,"keys":keys,"preserve_timing":true})
    )
    .is_err());
    assert_ne!(candidate, p);
    timeline::apply(
        &mut p,
        &json!({"action":"film_motion","id":c.id,"preset":"none"}),
    )
    .unwrap();
    assert!(p.production.timeline.clips[1].motion.is_none());
}
#[test]
fn motion_portable_envelopes_refuse_silent_downgrades() {
    use std::io::Read;
    let mut p = project();
    let id = p.production.timeline.clips[0].id.clone();
    timeline::apply(
        &mut p,
        &json!({"action":"film_motion","id":id,"preset":"punch"}),
    )
    .unwrap();
    let folder = std::env::temp_dir().join(crate::model::generate_document_id());
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("motion.canvas");
    project_package::save(&p, &path).unwrap();
    assert_eq!(project_package::load(&path).unwrap(), p);
    let bytes = std::fs::read(&path).unwrap();
    let mut text = String::new();
    flate2::read::GzDecoder::new(&bytes[b"CANVAS-STUDIO-PACKAGE\n1\n".len()..])
        .read_to_string(&mut text)
        .unwrap();
    let v: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v["version"], 4);
    assert!(v["required_features"]
        .as_array()
        .unwrap()
        .iter()
        .any(|f| f == "motion-keyframes-v1"));
    let _ = std::fs::remove_dir_all(folder);
}
