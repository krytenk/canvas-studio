//! Personal Reference Desk output bridge. No downloads, directory mutation or remote calls.
use crate::model::Project;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    fs,
    io::{Read, Write},
    path::{Component, Path, PathBuf},
    process::{Command, Stdio},
};

const TEXT_LIMIT: usize = 1_000_000;
const MEDIA_LIMIT: u64 = 128 * 1024 * 1024;
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Cue {
    pub start_ms: u64,
    pub end_ms: Option<u64>,
    pub text: String,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub title: String,
    pub filename: String,
    pub kind: String,
    pub source_url: Option<String>,
    pub method: Option<String>,
    pub fingerprint: String,
    pub text: String,
    pub cues: Vec<Cue>,
    pub media_asset: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub bytes: u64,
    pub modified: u64,
}

pub fn root() -> PathBuf {
    if let Some(path) = std::env::var_os("CANVAS_STUDIO_REFERENCE_ROOT") {
        return PathBuf::from(path);
    }
    let home = PathBuf::from(std::env::var_os("HOME").unwrap_or_default());
    let fallback = home.join("Reference Desk");
    let config = home.join(".config/reference-desk/config.json");
    let configured = fs::read(&config)
        .ok()
        .filter(|b| b.len() < 65536)
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["output_dir"].as_str().map(str::to_owned));
    let Some(raw) = configured else {
        return fallback;
    };
    let path = if let Some(rest) = raw.strip_prefix("~/") {
        home.join(rest)
    } else {
        PathBuf::from(raw)
    };
    let path = if path.is_absolute() {
        path
    } else {
        home.join(path)
    };
    match path.canonicalize() {
        Ok(p) if p.starts_with(&home) || p.starts_with("/tmp") => p,
        _ => fallback,
    }
}
fn kind(folder: &str, name: &str) -> Option<&'static str> {
    let ext = Path::new(name).extension()?.to_str()?.to_ascii_lowercase();
    match (folder, ext.as_str()) {
        ("Transcripts", "txt") => Some("txt"),
        ("Transcripts", "srt") => Some("srt"),
        ("Audio", "mp3" | "wav" | "m4a" | "ogg" | "flac") => Some("audio"),
        ("Video", "mp4" | "mov" | "webm" | "mkv" | "m4v") => Some("video"),
        _ => None,
    }
}
pub fn list_at(root: &Path) -> Result<Vec<Entry>, String> {
    if !root.exists() {
        return Ok(vec![]);
    }
    let mut entries = vec![];
    for folder in ["Transcripts", "Audio", "Video"] {
        let dir = root.join(folder);
        if !dir.exists() {
            continue;
        }
        if fs::symlink_metadata(&dir)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            continue;
        }
        for entry in fs::read_dir(dir).map_err(|e| e.to_string())?.take(10_000) {
            let entry = entry.map_err(|e| e.to_string())?;
            if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
                continue;
            }
            let Some(name) = entry.file_name().to_str().map(str::to_owned) else {
                continue;
            };
            let Some(kind) = kind(folder, &name) else {
                continue;
            };
            let meta = entry.metadata().map_err(|e| e.to_string())?;
            entries.push(Entry {
                id: base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .encode(format!("{folder}/{name}")),
                name,
                kind: kind.into(),
                bytes: meta.len(),
                modified: meta
                    .modified()
                    .ok()
                    .and_then(|x| x.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|x| x.as_secs())
                    .unwrap_or(0),
            });
        }
    }
    entries.sort_by(|a, b| b.modified.cmp(&a.modified).then(a.name.cmp(&b.name)));
    entries.truncate(250);
    Ok(entries)
}
pub fn path_at(root: &Path, id: &str) -> Result<PathBuf, String> {
    if id.len() > 2048 {
        return Err("Invalid Reference Desk file".into());
    }
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(id)
        .map_err(|_| "Invalid Reference Desk file")?;
    let name = std::str::from_utf8(&bytes).map_err(|_| "Invalid filename")?;
    let parts = Path::new(name).components().collect::<Vec<_>>();
    if parts.len() != 2
        || parts.iter().any(|p| !matches!(p, Component::Normal(_)))
        || name.contains('\\')
    {
        return Err("Choose a file from the Reference Desk library".into());
    }
    let folder = parts[0].as_os_str().to_str().ok_or("Invalid folder")?;
    let leaf = parts[1].as_os_str().to_str().ok_or("Invalid filename")?;
    if kind(folder, leaf).is_none() {
        return Err("Unsupported Reference Desk file".into());
    }
    let parent = root.join(folder);
    let path = root.join(name);
    if fs::symlink_metadata(&parent)
        .map_err(|e| e.to_string())?
        .file_type()
        .is_symlink()
        || fs::symlink_metadata(&path)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
    {
        return Err(
            "Linked files are not imported from the library. Choose a saved original.".into(),
        );
    }
    let actual = path.canonicalize().map_err(|e| e.to_string())?;
    if !actual.starts_with(root.canonicalize().map_err(|e| e.to_string())?) || !actual.is_file() {
        return Err("File outside the Reference Desk library".into());
    }
    Ok(actual)
}
fn digest(bytes: &[u8]) -> Result<String, String> {
    let mut child = Command::new("sha256sum")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|e| e.to_string())?;
    child
        .stdin
        .take()
        .ok_or("Cannot fingerprint source")?
        .write_all(bytes)
        .map_err(|e| e.to_string())?;
    let out = child.wait_with_output().map_err(|e| e.to_string())?;
    let hash = String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_string();
    if !out.status.success() || hash.len() != 64 || !hash.bytes().all(|c| c.is_ascii_hexdigit()) {
        return Err("Cannot fingerprint source".into());
    }
    Ok(hash)
}
fn web_url(value: &str) -> bool {
    (value.starts_with("https://") || value.starts_with("http://"))
        && value.len() <= 2048
        && !value.chars().any(|c| c.is_whitespace() || c.is_control())
}
fn timestamp(value: &str) -> Result<u64, String> {
    let normalized = value.trim().replace(',', ".");
    let (clock, fraction) = normalized.split_once('.').unwrap_or((&normalized, "0"));
    let parts = clock
        .split(':')
        .map(str::parse::<u64>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| "Invalid timestamp")?;
    if !(2..=3).contains(&parts.len())
        || fraction.is_empty()
        || fraction.len() > 3
        || !fraction.bytes().all(|c| c.is_ascii_digit())
    {
        return Err("Use a valid transcript timestamp".into());
    }
    let (h, m, s) = if parts.len() == 3 {
        (parts[0], parts[1], parts[2])
    } else {
        (0, parts[0], parts[1])
    };
    if h > 23 || m > 59 || s > 59 {
        return Err("Transcript timestamp outside 24 hours".into());
    }
    Ok((h * 3600 + m * 60 + s) * 1000
        + fraction
            .parse::<u64>()
            .map_err(|_| "Invalid milliseconds")?
            * 10u64.pow((3 - fraction.len()) as u32))
}
pub fn parse(name: &str, bytes: &[u8]) -> Result<Source, String> {
    if bytes.is_empty() || bytes.len() > TEXT_LIMIT {
        return Err("Use a non-empty transcript or SRT below 1 MB".into());
    }
    let ext = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !matches!(ext.as_str(), "txt" | "srt") {
        return Err("Choose a UTF-8 TXT transcript or SRT captions".into());
    }
    let text = std::str::from_utf8(bytes)
        .map_err(|_| "Save the transcript as UTF-8")?
        .trim_start_matches('\u{feff}')
        .replace("\r\n", "\n");
    if text.contains('\0') {
        return Err("Transcript contains unsupported binary data".into());
    }
    let mut source = Source {
        id: crate::model::generate_document_id(),
        title: Path::new(name)
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("Research source")
            .chars()
            .take(240)
            .collect(),
        filename: Path::new(name)
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("source.txt")
            .chars()
            .take(300)
            .collect(),
        kind: ext.clone(),
        source_url: None,
        method: None,
        fingerprint: digest(bytes)?,
        text,
        cues: vec![],
        media_asset: None,
    };
    if ext == "srt" {
        for block in source.text.split("\n\n").filter(|b| !b.trim().is_empty()) {
            let lines = block.lines().collect::<Vec<_>>();
            let at = lines
                .iter()
                .position(|line| line.contains("-->"))
                .ok_or("SRT is missing a timing line")?;
            let (a, b) = lines[at].split_once("-->").ok_or("Invalid SRT timing")?;
            let (start, end) = (timestamp(a)?, timestamp(b)?);
            let text = lines[at + 1..].join("\n");
            if end <= start || text.trim().is_empty() {
                return Err("SRT captions need text and a positive duration".into());
            }
            source.cues.push(Cue {
                start_ms: start,
                end_ms: Some(end),
                text,
            });
        }
        if source.cues.is_empty() {
            return Err("No captions found".into());
        }
    } else {
        let lines = source.text.lines().collect::<Vec<_>>();
        if lines.len() >= 3 && web_url(lines[1]) {
            source.title = lines[0].chars().take(240).collect();
            source.source_url = Some(lines[1].into());
            source.method = Some(lines[2].chars().take(200).collect());
        }
        for line in &lines {
            if let Some(rest) = line.strip_prefix('[') {
                if let Some((clock, body)) = rest.split_once("] ") {
                    if let Ok(start) = timestamp(clock) {
                        if !body.trim().is_empty() {
                            source.cues.push(Cue {
                                start_ms: start,
                                end_ms: None,
                                text: body.into(),
                            });
                        }
                    }
                }
            }
        }
    }
    if source.cues.len() > 2000 {
        return Err("Use a transcript with at most 2000 passages".into());
    }
    Ok(source)
}
pub fn parse_file(path: &Path) -> Result<Source, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut bytes = Vec::new();
    file.take(TEXT_LIMIT as u64 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    parse(
        path.file_name()
            .and_then(|x| x.to_str())
            .ok_or("Invalid filename")?,
        &bytes,
    )
}
pub fn preview_at(root: &Path, id: &str) -> Result<Source, String> {
    parse_file(&path_at(root, id)?)
}
pub fn summaries(p: &Project) -> Value {
    json!(p.production.research.iter().map(|s|json!({"id":s.id,"title":s.title,"filename":s.filename,"kind":s.kind,"source_url":s.source_url,"passages":s.cues.len(),"fingerprint":s.fingerprint,"media_asset":s.media_asset})).collect::<Vec<_>>())
}
pub fn validate(p: &Project) -> Result<(), String> {
    if p.production
        .research
        .iter()
        .map(|s| s.text.len() + s.cues.iter().map(|c| c.text.len()).sum::<usize>())
        .sum::<usize>()
        > 8_000_000
    {
        return Err("Keep research text and passages below 8 MB per project".into());
    }
    if p.production.research.len() > 100
        || p.production
            .research
            .iter()
            .map(|s| s.text.len())
            .sum::<usize>()
            > 4_000_000
    {
        return Err(
            "Keep at most 100 research sources and 4 MB of transcript text per project".into(),
        );
    }
    let mut ids = std::collections::BTreeSet::new();
    for s in &p.production.research {
        if !ids.insert(&s.id)
            || s.id.is_empty()
            || s.id.len() > 80
            || s.title.len() > 1000
            || s.filename.len() > 1200
            || s.fingerprint.len() != 64
            || !s.fingerprint.bytes().all(|c| c.is_ascii_hexdigit())
            || s.text.len() > TEXT_LIMIT
            || s.cues.len() > 2000
            || s.cues.iter().map(|c| c.text.len()).sum::<usize>() > TEXT_LIMIT
            || !matches!(s.kind.as_str(), "txt" | "srt" | "audio" | "video")
            || s.source_url.as_ref().is_some_and(|u| !web_url(u))
            || s.method.as_ref().is_some_and(|m| m.len() > 1000)
            || (s.kind == "srt" && (s.cues.is_empty() || s.cues.iter().any(|c| c.end_ms.is_none())))
        {
            return Err("Invalid or oversized research source".into());
        }
        for c in &s.cues {
            if c.start_ms >= 86_400_000
                || c.end_ms.is_some_and(|e| e <= c.start_ms || e >= 86_400_000)
                || c.text.len() > TEXT_LIMIT
            {
                return Err("Invalid research passage".into());
            }
        }
    }
    Ok(())
}
pub fn import_source(p: &mut Project, source: Source) -> Result<String, String> {
    if let Some(existing) = p
        .production
        .research
        .iter()
        .find(|s| s.fingerprint == source.fingerprint && s.kind == source.kind)
    {
        return Ok(existing.id.clone());
    }
    let id = source.id.clone();
    let mut candidate = p.clone();
    candidate.production.research.push(source);
    validate(&candidate)?;
    *p = candidate;
    Ok(id)
}
pub fn import_library(p: &mut Project, id: &str, fingerprint: &str) -> Result<String, String> {
    import_library_at(p, &root(), id, fingerprint)
}
pub fn import_library_at(
    p: &mut Project,
    root: &Path,
    id: &str,
    fingerprint: &str,
) -> Result<String, String> {
    let source = preview_at(root, id)?;
    if source.fingerprint != fingerprint {
        return Err(
            "The source file changed since preview. Preview it again before importing.".into(),
        );
    }
    import_source(p, source)
}
pub fn import_media(p: &mut Project, path: &Path, name: &str) -> Result<String, String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > MEDIA_LIMIT {
        return Err("Keep each imported media file below 128 MiB".into());
    }
    let asset = crate::media_store::import(path, name)?;
    if !matches!(asset.kind.as_str(), "audio" | "video") {
        return Err("Choose audio or video".into());
    }
    let mut candidate = p.clone();
    candidate
        .production
        .timeline
        .assets
        .insert(asset.id.clone(), asset.clone());
    let source = Source {
        id: crate::model::generate_document_id(),
        title: name.chars().take(240).collect(),
        filename: name.chars().take(300).collect(),
        kind: asset.kind,
        source_url: None,
        method: Some("Imported media snapshot".into()),
        fingerprint: asset.id.clone(),
        text: String::new(),
        cues: vec![],
        media_asset: Some(asset.id),
    };
    let id = import_source(&mut candidate, source)?;
    crate::timeline::validate(&candidate.production.timeline)?;
    *p = candidate;
    Ok(id)
}
fn stamp(ms: u64) -> String {
    format!(
        "{:02}:{:02}:{:02},{:03}",
        ms / 3_600_000,
        ms / 60_000 % 60,
        ms / 1000 % 60,
        ms % 1000
    )
}
pub fn excerpt(s: &Source, cue: Option<usize>) -> Result<String, String> {
    let (body, at) = if let Some(i) = cue {
        let c = s.cues.get(i).ok_or("Choose a passage")?;
        (c.text.clone(), format!(" [{}]", stamp(c.start_ms)))
    } else {
        (s.text.clone(), String::new())
    };
    if body.trim().is_empty() {
        return Err("This source has no transcript text".into());
    }
    Ok(format!(
        "{}{}\n{}\n{}",
        s.title,
        at,
        s.source_url.as_deref().unwrap_or(&s.filename),
        body
    ))
}
pub fn apply(p: &mut Project, v: &Value) -> Result<(), String> {
    validate(p)?;
    let id = v["id"].as_str().ok_or("Choose a research source")?;
    let s = p
        .production
        .research
        .iter()
        .find(|s| s.id == id)
        .ok_or("Research source is no longer in this project")?
        .clone();
    match v["action"].as_str().unwrap_or("") {
        "research_note" => {
            let page = v["page"].as_u64().ok_or("Choose a page")? as usize;
            let cue = v
                .get("cue")
                .map(|n| n.as_u64().ok_or("Invalid passage"))
                .transpose()?
                .map(|x| x as usize);
            let note = excerpt(&s, cue)?;
            let page = p.pages.get_mut(page).ok_or("Page not found")?;
            if page.notes.len() + note.len() + 2 > 100_000 {
                return Err(
                    "Choose a shorter passage; page notes support up to 100,000 bytes".into(),
                );
            }
            if !page.notes.is_empty() {
                page.notes.push_str("\n\n");
            }
            page.notes.push_str(&note);
        }
        "research_captions" => {
            if s.kind != "srt" {
                return Err("Use an SRT source with start and end timings for captions".into());
            }
            if s.cues.len() > 200 {
                return Err("Use a shorter SRT with at most 200 captions for a video".into());
            }
            let end = s.cues.iter().filter_map(|c| c.end_ms).max().unwrap_or(0);
            if end > 600_000 {
                return Err("Use an SRT within the ten-minute video limit".into());
            }
            let fps = p.production.timeline.fps as u64;
            let mut candidate = p.clone();
            candidate.production.timeline.duration = candidate
                .production
                .timeline
                .duration
                .max((end * fps + 999) / 1000);
            let text = s
                .cues
                .iter()
                .enumerate()
                .map(|(i, c)| {
                    format!(
                        "{}\n{} --> {}\n{}\n\n",
                        i + 1,
                        stamp(c.start_ms),
                        stamp(c.end_ms.unwrap()),
                        c.text
                    )
                })
                .collect::<String>();
            crate::timeline::import_srt(&mut candidate, &text)?;
            crate::timeline::validate(&candidate.production.timeline)?;
            *p = candidate;
        }
        "research_remove" => p.production.research.retain(|s| s.id != id),
        _ => return Err("Unsupported research action".into()),
    }
    Ok(())
}
pub fn launcher() -> PathBuf {
    PathBuf::from(std::env::var_os("HOME").unwrap_or_default())
        .join("Applications/Reference-Desk/launch-reference-desk")
}
pub fn launch() -> Result<(), String> {
    let path = launcher();
    if !path.is_file() {
        return Err(
            "Reference Desk is not installed on this computer. Import saved files instead.".into(),
        );
    }
    Command::new(path)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
