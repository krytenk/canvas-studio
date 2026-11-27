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
        return Err("Unknown generation field; references must come from a project page".into());
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
