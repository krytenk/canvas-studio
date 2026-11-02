//! Private, content-addressed originals; generated proxies are disposable.
use crate::{model::generate_document_id, timeline::MediaAsset};
use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};
pub fn extensions() -> &'static [&'static str] {
    &[
        "mp4", "m4v", "mov", "m4a", "webm", "mkv", "wav", "mp3", "flac", "ogg", "png", "jpg",
        "jpeg", "webp",
    ]
}
pub fn root() -> PathBuf {
    crate::application_data_dir().join("canvas-studio/media")
}
pub fn folder(name: &str) -> Result<PathBuf, String> {
    let p = root().join(name);
    fs::create_dir_all(&p).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&p, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(p)
}
pub fn original(a: &MediaAsset) -> Result<PathBuf, String> {
    if a.id.len() != 64
        || !a.id.bytes().all(|c| c.is_ascii_hexdigit())
        || !extensions().contains(&a.extension.as_str())
    {
        return Err("Invalid media identity".into());
    }
    Ok(folder("originals")?.join(format!("{}.{}", a.id, a.extension)))
}
pub fn digest(p: &Path) -> Result<String, String> {
    let out = Command::new("sha256sum")
        .arg(p)
        .output()
        .map_err(|e| format!("Cannot verify media digest: {e}"))?;
    let s = String::from_utf8_lossy(&out.stdout);
    let hash = s.split_whitespace().next().ok_or("Media digest failed")?;
    if !out.status.success() || hash.len() != 64 || !hash.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Media digest failed".into());
    }
    Ok(hash.into())
}
pub fn verify(a: &MediaAsset) -> Result<PathBuf, String> {
    let p = original(a)?;
    if !p.is_file() {
        return Err(format!("Original media '{}' is missing. Restore its portable project or import the same original again",a.name));
    }
    if fs::metadata(&p).map_err(|e| e.to_string())?.len() != a.bytes || digest(&p)? != a.id {
        return Err(format!(
            "Original media '{}' is damaged; restore or relink it",
            a.name
        ));
    }
    Ok(p)
}
pub fn import(path: &Path, name: &str) -> Result<MediaAsset, String> {
    let ext = Path::new(name)
        .extension()
        .and_then(|x| x.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    if !extensions().contains(&ext.as_str()) {
        return Err("Use MP4/MOV/WebM/MKV, WAV/MP3/FLAC/OGG/M4A, or PNG/JPEG/WebP".into());
    }
    let meta = crate::media_service::sync_worker(
        serde_json::json!({"op":"probe","source":path,"extension":format!(".{ext}"),"name":name}),
    )?;
    let asset: MediaAsset = serde_json::from_value(meta).map_err(|e| e.to_string())?;
    let dest = original(&asset)?;
    if !dest.exists() {
        let tmp = dest.with_extension(format!("{}.tmp", generate_document_id()));
        fs::copy(path, &tmp).map_err(|e| e.to_string())?;
        if digest(&tmp)? != asset.id {
            let _ = fs::remove_file(tmp);
            return Err("Source changed while importing; retry the import".into());
        }
        fs::rename(tmp, &dest).map_err(|e| e.to_string())?;
    }
    verify(&asset)?;
    Ok(asset)
}
pub fn install(a: &MediaAsset, bytes: &[u8]) -> Result<(), String> {
    if bytes.len() as u64 != a.bytes || bytes.len() > 128 * 1024 * 1024 {
        return Err("Invalid portable media size".into());
    }
    let dest = original(a)?;
    if dest.exists() && verify(a).is_ok() {
        return Ok(());
    }
    let tmp = dest.with_extension(format!("{}.tmp", generate_document_id()));
    fs::write(&tmp, bytes).map_err(|e| e.to_string())?;
    if digest(&tmp)? != a.id {
        let _ = fs::remove_file(&tmp);
        return Err("Portable media checksum mismatch".into());
    }
    fs::rename(tmp, dest).map_err(|e| e.to_string())
}
pub fn cached(a: &MediaAsset, kind: &str) -> Result<PathBuf, String> {
    let suffix = match kind {
        "poster" => "poster.jpg",
        "wave" => "wave.png",
        "proxy" => match a.kind.as_str() {
            "video" => "proxy.mp4",
            "audio" => "proxy.m4a",
            _ => "proxy.png",
        },
        _ => return Err("Unknown media cache kind".into()),
    };
    Ok(folder("cache")?.join(format!("{}-v1.{suffix}", a.id)))
}
