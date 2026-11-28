use super::*;
use crate::reference_desk as desk;
use base64::Engine;
use std::io::{Read, Write};

struct Temp(PathBuf);
impl Temp {
    fn new() -> Self {
        let p =
            std::env::temp_dir().join(format!("canvas-research-{}", model::generate_document_id()));
        std::fs::create_dir_all(p.join("Transcripts")).unwrap();
        Self(p)
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}
const TXT:&str="A useful tutorial\nhttps://example.com/owned-video\nOn-device speech recognition (en)\n\n[0:01] First useful passage.\n[1:02] A second passage with café and <script> as plain text.\n";
const SRT:&str="1\n00:00:01,000 --> 00:00:02,500\nFirst caption\n\n2\n00:01:01,000 --> 00:01:02,000\nSecond caption\n";
fn source() -> desk::Source {
    desk::parse("source.txt", TXT.as_bytes()).unwrap()
}
#[test]
fn reference_desk_headers_timestamps_and_unicode_are_retained() {
    let s = source();
    assert_eq!(s.title, "A useful tutorial");
    assert_eq!(
        s.source_url.as_deref(),
        Some("https://example.com/owned-video")
    );
    assert_eq!(s.cues.len(), 2);
    assert_eq!(s.cues[1].start_ms, 62_000);
    assert!(s.cues[1].end_ms.is_none());
    assert!(s.text.contains("café"));
    assert!(s.text.contains("<script>"));
    let plain = desk::parse("notes.txt", b"Plain personal notes without a video header.").unwrap();
    assert!(plain.source_url.is_none());
    assert_eq!(plain.title, "notes");
}
#[test]
fn malformed_and_oversized_captions_fail_without_a_partial_source() {
    let s = desk::parse(
        "captions.srt",
        format!("\u{feff}{}", SRT.replace('\n', "\r\n")).as_bytes(),
    )
    .unwrap();
    assert_eq!(s.cues[0].end_ms, Some(2500));
    for text in [
        "1\n00:00:05,000 --> 00:00:04,000\nbad\n",
        "1\n24:00:00,000 --> 24:00:01,000\nbad\n",
        "not captions",
        "1\n00:00:00,000 --> 00:00:01,000\n",
    ] {
        assert!(desk::parse("bad.srt", text.as_bytes()).is_err());
    }
    assert!(desk::parse("binary.txt", b"a\0b").is_err());
    assert!(desk::parse("bad.txt", &[0xff, 0xfe]).is_err());
    assert!(desk::parse("large.txt", &vec![b'x'; 1_000_001]).is_err());
    let mut p = Project::default();
    let mut broken = desk::parse("captions.srt", SRT.as_bytes()).unwrap();
    broken.cues[0].end_ms = None;
    p.production.research.push(broken.clone());
    assert!(desk::validate(&p).is_err());
    assert!(desk::apply(
        &mut p,
        &serde_json::json!({"action":"research_captions","id":broken.id})
    )
    .is_err());
}
#[test]
fn library_is_scoped_and_rejects_traversal_and_symbolic_links() {
    let root = Temp::new();
    std::fs::write(root.0.join("Transcripts/source.txt"), TXT).unwrap();
    std::fs::write(root.0.join("private.txt"), "not a library file").unwrap();
    let entries = desk::list_at(&root.0).unwrap();
    assert_eq!(entries.len(), 1);
    assert!(desk::preview_at(&root.0, &entries[0].id).is_ok());
    for path in [
        "../private.txt",
        "Transcripts/../../private.txt",
        "/tmp/private.txt",
        "private.txt",
        "Transcripts/other.exe",
    ] {
        let id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(path);
        assert!(desk::path_at(&root.0, &id).is_err());
    }
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(
            root.0.join("private.txt"),
            root.0.join("Transcripts/link.txt"),
        )
        .unwrap();
        let id = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode("Transcripts/link.txt");
        assert!(desk::path_at(&root.0, &id).is_err());
        assert_eq!(desk::list_at(&root.0).unwrap().len(), 1);
    }
    assert_eq!(
        std::fs::read_to_string(root.0.join("private.txt")).unwrap(),
        "not a library file"
    );
}
#[test]
fn import_snapshots_deduplicate_and_detect_a_changed_preview() {
    let root = Temp::new();
    let file = root.0.join("Transcripts/source.txt");
    std::fs::write(&file, TXT).unwrap();
    let entry = desk::list_at(&root.0).unwrap().remove(0);
    let preview = desk::preview_at(&root.0, &entry.id).unwrap();
    let mut p = Project::default();
    let id = desk::import_library_at(&mut p, &root.0, &entry.id, &preview.fingerprint).unwrap();
    assert_eq!(
        id,
        desk::import_library_at(&mut p, &root.0, &entry.id, &preview.fingerprint).unwrap()
    );
    assert_eq!(p.production.research.len(), 1);
    std::fs::write(&file, "A changed transcript").unwrap();
    let before = p.clone();
    assert!(
        desk::import_library_at(&mut p, &root.0, &entry.id, &preview.fingerprint)
            .unwrap_err()
            .contains("changed")
    );
    assert_eq!(p, before);
    assert!(p.production.research[0]
        .text
        .contains("First useful passage"));
    assert_eq!(
        std::fs::read_to_string(&file).unwrap(),
        "A changed transcript"
    );
}
#[test]
fn selected_passages_become_cited_notes_without_replacing_existing_notes() {
    let mut p = Project::default();
    p.pages[0].notes = "My own note".into();
    let id = desk::import_source(&mut p, source()).unwrap();
    desk::apply(
        &mut p,
        &serde_json::json!({"action":"research_note","id":id,"cue":1,"page":0}),
    )
    .unwrap();
    assert!(p.pages[0].notes.starts_with("My own note\n\n"));
    assert!(p.pages[0].notes.contains("https://example.com/owned-video"));
    assert!(p.pages[0].notes.contains("00:01:02,000"));
    assert!(!p.pages[0].notes.contains("First useful passage"));
    let before = p.clone();
    assert!(desk::apply(
        &mut p,
        &serde_json::json!({"action":"research_note","id":id,"cue":999,"page":0})
    )
    .is_err());
    assert_eq!(p, before);
}
#[test]
fn caption_import_extends_the_movie_and_rejects_overlaps_atomically() {
    let mut p = Project::default();
    let id =
        desk::import_source(&mut p, desk::parse("captions.srt", SRT.as_bytes()).unwrap()).unwrap();
    let op = serde_json::json!({"action":"research_captions","id":id});
    desk::apply(&mut p, &op).unwrap();
    assert_eq!(p.production.timeline.clips.len(), 2);
    assert_eq!(p.production.timeline.duration, 1860);
    crate::timeline::validate(&p.production.timeline).unwrap();
    let before = p.clone();
    assert!(desk::apply(&mut p, &op).is_err());
    assert_eq!(p, before);
    desk::apply(
        &mut p,
        &serde_json::json!({"action":"research_remove","id":id}),
    )
    .unwrap();
    assert!(p.production.research.is_empty());
    assert_eq!(p.production.timeline.clips.len(), 2);
}
#[test]
fn research_is_portable_and_silent_downgrades_are_rejected() {
    let root = Temp::new();
    let mut p = Project::default();
    desk::import_source(&mut p, source()).unwrap();
    assert!(!crate::workflows::is_blank(&p));
    let path = root.0.join("research.canvas");
    crate::project_package::save(&p, &path).unwrap();
    let restored = load_project_json(&path).unwrap();
    assert_eq!(restored.production.research, p.production.research);
    let bytes = std::fs::read(&path).unwrap();
    let mut data = Vec::new();
    flate2::read::GzDecoder::new(&bytes[b"CANVAS-STUDIO-PACKAGE\n1\n".len()..])
        .read_to_end(&mut data)
        .unwrap();
    let mut envelope: serde_json::Value = serde_json::from_slice(&data).unwrap();
    assert_eq!(envelope["version"], 6);
    envelope["version"] = serde_json::json!(5);
    envelope["required_features"] = serde_json::json!(["product-mockups-v1"]);
    let mut gz = flate2::write::GzEncoder::new(
        b"CANVAS-STUDIO-PACKAGE\n1\n".to_vec(),
        flate2::Compression::fast(),
    );
    gz.write_all(&serde_json::to_vec(&envelope).unwrap())
        .unwrap();
    std::fs::write(&path, gz.finish().unwrap()).unwrap();
    assert!(load_project_json(&path)
        .unwrap_err()
        .contains("research-sources"));
    let old: crate::production::ProductionSettings = serde_json::from_str("{}").unwrap();
    assert!(old.research.is_empty());
}
