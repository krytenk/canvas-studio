//! Typed, portable product scenes. Browser and headless output share one Three.js renderer.
use crate::model::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{fs, path::Path};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Mockup {
    pub id: String,
    pub name: String,
    pub page_id: String,
    pub back_page: Option<String>,
    pub kind: String,
    pub width: f32,
    pub height: f32,
    pub depth: f32,
    pub yaw: f32,
    pub tilt: f32,
    pub elevation: f32,
    pub zoom: f32,
    pub sweep: f32,
    pub background: String,
    pub cover_color: String,
    pub light: String,
    pub roughness: f32,
    pub output_width: u32,
    pub output_height: u32,
}
impl Default for Mockup {
    fn default() -> Self {
        Self {
            id: generate_document_id(),
            name: "Book · studio portrait".into(),
            page_id: "1".into(),
            back_page: None,
            kind: "book".into(),
            width: 1.5,
            height: 2.2,
            depth: 0.24,
            yaw: -28.,
            tilt: 0.,
            elevation: 12.,
            zoom: 1.,
            sweep: 48.,
            background: "#e8e4dc".into(),
            cover_color: "#233f37".into(),
            light: "warm".into(),
            roughness: 0.52,
            output_width: 1200,
            output_height: 1200,
        }
    }
}
pub fn validate(p: &Project) -> Result<(), String> {
    if p.production.mockups.len() > 24 {
        return Err("Keep at most 24 product scenes per document".into());
    }
    for m in &p.production.mockups {
        validate_scene(m)?;
    }
    Ok(())
}
fn validate_scene(m: &Mockup) -> Result<(), String> {
    if m.id.len() > 80
        || m.id.is_empty()
        || !m.id.bytes().all(|v| v.is_ascii_alphanumeric() || v == b'-')
    {
        return Err("Invalid mockup identity".into());
    }
    if m.name.len() > 120 || !["book", "box", "cards"].contains(&m.kind.as_str()) {
        return Err("Choose a book, box or card stack".into());
    }
    for (n, a, b) in [
        (m.width, 0.2, 5.),
        (m.height, 0.2, 5.),
        (m.depth, 0.08, 2.),
        (m.yaw, -75., 75.),
        (m.tilt, -20., 20.),
        (m.elevation, 0., 45.),
        (m.zoom, 0.6, 1.4),
        (m.sweep, -90., 90.),
        (m.roughness, 0.25, 1.),
    ] {
        if !n.is_finite() || n < a || n > b {
            return Err("Mockup measurements outside supported ranges".into());
        }
    }
    if m.depth > m.width
        || m.output_width < 256
        || m.output_width > 2400
        || m.output_height < 256
        || m.output_height > 2400
        || m.output_width % 2 != 0
        || m.output_height % 2 != 0
        || !["warm", "neutral"].contains(&m.light.as_str())
    {
        return Err("Use a supported mockup size and lighting".into());
    }
    for c in [&m.background, &m.cover_color] {
        if c.len() != 7 || !c.starts_with('#') || !c[1..].bytes().all(|v| v.is_ascii_hexdigit()) {
            return Err("Use #rrggbb colours".into());
        }
    }
    Ok(())
}
pub fn scene<'a>(p: &'a Project, id: &str) -> Result<&'a Mockup, String> {
    p.production
        .mockups
        .iter()
        .find(|m| m.id == id)
        .ok_or("Mockup no longer exists".into())
}
pub fn supports(a: &str) -> bool {
    matches!(
        a,
        "mockup_create" | "mockup_update" | "mockup_delete" | "mockup_place" | "mockup_film"
    )
}
pub fn apply(p: &mut Project, v: &Value) -> Result<(), String> {
    let action = v["action"].as_str().ok_or("Missing action")?;
    if action == "mockup_create" {
        if p.production.mockups.len() >= 24 {
            return Err("Keep at most 24 product scenes".into());
        }
        let page = v["page"].as_u64().ok_or("Choose a cover page")? as usize;
        let page = p.pages.get(page).ok_or("Cover page outside document")?;
        let mut m = Mockup {
            page_id: page.id.to_string(),
            width: (p.width / p.height * 2.2).clamp(0.2, 5.),
            ..Default::default()
        };
        if let Some(kind) = v["kind"].as_str() {
            m.kind = kind.into();
            m.name = match kind {
                "box" => "Product box · studio portrait",
                "cards" => "Stationery · card stack",
                _ => "Book · studio portrait",
            }
            .into();
            if kind == "box" {
                m.depth = 0.6;
            }
            if kind == "cards" {
                m.depth = 0.1;
            }
        }
        validate_scene(&m)?;
        p.production.mockups.push(m);
        return Ok(());
    }
    let id = v["id"].as_str().ok_or("Choose a mockup")?;
    if action == "mockup_delete" {
        scene(p, id)?;
        p.production.mockups.retain(|m| m.id != id);
        return Ok(());
    }
    if action == "mockup_place" || action == "mockup_film" {
        return apply_output(p, v, action == "mockup_film");
    }
    let index = p
        .production
        .mockups
        .iter()
        .position(|m| m.id == id)
        .ok_or("Mockup no longer exists")?;
    let mut m = serde_json::to_value(&p.production.mockups[index]).unwrap();
    for key in [
        "name",
        "kind",
        "width",
        "height",
        "depth",
        "yaw",
        "tilt",
        "elevation",
        "zoom",
        "sweep",
        "background",
        "cover_color",
        "light",
        "roughness",
        "output_width",
        "output_height",
        "back_page",
    ] {
        if let Some(value) = v.get(key) {
            m[key] = value.clone();
        }
    }
    if let Some(page) = v["page"].as_u64() {
        m["page_id"] = json!(p
            .pages
            .get(page as usize)
            .ok_or("Cover page outside document")?
            .id
            .to_string());
    }
    let m: Mockup = serde_json::from_value(m).map_err(|e| e.to_string())?;
    validate_scene(&m)?;
    p.production.mockups[index] = m;
    Ok(())
}
pub fn cover(p: &Project, m: &Mockup, back: bool) -> Result<image::RgbaImage, String> {
    let id = if back {
        m.back_page.as_ref().ok_or("No back artwork selected")?
    } else {
        &m.page_id
    };
    let page = p
        .pages
        .iter()
        .position(|page| page.id.to_string() == *id)
        .ok_or("The cover page is missing. Reassign the scene cover before rendering")?;
    let resolved = crate::production::resolve_page(p, page, true)?.into_owned();
    let mut check = p.clone();
    check.pages = vec![resolved.clone()];
    if crate::model::pdf_export::run_export_preflight(&check, 96).has_errors() {
        return Err("Fix cover preflight errors before rendering a mockup".into());
    }
    let ratio = 1600. / p.width.max(p.height);
    Ok(crate::exporter::render_page_to_rgba_image(
        &resolved,
        p.width,
        p.height,
        (p.width * ratio).round().max(1.) as u32,
        (p.height * ratio).round().max(1.) as u32,
        false,
    ))
}
fn prepare(p: &Project, m: &Mockup, root: &Path) -> Result<(), String> {
    fs::create_dir_all(root.join("assets")).map_err(|e| e.to_string())?;
    cover(p, m, false)?
        .save(root.join("assets/front.png"))
        .map_err(|e| e.to_string())?;
    if m.back_page.is_some() {
        cover(p, m, true)?
            .save(root.join("assets/back.png"))
            .map_err(|e| e.to_string())?;
    }
    fs::write(root.join("scene.json"), serde_json::to_vec(m).unwrap())
        .map_err(|e| e.to_string())?;
    Ok(())
}
// A content fingerprint includes resolved pixels, not the global revision: unrelated edits may continue.
fn fingerprint(root: &Path) -> Result<String, String> {
    let mut out = String::new();
    for name in ["scene.json", "assets/front.png", "assets/back.png"] {
        let path = root.join(name);
        if path.exists() {
            out.push_str(&crate::media_store::digest(&path)?);
        }
    }
    Ok(out)
}
pub fn start(p: &Project, revision: u64, id: &str, movie: bool) -> Result<String, String> {
    let m = scene(p, id)?.clone();
    validate_scene(&m)?;
    cover(p, &m, false)?;
    let cli = crate::motion::cli()?;
    let p = p.clone();
    crate::media_service::begin(
        &p.document_id.clone(),
        revision,
        if movie {
            "mockup_movie"
        } else {
            "mockup_poster"
        },
        move |folder| {
            let root = folder.join("source");
            prepare(&p, &m, &root)?;
            let fingerprint = fingerprint(&root)?;
            crate::project_package::save(&p, &root.join("editable-master.canvas"))?;
            crate::media_service::invoke(
                folder,
                &json!({"op":"mockup","directory":root,"scene":m,"movie":movie,"cli":cli,"fingerprint":fingerprint,"source_revision":revision,"document_id":p.document_id}),
            )
        },
    )
}
fn apply_output(p: &mut Project, v: &Value, movie: bool) -> Result<(), String> {
    let id = v["id"].as_str().ok_or("Choose a scene")?;
    let m = scene(p, id)?.clone();
    let job = v["job"].as_str().ok_or("Choose a completed mockup job")?;
    let status = crate::media_service::status(job, &p.document_id)?;
    if status["state"] != "complete"
        || status["mode"]
            != if movie {
                "mockup_movie"
            } else {
                "mockup_poster"
            }
        || status["result"]["scene_id"] != id
    {
        return Err("Choose the matching completed mockup render".into());
    }
    let temp = crate::media_store::folder("scratch")?.join(generate_document_id());
    prepare(p, &m, &temp)?;
    let hash = fingerprint(&temp);
    let _ = fs::remove_dir_all(temp);
    if status["result"]["fingerprint"].as_str() != Some(&hash?) {
        return Err("The scene or cover changed. Render it again before placing it".into());
    }
    let path = crate::media_service::output(job, &p.document_id)?;
    if movie {
        let a = crate::media_store::import(&path, &format!("{}.mp4", m.name))?;
        let aid = a.id.clone();
        p.production.timeline.assets.insert(aid.clone(), a);
        crate::timeline::apply(
            p,
            &json!({"action":"film_add","asset":aid,"start":v["start"].as_u64().unwrap_or(0)}),
        )?;
    } else {
        let image = image::open(path).map_err(|e| e.to_string())?.to_rgba8();
        let mut e = crate::templates::make_basic_text_element(p.alloc_id());
        e.name = format!("{} · poster snapshot", m.name);
        let scale =
            (p.width * 0.8 / image.width() as f32).min(p.height * 0.8 / image.height() as f32);
        e.width = image.width() as f32 * scale;
        e.height = image.height() as f32 * scale;
        e.x = (p.width - e.width) / 2.;
        e.y = (p.height - e.height) / 2.;
        e.kind = ElementKind::Image {
            source_name: e.name.clone(),
            width_px: image.width(),
            height_px: image.height(),
            raw_rgba: image.into_raw().into(),
            mask: ImageMask::None,
            adjustments: Default::default(),
            border_color: [0, 0, 0, 0],
            border_width: 0.,
            revision: 1,
        };
        let index = v["page"].as_u64().ok_or("Choose destination page")? as usize;
        if p.pages.get(index).is_some_and(|pg| {
            pg.id.to_string() == m.page_id || Some(&pg.id.to_string()) == m.back_page.as_ref()
        }) {
            return Err(
                "Place the poster on a different page to keep the cover independent".into(),
            );
        }
        p.pages
            .get_mut(index)
            .ok_or("Destination page outside document")?
            .elements
            .push(e);
    }
    Ok(())
}
pub fn texture_bytes(p: &Project, id: &str, back: bool) -> Result<Vec<u8>, String> {
    let image = cover(p, scene(p, id)?, back)?;
    let mut bytes = std::io::Cursor::new(Vec::new());
    image
        .write_to(&mut bytes, image::ImageFormat::Png)
        .map_err(|e| e.to_string())?;
    Ok(bytes.into_inner())
}
