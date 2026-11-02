//! Portable campaign layouts, named colour links and reusable editable groups.
use crate::{
    model::*,
    production::{valid_key, DataField},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StudioSettings {
    pub tokens: BTreeMap<String, ColorRgba>,
    pub colors: BTreeMap<u64, BTreeMap<String, String>>,
    pub backgrounds: BTreeMap<u64, String>,
    pub components: BTreeMap<String, Component>,
    pub active_variant: Option<String>,
    pub variants: BTreeMap<String, VariantLayout>,
}
impl StudioSettings {
    pub fn active(&self) -> bool {
        !self.tokens.is_empty()
            || !self.colors.is_empty()
            || !self.backgrounds.is_empty()
            || !self.components.is_empty()
            || self.active_variant.is_some()
            || !self.variants.is_empty()
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Component {
    pub name: String,
    pub width: f32,
    pub height: f32,
    pub elements: Vec<CanvasElement>,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct VariantLayout {
    pub format: DesignFormat,
    pub width: f32,
    pub height: f32,
    pub width_mm: f32,
    pub height_mm: f32,
    pub units_per_inch: f32,
    pub bleed_mm: f32,
    pub crop_marks: bool,
    pub pages: Vec<CanvasPage>,
    pub print_settings: PrintSettings,
    pub book: crate::book_production::BookSettings,
    pub puzzles: Vec<crate::puzzle_production::WordSearchProduct>,
}
impl VariantLayout {
    pub fn capture(p: &Project) -> Self {
        Self {
            format: p.format,
            width: p.width,
            height: p.height,
            width_mm: p.width_mm,
            height_mm: p.height_mm,
            units_per_inch: p.units_per_inch,
            bleed_mm: p.bleed_mm,
            crop_marks: p.crop_marks,
            pages: p.pages.clone(),
            print_settings: p.print_settings.clone(),
            book: p.production.book.clone(),
            puzzles: p.production.puzzles.clone(),
        }
    }
    pub fn apply(self, p: &mut Project) {
        p.format = self.format;
        p.width = self.width;
        p.height = self.height;
        p.width_mm = self.width_mm;
        p.height_mm = self.height_mm;
        p.units_per_inch = self.units_per_inch;
        p.bleed_mm = self.bleed_mm;
        p.crop_marks = self.crop_marks;
        p.pages = self.pages;
        p.print_settings = self.print_settings;
        p.production.book = self.book;
        p.production.puzzles = self.puzzles;
    }
}
pub fn activate(p: &mut Project, name: &str) -> Result<(), String> {
    if p.production.studio.active_variant.as_deref() == Some(name) {
        return Ok(());
    }
    let target = p
        .production
        .studio
        .variants
        .get(name)
        .cloned()
        .ok_or("Campaign layout not found")?;
    let current = p
        .production
        .studio
        .active_variant
        .clone()
        .unwrap_or_else(|| "Original".into());
    if p.production.studio.variants.contains_key(&current) {
        return Err("Campaign layout names must be unique".into());
    }
    p.production.studio.variants.remove(name);
    let snapshot = VariantLayout::capture(p);
    p.production.studio.variants.insert(current, snapshot);
    p.production.studio.active_variant = Some(name.into());
    target.apply(p);
    Ok(())
}
pub fn save_variant(p: &mut Project, name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name.len() > 80 {
        return Err("Use a layout name of 1–80 characters".into());
    }
    if p.production.studio.variants.len() >= 15 {
        return Err("Keep at most 16 campaign layouts".into());
    }
    if p.production
        .studio
        .active_variant
        .as_deref()
        .unwrap_or("Original")
        == name
        || p.production.studio.variants.contains_key(name)
    {
        return Err("Choose a new layout name".into());
    }
    let mut layout = VariantLayout::capture(p);
    for page in &mut layout.pages {
        let source_id = page.id;
        page.id = p.alloc_id();
        if let Some(token) = p.production.studio.backgrounds.get(&source_id).cloned() {
            p.production.studio.backgrounds.insert(page.id, token);
        }
        page.elements = crate::document_ops::fresh_copies(p, &page.elements, 0., 0.);
    }
    layout.book.master_elements =
        crate::document_ops::fresh_copies(p, &layout.book.master_elements, 0., 0.);
    if !layout.puzzles.is_empty() {
        return Err("Detach generated puzzle pages before duplicating a campaign layout".into());
    }
    p.production
        .studio
        .active_variant
        .get_or_insert_with(|| "Original".into());
    p.production.studio.variants.insert(name.into(), layout);
    Ok(())
}
pub fn asset_sets(s: &StudioSettings) -> Vec<(&str, &[CanvasElement])> {
    let mut out = Vec::new();
    for (name, v) in &s.variants {
        for page in &v.pages {
            out.push((page.title.as_str(), page.elements.as_slice()));
        }
        out.push((name.as_str(), v.book.master_elements.as_slice()));
    }
    for c in s.components.values() {
        out.push((c.name.as_str(), c.elements.as_slice()));
    }
    out
}
pub fn for_each_extra_mut(s: &mut StudioSettings, mut f: impl FnMut(&mut CanvasElement)) {
    for v in s.variants.values_mut() {
        for page in &mut v.pages {
            for e in &mut page.elements {
                f(e);
            }
        }
        for e in &mut v.book.master_elements {
            f(e);
        }
    }
    for c in s.components.values_mut() {
        for e in &mut c.elements {
            f(e);
        }
    }
}
pub fn color_hex(c: ColorRgba) -> String {
    format!("#{:02x}{:02x}{:02x}", c[0], c[1], c[2])
}
pub fn resolve_element(
    e: &mut CanvasElement,
    s: &StudioSettings,
    strict: bool,
) -> Result<(), String> {
    let Some(bindings) = s.colors.get(&e.id) else {
        return Ok(());
    };
    for (target, key) in bindings {
        let Some(color) = s.tokens.get(key).copied() else {
            if strict {
                return Err(format!("Missing brand colour '{key}' on '{}'", e.name));
            }
            continue;
        };
        apply_color(e, target, color)?;
    }
    Ok(())
}
pub fn apply_color(e: &mut CanvasElement, target: &str, color: ColorRgba) -> Result<(), String> {
    match (target, &mut e.kind) {
        ("text", ElementKind::Text { color: c, .. })
        | ("fill", ElementKind::FreehandPath { color: c, .. }) => *c = color,
        ("fill", ElementKind::Shape { fill, .. }) => fill.set_primary_color(color),
        ("stroke", ElementKind::Shape { stroke_color, .. }) => *stroke_color = color,
        ("stroke", ElementKind::Image { border_color, .. }) => *border_color = color,
        _ => return Err("That brand-colour target is not supported by this layer".into()),
    }
    Ok(())
}
pub fn bind(p: &mut Project, ids: &[u64], target: &str, key: Option<&str>) -> Result<(), String> {
    if let Some(key) = key {
        if !p.production.studio.tokens.contains_key(key) {
            return Err("Define the named colour first".into());
        }
    }
    for id in ids {
        let e = p
            .pages
            .iter_mut()
            .flat_map(|p| &mut p.elements)
            .find(|e| e.id == *id)
            .ok_or("Layer not found")?;
        let actual = if target == "primary" {
            if matches!(e.kind, ElementKind::Text { .. }) {
                "text"
            } else {
                "fill"
            }
        } else {
            target
        };
        let old = p
            .production
            .studio
            .colors
            .get(id)
            .and_then(|b| b.get(actual))
            .and_then(|key| p.production.studio.tokens.get(key))
            .copied();
        let color = key
            .and_then(|k| p.production.studio.tokens.get(k))
            .copied()
            .or(old)
            .unwrap_or(rgb(24, 30, 36));
        apply_color(e, actual, color)?;
        let b = p.production.studio.colors.entry(*id).or_default();
        if let Some(key) = key {
            b.insert(actual.into(), key.into());
        } else {
            b.remove(actual);
        }
    }
    Ok(())
}
pub fn capture_component(
    p: &mut Project,
    page: usize,
    ids: &[u64],
    key: &str,
    name: &str,
) -> Result<(), String> {
    if !valid_key(key) || name.trim().is_empty() || name.len() > 80 {
        return Err("Use a simple component key and a name of 1–80 characters".into());
    }
    if p.production.studio.components.contains_key(key) {
        return Err("Choose a new component key".into());
    }
    if p.production.studio.components.len() >= 100 {
        return Err("Keep at most 100 reusable groups".into());
    }
    let page = p.pages.get(page).ok_or("Page not found")?;
    let elements = crate::document_ops::selected_elements(page, ids, None);
    if elements.is_empty() || elements.len() > 500 {
        return Err("Select 1–500 layers to save a reusable group".into());
    }
    let bounds = crate::document_ops::bounds(&elements).ok_or("Select layers first")?;
    let mut copies = crate::document_ops::fresh_copies(p, &elements, -bounds.left(), -bounds.top());
    for e in &mut copies {
        crate::production::apply_style(e, &p.production, true)?;
        e.style_id = None;
        e.locked = false;
        e.group_id = None;
    }
    p.production.studio.components.insert(
        key.into(),
        Component {
            name: name.into(),
            width: bounds.width(),
            height: bounds.height(),
            elements: copies,
        },
    );
    Ok(())
}
pub fn insert_component(
    p: &mut Project,
    page: usize,
    key: &str,
    x: f32,
    y: f32,
    width: Option<f32>,
) -> Result<Vec<u64>, String> {
    let c = p
        .production
        .studio
        .components
        .get(key)
        .cloned()
        .ok_or("Reusable group not found")?;
    if page >= p.pages.len() {
        return Err("Page not found".into());
    }
    let scale = width.unwrap_or(c.width) / c.width.max(0.001);
    if ![x, y, scale].iter().all(|n| n.is_finite()) || !(0.01..=100.).contains(&scale) {
        return Err("Use a finite position and a reasonable component size".into());
    }
    let group = p.alloc_id();
    let mut copies = crate::document_ops::fresh_copies(p, &c.elements, 0., 0.);
    for e in &mut copies {
        e.x = e.x * scale + x;
        e.y = e.y * scale + y;
        crate::document_ops::scale_element(e, scale);
        e.group_id = Some(group);
    }
    let ids = copies.iter().map(|e| e.id).collect();
    p.pages[page].elements.extend(copies);
    Ok(ids)
}
pub fn validate(p: &Project) -> Result<(), String> {
    let s = &p.production.studio;
    if s.variants.len() > 15 || s.components.len() > 100 || s.tokens.len() > 100 {
        return Err("Studio library exceeds its supported size".into());
    }
    if s.active_variant
        .as_ref()
        .is_some_and(|name| s.variants.contains_key(name))
    {
        return Err("Duplicate active campaign layout".into());
    }
    for (key, color) in &s.tokens {
        if !valid_key(key) || color[3] != 255 {
            return Err("Brand colours require a simple key and opaque RGB value".into());
        }
    }
    for v in s.variants.values() {
        if v.pages.is_empty()
            || v.pages.len() > 1000
            || [v.width, v.height, v.width_mm, v.height_mm, v.units_per_inch]
                .iter()
                .any(|n| !n.is_finite() || *n <= 0.)
        {
            return Err("Invalid campaign layout dimensions or page count".into());
        }
    }
    let mut layer_ids = std::collections::BTreeSet::new();
    if s.active() {
        for e in p
            .pages
            .iter()
            .flat_map(|p| &p.elements)
            .chain(&p.production.book.master_elements)
            .chain(asset_sets(s).into_iter().flat_map(|(_, e)| e))
        {
            if !layer_ids.insert(e.id) {
                return Err(
                    "Campaign layers require unique identities across layouts and reusable groups"
                        .into(),
                );
            }
        }
    }
    for c in s.components.values() {
        if c.elements.is_empty()
            || c.elements.len() > 500
            || [c.width, c.height]
                .iter()
                .any(|n| !n.is_finite() || *n <= 0.)
        {
            return Err("Invalid reusable group".into());
        }
    }
    Ok(())
}

fn text(
    p: &mut Project,
    page: &mut CanvasPage,
    name: &str,
    content: &str,
    rect: [f32; 4],
    pt: f32,
    token: &str,
) {
    let mut e = crate::templates::make_basic_text_element(p.alloc_id());
    e.name = name.into();
    e.x = rect[0];
    e.y = rect[1];
    e.width = rect[2];
    e.height = rect[3];
    if let ElementKind::Text {
        content: c,
        font_size,
        bold,
        line_height,
        ..
    } = &mut e.kind
    {
        *c = content.into();
        *font_size = pt * 96. / 72.;
        *bold = true;
        *line_height = 1.08;
    }
    p.production
        .studio
        .colors
        .insert(e.id, BTreeMap::from([("text".into(), token.into())]));
    page.elements.push(e);
}
fn shape(
    p: &mut Project,
    page: &mut CanvasPage,
    name: &str,
    rect: [f32; 4],
    circle: bool,
    token: &str,
) {
    let mut e = crate::templates::make_basic_text_element(p.alloc_id());
    e.name = name.into();
    e.x = rect[0];
    e.y = rect[1];
    e.width = rect[2];
    e.height = rect[3];
    e.kind = ElementKind::Shape {
        shape_type: if circle {
            ShapeType::Circle
        } else {
            ShapeType::Rectangle
        },
        fill: FillStyle::Solid(rgb(24, 30, 36)),
        stroke_color: rgb(24, 30, 36),
        stroke_width: 0.,
        corner_radius: 0.,
    };
    p.production
        .studio
        .colors
        .insert(e.id, BTreeMap::from([("fill".into(), token.into())]));
    page.elements.push(e);
}
/// Purpose-composed variants, sharing field and colour links, never a stretched page.
pub fn starter() -> Project {
    let mut p = Project::default();
    p.title = "The next chapter · Campaign".into();
    p.pages.clear();
    p.production.studio.tokens = BTreeMap::from([
        ("ink".into(), rgb(24, 30, 36)),
        ("paper".into(), rgb(245, 241, 232)),
        ("accent".into(), rgb(213, 241, 113)),
        ("muted".into(), rgb(112, 125, 134)),
    ]);
    for (key, value) in [
        ("headline", "A little room\nfor big ideas."),
        (
            "description",
            "Thoughtful things for your next chapter.\nMade to be part of your everyday.",
        ),
        ("cta", "EXPLORE THE COLLECTION"),
        ("brand", "YOUR STUDIO"),
        ("edition", "THE NEXT CHAPTER / 01"),
    ] {
        p.production.fields.insert(
            key.into(),
            DataField {
                value: value.into(),
                required: true,
                ..Default::default()
            },
        );
    }
    for (index, (name, w, h)) in [
        ("Instagram post", 1080., 1350.),
        ("Story", 1080., 1920.),
        ("YouTube thumbnail", 1280., 720.),
        ("Print A4", 793.7008, 1122.5197),
    ]
    .into_iter()
    .enumerate()
    {
        p.width = w;
        p.height = h;
        p.width_mm = w / 96. * 25.4;
        p.height_mm = h / 96. * 25.4;
        p.format = if index == 3 {
            DesignFormat::DocumentA4
        } else {
            DesignFormat::Custom(w as u32, h as u32)
        };
        let dark = index == 1;
        let ink = if dark { "paper" } else { "ink" };
        let bg = if dark { "ink" } else { "paper" };
        let mut a = CanvasPage::new(p.alloc_id(), name);
        p.production.studio.backgrounds.insert(a.id, bg.into());
        let landscape = index == 2;
        let margin = w * 0.075;
        let content_w = if landscape { w * 0.61 } else { w * 0.85 };
        text(
            &mut p,
            &mut a,
            "Brand",
            "{{brand}}",
            [margin, h * 0.06, w * 0.6, 45.],
            if index == 3 { 14. } else { 20. },
            ink,
        );
        let size = if landscape { w * 0.19 } else { w * 0.24 };
        shape(
            &mut p,
            &mut a,
            "Accent",
            [w - margin - size, h * 0.12, size, size],
            true,
            "accent",
        );
        let y = if landscape { h * 0.25 } else { h * 0.34 };
        text(
            &mut p,
            &mut a,
            "Campaign headline",
            "{{headline}}",
            [
                margin,
                y,
                content_w,
                if landscape { h * 0.46 } else { h * 0.31 },
            ],
            if landscape {
                65.
            } else if index == 3 {
                51.
            } else {
                74.
            },
            ink,
        );
        if !landscape {
            text(
                &mut p,
                &mut a,
                "Campaign description",
                "{{description}}",
                [margin, h * 0.70, w * 0.85, h * 0.10],
                if index == 3 { 15. } else { 23. },
                ink,
            );
        }
        // Dedicated CTA group: editable text and shape, common field/token links.
        let cta_y = if landscape { h * 0.82 } else { h * 0.85 };
        let cta_w = if landscape { w * 0.56 } else { w * 0.85 };
        let cta_h = if landscape { h * 0.11 } else { h * 0.075 };
        let group = p.alloc_id();
        let start = a.elements.len();
        shape(
            &mut p,
            &mut a,
            "CTA panel",
            [margin, cta_y, cta_w, cta_h],
            false,
            "accent",
        );
        text(
            &mut p,
            &mut a,
            "CTA label",
            "{{cta}}",
            [
                margin + cta_w * 0.04,
                cta_y + cta_h * 0.22,
                cta_w * 0.92,
                cta_h * 0.65,
            ],
            if index == 3 {
                16.
            } else if landscape {
                20.
            } else {
                26.
            },
            "ink",
        );
        for e in &mut a.elements[start..] {
            e.group_id = Some(group);
        }
        p.pages = vec![a];
        if index == 0 {
            let ids = p.pages[0].elements[start..]
                .iter()
                .map(|e| e.id)
                .collect::<Vec<_>>();
            capture_component(&mut p, 0, &ids, "campaign_cta", "Campaign call to action").unwrap();
        }
        let snapshot = VariantLayout::capture(&p);
        p.production.studio.variants.insert(name.into(), snapshot);
    }
    let first = p
        .production
        .studio
        .variants
        .remove("Instagram post")
        .unwrap();
    p.production.studio.active_variant = Some("Instagram post".into());
    first.apply(&mut p);
    p
}
