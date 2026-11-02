//! Frame-based editing shared by native, browser and coding-agent interfaces.
use crate::model::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Timeline {
    pub enabled: bool,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration: u64,
    pub background: ColorRgba,
    pub assets: BTreeMap<String, MediaAsset>,
    pub tracks: Vec<Track>,
    pub clips: Vec<Clip>,
    pub next_id: u64,
}
impl Default for Timeline {
    fn default() -> Self {
        Self {
            enabled: false,
            width: 1920,
            height: 1080,
            fps: 30,
            duration: 900,
            background: rgb(16, 22, 29),
            assets: BTreeMap::new(),
            tracks: vec![],
            clips: vec![],
            next_id: 1,
        }
    }
}
impl Timeline {
    pub fn active(&self) -> bool {
        self.enabled || !self.assets.is_empty() || !self.clips.is_empty()
    }
    fn alloc(&mut self) -> String {
        let id = self.next_id.to_string();
        self.next_id += 1;
        id
    }
    pub fn us(&self, frames: u64) -> u64 {
        frames * 1_000_000 / self.fps as u64
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MediaAsset {
    pub id: String,
    pub name: String,
    pub extension: String,
    pub kind: String,
    pub bytes: u64,
    pub duration_us: u64,
    pub width: u32,
    pub height: u32,
    pub has_audio: bool,
    pub sample_rate: u32,
    pub channels: u32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Track {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub enabled: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct AudioMix {
    pub enabled: bool,
    pub gain_db: f32,
    pub fade_in: u64,
    pub fade_out: u64,
    pub origin: i64,
    pub span: u64,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DesignSource {
    pub variant: Option<String>,
    pub page_id: u64,
    pub transparent: bool,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CaptionStyle {
    pub family: FontFamily,
    pub size: f32,
    pub color: ColorRgba,
    pub box_color: ColorRgba,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub align: TextAlign,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Clip {
    pub id: String,
    pub track: String,
    pub name: String,
    pub asset: Option<String>,
    pub design: Option<DesignSource>,
    pub text: Option<String>,
    pub start: u64,
    pub duration: u64,
    pub source_in_us: u64,
    pub source_anchor_us: u64,
    pub source_offset_frames: u64,
    pub rect: [f32; 4],
    pub fit: String,
    pub opacity: f32,
    pub audio: AudioMix,
    pub caption: Option<CaptionStyle>,
    #[serde(default)]
    pub motion: Option<Motion>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Motion {
    pub origin: i64,
    pub span: u64,
    pub keys: Vec<MotionKey>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct MotionKey {
    pub frame: u64,
    pub x: f32,
    pub y: f32,
    pub scale: f32,
    pub rotation: f32,
    pub opacity: f32,
    pub ease: String,
}
impl MotionKey {
    fn neutral(frame: u64) -> Self {
        Self {
            frame,
            x: 0.,
            y: 0.,
            scale: 1.,
            rotation: 0.,
            opacity: 1.,
            ease: "ease-out".into(),
        }
    }
}
pub fn has_motion(t: &Timeline) -> bool {
    t.clips.iter().any(|c| c.motion.is_some())
}
fn validate_motion(m: &Motion) -> Result<(), String> {
    if m.span == 0
        || m.span > 36000
        || m.origin.unsigned_abs() > 72000
        || m.keys.is_empty()
        || m.keys.len() > 64
    {
        return Err("Use 1–64 motion keyframes within the original clip span".into());
    }
    let mut previous = None;
    for k in &m.keys {
        if k.frame >= m.span
            || previous.is_some_and(|p| p >= k.frame)
            || [k.x, k.y, k.scale, k.rotation, k.opacity]
                .iter()
                .any(|x| !x.is_finite())
            || k.x.abs() > 16000.
            || k.y.abs() > 16000.
            || !(0.05..=8.).contains(&k.scale)
            || k.rotation.abs() > 3600.
            || !(0.0..=1.).contains(&k.opacity)
            || !["linear", "ease-in", "ease-out", "ease-in-out", "hold"].contains(&k.ease.as_str())
        {
            return Err("Motion keys must be ordered and unique; use finite placement, scale 0.05–8 and opacity 0–1".into());
        }
        previous = Some(k.frame);
    }
    Ok(())
}
fn s<'a>(v: &'a Value, k: &str) -> Result<&'a str, String> {
    v[k].as_str().ok_or_else(|| format!("Missing text: {k}"))
}
fn n(v: &Value, k: &str) -> Result<u64, String> {
    v[k].as_u64()
        .ok_or_else(|| format!("Use a non-negative frame count: {k}"))
}
fn f(v: &Value, k: &str) -> Result<f32, String> {
    let x = v[k]
        .as_f64()
        .ok_or_else(|| format!("Missing number: {k}"))? as f32;
    if !x.is_finite() {
        return Err("Use finite values".into());
    }
    Ok(x)
}
fn color(v: &Value, k: &str) -> Result<ColorRgba, String> {
    let t = s(v, k)?;
    if t.len() != 7 || !t.starts_with('#') {
        return Err("Use #rrggbb".into());
    }
    let x = u32::from_str_radix(&t[1..], 16).map_err(|_| "Invalid colour")?;
    Ok(rgb((x >> 16) as u8, (x >> 8) as u8, x as u8))
}
pub fn validate(t: &Timeline) -> Result<(), String> {
    if !t.active() {
        return Ok(());
    }
    if ![24, 25, 30, 50, 60].contains(&t.fps)
        || t.width < 64
        || t.height < 64
        || t.width > 3840
        || t.height > 3840
        || t.width % 2 != 0
        || t.height % 2 != 0
        || t.width as u64 * t.height as u64 > 8_294_400
        || t.duration == 0
        || t.duration > t.fps as u64 * 600
    {
        return Err("Use even 64–3840 dimensions (up to 4K pixels), 24/25/30/50/60 FPS and up to 10 minutes".into());
    }
    if t.assets.len() > 128 || t.tracks.len() > 16 || t.clips.len() > 200 {
        return Err("Timeline limits: 128 media assets, 16 tracks and 200 clips".into());
    }
    if t.assets.values().map(|a| a.bytes as u128).sum::<u128>() > 256 * 1024 * 1024 {
        return Err("Keep originals in one portable film below 256 MiB".into());
    }
    for (id, a) in &t.assets {
        if id != &a.id
            || id.len() != 64
            || !id.bytes().all(|b| b.is_ascii_hexdigit())
            || !crate::media_store::extensions().contains(&a.extension.as_str())
            || !matches!(a.kind.as_str(), "video" | "audio" | "image")
            || a.bytes == 0
            || (a.kind != "audio" && (a.width == 0 || a.height == 0))
            || (a.kind != "image" && a.duration_us == 0)
            || a.bytes > 128 * 1024 * 1024
            || a.width > 8192
            || a.height > 8192
            || a.duration_us > 7_200_000_000
        {
            return Err("Invalid or oversized media asset".into());
        }
    }
    if t.next_id == 0 || t.next_id > 1_000_000 {
        return Err("Invalid timeline identity counter".into());
    }
    let safe_id = |id: &str| {
        id.parse::<u64>().is_ok_and(|n| n > 0 && n < t.next_id)
            && id.bytes().all(|b| b.is_ascii_digit())
    };
    let mut ids = BTreeSet::new();
    for track in &t.tracks {
        if !safe_id(&track.id)
            || !ids.insert(&track.id)
            || !matches!(track.kind.as_str(), "visual" | "audio" | "caption")
            || track.name.len() > 160
        {
            return Err("Invalid timeline track".into());
        }
    }
    let mut clipids = BTreeSet::new();
    for c in &t.clips {
        let track = t
            .tracks
            .iter()
            .find(|tr| tr.id == c.track)
            .ok_or("Clip track missing")?;
        if !safe_id(&c.id)
            || ids.contains(&c.id)
            || !clipids.insert(&c.id)
            || c.duration == 0
            || c.start.saturating_add(c.duration) > t.duration
            || c.name.len() > 160
            || c.rect.iter().any(|x| !x.is_finite() || x.abs() > 16000.)
            || c.rect[2] <= 0.
            || c.rect[3] <= 0.
            || !c.opacity.is_finite()
            || !(0.0..=1.).contains(&c.opacity)
            || !matches!(c.fit.as_str(), "cover" | "contain")
        {
            return Err("Invalid clip timing or placement".into());
        }
        let sources = c.asset.is_some() as u8 + c.design.is_some() as u8 + c.text.is_some() as u8;
        if sources != 1 {
            return Err("Each clip needs exactly one media, design or caption source".into());
        }
        if let Some(id) = &c.asset {
            let a = t
                .assets
                .get(id)
                .ok_or("Clip media is missing from the library")?;
            if (track.kind == "audio" && !a.has_audio)
                || (track.kind == "visual" && a.kind == "audio")
                || track.kind == "caption"
            {
                return Err("Media does not match this track type".into());
            }
            if a.kind != "image"
                && c.source_in_us.saturating_add(t.us(c.duration)) > a.duration_us + 50_000
            {
                return Err(
                    "Clip extends beyond its source. Shorten its duration or source-in point"
                        .into(),
                );
            }
            if c.source_offset_frames > t.fps as u64 * 600
                || c.source_anchor_us
                    .saturating_add(t.us(c.source_offset_frames))
                    != c.source_in_us
            {
                return Err("Invalid source sampling anchor".into());
            }
            if a.kind == "image" && (c.source_in_us != 0 || c.source_anchor_us != 0) {
                return Err("Still images start at source time zero".into());
            }
        }
        if c.design.is_some() && track.kind != "visual" {
            return Err("Design pages belong on visual tracks".into());
        }
        if let Some(text) = &c.text {
            if track.kind != "caption"
                || text.trim().is_empty()
                || text.len() > 4000
                || c.caption.is_none()
            {
                return Err("Use a non-empty caption on a caption track".into());
            }
        }
        if let Some(m) = &c.motion {
            if track.kind == "audio" {
                return Err("Motion belongs on a picture or caption clip".into());
            }
            validate_motion(m)?;
        }
        let a = &c.audio;
        if !a.gain_db.is_finite()
            || !(-60.0..=12.).contains(&a.gain_db)
            || a.fade_in.saturating_add(a.fade_out) > a.span
            || a.span > t.fps as u64 * 600
            || a.origin.unsigned_abs() > t.fps as u64 * 1200
        {
            return Err("Invalid gain or audio fade envelope".into());
        }
        if let Some(st) = &c.caption {
            if !st.size.is_finite()
                || !(8.0..=300.).contains(&st.size)
                || [st.x, st.y, st.width, st.height]
                    .iter()
                    .any(|n| !n.is_finite())
                || st.width <= 0.
                || st.height <= 0.
                || st.x < 0.
                || st.y < 0.
                || st.x + st.width > t.width as f32 + 0.01
                || st.y + st.height > t.height as f32 + 0.01
            {
                return Err("Caption frame must fit inside the movie canvas".into());
            }
        }
    }
    for track in &t.tracks {
        let mut clips = t
            .clips
            .iter()
            .filter(|c| c.track == track.id)
            .collect::<Vec<_>>();
        clips.sort_by_key(|c| c.start);
        if clips
            .windows(2)
            .any(|w| w[0].start + w[0].duration > w[1].start)
        {
            return Err(format!(
                "Clips overlap on '{}'. Use another track to layer or mix them",
                track.name
            ));
        }
    }
    Ok(())
}
pub fn supports(a: &str) -> bool {
    a.starts_with("film_")
}
pub fn apply(p: &mut Project, v: &Value) -> Result<(), String> {
    let action = s(v, "action")?;
    if action == "film_design" {
        let variant = p.production.studio.active_variant.clone();
        let page = p
            .pages
            .get(v["page"].as_u64().unwrap_or(0) as usize)
            .ok_or("Design page not found")?
            .id;
        let t = &mut p.production.timeline;
        ensure(t);
        let track = if let Some(id) = v["track"].as_str() {
            id.into()
        } else {
            let id = t.alloc();
            t.tracks.push(Track {
                id: id.clone(),
                name: "Design overlay".into(),
                kind: "visual".into(),
                enabled: true,
            });
            id
        };
        let mut c = base(
            t,
            track,
            "Design overlay",
            n(v, "start")?,
            n(v, "duration")?,
        );
        c.fit = "contain".into();
        c.design = Some(DesignSource {
            variant,
            page_id: page,
            transparent: v["transparent"].as_bool().unwrap_or(true),
        });
        c.audio.enabled = false;
        t.clips.push(c);
        return validate(t);
    }
    let requested_family = if let Some(f) = v.get("family").and_then(Value::as_str) {
        Some(if let Some(id) = f.strip_prefix("imported:") {
            let id = u64::from_str_radix(id, 16).map_err(|_| "Invalid caption font")?;
            if !p.imported_fonts.iter().any(|f| f.id == id) {
                return Err("Import the caption font first".into());
            }
            FontFamily::Imported(id)
        } else {
            serde_json::from_value(Value::String(f.into())).map_err(|_| "Unknown caption font")?
        })
    } else {
        None
    };
    let t = &mut p.production.timeline;
    match action {
        "film_setup" => {
            let width = u32::try_from(n(v, "width")?).map_err(|_| "Invalid width")?;
            let height = u32::try_from(n(v, "height")?).map_err(|_| "Invalid height")?;
            let fps = u32::try_from(n(v, "fps")?).map_err(|_| "Invalid FPS")?;
            let duration = n(v, "duration")?;
            if !t.clips.is_empty() && (fps != t.fps || width != t.width || height != t.height) {
                return Err("Keep canvas/FPS fixed after adding clips; start a new film project to change them".into());
            }
            t.enabled = true;
            t.width = width;
            t.height = height;
            t.fps = fps;
            t.duration = duration;
            if v.get("background").is_some() {
                t.background = color(v, "background")?;
            }
            ensure(t);
        }
        "film_track" => {
            ensure(t);
            let kind = s(v, "kind")?;
            let id = t.alloc();
            t.tracks.push(Track {
                id,
                name: s(v, "name")?.into(),
                kind: kind.into(),
                enabled: true,
            });
        }
        "film_track_update" => {
            let tr = t
                .tracks
                .iter_mut()
                .find(|x| x.id == v["id"].as_str().unwrap_or(""))
                .ok_or("Track not found")?;
            if let Some(name) = v["name"].as_str() {
                tr.name = name.into();
            }
            if let Some(enabled) = v["enabled"].as_bool() {
                tr.enabled = enabled;
            }
            if let Some(to) = v["to"].as_u64() {
                let id = tr.id.clone();
                let at = t.tracks.iter().position(|x| x.id == id).unwrap();
                if to as usize >= t.tracks.len() {
                    return Err("Track position outside the timeline".into());
                }
                let tr = t.tracks.remove(at);
                t.tracks.insert(to as usize, tr);
            }
        }
        "film_add" => {
            ensure(t);
            let asset = s(v, "asset")?;
            let a = t.assets.get(asset).ok_or("Media asset not found")?.clone();
            let kind = if a.kind == "audio" { "audio" } else { "visual" };
            let track = if let Some(id) = v["track"].as_str() {
                id.into()
            } else {
                visual_track(t, kind)
            };
            let start = n(v, "start")?;
            let source = v["source_in_us"].as_u64().unwrap_or(0);
            let duration = v["duration"].as_u64().unwrap_or_else(|| {
                if a.kind == "image" {
                    (t.fps as u64 * 3).min(t.duration.saturating_sub(start))
                } else {
                    ((a.duration_us.saturating_sub(source)) * t.fps as u64 / 1_000_000)
                        .min(t.duration.saturating_sub(start))
                }
            });
            let mut c = base(t, track, &a.name, start, duration);
            c.asset = Some(asset.into());
            c.source_in_us = source;
            c.source_anchor_us = source;
            c.audio.enabled = a.has_audio;
            t.clips.push(c);
        }
        "film_caption" => {
            ensure(t);
            let track = if let Some(id) = v["track"].as_str() {
                id.into()
            } else {
                visual_track(t, "caption")
            };
            let mut c = base(t, track, "Caption", n(v, "start")?, n(v, "duration")?);
            c.text = Some(s(v, "text")?.into());
            c.caption = Some(CaptionStyle {
                family: FontFamily::Sans,
                size: (t.height as f32 * 0.032).max(8.),
                color: rgb(255, 255, 255),
                box_color: [12, 16, 22, 220],
                x: t.width as f32 * 0.10,
                y: t.height as f32 * 0.73,
                width: t.width as f32 * 0.8,
                height: t.height as f32 * 0.22,
                align: TextAlign::Center,
            });
            c.audio.enabled = false;
            t.clips.push(c);
        }
        "film_motion" => {
            let id = s(v, "id")?;
            let c = t
                .clips
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or("Clip not found")?;
            if v["preset"] == "none" {
                c.motion = None;
            } else {
                let keys = if let Some(keys) = v.get("keys") {
                    serde_json::from_value(keys.clone())
                        .map_err(|e| format!("Invalid motion keys: {e}"))?
                } else {
                    if c.duration < 2 {
                        return Err("Motion presets need at least two frames".into());
                    }
                    let end = (t.fps as u64).min(c.duration - 1);
                    let mut a = MotionKey::neutral(0);
                    let mut b = MotionKey::neutral(end);
                    match s(v, "preset")? {
                        "fade" => a.opacity = 0.,
                        "rise" => {
                            a.y = t.height as f32 * 0.08;
                            a.opacity = 0.;
                        }
                        "slide" => {
                            a.x = -(t.width as f32) * 0.12;
                            a.opacity = 0.;
                        }
                        "punch" => {
                            a.scale = 0.85;
                            a.opacity = 0.;
                        }
                        "drift" => {
                            a.scale = 1.;
                            b.scale = 1.08;
                            b.frame = c.duration - 1;
                            b.ease = "linear".into();
                            a.ease = "linear".into();
                        }
                        _ => return Err("Choose none, fade, rise, slide, punch or drift".into()),
                    }
                    vec![a, b]
                };
                let inherited = c
                    .motion
                    .as_ref()
                    .filter(|_| v["preserve_timing"] == true && v.get("keys").is_some());
                c.motion = Some(Motion {
                    origin: inherited.map_or(c.start as i64, |m| m.origin),
                    span: inherited.map_or(c.duration, |m| m.span),
                    keys,
                });
            }
        }
        "film_clip" => {
            let id = s(v, "id")?;
            let c = t
                .clips
                .iter_mut()
                .find(|c| c.id == id)
                .ok_or("Clip not found")?;
            let oldstart = c.start;
            let oldsource = c.source_in_us;
            let oldduration = c.duration;
            if let Some(x) = v["name"].as_str() {
                c.name = x.into();
            }
            if let Some(x) = v["track"].as_str() {
                c.track = x.into();
            }
            for (key, target) in [
                ("start", &mut c.start),
                ("duration", &mut c.duration),
                ("source_in_us", &mut c.source_in_us),
            ] {
                if v.get(key).is_some() {
                    *target = n(v, key)?;
                }
            }
            if c.source_in_us != oldsource {
                let offset = c
                    .source_in_us
                    .saturating_sub(c.source_anchor_us)
                    .saturating_mul(t.fps as u64)
                    .saturating_add(500_000)
                    / 1_000_000;
                let candidate = c
                    .source_anchor_us
                    .saturating_add(offset * 1_000_000 / t.fps as u64);
                if c.source_in_us >= c.source_anchor_us
                    && candidate.abs_diff(c.source_in_us) <= 1
                    && offset <= t.fps as u64 * 600
                {
                    c.source_offset_frames = offset;
                    c.source_in_us = candidate;
                } else {
                    c.source_anchor_us = c.source_in_us;
                    c.source_offset_frames = 0;
                }
            }
            if c.source_in_us != oldsource || c.duration != oldduration {
                c.audio.origin = c.start as i64;
                c.audio.span = c.duration;
                c.audio.fade_in = c.audio.fade_in.min(c.duration);
                c.audio.fade_out = c.audio.fade_out.min(c.duration - c.audio.fade_in);
            } else {
                c.audio.origin += c.start as i64 - oldstart as i64;
            }
            if c.source_in_us == oldsource && c.duration == oldduration {
                if let Some(m) = &mut c.motion {
                    m.origin += c.start as i64 - oldstart as i64;
                }
            }
            if v.get("gain_db").is_some() {
                c.audio.gain_db = f(v, "gain_db")?;
            }
            if let Some(x) = v["audio_enabled"].as_bool() {
                c.audio.enabled = x;
            }
            if v.get("fade_in").is_some() || v.get("fade_out").is_some() {
                c.audio.fade_in = n(v, "fade_in")?;
                c.audio.fade_out = n(v, "fade_out")?;
                c.audio.origin = c.start as i64;
                c.audio.span = c.duration;
            }
            if let Some(text) = v["text"].as_str() {
                if c.text.is_none() {
                    return Err("Select a caption clip".into());
                }
                c.text = Some(text.into());
            }
            if let Some(rect) = v["rect"].as_array() {
                if rect.len() != 4 {
                    return Err("Use x, y, width, height for clip placement".into());
                }
                for (i, x) in rect.iter().enumerate() {
                    c.rect[i] = x.as_f64().ok_or("Invalid placement")? as f32;
                }
            }
            if let Some(fit) = v["fit"].as_str() {
                c.fit = fit.into();
            }
            if v.get("opacity").is_some() {
                c.opacity = f(v, "opacity")?;
            }
            if let Some(st) = &mut c.caption {
                for (key, target) in [
                    ("size", &mut st.size),
                    ("x", &mut st.x),
                    ("y", &mut st.y),
                    ("width", &mut st.width),
                    ("height", &mut st.height),
                ] {
                    if v.get(key).is_some() {
                        *target = f(v, key)?;
                    }
                }
                if v.get("color").is_some() {
                    st.color = color(v, "color")?;
                }
                if let Some(f) = requested_family {
                    st.family = f;
                }
            }
        }
        "film_split" => {
            let at = n(v, "at")?;
            let i = t
                .clips
                .iter()
                .position(|c| c.id == v["id"].as_str().unwrap_or(""))
                .ok_or("Clip not found")?;
            let mut right = t.clips[i].clone();
            if at <= right.start || at >= right.start + right.duration {
                return Err("Place the playhead strictly inside the selected clip".into());
            }
            let offset = at - right.start;
            right.id = t.alloc();
            right.start = at;
            right.duration -= offset;
            if right
                .asset
                .as_ref()
                .and_then(|id| t.assets.get(id))
                .is_some_and(|a| a.kind != "image")
            {
                right.source_offset_frames += offset;
                right.source_in_us = right.source_anchor_us + t.us(right.source_offset_frames);
            }
            t.clips[i].duration = offset;
            t.clips.push(right);
        }
        "film_forget_asset" => {
            let id = s(v, "asset")?;
            if t.clips.iter().any(|c| c.asset.as_deref() == Some(id)) {
                return Err(
                    "Delete this asset’s timeline clips before removing it from the project".into(),
                );
            }
            if t.assets.remove(id).is_none() {
                return Err("Media asset not found".into());
            }
        }
        "film_delete" => {
            let id = s(v, "id")?;
            if !t.clips.iter().any(|c| c.id == id) {
                return Err("Clip not found".into());
            }
            t.clips.retain(|c| c.id != id);
        }
        "film_detach_audio" => {
            let id = s(v, "id")?;
            let i = t
                .clips
                .iter()
                .position(|c| c.id == id)
                .ok_or("Clip not found")?;
            let mut c = t.clips[i].clone();
            if !c.audio.enabled
                || !c
                    .asset
                    .as_ref()
                    .and_then(|id| t.assets.get(id))
                    .is_some_and(|a| a.has_audio)
            {
                return Err("This clip has no attached source audio".into());
            }
            let track = t.alloc();
            t.tracks.push(Track {
                id: track.clone(),
                name: "Detached source audio".into(),
                kind: "audio".into(),
                enabled: true,
            });
            c.id = t.alloc();
            c.track = track;
            c.name = "Detached source audio".into();
            c.motion = None;
            c.design = None;
            c.text = None;
            c.caption = None;
            t.clips[i].audio.enabled = false;
            t.clips.push(c);
        }
        _ => return Err("Unknown film command".into()),
    }
    validate(t)
}
fn ensure(t: &mut Timeline) {
    t.enabled = true;
    if t.tracks.is_empty() {
        let id = t.alloc();
        t.tracks.push(Track {
            id,
            name: "Video 1".into(),
            kind: "visual".into(),
            enabled: true,
        });
    }
}
fn visual_track(t: &mut Timeline, kind: &str) -> String {
    if let Some(tr) = t.tracks.iter().find(|x| x.kind == kind) {
        return tr.id.clone();
    }
    let id = t.alloc();
    t.tracks.push(Track {
        id: id.clone(),
        name: match kind {
            "audio" => "Audio 1",
            "caption" => "Captions",
            _ => "Video 1",
        }
        .into(),
        kind: kind.into(),
        enabled: true,
    });
    id
}
fn base(t: &mut Timeline, track: String, name: &str, start: u64, duration: u64) -> Clip {
    Clip {
        id: t.alloc(),
        track,
        name: name.into(),
        asset: None,
        design: None,
        text: None,
        start,
        duration,
        source_in_us: 0,
        source_anchor_us: 0,
        source_offset_frames: 0,
        rect: [0., 0., t.width as f32, t.height as f32],
        fit: "cover".into(),
        opacity: 1.,
        audio: AudioMix {
            enabled: true,
            gain_db: 0.,
            fade_in: 0,
            fade_out: 0,
            origin: start as i64,
            span: duration,
        },
        caption: None,
        motion: None,
    }
}
pub fn srt(t: &Timeline) -> String {
    let mut clips = t
        .clips
        .iter()
        .filter(|c| c.text.is_some())
        .collect::<Vec<_>>();
    clips.sort_by_key(|c| c.start);
    fn stamp(us: u64) -> String {
        let ms = us / 1000;
        format!(
            "{:02}:{:02}:{:02},{:03}",
            ms / 3_600_000,
            ms / 60_000 % 60,
            ms / 1000 % 60,
            ms % 1000
        )
    }
    clips
        .iter()
        .enumerate()
        .map(|(i, c)| {
            format!(
                "{}\n{} --> {}\n{}\n\n",
                i + 1,
                stamp(t.us(c.start)),
                stamp(t.us(c.start + c.duration)),
                c.text.as_deref().unwrap_or("")
            )
        })
        .collect()
}
pub fn import_srt(p: &mut Project, text: &str) -> Result<(), String> {
    if text.len() > 1_000_000 {
        return Err("Subtitle file is too large".into());
    }
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");
    let fps = p.production.timeline.fps as u64;
    let mut commands = Vec::new();
    for block in text.split("\n\n").filter(|b| !b.trim().is_empty()) {
        let lines = block.lines().collect::<Vec<_>>();
        let at = lines
            .iter()
            .position(|s| s.contains(" --> "))
            .ok_or("Invalid SRT timing line")?;
        let (a, b) = lines[at].split_once(" --> ").ok_or("Invalid SRT timing")?;
        fn time(s: &str) -> Result<u64, String> {
            let n = s
                .trim()
                .split([':', ','])
                .map(str::parse::<u64>)
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| "Invalid SRT timestamp")?;
            if n.len() != 4 || n[0] > 24 || n[1] > 59 || n[2] > 59 || n[3] > 999 {
                return Err("Use HH:MM:SS,mmm SRT timestamps".into());
            }
            Ok(n[0] * 3_600_000 + n[1] * 60_000 + n[2] * 1000 + n[3])
        }
        let start = (time(a)? * fps + 500) / 1000;
        let end = (time(b)? * fps + 500) / 1000;
        if end <= start {
            return Err("Caption must last at least one frame".into());
        }
        commands.push(serde_json::json!({"action":"film_caption","start":start,"duration":end-start,"text":lines[at+1..].join("\n")}));
    }
    if commands.len() > 200 {
        return Err("Keep at most 200 caption clips".into());
    }
    for c in commands {
        apply(p, &c)?;
    }
    Ok(())
}
