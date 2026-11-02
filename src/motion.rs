//! Typed motion → a local, self-contained HyperFrames project and snapshot job.
use crate::{model::Project, timeline};
use serde_json::{json, Value};
use std::{fs, path::PathBuf};
pub const HF_VERSION: &str = "0.8.206";
pub fn cli() -> Result<PathBuf, String> {
    let relative = "tools/hyperframes/node_modules/hyperframes/bin/hyperframes.mjs";
    let candidates = [
        std::env::var_os("CANVAS_STUDIO_HYPERFRAMES").map(PathBuf::from),
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|p| p.join(relative))),
        Some(
            crate::application_data_dir()
                .join("canvas-studio/motion-tools/node_modules/hyperframes/bin/hyperframes.mjs"),
        ),
    ];
    candidates.into_iter().flatten().find(|p|p.is_file()).ok_or("Motion tools are not installed. Run python3 scripts/setup_motion.py from the app folder (Node 22+ and FFmpeg required)".into())
}
pub fn doctor() -> Value {
    json!({"installed":cli().is_ok(),"hyperframes_version":HF_VERSION,"node_available":std::process::Command::new("node").arg("--version").output().is_ok_and(|r|r.status.success())})
}
pub fn start(
    project: &Project,
    revision: u64,
    mode: &str,
    frame: Option<u64>,
) -> Result<String, String> {
    if !["preview", "export", "frame", "source"].contains(&mode)
        || (mode == "frame") != frame.is_some()
    {
        return Err("Choose preview/export/source, or frame with a frame position".into());
    }
    timeline::validate(&project.production.timeline)?;
    if !project.production.timeline.enabled {
        return Err("Create a movie first".into());
    }
    if frame.is_some_and(|f| f >= project.production.timeline.duration) {
        return Err("Frame is outside the movie".into());
    }
    let cli = cli()?;
    let snapshot = project.clone();
    let mode = mode.to_string();
    crate::media_service::begin(
        &project.document_id,
        revision,
        &mode.clone(),
        move |folder| {
            let project_dir = folder.join("source");
            fs::create_dir_all(project_dir.join("assets")).map_err(|e| e.to_string())?;
            let mut sources = serde_json::Map::new();
            for c in &snapshot.production.timeline.clips {
                if folder.join("cancel").exists() {
                    return Err("Motion preparation cancelled; edit preserved".into());
                }
                if !snapshot
                    .production
                    .timeline
                    .tracks
                    .iter()
                    .any(|t| t.id == c.track && t.enabled)
                {
                    continue;
                }
                if frame.is_some_and(|f| f < c.start || f >= c.start + c.duration) {
                    continue;
                }
                let path = if let Some(id) = &c.asset {
                    crate::media_store::verify(
                        snapshot
                            .production
                            .timeline
                            .assets
                            .get(id)
                            .ok_or("Missing source media")?,
                    )?
                } else {
                    let path = project_dir.join(format!("assets/graphic-{}.png", c.id));
                    crate::media_service::render_graphic(&snapshot, c, &path)?;
                    path
                };
                sources.insert(c.id.clone(), json!(path));
            }
            if mode != "frame" {
                crate::project_package::save(
                    &snapshot,
                    &project_dir.join("editable-master.canvas"),
                )?;
            }
            crate::media_service::write(
                &folder.join("progress.json"),
                &json!({"state":"running","phase":"preparing motion","progress":0}),
            )?;
            crate::media_service::invoke(
                folder,
                &json!({"op":"motion","timeline":snapshot.production.timeline,"sources":sources,"directory":project_dir,"cli":cli,"mode":mode,"frame":frame,"document_id":snapshot.document_id,"source_revision":revision,"engine_version":HF_VERSION,"studio_version":env!("CARGO_PKG_VERSION")}),
            )
        },
    )
}
