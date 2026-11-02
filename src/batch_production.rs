use crate::model::Project;
use serde::Serialize;
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn parse_csv(input: &str) -> Result<Vec<Vec<String>>, String> {
    let mut chars = input.trim_start_matches('\u{feff}').chars().peekable();
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut cell = String::new();
    let mut quoted = false;
    let mut closed = false;
    while let Some(ch) = chars.next() {
        if quoted {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    chars.next();
                    cell.push('"');
                } else {
                    quoted = false;
                    closed = true;
                }
            } else {
                cell.push(ch);
            }
            continue;
        }
        match ch {
            '"' if cell.is_empty() && !closed => quoted = true,
            '"' => return Err("Unexpected quote in CSV field".into()),
            ',' => {
                row.push(std::mem::take(&mut cell));
                closed = false;
            }
            '\r' | '\n' => {
                if ch == '\r' && chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut cell));
                rows.push(std::mem::take(&mut row));
                closed = false;
            }
            ' ' | '\t' if closed => {}
            _ if closed => return Err("Unexpected content after a quoted CSV field".into()),
            _ => cell.push(ch),
        }
        if rows.len() > 10_000 || row.len() > 1000 {
            return Err("CSV exceeds the supported batch size".into());
        }
    }
    if quoted {
        return Err("Unclosed quoted CSV field".into());
    }
    if !row.is_empty() || !cell.is_empty() || closed {
        row.push(cell);
        rows.push(row);
    }
    Ok(rows)
}

#[derive(Serialize, Debug)]
pub struct RowResult {
    pub csv_row: usize,
    pub success: bool,
    pub output: Option<String>,
    pub error: Option<String>,
}
#[derive(Serialize, Debug)]
pub struct BatchReport {
    pub folder: PathBuf,
    pub records: Vec<RowResult>,
}

fn filename(value: &str) -> String {
    let name: String = value
        .chars()
        .take(60)
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '-'
            }
        })
        .collect();
    let name = name.trim_matches('-');
    if name.is_empty() {
        "record".into()
    } else {
        name.into()
    }
}

pub fn export_csv(
    master: &Project,
    csv_path: &Path,
    output: &Path,
    editable: bool,
) -> Result<BatchReport, String> {
    if fs::metadata(csv_path).map_err(|e| e.to_string())?.len() > 10_000_000 {
        return Err("CSV must be smaller than 10 MB".into());
    }
    let rows = parse_csv(&fs::read_to_string(csv_path).map_err(|e| e.to_string())?)?;
    let header = rows.first().ok_or("CSV is empty")?;
    let keys: Vec<_> = header.iter().map(|s| s.trim().to_string()).collect();
    let mut seen = std::collections::HashSet::new();
    for key in &keys {
        if !seen.insert(key) || !master.production.fields.contains_key(key) {
            return Err(format!(
                "CSV column '{key}' is duplicated or is not a defined field"
            ));
        }
    }
    if rows.len() < 2 {
        return Err("CSV needs at least one data record".into());
    }
    let folder = output.join(format!("batch-{}", crate::model::generate_document_id()));
    fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let mut report = BatchReport {
        folder: folder.clone(),
        records: Vec::new(),
    };
    for (index, row) in rows.iter().enumerate().skip(1) {
        let result = (|| {
            if row.len() != keys.len() {
                return Err(format!(
                    "Expected {} columns, found {}",
                    keys.len(),
                    row.len()
                ));
            }
            let mut working = master.clone();
            for (key, value) in keys.iter().zip(row) {
                let field = working.production.fields.get_mut(key).unwrap();
                field.value = value.clone();
                if field.image {
                    field.image_png = None;
                    if !value.trim().is_empty() {
                        let path = Path::new(value);
                        let path = if path.is_absolute() {
                            path.to_path_buf()
                        } else {
                            csv_path
                                .parent()
                                .unwrap_or_else(|| Path::new("."))
                                .join(path)
                        };
                        crate::production::attach_image(field, &path)?;
                    }
                }
            }
            let resolved = crate::production::resolve(&working, true)?;
            let preflight = crate::model::pdf_export::run_export_preflight(&resolved, 300);
            if preflight.has_errors() || preflight.has_overflows() {
                return Err(preflight
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
            let label = ["guest_name", "name", "title"]
                .iter()
                .find_map(|key| {
                    working
                        .production
                        .fields
                        .get(*key)
                        .map(|f| f.value.as_str())
                })
                .unwrap_or("record");
            let stem = format!("{index:04}-{}", filename(label));
            crate::model::pdf_export::export_project_to_pdf(
                &resolved,
                &Default::default(),
                &folder.join(format!("{stem}.pdf")),
            )?;
            crate::exporter::render_project_page_at_dpi(&resolved, 0, 96, false)?
                .save(folder.join(format!("{stem}-proof.png")))
                .map_err(|e| e.to_string())?;
            if editable {
                crate::exporter::save_project_json(
                    &working,
                    &folder.join(format!("{stem}.canvas")),
                )?;
            }
            Ok(stem)
        })();
        report.records.push(match result {
            Ok(stem) => RowResult {
                csv_row: index + 1,
                success: true,
                output: Some(stem),
                error: None,
            },
            Err(error) => RowResult {
                csv_row: index + 1,
                success: false,
                output: None,
                error: Some(error),
            },
        });
    }
    fs::write(
        folder.join("report.json"),
        serde_json::to_vec_pretty(&report).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(report)
}
