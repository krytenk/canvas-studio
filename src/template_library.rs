//! Local production masters. Every open creates an independent, unsaved copy.
use crate::{
    exporter::{load_project_json, render_project_page_at_dpi, save_project_json},
    model::{generate_document_id, Project},
};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TemplateEntry {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub tags: Vec<String>,
    pub created_at: String,
    pub format: String,
    pub page_count: usize,
}

impl TemplateEntry {
    pub fn matches(&self, query: &str) -> bool {
        let query = query.trim().to_lowercase();
        query.is_empty()
            || self.name.to_lowercase().contains(&query)
            || self.tags.iter().any(|t| t.to_lowercase().contains(&query))
    }
}

fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() < 128 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-')
}

pub fn list(root: &Path) -> Result<(Vec<TemplateEntry>, Vec<String>), String> {
    if !root.exists() {
        return Ok((Vec::new(), Vec::new()));
    }
    let mut entries = Vec::new();
    let mut warnings = Vec::new();
    for item in fs::read_dir(root).map_err(|e| e.to_string())? {
        let item = item.map_err(|e| e.to_string())?;
        let id = item.file_name().to_string_lossy().into_owned();
        if !valid_id(&id) || !item.file_type().map_err(|e| e.to_string())?.is_dir() {
            continue;
        }
        let result = fs::read(item.path().join("metadata.json"))
            .map_err(|e| e.to_string())
            .and_then(|bytes| {
                serde_json::from_slice::<TemplateEntry>(&bytes).map_err(|e| e.to_string())
            });
        match result {
            Ok(entry) if entry.schema_version == 1 && entry.id == id => entries.push(entry),
            Ok(_) => warnings.push(format!("Template {id}: unsupported or mismatched metadata")),
            Err(e) => warnings.push(format!("Template {id}: {e}")),
        }
    }
    entries.sort_by(|a, b| b.created_at.cmp(&a.created_at).then(a.name.cmp(&b.name)));
    Ok((entries, warnings))
}

pub fn save(
    root: &Path,
    name: &str,
    tags: &str,
    project: &Project,
) -> Result<TemplateEntry, String> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 160 {
        return Err("Use a template name of 1–160 characters".into());
    }
    // Production masters must be exportable; ordinary project saves remain unrestricted.
    let preview = render_project_page_at_dpi(project, 0, 96, false)?;
    let entry = TemplateEntry {
        schema_version: 1,
        id: generate_document_id(),
        name: name.into(),
        tags: tags
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect(),
        created_at: chrono::Utc::now().to_rfc3339(),
        format: project.format.label().into(),
        page_count: project.pages.len(),
    };
    let report = crate::model::pdf_export::run_export_preflight(project, 96);
    if report.has_errors() {
        return Err(report
            .issues
            .iter()
            .filter(|i| i.severity == crate::model::pdf_export::PreflightSeverity::Error)
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    fs::create_dir_all(root).map_err(|e| e.to_string())?;
    let staging = root.join(format!(".staging-{}", entry.id));
    let result = (|| {
        let mut master = project.clone();
        master.document_id = generate_document_id();
        master.recovery_origin_path = None;
        master.title = name.into();
        save_project_json(&master, &staging.join("project.canvas.json"))?;
        image::imageops::thumbnail(&preview, 320, 320)
            .save(staging.join("preview.png"))
            .map_err(|e| e.to_string())?;
        fs::write(
            staging.join("metadata.json"),
            serde_json::to_vec_pretty(&entry).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        fs::rename(&staging, root.join(&entry.id)).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if let Err(error) = result {
        let _ = fs::remove_dir_all(staging);
        return Err(error);
    }
    Ok(entry)
}

pub fn open_copy(root: &Path, id: &str) -> Result<Project, String> {
    if !valid_id(id) {
        return Err("Invalid template identifier".into());
    }
    let mut project = load_project_json(&root.join(id).join("project.canvas.json"))?;
    project.document_id = generate_document_id();
    project.recovery_origin_path = None;
    Ok(project)
}
