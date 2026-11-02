//! Versioned, compressed single-file projects. Extraction accepts only asset PNGs.
use crate::model::{generate_document_id, Project};
use base64::Engine;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const MAGIC: &[u8] = b"CANVAS-STUDIO-PACKAGE\n1\n";
const MAX_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Serialize, Deserialize)]
struct Envelope {
    version: u32,
    project: String,
    assets: BTreeMap<String, String>,
    #[serde(default)]
    required_features: Vec<String>,
    #[serde(default)]
    media: BTreeMap<String, String>,
    #[serde(default)]
    linked_media: bool,
}

struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!("canvas-package-{}", generate_document_id()));
        fs::create_dir(&path).map_err(|e| e.to_string())?;
        Ok(Self(path))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

pub fn is_package(path: &Path) -> bool {
    let mut prefix = vec![0; MAGIC.len()];
    fs::File::open(path)
        .and_then(|mut f| f.read_exact(&mut prefix))
        .is_ok()
        && prefix == MAGIC
}

pub fn save(project: &Project, path: &Path) -> Result<(), String> {
    save_mode(project, path, true)
}
pub fn save_recovery(project: &Project, path: &Path) -> Result<(), String> {
    save_mode(project, path, false)
}
fn save_mode(project: &Project, path: &Path, embed: bool) -> Result<(), String> {
    let scratch = Scratch::new()?;
    let manifest = scratch.0.join("project.canvas.json");
    crate::exporter::save_project_manifest(project, &manifest)?;
    let mut assets = BTreeMap::new();
    let folder = scratch.0.join("assets");
    if folder.is_dir() {
        for entry in fs::read_dir(folder).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            assets.insert(
                entry.file_name().to_string_lossy().into_owned(),
                base64::engine::general_purpose::STANDARD
                    .encode(fs::read(entry.path()).map_err(|e| e.to_string())?),
            );
        }
    }
    let mut media = BTreeMap::new();
    if embed {
        for a in project.production.timeline.assets.values() {
            let path = crate::media_store::verify(a)?;
            media.insert(
                a.id.clone(),
                base64::engine::general_purpose::STANDARD
                    .encode(fs::read(path).map_err(|e| e.to_string())?),
            );
        }
    }
    let mut features = Vec::new();
    if project.production.studio.active() {
        features.push("studio-campaign-v1".into());
    }
    if project.production.timeline.active() {
        features.push("video-timeline-v1".into());
    }
    let envelope = Envelope {
        version: if project.production.timeline.active() {
            3
        } else if project.production.studio.active() {
            2
        } else {
            1
        },
        required_features: features,
        media,
        linked_media: !embed,
        project: fs::read_to_string(manifest).map_err(|e| e.to_string())?,
        assets,
    };
    let json = serde_json::to_vec(&envelope).map_err(|e| e.to_string())?;
    if json.len() as u64 > MAX_BYTES {
        return Err("Portable project exceeds the 512 MB unpacked limit".into());
    }
    let mut encoder = flate2::write::GzEncoder::new(Vec::new(), flate2::Compression::fast());
    encoder.write_all(&json).map_err(|e| e.to_string())?;
    let compressed = encoder.finish().map_err(|e| e.to_string())?;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    let temporary = parent.join(format!(
        ".{}.{}.tmp",
        path.file_name().unwrap_or_default().to_string_lossy(),
        generate_document_id()
    ));
    let result = (|| {
        let mut output = fs::File::create(&temporary).map_err(|e| e.to_string())?;
        output
            .write_all(MAGIC)
            .and_then(|_| output.write_all(&compressed))
            .and_then(|_| output.sync_all())
            .map_err(|e| e.to_string())?;
        if path.exists() {
            fs::copy(path, path.with_extension("canvas.bak")).map_err(|e| e.to_string())?;
        }
        fs::rename(&temporary, path).map_err(|e| e.to_string())
    })();
    if result.is_err() {
        let _ = fs::remove_file(temporary);
    }
    result
}

pub fn load(path: &Path) -> Result<Project, String> {
    let mut source = fs::File::open(path).map_err(|e| e.to_string())?;
    if source.metadata().map_err(|e| e.to_string())?.len() > MAX_BYTES {
        return Err("Portable project exceeds the size limit".into());
    }
    let mut prefix = vec![0; MAGIC.len()];
    source.read_exact(&mut prefix).map_err(|e| e.to_string())?;
    if prefix != MAGIC {
        return Err("Unrecognised portable project header".into());
    }
    let mut bytes = Vec::new();
    flate2::read::GzDecoder::new(source)
        .take(MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("Damaged portable project: {e}"))?;
    if bytes.len() as u64 > MAX_BYTES {
        return Err("Unpacked project exceeds the size limit".into());
    }
    let envelope: Envelope = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    if ![1, 2, 3].contains(&envelope.version)
        || envelope.assets.len() > 10_000
        || envelope
            .required_features
            .iter()
            .any(|f| f != "studio-campaign-v1" && f != "video-timeline-v1")
        || (envelope.version >= 2 && envelope.required_features.is_empty())
    {
        return Err("Unsupported portable project version or asset count".into());
    }
    let scratch = Scratch::new()?;
    fs::create_dir(scratch.0.join("assets")).map_err(|e| e.to_string())?;
    for (name, encoded) in envelope.assets {
        let stem = name
            .strip_suffix(".png")
            .ok_or("Portable assets must be PNG files")?;
        if stem.len() != 32 || !stem.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("Invalid portable asset name".into());
        }
        let data = base64::engine::general_purpose::STANDARD
            .decode(encoded)
            .map_err(|e| e.to_string())?;
        let reader = image::ImageReader::new(std::io::Cursor::new(&data))
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
        if w == 0 || h == 0 || w > 18_000 || h > 18_000 || w as u64 * h as u64 > 100_000_000 {
            return Err("Portable image exceeds safe dimensions".into());
        }
        fs::write(scratch.0.join("assets").join(name), data).map_err(|e| e.to_string())?;
    }
    let manifest = scratch.0.join("project.canvas.json");
    fs::write(&manifest, envelope.project).map_err(|e| e.to_string())?;
    let project = crate::exporter::load_project_json(&manifest)?;
    if project.production.timeline.active()
        && (envelope.version != 3
            || !envelope
                .required_features
                .iter()
                .any(|f| f == "video-timeline-v1"))
    {
        return Err("Film project requires version 3 and video-timeline-v1 support".into());
    }
    if !envelope.linked_media {
        for a in project.production.timeline.assets.values() {
            let encoded = envelope
                .media
                .get(&a.id)
                .ok_or("Portable movie is missing an original media payload")?;
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(encoded)
                .map_err(|e| e.to_string())?;
            crate::media_store::install(a, &bytes)?;
        }
    }
    Ok(project)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unsupported_paths_and_damaged_archives_are_rejected() {
        let root = Scratch::new().unwrap();
        let path = root.0.join("invalid.canvas");
        let bad = Envelope {
            version: 1,
            required_features: vec![],
            media: BTreeMap::new(),
            linked_media: false,
            project: "{}".into(),
            assets: BTreeMap::from([("../outside.png".into(), String::new())]),
        };
        let mut encoder =
            flate2::write::GzEncoder::new(MAGIC.to_vec(), flate2::Compression::fast());
        encoder
            .write_all(&serde_json::to_vec(&bad).unwrap())
            .unwrap();
        fs::write(&path, encoder.finish().unwrap()).unwrap();
        assert!(load(&path)
            .unwrap_err()
            .contains("Invalid portable asset name"));
        fs::write(&path, MAGIC).unwrap();
        assert!(load(&path).is_err());
    }
}
