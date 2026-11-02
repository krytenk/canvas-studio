//! Loopback browser editing using the native model, typography and export engine.
use crate::model::{FontFamily, TextRun};
use crate::*;
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    fs,
    io::{Read, Write},
    net::{TcpListener, TcpStream},
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Duration,
};
const LIMIT: usize = 128 * 1024 * 1024;
struct Scratch(PathBuf);
impl Scratch {
    fn new() -> Result<Self, String> {
        let p =
            std::env::temp_dir().join(format!("canvas-browser-{}", model::generate_document_id()));
        fs::create_dir(&p).map_err(|e| e.to_string())?;
        Ok(Self(p))
    }
}
impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}
fn validate_document(project: &Project) -> Result<(), String> {
    crate::campaign::validate(project)?;
    crate::timeline::validate(&project.production.timeline)?;
    if project.pages.is_empty()
        || project.pages.len() > 1000
        || [
            project.width,
            project.height,
            project.effective_width_mm(),
            project.effective_height_mm(),
        ]
        .iter()
        .any(|v| !v.is_finite() || *v <= 0.)
    {
        return Err("Use a document with positive dimensions and 1–1000 pages".into());
    }
    Ok(())
}
fn check_delivery(project: &Project) -> Result<(), String> {
    let report = run_export_preflight(project, 300);
    if report.has_errors() || report.has_overflows() {
        return Err(report
            .issues
            .iter()
            .filter(|i| {
                i.severity == model::pdf_export::PreflightSeverity::Error
                    || i.category == "text_overflow"
            })
            .map(|i| i.message.as_str())
            .collect::<Vec<_>>()
            .join("; "));
    }
    Ok(())
}
struct Session {
    project: Project,
    revision: u64,
    undo: Vec<Project>,
    redo: Vec<Project>,
    recovery: Option<PathBuf>,
    last_actor: String,
}
impl Session {
    fn commit(&mut self, candidate: Project) -> Result<(), String> {
        validate_document(&candidate)?;
        crate::imported_fonts::register_project(&candidate)?;
        if let Some(path) = &self.recovery {
            crate::project_package::save_recovery(&candidate, path)?;
        }
        self.undo.push(self.project.clone());
        if self.undo.len() > 20 {
            self.undo.remove(0);
        }
        self.redo.clear();
        self.project = candidate;
        self.revision += 1;
        Ok(())
    }
    fn travel(&mut self, redo: bool) -> Result<(), String> {
        let source = if redo { &self.redo } else { &self.undo };
        let Some(candidate) = source.last().cloned() else {
            return Err("No history in that direction".into());
        };
        if let Some(path) = &self.recovery {
            crate::project_package::save_recovery(&candidate, path)?;
        }
        let prior = std::mem::replace(&mut self.project, candidate);
        if redo {
            self.redo.pop();
            self.undo.push(prior);
        } else {
            self.undo.pop();
            self.redo.push(prior);
        }
        self.revision += 1;
        Ok(())
    }
}
struct Request {
    method: String,
    path: String,
    headers: HashMap<String, String>,
    body: Vec<u8>,
}
struct HttpError {
    status: u16,
    message: String,
}
impl From<String> for HttpError {
    fn from(message: String) -> Self {
        Self {
            status: 400,
            message,
        }
    }
}
impl From<&str> for HttpError {
    fn from(message: &str) -> Self {
        message.to_string().into()
    }
}
fn request(stream: &mut TcpStream, gate: Option<(&str, &str)>) -> Result<Request, HttpError> {
    stream
        .set_read_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| e.to_string())?;
    let mut data = Vec::new();
    let end;
    loop {
        let mut b = [0; 8192];
        let n = stream.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Incomplete request".into());
        }
        data.extend_from_slice(&b[..n]);
        if let Some(index) = data.windows(4).position(|w| w == b"\r\n\r\n") {
            end = index + 4;
            break;
        }
        if data.len() > 32768 {
            return Err("Headers too large".into());
        }
    }
    if end > 32768 {
        return Err("Headers too large".into());
    }
    let raw = std::str::from_utf8(&data[..end]).map_err(|_| "Invalid request headers")?;
    let mut lines = raw.split("\r\n");
    let first = lines
        .next()
        .unwrap_or("")
        .split_whitespace()
        .collect::<Vec<_>>();
    if first.len() != 3 || first[2] != "HTTP/1.1" {
        return Err("Use HTTP/1.1".into());
    }
    let method = first[0].to_string();
    let path = first[1].to_string();
    let mut headers = HashMap::new();
    for line in lines.filter(|l| !l.is_empty()) {
        let (key, value) = line.split_once(':').ok_or("Invalid header")?;
        let key = key.trim().to_ascii_lowercase();
        if headers.insert(key, value.trim().to_string()).is_some() {
            return Err("Duplicate request header".into());
        }
    }
    if headers.contains_key("transfer-encoding") {
        return Err("Chunked requests are not supported".into());
    }
    let size = headers
        .get("content-length")
        .map(|v| v.parse::<usize>())
        .transpose()
        .map_err(|_| "Invalid Content-Length")?
        .unwrap_or(0);
    if size > LIMIT {
        return Err("Upload exceeds 128 MiB".into());
    }
    if let Some((host, token)) = gate {
        let static_page = method == "GET"
            && [
                "/",
                "/app.js",
                "/studio.js",
                "/layout.js",
                "/bundle.js",
                "/film.js",
                "/motion.js",
                "/mockups.js",
                "/mockup-scene.js",
                "/vendor/three.module.js",
                "/vendor/three.core.js",
                "/mockups.css",
                "/style.css",
            ]
            .contains(&path.as_str());
        if headers.get("host").map(String::as_str) != Some(host)
            || headers
                .get("origin")
                .is_some_and(|origin| origin != &format!("http://{host}"))
            || (!static_page && headers.get("x-canvas-token").map(String::as_str) != Some(token))
        {
            return Err(HttpError {
                status: 403,
                message: "Open the private local session link; cross-origin requests are disabled"
                    .into(),
            });
        }
    }
    while data.len() < end + size {
        let mut b = [0; 65536];
        let n = stream.read(&mut b).map_err(|e| e.to_string())?;
        if n == 0 {
            return Err("Incomplete upload".into());
        }
        data.extend_from_slice(&b[..n]);
    }
    Ok(Request {
        method,
        path,
        headers,
        body: data[end..end + size].to_vec(),
    })
}
struct Response {
    status: u16,
    mime: &'static str,
    body: Vec<u8>,
    download: Option<&'static str>,
    revision: u64,
}
impl Response {
    fn bytes(mime: &'static str, body: Vec<u8>, revision: u64) -> Self {
        Self {
            status: 200,
            mime,
            body,
            download: None,
            revision,
        }
    }
    fn value(value: Value, revision: u64) -> Self {
        Self::bytes(
            "application/json",
            serde_json::to_vec(&value).unwrap(),
            revision,
        )
    }
    fn error(status: u16, message: &str) -> Self {
        let mut response = Self::value(json!({"error":message}), 0);
        response.status = status;
        response
    }
}
fn send(stream: &mut TcpStream, r: Response) {
    let reason = match r.status {
        200 => "OK",
        400 => "Bad Request",
        403 => "Forbidden",
        404 => "Not Found",
        409 => "Conflict",
        _ => "Unprocessable Content",
    };
    let download = r
        .download
        .map(|n| format!("Content-Disposition: attachment; filename=\"{n}\"\r\n"))
        .unwrap_or_default();
    let header=format!("HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nCross-Origin-Resource-Policy: same-origin\r\nX-Canvas-Revision: {}\r\nContent-Security-Policy: default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; font-src 'self' data:; media-src 'self' blob:; connect-src 'self'; object-src 'none'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'\r\n{}Connection: close\r\n\r\n",r.status,reason,r.mime,r.body.len(),r.revision,download);
    let _ = stream.set_write_timeout(Some(Duration::from_secs(60)));
    let _ = stream
        .write_all(header.as_bytes())
        .and_then(|_| stream.write_all(&r.body));
}
fn number(v: &Value, key: &str) -> Result<f32, String> {
    let n = v
        .get(key)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("Missing number: {key}"))? as f32;
    if !n.is_finite() {
        return Err("Numbers must be finite".into());
    }
    Ok(n)
}
fn id(v: &Value, key: &str) -> Result<u64, String> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or("Missing layer/page identity")?
        .parse()
        .map_err(|_| "Invalid identity".into())
}
fn string<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("Missing text: {key}"))
}
fn color(v: &Value) -> Result<ColorRgba, String> {
    let s = string(v, "color")?;
    if s.len() != 7 || !s.starts_with('#') {
        return Err("Use #rrggbb".into());
    }
    let value = u32::from_str_radix(&s[1..], 16).map_err(|_| "Invalid colour")?;
    Ok(rgb((value >> 16) as u8, (value >> 8) as u8, value as u8))
}
fn family(f: FontFamily) -> String {
    match f {
        FontFamily::Imported(id) => format!("imported:{id:016x}"),
        _ => serde_json::to_value(f)
            .unwrap()
            .as_str()
            .unwrap()
            .to_string(),
    }
}
fn parse_family(project: &Project, key: &str) -> Result<FontFamily, String> {
    if let Some(id) = key.strip_prefix("imported:") {
        let id = u64::from_str_radix(id, 16).map_err(|_| "Invalid font")?;
        if !project.imported_fonts.iter().any(|f| f.id == id) {
            return Err("Import the font face first".into());
        }
        Ok(FontFamily::Imported(id))
    } else {
        serde_json::from_value(json!(key)).map_err(|_| "Unknown font".into())
    }
}
fn element_mut<'a>(p: &'a mut Project, v: &Value) -> Result<&'a mut CanvasElement, String> {
    let id = id(v, "id")?;
    let e = p
        .pages
        .iter_mut()
        .flat_map(|p| &mut p.elements)
        .find(|e| e.id == id)
        .ok_or("Layer not found")?;
    if e.locked {
        return Err("Unlock the layer before editing".into());
    }
    Ok(e)
}
fn edit(project: &mut Project, v: &Value) -> Result<(), String> {
    let action = string(v, "action")?;
    if crate::layout_commands::supports(action) {
        return crate::layout_commands::apply(project, v);
    }
    if matches!(action, "text" | "shape") {
        let layer = id(v, "id")?;
        if let Some(bindings) = project.production.studio.colors.get(&layer) {
            for (property, target) in if action == "text" {
                vec![("color", "text")]
            } else {
                vec![("color", "fill"), ("stroke_color", "stroke")]
            } {
                if v.get(property).is_some() && bindings.contains_key(target) {
                    return Err(format!(
                        "Unlink the named {target} colour before applying a local override"
                    ));
                }
            }
        }
    }
    match action {
        "title" => {
            let value = string(v, "value")?;
            if value.len() > 1000 {
                return Err("Title too long".into());
            }
            project.title = value.into();
        }
        "field" => {
            let key = string(v, "key")?;
            let value = string(v, "value")?;
            if value.len() > 100_000 {
                return Err("Field too long".into());
            }
            let field = project
                .production
                .fields
                .get_mut(key)
                .ok_or("Field not found")?;
            if field.image {
                return Err("Use photo upload for this field".into());
            }
            field.value = value.into();
        }
        "text" => {
            let f = v
                .get("family")
                .and_then(Value::as_str)
                .map(|s| parse_family(project, s))
                .transpose()?;
            let actual = f.and_then(|family| {
                if let FontFamily::Imported(id) = family {
                    project
                        .imported_fonts
                        .iter()
                        .find(|font| font.id == id)
                        .map(|font| (font.bold, font.italic))
                } else {
                    None
                }
            });
            let unit = project.effective_units_per_inch() / 72.;
            let e = element_mut(project, v)?;
            let ElementKind::Text {
                content,
                font_family,
                font_size,
                color: ink,
                bold,
                italic,
                underline,
                line_height,
                letter_spacing,
                align,
                uppercase,
                ..
            } = &mut e.kind
            else {
                return Err("Select a text layer".into());
            };
            if let Some(new) = v.get("content").and_then(Value::as_str) {
                if new.len() > 100_000 {
                    return Err("Text too long".into());
                }
                e.text_format.reconcile_edit(content, new);
                *content = new.into();
            }
            if e.style_id.is_some() {
                if v.as_object().is_some_and(|v| {
                    v.keys().any(|k| {
                        ![
                            "action", "id", "content", "ref", "page", "name", "x", "y", "width",
                            "height", "rotation",
                        ]
                        .contains(&k.as_str())
                    })
                }) {
                    return Err("Edit linked typography styles on desktop; content-only edits are supported here".into());
                }
                return Ok(());
            }
            if let Some(f) = f {
                *font_family = f;
            }
            if v.get("size").is_some() {
                let size = number(v, "size")?;
                if !(1.0..=300.).contains(&size) {
                    return Err("Type must be 1–300 pt".into());
                }
                *font_size = size * unit;
            }
            if v.get("color").is_some() {
                *ink = color(v)?;
            }
            for (key, target) in [
                ("bold", &mut *bold),
                ("italic", &mut *italic),
                ("underline", &mut *underline),
            ] {
                if let Some(value) = v.get(key).and_then(Value::as_bool) {
                    *target = value;
                }
            }
            if let Some((actual_bold, actual_italic)) = actual {
                *bold = actual_bold;
                *italic = actual_italic;
            }
            if v.get("leading").is_some() {
                let value = number(v, "leading")?;
                if !(0.5..=5.).contains(&value) {
                    return Err("Leading must be 0.5–5".into());
                }
                *line_height = value;
            }
            if v.get("tracking").is_some() {
                let value = number(v, "tracking")?;
                if !(-20.0..=100.).contains(&value) {
                    return Err("Tracking outside supported range".into());
                }
                *letter_spacing = value;
            }
            for (key, target) in [
                ("space_before", &mut e.text_format.space_before),
                ("space_after", &mut e.text_format.space_after),
                ("first_line_indent", &mut e.text_format.first_line_indent),
                ("inset_left", &mut e.text_format.inset_left),
                ("inset_right", &mut e.text_format.inset_right),
                ("inset_top", &mut e.text_format.inset_top),
                ("inset_bottom", &mut e.text_format.inset_bottom),
            ] {
                if v.get(key).is_some() {
                    let value = number(v, key)?;
                    if !(0.0..=300.).contains(&value) {
                        return Err("Paragraph measurements must be 0–300 pt".into());
                    }
                    *target = value;
                }
            }
            if let Some(value) = v.get("align") {
                *align = serde_json::from_value(value.clone())
                    .map_err(|_| "Use Left, Center or Right alignment")?;
            }
            if let Some(value) = v.get("uppercase").and_then(Value::as_bool) {
                *uppercase = value;
            }
            e.text_format.validate(content)?;
        }
        "emphasis" => {
            let e = element_mut(project, v)?;
            let ElementKind::Text { content, .. } = &e.kind else {
                return Err("Select text".into());
            };
            let start = v["start"].as_u64().ok_or("Missing selection start")? as usize;
            let end = v["end"].as_u64().ok_or("Missing selection end")? as usize;
            let property = string(v, "property")?;
            let value = v["value"].as_bool().ok_or("Missing emphasis value")?;
            let mut run = TextRun {
                start,
                end,
                ..Default::default()
            };
            match property {
                "bold" => run.bold = Some(value),
                "italic" => run.italic = Some(value),
                "underline" => run.underline = Some(value),
                _ => return Err("Unknown emphasis".into()),
            }
            e.text_format.apply_run(run, content.chars().count());
        }
        "geometry" => {
            let e = element_mut(project, v)?;
            let values = [
                number(v, "x")?,
                number(v, "y")?,
                number(v, "width")?,
                number(v, "height")?,
            ];
            if values.iter().any(|v| v.abs() > 1_000_000.) {
                return Err("Geometry outside supported range".into());
            }
            document_ops::set_geometry(e, values, [true; 4], 1., false)?;
            if v.get("rotation").is_some() {
                e.rotation = number(v, "rotation")? % 360.;
            }
        }
        "visible" | "lock" => {
            let id = id(v, "id")?;
            let e = project
                .pages
                .iter_mut()
                .flat_map(|p| &mut p.elements)
                .find(|e| e.id == id)
                .ok_or("Layer not found")?;
            let value = v["value"].as_bool().ok_or("Missing value")?;
            if action == "lock" {
                e.locked = value;
            } else {
                if e.locked {
                    return Err("Unlock layer first".into());
                }
                e.visible = value;
            }
        }
        "delete" => {
            let e = element_mut(project, v)?;
            let id = e.id;
            for p in &mut project.pages {
                p.elements.retain(|e| e.id != id);
            }
        }
        "duplicate" => {
            let e = element_mut(project, v)?.clone();
            let page = project
                .pages
                .iter()
                .position(|p| p.elements.iter().any(|a| a.id == e.id))
                .unwrap();
            let copies = document_ops::fresh_copies(project, &[e], 24., 24.);
            project.pages[page].elements.extend(copies);
        }
        "page_title" => {
            let id = id(v, "id")?;
            let title = string(v, "value")?;
            project
                .pages
                .iter_mut()
                .find(|p| p.id == id)
                .ok_or("Page not found")?
                .title = title.into();
        }
        "add_page" => {
            let id = project.alloc_id();
            project.pages.push(CanvasPage::new(id, "New page"));
        }
        "delete_page" => {
            if project.pages.len() < 2 {
                return Err("Keep at least one page".into());
            }
            let id = id(v, "id")?;
            let index = project
                .pages
                .iter()
                .position(|p| p.id == id)
                .ok_or("Page not found")?;
            project.pages.remove(index);
        }
        "move_page" => {
            let id = id(v, "id")?;
            let from = project
                .pages
                .iter()
                .position(|p| p.id == id)
                .ok_or("Page not found")?;
            let to = v["to"].as_u64().ok_or("Missing page position")? as usize;
            if to >= project.pages.len() {
                return Err("Page outside document".into());
            }
            let p = project.pages.remove(from);
            project.pages.insert(to, p);
        }
        "background" => {
            let page = v["page"].as_u64().ok_or("Missing page")? as usize;
            let p = project.pages.get_mut(page).ok_or("Page outside document")?;
            p.background = PageBackground::Solid(color(v)?);
        }
        "layer" => {
            let e = element_mut(project, v)?;
            if let Some(name) = v.get("name").and_then(Value::as_str) {
                if name.len() > 1000 {
                    return Err("Layer name too long".into());
                }
                e.name = name.into();
            }
            if v.get("opacity").is_some() {
                let n = number(v, "opacity")?;
                if !(0.0..=1.).contains(&n) {
                    return Err("Opacity must be 0–1".into());
                }
                e.opacity = n;
            }
        }
        "reorder_layer" => {
            let layer = element_mut(project, v)?.id;
            let p = project
                .pages
                .iter_mut()
                .find(|p| p.elements.iter().any(|e| e.id == layer))
                .unwrap();
            let from = p.elements.iter().position(|e| e.id == layer).unwrap();
            let to = v["to"].as_u64().ok_or("Missing layer position")? as usize;
            if to >= p.elements.len() {
                return Err("Layer position outside page".into());
            }
            let e = p.elements.remove(from);
            p.elements.insert(to, e);
        }
        "shape" => {
            let e = element_mut(project, v)?;
            let ElementKind::Shape {
                fill,
                stroke_color,
                stroke_width,
                corner_radius,
                shape_type,
                ..
            } = &mut e.kind
            else {
                return Err("Select a shape".into());
            };
            if let Some(shape) = v.get("shape") {
                *shape_type = serde_json::from_value(shape.clone()).map_err(|_| "Unknown shape")?;
            }
            if v.get("color").is_some() {
                *fill = FillStyle::Solid(color(v)?);
            }
            if let Some(end) = v.get("gradient_end") {
                let end = color(&json!({"color":end}))?;
                *fill = FillStyle::LinearGradient {
                    start: fill.primary_color(),
                    end,
                    vertical: v["vertical"].as_bool().unwrap_or(true),
                };
            }
            if let Some(c) = v.get("stroke_color") {
                *stroke_color = color(&json!({"color":c}))?;
            }
            for (key, target) in [
                ("stroke_width", stroke_width),
                ("corner_radius", corner_radius),
            ] {
                if v.get(key).is_some() {
                    let n = number(v, key)?;
                    if !(0.0..=1000.).contains(&n) {
                        return Err("Shape measurements must be 0–1000".into());
                    }
                    *target = n;
                }
            }
        }
        "add_text" | "add_shape" => {
            let page = v["page"].as_u64().ok_or("Missing page")? as usize;
            if page >= project.pages.len() {
                return Err("Page outside document".into());
            }
            let mut e = templates::make_basic_text_element(project.alloc_id());
            let eid = e.id.to_string();
            e.name = if action == "add_text" {
                "New text"
            } else {
                "New shape"
            }
            .into();
            e.x = 50.;
            e.y = 50.;
            e.width = (project.width - 100.).max(100.);
            e.height = 100.;
            if action == "add_text" {
                if let ElementKind::Text { font_size, .. } = &mut e.kind {
                    *font_size = 24. * project.effective_units_per_inch() / 72.;
                }
            } else {
                let shape_type =
                    serde_json::from_value(v.get("shape").cloned().unwrap_or(json!("Rectangle")))
                        .map_err(|_| "Unknown shape")?;
                e.kind = ElementKind::Shape {
                    shape_type,
                    fill: FillStyle::Solid(rgb(49, 74, 230)),
                    stroke_color: rgb(24, 30, 36),
                    stroke_width: 0.,
                    corner_radius: 0.,
                };
            }
            project.pages[page].elements.push(e);
            let mut attrs = v.clone();
            attrs["id"] = json!(eid);
            attrs["action"] = json!(if action == "add_text" {
                "text"
            } else {
                "shape"
            });
            edit(project, &attrs)?;
            attrs["action"] = json!("layer");
            edit(project, &attrs)?;
            if v.get("x").is_some() || v.get("width").is_some() {
                attrs["action"] = json!("geometry");
                edit(project, &attrs)?;
            }
        }
        _ => return Err("Unknown editing action".into()),
    }
    Ok(())
}
fn state(s: &Session, page: usize) -> Value {
    let p = &s.project;
    let page = page.min(p.pages.len().saturating_sub(1));
    let unit = p.effective_units_per_inch() / 72.;
    let elements=p.pages[page].elements.iter().map(|e|{
        let mut value=json!({"id":e.id.to_string(),"name":e.name,"x":e.x,"y":e.y,"width":e.width,"height":e.height,"rotation":e.rotation,"opacity":e.opacity,"locked":e.locked,"visible":e.visible,"linked_style":e.style_id,"binding":e.binding,"group_id":e.group_id.map(|id|id.to_string()),"brand_colors":p.production.studio.colors.get(&e.id)});
        match &e.kind{ElementKind::Text{content,font_family,font_size,color,bold,italic,underline,line_height,letter_spacing,..}=>{value["type"]=json!("text");value["content"]=json!(content);value["family"]=json!(family(*font_family));value["size"]=json!(*font_size/unit);value["color"]=json!(format!("#{:02x}{:02x}{:02x}",color[0],color[1],color[2]));value["bold"]=json!(bold);value["italic"]=json!(italic);value["underline"]=json!(underline);value["leading"]=json!(line_height);value["tracking"]=json!(letter_spacing);value["paragraph"]=json!(e.text_format);},ElementKind::Image{..}=>value["type"]=json!("image"),ElementKind::Shape{shape_type,fill,stroke_color,stroke_width,corner_radius}=>{value["type"]=json!("shape");value["shape"]=json!(shape_type);value["fill"]=json!(fill);value["stroke_color"]=json!(stroke_color);value["stroke_width"]=json!(stroke_width);value["corner_radius"]=json!(corner_radius);},_=>value["type"]=json!("artwork")};value
    }).collect::<Vec<_>>();
    let fields=p.production.fields.iter().map(|(key,field)|json!({"key":key,"value":field.value,"required":field.required,"image":field.image})).collect::<Vec<_>>();
    let mut fonts = FontFamily::all()
        .iter()
        .map(|f| json!({"id":family(*f),"name":f.label()}))
        .collect::<Vec<_>>();
    fonts.extend(
        p.imported_fonts
            .iter()
            .map(|f| json!({"id":family(FontFamily::Imported(f.id)),"name":f.name})),
    );
    let mut timeline = serde_json::to_value(&p.production.timeline).unwrap();
    for (i, c) in p.production.timeline.clips.iter().enumerate() {
        if let Some(st) = &c.caption {
            timeline["clips"][i]["caption"]["family"] = json!(family(st.family));
        }
    }
    json!({"version":env!("CARGO_PKG_VERSION"),"revision":s.revision,"document_id":p.document_id,"actor":s.last_actor,"title":p.title,"width":p.width,"height":p.height,"width_mm":p.effective_width_mm(),"height_mm":p.effective_height_mm(),"page":page,"pages":p.pages.iter().map(|p|json!({"id":p.id.to_string(),"title":p.title})).collect::<Vec<_>>(),"elements":elements,"fields":fields,"fonts":fonts,"undo":!s.undo.is_empty(),"redo":!s.redo.is_empty(),"recovery":s.recovery.is_some(),"studio":studio_state(p),"timeline":timeline,"mockups":p.production.mockups})
}
fn studio_state(p: &Project) -> Value {
    let s = &p.production.studio;
    let mut variants=s.variants.iter().map(|(name,v)|json!({"name":name,"width":v.width,"height":v.height,"pages":v.pages.len(),"active":false})).collect::<Vec<_>>();
    if let Some(name) = &s.active_variant {
        variants.insert(0,json!({"name":name,"width":p.width,"height":p.height,"pages":p.pages.len(),"active":true}));
    }
    json!({"tokens":s.tokens.iter().map(|(key,c)|json!({"key":key,"color":crate::campaign::color_hex(*c)})).collect::<Vec<_>>(),"background":s.backgrounds.get(&p.pages[0].id),"variants":variants,"components":s.components.iter().map(|(key,c)|json!({"key":key,"name":c.name,"width":c.width,"height":c.height,"layers":c.elements.len()})).collect::<Vec<_>>()})
}
fn preflight_value(project: &Project) -> Value {
    let report = run_export_preflight(project, 300);
    json!({"issues":report.issues.iter().map(|i|json!({"page_index":i.page_index,"element_id":i.element_id.to_string(),"element_name":i.element_name,"severity":format!("{:?}",i.severity),"category":i.category,"message":i.message})).collect::<Vec<_>>()})
}
fn query(path: &str, key: &str) -> Option<String> {
    let value = path.split_once('?')?.1.split('&').find_map(|pair| {
        let (k, v) = pair.split_once('=')?;
        (k == key).then_some(v)
    })?;
    let mut out = Vec::new();
    let bytes = value.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            if i + 2 >= bytes.len() {
                return None;
            }
            let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).ok()?;
            out.push(u8::from_str_radix(hex, 16).ok()?);
            i += 3;
        } else {
            out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}
fn route(req: &Request, s: &mut Session) -> Result<Response, String> {
    let base = req.path.split('?').next().unwrap_or("");
    let page = query(&req.path, "page")
        .unwrap_or("0".into())
        .parse::<usize>()
        .map_err(|_| "Invalid page")?;
    if let Some(id) = req.headers.get("x-canvas-document") {
        if id != &s.project.document_id
            || req
                .headers
                .get("x-canvas-revision")
                .is_some_and(|r| r.parse::<u64>().ok() != Some(s.revision))
        {
            let mut r = Response::error(
                409,
                "Document or revision changed; inspect the studio again.",
            );
            r.revision = s.revision;
            return Ok(r);
        }
    }
    if req.method == "GET" {
        let mut alternate = None;
        if let Some(name) = query(&req.path, "variant") {
            let mut p = s.project.clone();
            crate::campaign::activate(&mut p, &name)?;
            alternate = Some(p);
        }
        let read_project = alternate.as_ref().unwrap_or(&s.project);
        return match base {
            "/api/mockups/texture" => Ok(Response::bytes(
                "image/png",
                crate::mockups::texture_bytes(
                    &s.project,
                    &query(&req.path, "id").ok_or("Missing scene")?,
                    query(&req.path, "back").is_some(),
                )?,
                s.revision,
            )),
            "/api/film/motion-tools" => Ok(Response::value(crate::motion::doctor(), s.revision)),
            "/api/film/source" => {
                let id = query(&req.path, "id").ok_or("Missing job")?;
                let p = crate::media_service::source_output(&id, &s.project.document_id)?;
                let mut r = Response::bytes(
                    "application/zip",
                    fs::read(p).map_err(|e| e.to_string())?,
                    s.revision,
                );
                r.download = Some("motion-source.zip");
                Ok(r)
            }
            "/api/film/storage" => Ok(Response::value(crate::media_service::storage(), s.revision)),
            "/api/film/jobs" => Ok(Response::value(
                crate::media_service::jobs(&s.project.document_id)?,
                s.revision,
            )),
            "/api/film/doctor" => Ok(Response::value(
                crate::media_service::dependencies(),
                s.revision,
            )),
            "/api/film/job" => Ok(Response::value(
                crate::media_service::status(
                    &query(&req.path, "id").ok_or("Missing job")?,
                    &s.project.document_id,
                )?,
                s.revision,
            )),
            "/api/film/srt" => {
                let mut r = Response::bytes(
                    "application/x-subrip",
                    crate::timeline::srt(&s.project.production.timeline).into_bytes(),
                    s.revision,
                );
                r.download = Some("captions.srt");
                Ok(r)
            }
            "/api/film/output" => {
                let id = query(&req.path, "id").ok_or("Missing job")?;
                let path = crate::media_service::output(&id, &s.project.document_id)?;
                let image = path.extension().is_some_and(|e| e == "png");
                let zip = path.extension().is_some_and(|e| e == "zip");
                let mut r = Response::bytes(
                    if zip {
                        "application/zip"
                    } else if image {
                        "image/png"
                    } else {
                        "video/mp4"
                    },
                    fs::read(path).map_err(|e| e.to_string())?,
                    s.revision,
                );
                r.download = Some(if zip {
                    "motion-source.zip"
                } else if image {
                    "frame.png"
                } else {
                    "film.mp4"
                });
                Ok(r)
            }
            "/api/film/cache" => {
                let id = query(&req.path, "asset").ok_or("Missing asset")?;
                let a = s
                    .project
                    .production
                    .timeline
                    .assets
                    .get(&id)
                    .ok_or("Media not found")?;
                let kind = query(&req.path, "kind").ok_or("Missing cache type")?;
                if kind != "poster" && kind != "wave" {
                    return Err("Choose poster or wave".into());
                }
                let path = crate::media_store::cached(a, &kind)?;
                Ok(Response::bytes(
                    if kind == "poster" {
                        "image/jpeg"
                    } else {
                        "image/png"
                    },
                    fs::read(path).map_err(|_| "Build a proxy to create the poster/waveform")?,
                    s.revision,
                ))
            }
            "/api/templates" => Ok(Response::value(
                crate::studio_catalog::catalog(),
                s.revision,
            )),
            "/api/template-preview" => {
                let id = query(&req.path, "id").ok_or("Missing template")?;
                Ok(Response::bytes(
                    "image/png",
                    crate::studio_catalog::template_preview(&id)?,
                    s.revision,
                ))
            }
            "/api/thumbnail" => Ok(Response::bytes(
                "image/png",
                crate::studio_catalog::thumbnail(read_project, page, 240)?,
                s.revision,
            )),
            "/api/revision" => Ok(Response::value(
                json!({"revision":s.revision,"document_id":s.project.document_id,"actor":s.last_actor}),
                s.revision,
            )),
            "/api/agent-preview" => {
                let edge = query(&req.path, "edge")
                    .unwrap_or("960".into())
                    .parse::<u32>()
                    .map_err(|_| "Invalid preview edge")?;
                Ok(Response::bytes(
                    "image/png",
                    crate::studio_catalog::thumbnail(read_project, page, edge)?,
                    s.revision,
                ))
            }
            "/api/state" => Ok(Response::value(state(s, page), s.revision)),
            "/api/preview.svg" => {
                let proof = production::resolve(read_project, false)?;
                let svg = exporter::export_project_page_to_svg(&proof, page, false)?;
                Ok(Response::bytes(
                    "image/svg+xml",
                    svg.into_bytes(),
                    s.revision,
                ))
            }
            "/api/preflight" => Ok(Response::value(preflight_value(read_project), s.revision)),
            "/api/download.canvas" => {
                let scratch = Scratch::new()?;
                let path = scratch.0.join("design.canvas");
                project_package::save(read_project, &path)?;
                let mut r = Response::bytes(
                    "application/octet-stream",
                    fs::read(path).map_err(|e| e.to_string())?,
                    s.revision,
                );
                r.download = Some("design.canvas");
                Ok(r)
            }
            "/api/download.pdf" => {
                check_delivery(read_project)?;
                let bytes = model::pdf_export::export_project_to_pdf_bytes(
                    read_project,
                    &PdfExportOptions::default(),
                )?;
                let mut r = Response::bytes("application/pdf", bytes, s.revision);
                r.download = Some("design.pdf");
                Ok(r)
            }
            "/api/download.png" => {
                check_delivery(read_project)?;
                let dpi = query(&req.path, "dpi")
                    .unwrap_or("150".into())
                    .parse::<u32>()
                    .map_err(|_| "Invalid DPI")?;
                if ![96, 150, 300].contains(&dpi) {
                    return Err("Use 96, 150 or 300 DPI".into());
                }
                let w = s.project.effective_width_mm() / 25.4 * dpi as f32;
                let h = s.project.effective_height_mm() / 25.4 * dpi as f32;
                if w * h > 100_000_000. || w.max(h) > 18000. {
                    return Err("Output exceeds raster size limit".into());
                }
                let image = exporter::render_project_page_at_dpi(read_project, page, dpi, false)?;
                let mut bytes = Vec::new();
                image
                    .write_to(
                        &mut std::io::Cursor::new(&mut bytes),
                        image::ImageFormat::Png,
                    )
                    .map_err(|e| e.to_string())?;
                let mut r = Response::bytes("image/png", bytes, s.revision);
                r.download = Some("page.png");
                Ok(r)
            }
            _ => Err("Route not found".into()),
        };
    }
    if req.method != "POST" {
        return Err("Use GET or POST".into());
    }
    let revision = req
        .headers
        .get("x-canvas-revision")
        .and_then(|r| r.parse::<u64>().ok())
        .ok_or("Missing document revision")?;
    if revision != s.revision {
        let mut r=Response::error(409,"This document changed in another tab. Refresh before editing; your pending input is still in the form.");
        r.revision = s.revision;
        return Ok(r);
    }
    if req
        .headers
        .get("x-canvas-document")
        .is_some_and(|id| id != &s.project.document_id)
    {
        let mut r = Response::error(
            409,
            "The active document changed. Inspect the current document before editing.",
        );
        r.revision = s.revision;
        return Ok(r);
    }
    match base {
        "/api/mockups/render" => {
            let v: Value = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
            let id = crate::mockups::start(
                &s.project,
                s.revision,
                v["id"].as_str().ok_or("Choose a scene")?,
                v["movie"].as_bool().unwrap_or(false),
            )?;
            return Ok(Response::value(
                json!({"id":id,"source_revision":s.revision}),
                s.revision,
            ));
        }
        "/api/film/clear-jobs" => {
            let count = crate::media_service::clear_finished(&s.project.document_id)?;
            return Ok(Response::value(json!({"cleared":count}), s.revision));
        }
        "/api/film/render" | "/api/film/proxy" | "/api/film/cancel" => {
            let v: Value = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
            if base == "/api/film/cancel" {
                crate::media_service::cancel(
                    v["id"].as_str().ok_or("Missing job")?,
                    &s.project.document_id,
                )?;
                return Ok(Response::value(
                    json!({"cancel_requested":true}),
                    s.revision,
                ));
            }
            let job = if base == "/api/film/proxy" {
                let a = s
                    .project
                    .production
                    .timeline
                    .assets
                    .get(v["asset"].as_str().ok_or("Missing asset")?)
                    .ok_or("Media not found")?;
                crate::media_service::proxy(a, &s.project.document_id, s.revision)?
            } else {
                crate::media_service::start(
                    &s.project,
                    s.revision,
                    v["mode"].as_str().ok_or("Missing mode")?,
                    v["frame"].as_u64(),
                )?
            };
            return Ok(Response::value(
                json!({"id":job,"source_revision":s.revision}),
                s.revision,
            ));
        }
        "/api/film/import" => {
            let name = query(&req.path, "name").ok_or("Missing filename")?;
            if name.contains(['/', '\\']) {
                return Err("Use a filename without folders".into());
            }
            let scratch = Scratch::new()?;
            let path = scratch.0.join(&name);
            fs::write(&path, &req.body).map_err(|e| e.to_string())?;
            let asset = crate::media_store::import(&path, &name)?;
            let mut candidate = s.project.clone();
            candidate
                .production
                .timeline
                .assets
                .insert(asset.id.clone(), asset.clone());
            s.commit(candidate)?;
            s.last_actor = "browser".into();
            return Ok(Response::value(
                json!({"asset":asset,"revision":s.revision,"document_id":s.project.document_id}),
                s.revision,
            ));
        }
        "/api/film/srt" => {
            let mut candidate = s.project.clone();
            crate::timeline::import_srt(
                &mut candidate,
                std::str::from_utf8(&req.body).map_err(|_| "Use UTF-8 SRT")?,
            )?;
            s.commit(candidate)?;
        }
        "/api/command" => {
            let value: Value = serde_json::from_slice(&req.body).map_err(|e| e.to_string())?;
            let action = string(&value, "action")?;
            if action == "undo" || action == "redo" {
                s.travel(action == "redo")?;
            } else if action == "template" {
                let key = string(&value, "template")?;
                if !templates::available_templates().iter().any(|t| t.id == key) {
                    return Err("Unknown template".into());
                }
                s.commit(templates::build_template_project(key))?;
            } else if action == "insert_template" {
                let key = string(&value, "template")?;
                if !templates::available_templates().iter().any(|t| t.id == key) {
                    return Err("Unknown template".into());
                }
                let mut project = s.project.clone();
                let source = templates::build_template_project(key);
                let pages = document_ops::fitted_pages(&mut project, &source);
                project.pages.extend(pages);
                s.commit(project)?;
            } else if action == "batch" {
                let commands = value["commands"]
                    .as_array()
                    .ok_or("Missing command array")?;
                if commands.is_empty() || commands.len() > 500 {
                    return Err("Use 1–500 commands per atomic batch".into());
                }
                let mut candidate = s.project.clone();
                let mut refs = HashMap::<String, String>::new();
                for command in commands {
                    let mut command = command.clone();
                    if let Some(alias) = command
                        .get("id")
                        .and_then(Value::as_str)
                        .and_then(|s| s.strip_prefix('$'))
                    {
                        command["id"] = json!(refs.get(alias).ok_or("Unknown layer reference")?);
                    }
                    if let Some(ids) = command.get_mut("ids").and_then(Value::as_array_mut) {
                        for id in ids {
                            if let Some(alias) = id.as_str().and_then(|s| s.strip_prefix('$')) {
                                *id = json!(refs.get(alias).ok_or("Unknown layer reference")?);
                            }
                        }
                    }
                    let creates =
                        matches!(command["action"].as_str(), Some("add_text" | "add_shape"));
                    let next = candidate.next_id;
                    edit(&mut candidate, &command)?;
                    if let Some(alias) = command.get("ref").and_then(Value::as_str) {
                        if !creates
                            || alias.is_empty()
                            || refs.insert(alias.into(), next.to_string()).is_some()
                        {
                            return Err("Use a unique ref on newly added text/shapes".into());
                        }
                    }
                }
                let dry_run = value["dry_run"].as_bool().unwrap_or(false);
                validate_document(&candidate)?;
                let report = preflight_value(&candidate);
                if !dry_run {
                    s.commit(candidate)?;
                    s.last_actor = req
                        .headers
                        .get("x-canvas-actor")
                        .filter(|a| a.as_str() == "mcp")
                        .cloned()
                        .unwrap_or("browser".into());
                }
                return Ok(Response::value(
                    json!({"revision":s.revision,"document_id":s.project.document_id,"refs":refs,"dry_run":dry_run,"preflight":report}),
                    s.revision,
                ));
            } else if action == "new" {
                let mut candidate = Project::default();
                if value.get("width").is_some() || value.get("height").is_some() {
                    let w = number(&value, "width")?;
                    let h = number(&value, "height")?;
                    if !(64.0..=16000.).contains(&w) || !(64.0..=16000.).contains(&h) {
                        return Err("Use dimensions from 64–16000 layout pixels".into());
                    }
                    candidate.width = w;
                    candidate.height = h;
                    candidate.width_mm = w / 96. * 25.4;
                    candidate.height_mm = h / 96. * 25.4;
                    candidate.format = DesignFormat::Custom(w as u32, h as u32);
                }
                s.commit(candidate)?;
            } else {
                let mut project = s.project.clone();
                edit(&mut project, &value)?;
                s.commit(project)?;
            }
        }
        "/api/open" => {
            let scratch = Scratch::new()?;
            let path = scratch.0.join("upload.canvas");
            fs::write(&path, &req.body).map_err(|e| e.to_string())?;
            let mut project = load_project_json(&path)?;
            project.recovery_origin_path = None;
            s.commit(project)?;
        }
        "/api/font" => {
            let font = imported_fonts::parse(req.body.clone())?;
            let mut project = s.project.clone();
            if !project.imported_fonts.iter().any(|f| f.id == font.id) {
                project.imported_fonts.push(font);
            }
            s.commit(project)?;
        }
        "/api/photo" | "/api/image" | "/api/add-image" => {
            if req.body.len() > 25_000_000 {
                return Err("Image exceeds 25 MB".into());
            }
            let scratch = Scratch::new()?;
            let path = scratch.0.join("uploaded-photo.png");
            let reader = image::ImageReader::new(std::io::Cursor::new(&req.body))
                .with_guessed_format()
                .map_err(|e| e.to_string())?;
            let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
            if w > 18_000 || h > 18_000 || w as u64 * h as u64 > 100_000_000 {
                return Err("Image dimensions exceed the safe limit".into());
            }
            image::load_from_memory(&req.body)
                .map_err(|e| e.to_string())?
                .save(&path)
                .map_err(|e| e.to_string())?;
            let mut project = s.project.clone();
            if base == "/api/add-image" {
                if page >= project.pages.len() {
                    return Err("Page outside document".into());
                }
                let asset = model::asset_store::AssetStore::global().import_image_file(&path)?;
                let stock = templates::SampleStockAsset {
                    name: "Uploaded photo",
                    subtitle: "Local photo",
                    width: asset.width_px,
                    height: asset.height_px,
                    rgba: asset.decoded_rgba.into(),
                };
                let mut element =
                    templates::make_stock_image_element(project.alloc_id(), &stock, 50., 100.);
                let scale = ((project.width - 100.).max(30.) / w as f32)
                    .min((project.height - 150.).max(30.) / h as f32)
                    .min(300. / w as f32);
                element.width = w as f32 * scale;
                element.height = h as f32 * scale;
                if let ElementKind::Image { mask, .. } = &mut element.kind {
                    *mask = ImageMask::None;
                }
                project.pages[page].elements.push(element);
            } else if base == "/api/photo" {
                let key = query(&req.path, "key").ok_or("Missing photo field")?;
                production::attach_image(
                    project
                        .production
                        .fields
                        .get_mut(&key)
                        .ok_or("Photo field not found")?,
                    &path,
                )?;
            } else {
                let key = query(&req.path, "id").ok_or("Missing image identity")?;
                let v = json!({"id":key});
                let e = element_mut(&mut project, &v)?;
                if !matches!(e.kind, ElementKind::Image { .. }) {
                    return Err("Select an image layer".into());
                }
                model::import_layout::replace_element_image_from_path(e, &path)?;
            }
            s.commit(project)?;
        }
        _ => return Err("Route not found".into()),
    }
    s.last_actor = req
        .headers
        .get("x-canvas-actor")
        .filter(|a| a.as_str() == "mcp")
        .cloned()
        .unwrap_or("browser".into());
    Ok(Response::value(
        json!({"revision":s.revision,"document_id":s.project.document_id}),
        s.revision,
    ))
}
fn handle(stream: &mut TcpStream, host: &str, token: &str, session: &Mutex<Session>) {
    let req = match request(stream, Some((host, token))) {
        Ok(r) => r,
        Err(e) => {
            send(stream, Response::error(e.status, &e.message));
            return;
        }
    };
    if req.headers.get("host").map(String::as_str) != Some(host) {
        send(stream, Response::error(403, "Invalid host"));
        return;
    }
    if let Some(origin) = req.headers.get("origin") {
        if origin != &format!("http://{host}") {
            send(
                stream,
                Response::error(403, "Cross-origin editing is disabled"),
            );
            return;
        }
    }
    if req.method == "GET" {
        let static_file = match req.path.as_str() {
            "/" => Some((
                "text/html; charset=utf-8",
                include_bytes!("../web/index.html").as_slice(),
            )),
            "/mockups.js" => Some((
                "application/javascript",
                include_bytes!("../web/mockups.js").as_slice(),
            )),
            "/mockup-scene.js" => Some((
                "application/javascript",
                include_bytes!("../web/mockup-scene.js").as_slice(),
            )),
            "/vendor/three.module.js" => Some((
                "application/javascript",
                include_bytes!("../web/vendor/three.module.js").as_slice(),
            )),
            "/vendor/three.core.js" => Some((
                "application/javascript",
                include_bytes!("../web/vendor/three.core.js").as_slice(),
            )),
            "/mockups.css" => Some(("text/css", include_bytes!("../web/mockups.css").as_slice())),
            "/motion.js" => Some((
                "text/javascript; charset=utf-8",
                include_bytes!("../web/motion.js").as_slice(),
            )),
            "/film.js" => Some((
                "text/javascript; charset=utf-8",
                include_bytes!("../web/film.js").as_slice(),
            )),
            "/bundle.js" => Some((
                "text/javascript; charset=utf-8",
                include_bytes!("../web/bundle.js").as_slice(),
            )),
            "/layout.js" => Some((
                "text/javascript; charset=utf-8",
                include_bytes!("../web/layout.js").as_slice(),
            )),
            "/studio.js" => Some((
                "text/javascript; charset=utf-8",
                include_bytes!("../web/studio.js").as_slice(),
            )),
            "/app.js" => Some((
                "text/javascript; charset=utf-8",
                include_bytes!("../web/app.js").as_slice(),
            )),
            "/style.css" => Some((
                "text/css; charset=utf-8",
                include_bytes!("../web/style.css").as_slice(),
            )),
            _ => None,
        };
        if let Some((mime, bytes)) = static_file {
            send(stream, Response::bytes(mime, bytes.into(), 0));
            return;
        }
    }
    if req.headers.get("x-canvas-token").map(String::as_str) != Some(token) {
        send(
            stream,
            Response::error(
                403,
                "Open the private session link printed by Canvas Studio",
            ),
        );
        return;
    }
    let response = match session.lock() {
        Ok(mut s) => route(&req, &mut s).unwrap_or_else(|e| Response::error(422, &e)),
        Err(_) => Response::error(
            422,
            "The session stopped after an internal error; restart to recover",
        ),
    };
    send(stream, response);
}
pub fn run(port: u16, initial: Option<PathBuf>) -> Result<(), String> {
    let listener = TcpListener::bind(("127.0.0.1", port))
        .map_err(|e| format!("Cannot start browser editor: {e}"))?;
    let port = listener.local_addr().map_err(|e| e.to_string())?.port();
    let recovery =
        application_data_dir().join(format!("canvas-studio/browser/recovery-{port}.canvas"));
    if let Some(folder) = recovery.parent() {
        fs::create_dir_all(folder).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(folder, fs::Permissions::from_mode(0o700))
                .map_err(|e| e.to_string())?;
        }
    }
    let project = if let Some(path) = initial {
        load_project_json(&path)?
    } else if recovery.is_file() {
        load_project_json(&recovery)
            .map_err(|e| format!("Browser recovery could not be read; original preserved: {e}"))?
    } else {
        Project::default()
    };
    validate_document(&project)?;
    let mut random = [0u8; 32];
    fs::File::open("/dev/urandom")
        .and_then(|mut f| f.read_exact(&mut random))
        .map_err(|e| format!("Cannot create private session token: {e}"))?;
    let token = random
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect::<String>();
    // A fresh 48-bit epoch fits exactly in JavaScript integers and prevents stale
    // revision handles from becoming valid again after a server restart.
    let revision_epoch = u64::from_le_bytes([
        random[0], random[1], random[2], random[3], random[4], random[5], 0, 0,
    ]);
    let host = format!("127.0.0.1:{port}");
    // Private loopback credential for local agent adapters, never a public project asset.
    let connection = recovery.with_file_name(format!("connection-{port}.json"));
    let staging = connection.with_extension("json.tmp");
    let mut options = fs::OpenOptions::new();
    options.write(true).create(true).truncate(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = options.open(&staging).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(fs::Permissions::from_mode(0o600))
            .map_err(|e| e.to_string())?;
    }
    file.write_all(
        serde_json::to_string(
            &json!({"port":port,"token":token,"version":env!("CARGO_PKG_VERSION")}),
        )
        .unwrap()
        .as_bytes(),
    )
    .map_err(|e| e.to_string())?;
    file.sync_all().map_err(|e| e.to_string())?;
    fs::rename(&staging, &connection).map_err(|e| e.to_string())?;
    let session = Arc::new(Mutex::new(Session {
        project,
        revision: revision_epoch,
        undo: Vec::new(),
        redo: Vec::new(),
        recovery: Some(recovery),
        last_actor: "browser".into(),
    }));
    println!("Canvas Studio browser editor: http://{host}/#{token}\nKeep this local session link private. Ctrl+C stops the server. Download a .canvas copy to save a deliverable.");
    let active = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        use std::sync::atomic::Ordering;
        if active.fetch_add(1, Ordering::SeqCst) >= 16 {
            active.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let active = active.clone();
        let session = session.clone();
        let host = host.clone();
        let token = token.clone();
        std::thread::spawn(move || {
            struct Active(Arc<std::sync::atomic::AtomicUsize>);
            impl Drop for Active {
                fn drop(&mut self) {
                    self.0.fetch_sub(1, Ordering::SeqCst);
                }
            }
            let _guard = Active(active);
            handle(&mut stream, &host, &token, &session)
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn session() -> Session {
        Session {
            project: Project::default(),
            revision: 0,
            undo: Vec::new(),
            redo: Vec::new(),
            recovery: None,
            last_actor: "browser".into(),
        }
    }
    fn command(s: &mut Session, value: Value) -> Result<Response, String> {
        let req = Request {
            method: "POST".into(),
            path: "/api/command".into(),
            headers: HashMap::from([("x-canvas-revision".into(), s.revision.to_string())]),
            body: serde_json::to_vec(&value).unwrap(),
        };
        route(&req, s)
    }
    #[test]
    fn agent_batches_are_atomic_undoable_scoped_and_previewable() {
        let mut s = session();
        command(&mut s, json!({"action":"new","width":1080,"height":1350})).unwrap();
        let before = s.project.clone();
        let revision = s.revision;
        assert!(command(&mut s,json!({"action":"batch","commands":[{"action":"title","value":"Should roll back"},{"action":"geometry","id":"missing"}]})).is_err());
        assert_eq!(s.project, before);
        assert_eq!(s.revision, revision);
        let value = json!({"action":"batch","dry_run":true,"commands":[
            {"action":"background","page":0,"color":"#181e24"},
            {"action":"add_shape","page":0,"ref":"disc","shape":"Circle","color":"#d5f171","x":700,"y":100,"width":240,"height":240},
            {"action":"add_text","page":0,"ref":"headline","content":"A better way","size":72,"color":"#f5f1e8","x":80,"y":470,"width":900,"height":180},
            {"action":"layer","id":"$headline","name":"Campaign headline"}]});
        let response = command(&mut s, value.clone()).unwrap();
        let result: Value = serde_json::from_slice(&response.body).unwrap();
        assert!(result["refs"]["headline"].is_string());
        assert_eq!(s.project, before);
        assert_eq!(s.revision, revision);
        let mut value = value;
        value["dry_run"] = json!(false);
        command(&mut s, value).unwrap();
        assert_eq!(s.revision, revision + 1);
        assert_eq!(s.project.pages[0].elements.len(), 2);
        assert_eq!(s.project.pages[0].elements[1].name, "Campaign headline");
        command(&mut s, json!({"action":"undo"})).unwrap();
        assert_eq!(s.project, before);
        command(&mut s, json!({"action":"redo"})).unwrap();
        let req = Request {
            method: "GET".into(),
            path: "/api/agent-preview?edge=320".into(),
            headers: HashMap::from([
                ("x-canvas-document".into(), s.project.document_id.clone()),
                ("x-canvas-revision".into(), s.revision.to_string()),
            ]),
            body: Vec::new(),
        };
        let preview = route(&req, &mut s).unwrap();
        let image = image::load_from_memory(&preview.body).unwrap();
        assert_eq!(image.height(), 320);
        let mut wrong = req;
        wrong
            .headers
            .insert("x-canvas-document".into(), "other".into());
        assert_eq!(route(&wrong, &mut s).unwrap().status, 409);
    }
    #[test]
    fn browser_edits_preserve_native_typography_and_undo_without_stale_overwrites() {
        let mut s = session();
        command(&mut s, json!({"action":"add_text","page":0})).unwrap();
        let id = s.project.pages[0].elements[0].id.to_string();
        command(&mut s,json!({"action":"text","id":id,"content":"Avery & Zoë\nA new chapter","size":22,"space_before":6,"inset_left":12})).unwrap();
        command(&mut s,json!({"action":"emphasis","id":id,"start":8,"end":11,"property":"italic","value":true})).unwrap();
        let snapshot = s.project.clone();
        assert_eq!(snapshot.pages[0].elements[0].text_format.inset_left, 12.);
        command(&mut s, json!({"action":"undo"})).unwrap();
        assert!(s.project.pages[0].elements[0].text_format.runs.is_empty());
        command(&mut s, json!({"action":"redo"})).unwrap();
        assert!(s.project == snapshot);
        let req = Request {
            method: "POST".into(),
            path: "/api/command".into(),
            headers: HashMap::from([("x-canvas-revision".into(), "0".into())]),
            body: br#"{"action":"title","value":"Stale"}"#.to_vec(),
        };
        assert_eq!(route(&req, &mut s).unwrap().status, 409);
        assert!(s.project == snapshot);
        let before = s.revision;
        assert!(command(
            &mut s,
            json!({"action":"geometry","id":id,"x":0,"y":0,"width":-1,"height":50})
        )
        .is_err());
        assert_eq!(s.revision, before);
        assert!(s.project == snapshot);
    }
    #[test]
    fn browser_portable_download_and_recovery_round_trip_photos_fonts_and_styles() {
        let root = Scratch::new().unwrap();
        let mut s = session();
        s.recovery = Some(root.0.join("recovery.canvas"));
        let mut p = templates::build_template_project("brand_logo");
        let assets = templates::generate_builtin_stock_assets();
        let image = templates::make_stock_image_element(p.alloc_id(), &assets[0], 40., 200.);
        p.pages[0].elements.push(image);
        let font =
            imported_fonts::parse(include_bytes!("../assets/fonts/NotoSerif-Regular.ttf").to_vec())
                .unwrap();
        p.imported_fonts.push(font);
        s.commit(p).unwrap();
        let req = Request {
            method: "GET".into(),
            path: "/api/download.canvas".into(),
            headers: HashMap::new(),
            body: Vec::new(),
        };
        let response = route(&req, &mut s).unwrap();
        let file = root.0.join("delivered.canvas");
        fs::write(&file, response.body).unwrap();
        let loaded = load_project_json(&file).unwrap();
        assert_eq!(loaded.imported_fonts.len(), 1);
        assert!(loaded.pages[0]
            .elements
            .iter()
            .any(|e| matches!(&e.kind,ElementKind::Image{raw_rgba,..}if !raw_rgba.is_empty())));
        assert_eq!(
            load_project_json(&root.0.join("recovery.canvas"))
                .unwrap()
                .title,
            s.project.title
        );
        let revision = s.revision;
        s.recovery = Some(root.0.join("blocked/recovery.canvas"));
        fs::write(root.0.join("blocked"), b"not a directory").unwrap();
        assert!(command(&mut s, json!({"action":"title","value":"Must not commit"})).is_err());
        assert_eq!(s.revision, revision);
        assert_ne!(s.project.title, "Must not commit");
    }
    #[test]
    fn browser_print_export_rejects_unfilled_fields_and_modified_puzzle_keys() {
        let mut s = session();
        s.project.production.fields.insert(
            "name".into(),
            production::DataField {
                required: true,
                ..Default::default()
            },
        );
        let req = Request {
            method: "GET".into(),
            path: "/api/download.pdf".into(),
            headers: HashMap::new(),
            body: Vec::new(),
        };
        assert!(route(&req, &mut s).is_err());
        s.project = Project::default();
        puzzle_production::append_series(
            &mut s.project,
            &Default::default(),
            &Default::default(),
            1,
            true,
        )
        .unwrap();
        let letter = s.project.pages[0]
            .elements
            .iter_mut()
            .find(|e| e.name == "Puzzle letter")
            .unwrap();
        if let ElementKind::Text { content, .. } = &mut letter.kind {
            *content = "?".into();
        }
        assert!(route(&req, &mut s).is_err());
    }
    #[test]
    fn browser_photo_upload_handles_jpeg_and_does_not_change_a_locked_frame() {
        let mut s = session();
        let assets = templates::generate_builtin_stock_assets();
        let e = templates::make_stock_image_element(s.project.alloc_id(), &assets[0], 20., 200.);
        let id = e.id;
        s.project.pages[0].elements.push(e);
        let photo = image::RgbImage::from_pixel(28, 18, image::Rgb([190, 65, 100]));
        let mut body = Vec::new();
        photo
            .write_to(
                &mut std::io::Cursor::new(&mut body),
                image::ImageFormat::Jpeg,
            )
            .unwrap();
        let mut req = Request {
            method: "POST".into(),
            path: format!("/api/image?id={id}"),
            headers: HashMap::from([("x-canvas-revision".into(), "0".into())]),
            body,
        };
        route(&req, &mut s).unwrap();
        assert!(matches!(
            &s.project.pages[0].elements[0].kind,
            ElementKind::Image {
                width_px: 28,
                height_px: 18,
                ..
            }
        ));
        s.project.pages[0].elements[0].locked = true;
        req.headers
            .insert("x-canvas-revision".into(), s.revision.to_string());
        assert!(route(&req, &mut s).is_err());
    }
    #[test]
    fn loopback_http_requires_a_valid_host_origin_and_session_token() {
        fn call(host: &str, token: bool, origin: Option<&str>) -> String {
            let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let address = listener.local_addr().unwrap();
            let expected = address.to_string();
            let thread = std::thread::spawn(move || {
                let (mut stream, _) = listener.accept().unwrap();
                handle(&mut stream, &expected, "test-token", &Mutex::new(session()));
            });
            let actual_host = if host == "valid" {
                address.to_string()
            } else {
                host.to_string()
            };
            let extra = format!(
                "{}{}",
                if token {
                    "X-Canvas-Token: test-token\r\n"
                } else {
                    ""
                },
                origin
                    .map(|o| format!("Origin: {o}\r\n"))
                    .unwrap_or_default()
            );
            let mut client = TcpStream::connect(address).unwrap();
            client
                .write_all(
                    format!("GET /api/state HTTP/1.1\r\nHost: {actual_host}\r\n{extra}\r\n")
                        .as_bytes(),
                )
                .unwrap();
            let mut response = String::new();
            client.read_to_string(&mut response).unwrap();
            thread.join().unwrap();
            response
        }
        assert!(call("valid", true, None).starts_with("HTTP/1.1 200"));
        assert!(call("valid", false, None).starts_with("HTTP/1.1 403"));
        assert!(call("attacker.invalid", true, None).starts_with("HTTP/1.1 403"));
        assert!(call("valid", true, Some("https://attacker.invalid")).starts_with("HTTP/1.1 403"));
    }
    #[test]
    fn invalid_documents_and_unauthorized_uploads_do_not_damage_the_session() {
        let mut s = session();
        let mut invalid = Project::default();
        invalid.pages.clear();
        assert!(s.commit(invalid).is_err());
        assert_eq!(s.revision, 0);
        let listener = TcpListener::bind(("127.0.0.1", 0)).unwrap();
        let address = listener.local_addr().unwrap();
        let host = address.to_string();
        let thread = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let error = match request(&mut stream, Some((&host, "secret"))) {
                Err(error) => error,
                Ok(_) => panic!("Unauthorized upload accepted"),
            };
            assert_eq!(error.status, 403);
            send(&mut stream, Response::error(error.status, &error.message));
        });
        let mut client = TcpStream::connect(address).unwrap();
        client
            .set_read_timeout(Some(Duration::from_secs(2)))
            .unwrap();
        client.write_all(format!("POST /api/open HTTP/1.1\r\nHost: {address}\r\nContent-Length: 100000000\r\n\r\n").as_bytes()).unwrap();
        let mut response = String::new();
        client.read_to_string(&mut response).unwrap();
        assert!(response.starts_with("HTTP/1.1 403"));
        thread.join().unwrap();
    }
    #[test]
    fn request_parser_rejects_ambiguous_or_oversized_requests() {
        fn attempt(raw: &'static [u8]) -> bool {
            let l = TcpListener::bind(("127.0.0.1", 0)).unwrap();
            let a = l.local_addr().unwrap();
            let t = std::thread::spawn(move || {
                let (mut s, _) = l.accept().unwrap();
                request(&mut s, None).is_err()
            });
            let mut s = TcpStream::connect(a).unwrap();
            s.write_all(raw).unwrap();
            s.shutdown(std::net::Shutdown::Write).unwrap();
            t.join().unwrap()
        }
        assert!(attempt(
            b"POST / HTTP/1.1\r\nContent-Length: 0\r\nContent-Length: 0\r\n\r\n"
        ));
        assert!(attempt(
            b"POST / HTTP/1.1\r\nContent-Length: 134217729\r\n\r\n"
        ));
        assert!(attempt(
            b"POST / HTTP/1.1\r\nTransfer-Encoding: chunked\r\n\r\n"
        ));
    }
}
