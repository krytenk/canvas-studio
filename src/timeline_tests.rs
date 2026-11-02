use super::*;
use serde_json::json;
#[test]
fn frame_edits_split_without_drifting_source_or_audio_envelopes() {
    let mut p = Project::default();
    timeline::apply(
        &mut p,
        &json!({"action":"film_setup","width":1920,"height":1080,"fps":30,"duration":900}),
    )
    .unwrap();
    let a = timeline::MediaAsset {
        id: "a".repeat(64),
        name: "Source".into(),
        extension: "mp4".into(),
        kind: "video".into(),
        bytes: 100,
        duration_us: 30_000_000,
        width: 1920,
        height: 1080,
        has_audio: true,
        sample_rate: 48000,
        channels: 2,
    };
    p.production.timeline.assets.insert(a.id.clone(), a.clone());
    timeline::apply(
        &mut p,
        &json!({"action":"film_add","asset":a.id,"start":0,"duration":900}),
    )
    .unwrap();
    let id = p.production.timeline.clips[0].id.clone();
    timeline::apply(
        &mut p,
        &json!({"action":"film_clip","id":id,"fade_in":60,"fade_out":60,"gain_db":-6}),
    )
    .unwrap();
    timeline::apply(&mut p, &json!({"action":"film_split","id":id,"at":45})).unwrap();
    let t = &p.production.timeline;
    assert_eq!(t.clips.len(), 2);
    assert_eq!(t.clips[1].source_in_us, 1_500_000);
    assert_eq!(t.clips[1].start, 45);
    assert_eq!(t.clips[1].audio, t.clips[0].audio);
    assert_eq!(t.clips[0].duration + t.clips[1].duration, 900);
    let c = t.clips[1].id.clone();
    timeline::apply(&mut p, &json!({"action":"film_clip","id":c,"start":44})).unwrap_err();
}
#[test]
fn srt_preserves_unicode_multiline_and_frame_boundaries() {
    let mut p = Project::default();
    timeline::apply(
        &mut p,
        &json!({"action":"film_setup","width":1080,"height":1920,"fps":30,"duration":900}),
    )
    .unwrap();
    timeline::import_srt(&mut p,"1\n00:00:00,000 --> 00:00:02,000\nA new chapter\nZoë & André\n\n2\n00:00:02,000 --> 00:00:04,000\nMake it yours.\n").unwrap();
    assert_eq!(p.production.timeline.clips[0].duration, 60);
    assert!(timeline::srt(&p.production.timeline).contains("Zoë & André"));
    let before = p.clone();
    let mut candidate = p.clone();
    assert!(
        timeline::import_srt(&mut candidate, "1\n00:00:01,000 --> 00:00:03,000\nOverlap").is_err()
    );
    assert_eq!(p, before);
}

#[test]
fn timeline_validation_rejects_path_ids_and_wrapping_canvas_values() {
    let mut p = Project::default();
    assert!(timeline::apply(
        &mut p,
        &json!({"action":"film_setup","width":4294969216u64,"height":1080,"fps":30,"duration":900})
    )
    .is_err());
    timeline::apply(
        &mut p,
        &json!({"action":"film_setup","width":320,"height":180,"fps":30,"duration":240}),
    )
    .unwrap();
    timeline::apply(
        &mut p,
        &json!({"action":"film_caption","text":"Safe caption","start":0,"duration":30}),
    )
    .unwrap();
    let mut hostile = p.production.timeline.clone();
    hostile.clips[0].id = "../../escape".into();
    assert!(timeline::validate(&hostile).is_err());
    assert!(timeline::import_srt(
        &mut p,
        "1\n999999999999999:00:00,000 --> 999999999999999:00:01,000\nBad timestamp"
    )
    .is_err());
}
#[test]
fn native_film_commands_share_history_and_preserve_original_recovery() {
    let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(
        std::env::temp_dir().join(crate::model::generate_document_id()),
    ));
    app.film_command(
        json!({"action":"film_setup","width":1280,"height":720,"fps":30,"duration":900}),
    );
    assert!(app.project.production.timeline.enabled);
    assert!(app.perform_undo());
    assert!(!app.project.production.timeline.enabled);
    assert!(app.perform_redo());
    assert!(app.project.production.timeline.enabled);
}

#[test]
fn detaching_dialogue_creates_a_free_track_when_music_already_occupies_the_edit() {
    let mut p = Project::default();
    timeline::apply(
        &mut p,
        &json!({"action":"film_setup","width":320,"height":180,"fps":30,"duration":240}),
    )
    .unwrap();
    let a = timeline::MediaAsset {
        id: "b".repeat(64),
        name: "Dialogue".into(),
        extension: "mp4".into(),
        kind: "video".into(),
        bytes: 100,
        duration_us: 8_000_000,
        width: 320,
        height: 180,
        has_audio: true,
        sample_rate: 48000,
        channels: 2,
    };
    p.production.timeline.assets.insert(a.id.clone(), a.clone());
    timeline::apply(
        &mut p,
        &json!({"action":"film_add","asset":a.id,"start":0,"duration":240}),
    )
    .unwrap();
    let source = p.production.timeline.clips[0].id.clone();
    timeline::apply(
        &mut p,
        &json!({"action":"film_track","kind":"audio","name":"Music"}),
    )
    .unwrap();
    let music = p.production.timeline.tracks.last().unwrap().id.clone();
    timeline::apply(
        &mut p,
        &json!({"action":"film_add","asset":a.id,"track":music,"start":0,"duration":240}),
    )
    .unwrap();
    timeline::apply(&mut p, &json!({"action":"film_detach_audio","id":source})).unwrap();
    assert!(!p.production.timeline.clips[0].audio.enabled);
    assert_ne!(p.production.timeline.clips.last().unwrap().track, music);
    assert!(p.production.timeline.clips.last().unwrap().audio.enabled);
}
