//! App-owned generation snapshots, model discovery and private connection settings.
use crate::model::*;
use serde_json::{json, Value};
use std::{
    fs,
    path::{Path, PathBuf},
};
pub(crate) fn root() -> PathBuf {
    crate::application_data_dir().join("canvas-studio/ai")
}
pub(crate) fn catalog() -> Value {
    let mut data: Value = serde_json::from_str(include_str!("../generation/catalog.json")).unwrap();
    if let Ok(bytes) = fs::read(root().join("custom-models.json")) {
        if let Ok(Value::Array(items)) = serde_json::from_slice(&bytes) {
            data["models"].as_array_mut().unwrap().extend(items);
        }
    }
    data
}
pub(crate) fn state() -> Value {
    let config: Value = fs::read(root().join("connections.json"))
        .ok()
        .and_then(|v| serde_json::from_slice(&v).ok())
        .unwrap_or(json!({}));
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    json!({"catalog":catalog(),"connections":{"kaggle_configured":config["kaggle_token"].as_str().is_some_and(|s|!s.is_empty())||std::env::var_os("KAGGLE_API_TOKEN").is_some()||home.join(".kaggle/access_token").exists()||home.join(".kaggle/kaggle.json").exists(),"kaggle_username":config["kaggle_username"].as_str().unwrap_or(""),"hf_configured":config["hf_token"].as_str().is_some_and(|s|!s.is_empty())||home.join(".cache/huggingface/token").exists(),"civitai_configured":config["civitai_token"].as_str().is_some_and(|s|!s.is_empty()),"cache_dir":config["cache_dir"].as_str().map(PathBuf::from).unwrap_or(root().join("models")),"tools_installed":python().is_ok()},"note":"Free weights do not mean unlimited free compute. Kaggle quota and GPU availability are controlled by your account."})
}
pub(crate) fn python() -> Result<PathBuf, String> {
    if let Some(v) = std::env::var_os("CANVAS_STUDIO_AI_PYTHON") {
        let p = PathBuf::from(v);
        if p.is_file() {
            return Ok(p);
        }
    }
    let p = crate::application_data_dir().join("canvas-studio/generation-tools/bin/python");
    if p.is_file() {
        Ok(p)
    } else {
        Err("Generation tools are not installed. Run python3 scripts/setup_generation.py; local inference additionally needs --local-inference".into())
    }
}
pub(crate) fn save_connections(v: Value) -> Result<(), String> {
    let fields = v.as_object().ok_or("Use connection fields")?;
    let allowed = [
        "kaggle_username",
        "kaggle_token",
        "hf_token",
        "civitai_token",
        "cache_dir",
    ];
    let mut config: Value = fs::read(root().join("connections.json"))
        .ok()
        .and_then(|s| serde_json::from_slice(&s).ok())
        .unwrap_or(json!({}));
    for (k, value) in fields {
        if !allowed.contains(&k.as_str()) {
            return Err("Unknown connection field".into());
        }
        let s = value.as_str().ok_or("Use text connection fields")?;
        if s.len() > 4096 {
            return Err("Connection value is too long".into());
        }
        if k == "kaggle_username"
            && !s.is_empty()
            && (!s
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
                || s.len() > 50)
        {
            return Err("Use your actual Kaggle username".into());
        }
        if k == "cache_dir" && !s.is_empty() {
            let path = Path::new(s);
            if !path.is_absolute() {
                return Err("Cache directory must be an absolute folder".into());
            }
            fs::create_dir_all(path).map_err(|e| e.to_string())?;
        }
        if !s.is_empty() {
            config[k] = value.clone();
        }
    }
    fs::create_dir_all(root()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(root(), fs::Permissions::from_mode(0o700))
            .map_err(|e| e.to_string())?;
    }
    let temporary = root().join(format!("connections-{}.tmp", generate_document_id()));
    fs::write(&temporary, serde_json::to_vec(&config).unwrap()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    fs::rename(temporary, root().join("connections.json")).map_err(|e| e.to_string())
}
fn reference_dir(document: &str) -> Result<PathBuf, String> {
    if document.is_empty()
        || document.len() > 160
        || !document
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid reference document".into());
    }
    Ok(root().join("references").join(document))
}
/// Decode only bounded raster input, honour EXIF rotation and remove original metadata.
/// Uploaded references are private generation inputs; they never change a design page.
pub(crate) fn upload_reference(document: &str, bytes: &[u8]) -> Result<Value, String> {
    use image::ImageDecoder;
    if bytes.is_empty() || bytes.len() > 25_000_000 {
        return Err("Choose a PNG, JPEG or WebP image up to 25 MB".into());
    }
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| e.to_string())?;
    if !matches!(
        reader.format(),
        Some(image::ImageFormat::Png | image::ImageFormat::Jpeg | image::ImageFormat::WebP)
    ) {
        return Err("Choose a PNG, JPEG or WebP reference image".into());
    }
    let mut decoder = reader.into_decoder().map_err(|e| e.to_string())?;
    let (w, h) = decoder.dimensions();
    if w == 0 || h == 0 || w as u64 * h as u64 > 16_000_000 {
        return Err("Reference image exceeds 16 megapixels".into());
    }
    let orientation = decoder.orientation().map_err(|e| e.to_string())?;
    let mut image = image::DynamicImage::from_decoder(decoder).map_err(|e| e.to_string())?;
    image.apply_orientation(orientation);
    if image.width().max(image.height()) > 2048 {
        image = image.resize(2048, 2048, image::imageops::FilterType::Lanczos3);
    }
    image = image::DynamicImage::ImageRgba8(image.to_rgba8());
    let mut normalized = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut normalized, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    let folder = reference_dir(document)?;
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        for p in [root(), root().join("references"), folder.clone()] {
            fs::set_permissions(p, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
        }
    }
    let temporary = folder.join(format!("{}.tmp", generate_document_id()));
    fs::write(&temporary, normalized.into_inner()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&temporary, fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    let result = (|| {
        let id = crate::media_store::digest(&temporary)?;
        let path = folder.join(format!("{id}.png"));
        fs::rename(&temporary, &path).map_err(|e| e.to_string())?;
        Ok(
            json!({"id":id,"width":image.width(),"height":image.height(),"bytes":fs::metadata(path).map_err(|e|e.to_string())?.len()}),
        )
    })();
    let _ = fs::remove_file(temporary);
    result
}
pub(crate) fn reference_bytes(document: &str, id: &str) -> Result<Vec<u8>, String> {
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid reference image ID".into());
    }
    let path = reference_dir(document)?.join(format!("{id}.png"));
    if fs::metadata(&path)
        .map_err(|_| "Choose a reference uploaded for this document")?
        .len()
        > 25_000_000
        || crate::media_store::digest(&path)? != id
    {
        return Err("Reference image failed its integrity check".into());
    }
    fs::read(path).map_err(|e| e.to_string())
}
fn reference_ids(value: &Value) -> Result<Vec<String>, String> {
    let values = value
        .as_array()
        .ok_or("Choose one to four reference images")?;
    if values.is_empty() || values.len() > 4 {
        return Err("Choose one to four reference images".into());
    }
    let mut ids = Vec::new();
    for value in values {
        let id = value.as_str().ok_or("Invalid reference ID")?;
        if id.len() != 64
            || !id.bytes().all(|b| b.is_ascii_hexdigit())
            || ids.iter().any(|v| v == id)
        {
            return Err("Choose distinct valid reference images".into());
        }
        ids.push(id.to_string());
    }
    Ok(ids)
}
fn private_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o700)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn private_file(path: &Path, bytes: &[u8]) -> Result<(), String> {
    fs::write(path, bytes).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600)).map_err(|e| e.to_string())?;
    }
    Ok(())
}
fn reference_set_dir(id: &str) -> Result<PathBuf, String> {
    if id.len() != 64 || !id.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("Invalid reference set ID".into());
    }
    Ok(root().join("reference-sets").join(id))
}
pub(crate) fn reference_sets() -> Value {
    let mut sets = Vec::new();
    if let Ok(dirs) = fs::read_dir(root().join("reference-sets")) {
        for dir in dirs.flatten() {
            let id = dir.file_name().to_string_lossy().into_owned();
            if let Ok(folder) = reference_set_dir(&id) {
                if let Ok(bytes) = fs::read(folder.join("manifest.json")) {
                    if let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) {
                        value["id"] = json!(id);
                        sets.push(value);
                    }
                }
            }
        }
    }
    sets.sort_by_key(|v| v["name"].as_str().unwrap_or("").to_lowercase());
    json!({"sets":sets})
}
pub(crate) fn reference_set_action(document: &str, value: &Value) -> Result<Value, String> {
    match value["operation"].as_str().unwrap_or("") {
        "save" => {
            let name = value["name"].as_str().unwrap_or("").trim();
            if name.is_empty() || name.chars().count() > 80 {
                return Err("Name this reference set using 1–80 characters".into());
            }
            let ids = reference_ids(&value["reference_ids"])?;
            let bytes = ids
                .iter()
                .map(|id| reference_bytes(document, id))
                .collect::<Result<Vec<_>, _>>()?;
            let manifest = json!({"name":name,"reference_ids":ids,"count":ids.len()});
            let encoded = serde_json::to_vec(&manifest).unwrap();
            private_dir(&root())?;
            let library = root().join("reference-sets");
            private_dir(&library)?;
            let stage = library.join(format!("stage-{}", generate_document_id()));
            private_dir(&stage)?;
            let result = (|| {
                private_file(&stage.join("manifest.json"), &encoded)?;
                let id = crate::media_store::digest(&stage.join("manifest.json"))?;
                let destination = reference_set_dir(&id)?;
                if !destination.exists() {
                    if reference_sets()["sets"]
                        .as_array()
                        .is_some_and(|v| v.len() >= 100)
                    {
                        return Err("Keep at most 100 saved reference sets".into());
                    }
                    for (hash, data) in ids.iter().zip(bytes) {
                        private_file(&stage.join(format!("{hash}.png")), &data)?;
                    }
                    fs::rename(&stage, &destination).map_err(|e| e.to_string())?;
                }
                Ok(json!({"id":id,"name":name,"count":ids.len(),"reference_ids":ids}))
            })();
            let _ = fs::remove_dir_all(stage);
            result
        }
        "load" => {
            let id = value["id"].as_str().ok_or("Choose a saved reference set")?;
            let folder = reference_set_dir(id)?;
            let manifest = folder.join("manifest.json");
            if crate::media_store::digest(&manifest)? != id {
                return Err("Saved reference set metadata changed".into());
            }
            let set: Value =
                serde_json::from_slice(&fs::read(manifest).map_err(|e| e.to_string())?)
                    .map_err(|e| e.to_string())?;
            let ids = reference_ids(&set["reference_ids"])?;
            let mut images = Vec::new();
            for hash in &ids {
                let file = folder.join(format!("{hash}.png"));
                if fs::metadata(&file).map_err(|e| e.to_string())?.len() > 25_000_000
                    || crate::media_store::digest(&file)? != *hash
                {
                    return Err("Saved reference image failed its integrity check".into());
                }
                images.push((hash, fs::read(&file).map_err(|e| e.to_string())?));
            }
            private_dir(&root())?;
            private_dir(&root().join("references"))?;
            let active = reference_dir(document)?;
            private_dir(&active)?;
            let mut refs = Vec::new();
            for (hash, bytes) in images {
                let dimensions = image::ImageReader::new(std::io::Cursor::new(&bytes))
                    .with_guessed_format()
                    .map_err(|e| e.to_string())?
                    .into_dimensions()
                    .map_err(|e| e.to_string())?;
                private_file(&active.join(format!("{hash}.png")), &bytes)?;
                refs.push(json!({"id":hash,"width":dimensions.0,"height":dimensions.1}));
            }
            Ok(json!({"id":id,"name":set["name"],"references":refs}))
        }
        _ => Err("Choose save or load for a reference set".into()),
    }
}

#[cfg(test)]
mod reference_tests {
    use super::*;
    #[test]
    fn uploaded_reference_is_normalized_scoped_private_and_integrity_checked() {
        let document = generate_document_id();
        let other = generate_document_id();
        let mut png = std::io::Cursor::new(Vec::new());
        image::DynamicImage::new_rgba8(24, 32)
            .write_to(&mut png, image::ImageFormat::Png)
            .unwrap();
        let info = upload_reference(&document, png.get_ref()).unwrap();
        let id = info["id"].as_str().unwrap();
        let bytes = reference_bytes(&document, id).unwrap();
        assert_eq!(
            (info["width"].as_u64(), info["height"].as_u64()),
            (Some(24), Some(32))
        );
        assert_eq!(image::load_from_memory(&bytes).unwrap().width(), 24);
        assert!(reference_bytes(&other, id).is_err());
        assert!(reference_bytes("../escape", id).is_err());
        assert!(reference_bytes(&document, "../escape").is_err());
        let folder = reference_dir(&document).unwrap();
        let path = folder.join(format!("{id}.png"));
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(&folder).unwrap().permissions().mode() & 0o777,
                0o700
            );
            assert_eq!(
                fs::metadata(&path).unwrap().permissions().mode() & 0o777,
                0o600
            );
        }
        fs::write(&path, b"changed image").unwrap();
        assert!(reference_bytes(&document, id).is_err());
        fs::remove_dir_all(folder).unwrap();
    }
    #[test]
    fn named_reference_set_is_ordered_reusable_and_rejects_tampering() {
        let original = generate_document_id();
        let fresh = generate_document_id();
        let mut ids = Vec::new();
        for size in [24, 32] {
            let mut png = std::io::Cursor::new(Vec::new());
            image::DynamicImage::new_rgb8(size, 32)
                .write_to(&mut png, image::ImageFormat::Png)
                .unwrap();
            ids.push(
                upload_reference(&original, png.get_ref()).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_string(),
            );
        }
        let request =
            json!({"operation":"save","name":format!("Test {}",original),"reference_ids":ids});
        let set = reference_set_action(&original, &request).unwrap();
        assert_eq!(set, reference_set_action(&original, &request).unwrap());
        assert!(reference_bytes(&fresh, &ids[0]).is_err());
        let id = set["id"].as_str().unwrap();
        let load = json!({"operation":"load","id":id});
        let loaded = reference_set_action(&fresh, &load).unwrap();
        assert_eq!(
            loaded["references"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["id"].as_str().unwrap())
                .collect::<Vec<_>>(),
            ids.iter().map(String::as_str).collect::<Vec<_>>()
        );
        for hash in &ids {
            assert_eq!(
                reference_bytes(&fresh, hash).unwrap(),
                reference_bytes(&original, hash).unwrap()
            );
        }
        let folder = reference_set_dir(id).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(
                fs::metadata(folder.join("manifest.json"))
                    .unwrap()
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert_eq!(
                fs::metadata(&folder).unwrap().permissions().mode() & 0o777,
                0o700
            );
        }
        fs::write(folder.join(format!("{}.png", ids[1])), b"tampered").unwrap();
        assert!(reference_set_action(&generate_document_id(), &load).is_err());
        fs::remove_dir_all(folder).unwrap();
        fs::remove_dir_all(reference_dir(&original).unwrap()).unwrap();
        fs::remove_dir_all(reference_dir(&fresh).unwrap()).unwrap();
    }
    #[test]
    fn reference_set_rejects_invalid_and_duplicate_ids_before_storage() {
        let document = generate_document_id();
        for values in [
            json!([]),
            json!(["../escape"]),
            json!(["a".repeat(64), "a".repeat(64)]),
            json!([
                "a".repeat(64),
                "b".repeat(64),
                "c".repeat(64),
                "d".repeat(64),
                "e".repeat(64)
            ]),
        ] {
            assert!(reference_set_action(
                &document,
                &json!({"operation":"save","name":"Rejected","reference_ids":values})
            )
            .is_err());
        }
        assert!(
            reference_set_action(&document, &json!({"operation":"load","id":"../escape"})).is_err()
        );
        assert!(!reference_dir(&document).unwrap().exists());
    }
    #[test]
    fn rejected_reference_never_creates_a_stored_input() {
        let document = generate_document_id();
        assert!(upload_reference(&document, b"not an image").is_err());
        assert!(upload_reference(&document, &vec![0; 25_000_001]).is_err());
        assert!(!reference_dir(&document).unwrap().exists());
    }
}
pub(crate) fn start(project: &Project, revision: u64, mut value: Value) -> Result<String, String> {
    let allowed = [
        "operation",
        "model_id",
        "prompt",
        "negative_prompt",
        "width",
        "height",
        "steps",
        "guidance",
        "seed",
        "precision",
        "quantization",
        "offload",
        "device",
        "content_mode",
        "use",
        "license_acknowledged",
        "download_limit_gb",
        "frames",
        "fps",
        "timeout",
        "reference_page",
        "reference_id",
        "reference_ids",
        "reference_strength",
        "reference_fit",
        "adapter",
        "provider",
        "query",
        "type",
        "include_mature",
        "task",
        "id",
        "version_id",
        "file_name",
        "run_id",
    ];
    if value
        .as_object()
        .ok_or("Use generation fields")?
        .keys()
        .any(|k| !allowed.contains(&k.as_str()))
    {
        return Err("Unknown generation field; upload references or use a project page".into());
    }
    let operation = value["operation"]
        .as_str()
        .ok_or("Choose a generation operation")?
        .to_string();
    if ![
        "search",
        "register",
        "prepare",
        "submit",
        "remote_status",
        "download",
        "generate",
    ]
    .contains(&operation.as_str())
    {
        return Err("Unsupported generation operation".into());
    }
    let _ = python()?;
    if ["reference_id", "reference_ids", "reference_page"]
        .iter()
        .filter(|k| value.get(**k).is_some())
        .count()
        > 1
    {
        return Err("Choose one reference source: uploaded images or current page".into());
    }
    let reference = value
        .get("reference_id")
        .map(|id| reference_bytes(&project.document_id, id.as_str().unwrap_or("")))
        .transpose()?;
    let references = if let Some(ids) = value.get("reference_ids") {
        Some(
            reference_ids(ids)?
                .iter()
                .map(|id| reference_bytes(&project.document_id, id))
                .collect::<Result<Vec<_>, _>>()?,
        )
    } else {
        None
    };
    let mut task = catalog()["models"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["id"] == value["model_id"])
        .map(|m| m["task"].as_str().unwrap_or("image").to_string())
        .unwrap_or("image".into());
    if operation == "download" {
        let id = value["run_id"].as_str().ok_or("Choose a remote run")?;
        if id.len() != 28
            || !id.starts_with("run-")
            || !id[4..].bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err("Invalid remote run identity".into());
        }
        let record: Value = serde_json::from_slice(
            &fs::read(root().join("runs").join(format!("{id}.json"))).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        if record["document_id"] != project.document_id {
            return Err("This GPU run belongs to another document".into());
        }
        task = record["model"]["task"].as_str().unwrap_or("image").into();
        value["task"] = json!(task);
    }
    let mode = match operation.as_str() {
        "generate" | "download" => {
            if value["task"].as_str().unwrap_or(&task) == "video" {
                "ai_video"
            } else {
                "ai_image"
            }
        }
        "prepare" => "ai_package",
        _ => "ai_task",
    };
    let snapshot = project.clone();
    value["document_id"] = json!(project.document_id);
    crate::media_service::begin(&project.document_id, revision, mode, move |folder| {
        if let Some(bytes) = reference {
            let path = folder.join("reference.png");
            fs::write(&path, bytes).map_err(|e| e.to_string())?;
            value["reference"] = json!(path);
        }
        if let Some(images) = references {
            let mut paths = Vec::new();
            for (index, bytes) in images.iter().enumerate() {
                let path = folder.join(format!("reference-{:02}.png", index + 1));
                fs::write(&path, bytes).map_err(|e| e.to_string())?;
                paths.push(path);
            }
            value["references"] = json!(paths);
        }
        if let Some(page) = value["reference_page"].as_u64() {
            let bytes = crate::studio_catalog::thumbnail(&snapshot, page as usize, 1600)?;
            let path = folder.join("reference.png");
            fs::write(&path, bytes).map_err(|e| e.to_string())?;
            value["reference"] = json!(path);
        }
        crate::media_service::invoke(
            folder,
            &json!({"op":"ai","request":value,"python":python()?}),
        )
    })
}
pub(crate) fn import(p: &mut Project, v: &Value) -> Result<(), String> {
    let id = v["id"].as_str().ok_or("Choose a completed generation")?;
    let status = crate::media_service::status(id, &p.document_id)?;
    if status["state"] != "complete"
        || !["ai_image", "ai_video"].contains(&status["mode"].as_str().unwrap_or(""))
    {
        return Err("Choose a completed image/video generation job".into());
    }
    let path = crate::media_service::output(id, &p.document_id)?;
    if status["mode"] == "ai_video" {
        let a = crate::media_store::import(&path, "Generated footage.mp4")?;
        let asset = a.id.clone();
        p.production.timeline.assets.insert(asset.clone(), a);
        crate::timeline::apply(
            p,
            &json!({"action":"film_add","asset":asset,"start":v["start"].as_u64().unwrap_or(0)}),
        )?;
    } else {
        let i = v["page"].as_u64().ok_or("Choose a destination page")? as usize;
        if i >= p.pages.len() {
            return Err("Page outside document".into());
        }
        if fs::metadata(&path).map_err(|e| e.to_string())?.len() > 64 * 1024 * 1024 {
            return Err("Generated image exceeds the 64 MB import limit".into());
        }
        let (width, height) = image::image_dimensions(&path).map_err(|e| e.to_string())?;
        if width == 0 || height == 0 || width as u64 * height as u64 > 16_000_000 {
            return Err("Generated image exceeds the 16 megapixel import limit".into());
        }
        let image = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
        let mut e = crate::templates::make_basic_text_element(p.alloc_id());
        e.name = "Generated image".into();
        let scale =
            (p.width * 0.85 / image.width() as f32).min(p.height * 0.85 / image.height() as f32);
        e.width = image.width() as f32 * scale;
        e.height = image.height() as f32 * scale;
        e.x = (p.width - e.width) / 2.;
        e.y = (p.height - e.height) / 2.;
        e.kind = ElementKind::Image {
            source_name: status["result"]["model"]
                .as_str()
                .unwrap_or("Generated image")
                .into(),
            width_px: image.width(),
            height_px: image.height(),
            raw_rgba: image.into_raw().into(),
            mask: ImageMask::None,
            adjustments: Default::default(),
            border_color: [0, 0, 0, 0],
            border_width: 0.,
            revision: 1,
        };
        p.pages[i].elements.push(e);
    }
    Ok(())
}
#[test]
fn catalog_has_pinned_free_choices_and_explicit_research_limits() {
    let c: Value = serde_json::from_str(include_str!("../generation/catalog.json")).unwrap();
    let models = c["models"].as_array().unwrap();
    assert!(models.len() >= 14);
    for m in models {
        assert_eq!(m["revision"].as_str().unwrap().len(), 40);
        assert_eq!(m["weights_free"], true);
    }
    assert!(models
        .iter()
        .any(|m| m["commercial"] == "research" && m["safety_profile"] == "declared_uncensored"));
    assert!(models
        .iter()
        .any(|m| m["commercial"] == "commercial" && m["task"] == "video"));
}
