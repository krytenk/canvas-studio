//! Snapshot-based media jobs. Encoding never holds the editor's document lock.
use crate::{
    model::*,
    timeline::{Clip, MediaAsset},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicUsize, Ordering},
        OnceLock,
    },
};
static ACTIVE: AtomicUsize = AtomicUsize::new(0);
static SCRIPT: OnceLock<Result<PathBuf, String>> = OnceLock::new();
fn script() -> Result<PathBuf, String> {
    SCRIPT
        .get_or_init(|| {
            let dir = crate::media_store::folder("runtime")?.join(env!("CARGO_PKG_VERSION"));
            fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
            let path = dir.join("worker.py");
            let tmp = path.with_extension(format!("{}.tmp", generate_document_id()));
            fs::write(&tmp, include_str!("../media/studio_media.py")).map_err(|e| e.to_string())?;
            fs::rename(tmp, &path).map_err(|e| e.to_string())?;
            for (name, data) in [
                (
                    "studio_motion.py",
                    include_str!("../media/studio_motion.py"),
                ),
                (
                    "Noto-OFL.txt",
                    include_str!("../assets/fonts/LICENSE-OFL.txt"),
                ),
            ] {
                let dest = path.parent().unwrap().join(name);
                let tmp = dest.with_extension(format!("{}.tmp", generate_document_id()));
                fs::write(&tmp, data).map_err(|e| e.to_string())?;
                fs::rename(tmp, dest).map_err(|e| e.to_string())?;
            }
            fs::create_dir_all(path.parent().unwrap().join("vendor")).map_err(|e| e.to_string())?;
            for (name, data) in [
                (
                    "studio_mockup.py",
                    include_str!("../media/studio_mockup.py"),
                ),
                ("mockup-scene.js", include_str!("../web/mockup-scene.js")),
                (
                    "vendor/three.module.js",
                    include_str!("../web/vendor/three.module.js"),
                ),
                (
                    "vendor/three.core.js",
                    include_str!("../web/vendor/three.core.js"),
                ),
                (
                    "vendor/THREE-LICENSE.txt",
                    include_str!("../web/vendor/THREE-LICENSE.txt"),
                ),
            ] {
                let dest = path.parent().unwrap().join(name);
                let temp = dest.with_extension(format!("{}.tmp", generate_document_id()));
                fs::write(&temp, data).map_err(|e| e.to_string())?;
                fs::rename(temp, dest).map_err(|e| e.to_string())?;
            }
            Ok(path)
        })
        .clone()
}
pub fn dependencies() -> Value {
    json!({"python":Command::new("python3").arg("--version").output().is_ok_and(|p|p.status.success()),"ffmpeg":Command::new("ffmpeg").arg("-version").output().is_ok_and(|p|p.status.success()),"ffprobe":Command::new("ffprobe").arg("-version").output().is_ok_and(|p|p.status.success())})
}
pub(crate) fn write(path: &Path, value: &Value) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, serde_json::to_vec(value).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    fs::rename(tmp, path).map_err(|e| e.to_string())
}
pub(crate) fn invoke(folder: &Path, spec: &Value) -> Result<Value, String> {
    write(&folder.join("spec.json"), spec)?;
    let status = Command::new("python3")
        .arg(script()?)
        .arg("--spec")
        .arg(folder.join("spec.json"))
        .arg("--result")
        .arg(folder.join("result.json"))
        .arg("--cancel")
        .arg(folder.join("cancel"))
        .status()
        .map_err(|e| format!("Video tools require Python 3 and FFmpeg/FFprobe: {e}"))?;
    if !status.success() {
        return Err("Media worker stopped; project preserved".into());
    }
    let result = read(&folder.join("result.json"))?;
    if result["state"] != "complete" {
        return Err(result["error"]
            .as_str()
            .unwrap_or("Media operation failed")
            .into());
    }
    Ok(result["result"].clone())
}
fn read(path: &Path) -> Result<Value, String> {
    serde_json::from_slice(&fs::read(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())
}
pub fn sync_worker(spec: Value) -> Result<Value, String> {
    let p = crate::media_store::folder("jobs")?.join(format!("probe-{}", generate_document_id()));
    fs::create_dir(&p).map_err(|e| e.to_string())?;
    let result = invoke(&p, &spec);
    let _ = fs::remove_dir_all(p);
    result
}
fn id_path(id: &str) -> Result<PathBuf, String> {
    if id.len() > 80
        || id.is_empty()
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
    {
        return Err("Invalid media job identity".into());
    }
    Ok(crate::media_store::folder("jobs")?.join(id))
}
pub(crate) fn begin(
    doc: &str,
    revision: u64,
    mode: &str,
    task: impl FnOnce(&Path) -> Result<Value, String> + Send + 'static,
) -> Result<String, String> {
    if storage_bytes(&crate::media_store::root().join("jobs")) > 1024 * 1024 * 1024 {
        return Err("Render history exceeds 1 GiB. Save any movies you need, then clear finished jobs in Film studio".into());
    }
    if ACTIVE.fetch_add(1, Ordering::SeqCst) >= 2 {
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
        return Err(
            "Two media jobs are already running. Wait or cancel one before starting another".into(),
        );
    }
    let id = generate_document_id();
    let folder = match id_path(&id) {
        Ok(p) => p,
        Err(e) => {
            ACTIVE.fetch_sub(1, Ordering::SeqCst);
            return Err(e);
        }
    };
    if let Err(e) = fs::create_dir(&folder) {
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
        return Err(e.to_string());
    }
    if let Err(e) = write(
        &folder.join("meta.json"),
        &json!({"id":id,"document_id":doc,"source_revision":revision,"mode":mode,"owner_pid":std::process::id(),"created_at_ms":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64}),
    ) {
        ACTIVE.fetch_sub(1, Ordering::SeqCst);
        return Err(e);
    }
    std::thread::spawn(move || {
        struct Release;
        impl Drop for Release {
            fn drop(&mut self) {
                ACTIVE.fetch_sub(1, Ordering::SeqCst);
            }
        }
        let _release = Release;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| task(&folder)))
            .unwrap_or_else(|_| Err("Media preparation stopped; project preserved".into()));
        if !folder.join("result.json").exists() {
            let v = match result {
                Ok(v) => json!({"state":"complete","result":v}),
                Err(e) => {
                    json!({"state":if folder.join("cancel").exists(){"cancelled"}else{"failed"},"error":e})
                }
            };
            let _ = write(&folder.join("result.json"), &v);
        }
    });
    Ok(id)
}
pub fn proxy(asset: &MediaAsset, doc: &str, revision: u64) -> Result<String, String> {
    let asset = asset.clone();
    let source = crate::media_store::verify(&asset)?;
    let cache = crate::media_store::folder("cache")?;
    begin(doc, revision, "proxy", move |folder| {
        invoke(
            folder,
            &json!({"op":"proxy","source":source,"asset":asset,"cache":cache,"rebuild":true}),
        )
    })
}
pub fn start(
    project: &Project,
    revision: u64,
    mode: &str,
    frame: Option<u64>,
) -> Result<String, String> {
    if mode == "source" || crate::timeline::has_motion(&project.production.timeline) {
        return crate::motion::start(project, revision, mode, frame);
    }
    if !["preview", "export", "frame"].contains(&mode) {
        return Err("Choose preview, export or frame".into());
    }
    crate::timeline::validate(&project.production.timeline)?;
    if !project.production.timeline.enabled {
        return Err("Create a film timeline first".into());
    }
    if (mode == "frame") != frame.is_some() {
        return Err("Frame jobs require a frame; movie jobs do not".into());
    }
    if frame.is_some_and(|f| f >= project.production.timeline.duration) {
        return Err("Playhead is outside the film".into());
    }
    let snapshot = project.clone();
    let mode = mode.to_string();
    begin(
        &project.document_id,
        revision,
        &mode.clone(),
        move |folder| {
            write(
                &folder.join("progress.json"),
                &json!({"state":"running","phase":"preparing","progress":0}),
            )?;
            let t = &snapshot.production.timeline;
            let mut sources = BTreeMap::new();
            for c in &t.clips {
                if folder.join("cancel").exists() {
                    return Err("Cancelled before encoding; project preserved".into());
                }
                if !t.tracks.iter().any(|tr| tr.id == c.track && tr.enabled) {
                    continue;
                }
                if frame.is_some_and(|f| f < c.start || f >= c.start + c.duration) {
                    continue;
                }
                let path = if let Some(id) = &c.asset {
                    let a = t.assets.get(id).ok_or("Missing clip media")?;
                    let original = crate::media_store::verify(a)?;
                    let cache = crate::media_store::cached(a, "proxy")?;
                    if mode != "export" && cache.is_file() {
                        cache
                    } else {
                        original
                    }
                } else {
                    let file = folder.join(format!("graphic-{}.png", c.id));
                    render_graphic(&snapshot, c, &file)?;
                    file
                };
                sources.insert(c.id.clone(), path);
            }
            let output = folder.join(if frame.is_some() {
                "frame.png"
            } else {
                "movie.mp4"
            });
            invoke(
                folder,
                &json!({"op":"render","mode":mode,"timeline":t,"sources":sources,"output":output,"frame":frame}),
            )
        },
    )
}
pub(crate) fn render_graphic(project: &Project, c: &Clip, path: &Path) -> Result<(), String> {
    let t = &project.production.timeline;
    let mut p = Project::default();
    p.imported_fonts = project.imported_fonts.clone();
    p.width = t.width as f32;
    p.height = t.height as f32;
    p.width_mm = p.width / 96. * 25.4;
    p.height_mm = p.height / 96. * 25.4;
    p.format = DesignFormat::Custom(t.width, t.height);
    if let Some(design) = &c.design {
        let mut source = project.clone();
        if let Some(name) = &design.variant {
            crate::campaign::activate(&mut source, name)?;
        }
        let page = source
            .pages
            .iter()
            .position(|p| p.id == design.page_id)
            .ok_or(
                "The referenced design page is missing. Restore it or add the current design again",
            )?;
        let resolved = crate::production::resolve_page(&source, page, true)?.into_owned();
        p.width = source.width;
        p.height = source.height;
        p.width_mm = source.width_mm;
        p.height_mm = source.height_mm;
        p.format = source.format;
        p.pages = vec![resolved];
        check_graphic(&p)?;
        let ratio = (t.width as f32 / p.width).min(t.height as f32 / p.height);
        let w = (p.width * ratio).round().max(1.) as u32;
        let h = (p.height * ratio).round().max(1.) as u32;
        if let Ok(svg) = crate::exporter::export_project_page_to_svg(&p, 0, design.transparent) {
            let _ = fs::write(path.with_extension("svg"), svg);
        }
        crate::exporter::render_page_to_rgba_image(
            &p.pages[0],
            p.width,
            p.height,
            w,
            h,
            design.transparent,
        )
        .save(path)
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    let style = c.caption.as_ref().ok_or("Caption style missing")?;
    let mut page = CanvasPage::new(1, "Film caption");
    let mut box_el = crate::templates::make_basic_text_element(2);
    box_el.x = style.x;
    box_el.y = style.y;
    box_el.width = style.width;
    box_el.height = style.height;
    box_el.name = "Caption background".into();
    box_el.kind = ElementKind::Shape {
        shape_type: ShapeType::RoundedRect,
        fill: FillStyle::Solid(style.box_color),
        stroke_color: [0, 0, 0, 0],
        stroke_width: 0.,
        corner_radius: 8.,
    };
    page.elements.push(box_el);
    let mut text = crate::templates::make_basic_text_element(3);
    let padx = (style.width * 0.03).min(12.);
    let pady = (style.height * 0.10).min(8.);
    text.x = style.x + padx;
    text.y = style.y + pady;
    text.width = (style.width - padx * 2.).max(1.);
    text.height = (style.height - pady * 2.).max(1.);
    text.name = "Film caption".into();
    if let ElementKind::Text {
        content,
        font_family,
        font_size,
        color,
        bold,
        align,
        ..
    } = &mut text.kind
    {
        *content = c.text.clone().unwrap_or_default();
        *font_family = style.family;
        *font_size = style.size;
        *color = style.color;
        *bold = true;
        *align = style.align;
    }
    page.elements.push(text);
    p.pages = vec![page];
    p.production.fields = project.production.fields.clone();
    let resolved = crate::production::resolve(&p, true)?;
    check_graphic(&resolved)?;
    if let Ok(svg) = crate::exporter::export_project_page_to_svg(&resolved, 0, true) {
        let _ = fs::write(path.with_extension("svg"), svg);
    }
    crate::exporter::render_page_to_rgba_image(
        &resolved.pages[0],
        p.width,
        p.height,
        t.width,
        t.height,
        true,
    )
    .save(path)
    .map_err(|e| e.to_string())
}
fn check_graphic(p: &Project) -> Result<(), String> {
    crate::imported_fonts::register_project(p)?;
    let report = crate::model::pdf_export::run_export_preflight(p, 96);
    if report.has_errors() || report.has_overflows() {
        return Err(report
            .issues
            .iter()
            .filter(|i| {
                i.severity == crate::model::pdf_export::PreflightSeverity::Error
                    || i.category == "text_overflow"
            })
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    Ok(())
}
pub fn status(id: &str, doc: &str) -> Result<Value, String> {
    let folder = id_path(id)?;
    let meta = read(&folder.join("meta.json"))?;
    if meta["document_id"].as_str() != Some(doc) {
        return Err("This job belongs to a different document".into());
    }
    let mut value = meta;
    let result = if folder.join("result.json").is_file() {
        read(&folder.join("result.json"))?
    } else if folder.join("progress.json").is_file() {
        read(&folder.join("progress.json"))?
    } else {
        json!({"state":"running","phase":"preparing","progress":0})
    };
    let owner_live = meta_pid_live(value["owner_pid"].as_u64());
    let worker_live = meta_pid_live(result["worker_pid"].as_u64());
    let result = if !folder.join("result.json").exists() && !owner_live && !worker_live {
        json!({"state":"failed","error":"Previous rendering session stopped. The saved edit is intact; start another render."})
    } else {
        result
    };
    for (key, v) in result.as_object().ok_or("Invalid media job state")? {
        value[key] = v.clone();
    }
    if let Some(r) = value.get_mut("result").and_then(Value::as_object_mut) {
        r.remove("output");
        r.remove("proxy");
    }
    Ok(value)
}
pub fn cancel(id: &str, doc: &str) -> Result<(), String> {
    let state = status(id, doc)?;
    if state["state"] == "running" {
        fs::write(id_path(id)?.join("cancel"), b"cancel").map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn output(id: &str, doc: &str) -> Result<PathBuf, String> {
    let state = status(id, doc)?;
    if state["state"] != "complete" {
        return Err("The media job has not completed".into());
    }
    if ![
        "preview",
        "export",
        "frame",
        "source",
        "mockup_poster",
        "mockup_movie",
    ]
    .contains(&state["mode"].as_str().unwrap_or(""))
    {
        return Err("This job has no movie/frame output".into());
    }
    Ok(id_path(id)?.join(if state["mode"] == "source" {
        "motion-source.zip"
    } else if state["mode"] == "frame" || state["mode"] == "mockup_poster" {
        "frame.png"
    } else {
        "movie.mp4"
    }))
}
pub fn jobs(doc: &str) -> Result<Value, String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(crate::media_store::folder("jobs")?).map_err(|e| e.to_string())? {
        let p = entry.map_err(|e| e.to_string())?.path();
        if let Some(id) = p.file_name().and_then(|s| s.to_str()) {
            if let Ok(state) = status(id, doc) {
                out.push(state);
            }
        }
    }
    out.sort_by(|a, b| {
        b["created_at_ms"]
            .as_u64()
            .cmp(&a["created_at_ms"].as_u64())
    });
    out.truncate(30);
    Ok(json!({"jobs":out}))
}

fn meta_pid_live(pid: Option<u64>) -> bool {
    pid.is_some_and(|p| Path::new("/proc").join(p.to_string()).exists())
}
fn storage_bytes(path: &Path) -> u64 {
    fs::read_dir(path)
        .map(|entries| {
            entries
                .filter_map(Result::ok)
                .map(|e| {
                    let p = e.path();
                    if p.is_dir() {
                        storage_bytes(&p)
                    } else {
                        e.metadata().map(|m| m.len()).unwrap_or(0)
                    }
                })
                .sum()
        })
        .unwrap_or(0)
}
pub fn storage() -> Value {
    json!({"originals_bytes":storage_bytes(&crate::media_store::root().join("originals")),"cache_bytes":storage_bytes(&crate::media_store::root().join("cache")),"jobs_bytes":storage_bytes(&crate::media_store::root().join("jobs")),"jobs_soft_limit_bytes":1024u64*1024*1024})
}
pub fn clear_finished(doc: &str) -> Result<usize, String> {
    let mut count = 0;
    for e in fs::read_dir(crate::media_store::folder("jobs")?).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        if let Some(id) = e.file_name().to_str() {
            if let Ok(s) = status(id, doc) {
                if s["state"] != "running" {
                    fs::remove_dir_all(e.path()).map_err(|e| e.to_string())?;
                    count += 1;
                }
            }
        }
    }
    Ok(count)
}

pub fn source_output(id: &str, doc: &str) -> Result<PathBuf, String> {
    let state = status(id, doc)?;
    if state["state"] != "complete" || state["result"]["source_available"] != true {
        return Err("This job has no completed motion source".into());
    }
    let p = id_path(id)?.join("motion-source.zip");
    if !p.is_file() {
        return Err("Motion source is missing".into());
    }
    Ok(p)
}
