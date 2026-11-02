//! Shared layout/library commands for desktop, browser and MCP callers.
use crate::{campaign, document_ops, model::*};
use serde_json::Value;
use std::collections::BTreeMap;
fn text<'a>(v: &'a Value, key: &str) -> Result<&'a str, String> {
    v[key]
        .as_str()
        .ok_or_else(|| format!("Missing text: {key}"))
}
fn number(v: &Value, key: &str) -> Result<f32, String> {
    let n = v[key]
        .as_f64()
        .ok_or_else(|| format!("Missing number: {key}"))? as f32;
    if !n.is_finite() || n.abs() > 1_000_000. {
        return Err("Use finite, bounded measurements".into());
    }
    Ok(n)
}
fn page(v: &Value, p: &Project) -> Result<usize, String> {
    let i = v["page"].as_u64().ok_or("Missing page")? as usize;
    if i >= p.pages.len() {
        return Err("Page outside document".into());
    }
    Ok(i)
}
pub fn ids(v: &Value, p: &Project, index: usize) -> Result<Vec<u64>, String> {
    let values = v["ids"].as_array().ok_or("Select layers first")?;
    if values.is_empty() || values.len() > 500 {
        return Err("Select 1–500 layers".into());
    }
    let ids = values
        .iter()
        .map(|s| {
            s.as_str()
                .ok_or("Use string layer IDs")?
                .parse::<u64>()
                .map_err(|_| "Invalid layer identity")
        })
        .collect::<Result<Vec<_>, _>>()?;
    if ids
        .iter()
        .any(|id| !p.pages[index].elements.iter().any(|e| e.id == *id))
    {
        return Err("A selected layer is no longer on this page".into());
    }
    Ok(document_ops::selection_ids(&p.pages[index], &ids, None))
}
fn rgb_value(v: &Value) -> Result<ColorRgba, String> {
    let s = text(v, "color")?;
    if s.len() != 7 || !s.starts_with('#') {
        return Err("Use #rrggbb".into());
    }
    let n = u32::from_str_radix(&s[1..], 16).map_err(|_| "Invalid colour")?;
    Ok(rgb((n >> 16) as u8, (n >> 8) as u8, n as u8))
}
pub fn supports(action: &str) -> bool {
    if crate::mockups::supports(action) {
        return true;
    }
    if crate::timeline::supports(action) {
        return true;
    }
    matches!(
        action,
        "selection_move"
            | "selection_transform"
            | "selection_align"
            | "selection_distribute"
            | "selection_group"
            | "selection_ungroup"
            | "selection_duplicate"
            | "selection_delete"
            | "brand_token"
            | "brand_bind"
            | "brand_background"
            | "component_save"
            | "component_insert"
            | "variant_save"
            | "variant_activate"
            | "field_define"
            | "campaign_create"
    )
}
pub fn apply(p: &mut Project, v: &Value) -> Result<(), String> {
    let action = text(v, "action")?;
    if crate::mockups::supports(action) {
        return crate::mockups::apply(p, v);
    }
    if crate::timeline::supports(action) {
        return crate::timeline::apply(p, v);
    }
    match action {
        "campaign_create" => {
            *p = campaign::starter();
        }
        "field_define" => {
            let key = text(v, "key")?;
            if !crate::production::valid_key(key) {
                return Err("Use a simple field key".into());
            }
            let value = text(v, "value")?;
            if value.len() > 100_000 {
                return Err("Field too long".into());
            }
            let f = p.production.fields.entry(key.into()).or_default();
            if f.image {
                return Err("Use the photo editor for image fields".into());
            }
            f.value = value.into();
            if let Some(required) = v["required"].as_bool() {
                f.required = required;
            }
        }
        "brand_token" => {
            let key = text(v, "key")?;
            if !crate::production::valid_key(key) {
                return Err("Use letters, numbers and underscores for colour keys".into());
            }
            if p.production.studio.tokens.len() >= 100
                && !p.production.studio.tokens.contains_key(key)
            {
                return Err("Keep at most 100 named colours".into());
            }
            p.production.studio.tokens.insert(key.into(), rgb_value(v)?);
        }
        "brand_bind" => {
            if v.get("token").is_none() {
                return Err("Choose a named colour, or null to unlink".into());
            }
            let i = page(v, p)?;
            let ids = ids(v, p, i)?;
            campaign::bind(
                p,
                &ids,
                v["target"].as_str().unwrap_or("primary"),
                v.get("token")
                    .filter(|t| !t.is_null())
                    .and_then(Value::as_str),
            )?;
        }
        "brand_background" => {
            if v.get("token").is_none() {
                return Err("Choose a named colour, or null to unlink".into());
            }
            let i = page(v, p)?;
            let id = p.pages[i].id;
            if let Some(key) = v["token"].as_str() {
                let color = p
                    .production
                    .studio
                    .tokens
                    .get(key)
                    .ok_or("Define this brand colour first")?;
                p.pages[i].background = PageBackground::Solid(*color);
                p.production.studio.backgrounds.insert(id, key.into());
            } else {
                if let Some(color) = p
                    .production
                    .studio
                    .backgrounds
                    .get(&id)
                    .and_then(|key| p.production.studio.tokens.get(key))
                {
                    p.pages[i].background = PageBackground::Solid(*color);
                }
                p.production.studio.backgrounds.remove(&id);
            }
        }
        "variant_save" => campaign::save_variant(p, text(v, "name")?)?,
        "variant_activate" => campaign::activate(p, text(v, "name")?)?,
        "component_save" => {
            let i = page(v, p)?;
            let ids = ids(v, p, i)?;
            campaign::capture_component(p, i, &ids, text(v, "key")?, text(v, "name")?)?;
        }
        "component_insert" => {
            let i = page(v, p)?;
            campaign::insert_component(
                p,
                i,
                text(v, "key")?,
                number(v, "x")?,
                number(v, "y")?,
                v.get("width").map(|_| number(v, "width")).transpose()?,
            )?;
        }
        _ => {
            if !supports(action) {
                return Err("Unknown layout command".into());
            }
            let i = page(v, p)?;
            let ids = ids(v, p, i)?;
            let mut originals = p.pages[i]
                .elements
                .iter()
                .filter(|e| ids.contains(&e.id))
                .cloned()
                .collect::<Vec<_>>();
            if originals.iter().any(|e| e.locked) {
                return Err("Unlock every selected layer before changing the selection".into());
            }
            let bounds = document_ops::bounds(&originals).ok_or("Select layers first")?;
            match action {
                "selection_move" => {
                    let dx = number(v, "dx")?;
                    let dy = number(v, "dy")?;
                    for e in &mut p.pages[i].elements {
                        if ids.contains(&e.id) {
                            e.x += dx;
                            e.y += dy;
                        }
                    }
                }
                "selection_transform" => {
                    let x = number(v, "x")?;
                    let y = number(v, "y")?;
                    let width = number(v, "width")?;
                    let height = number(v, "height")?;
                    if width <= 0. || height <= 0. || bounds.width() <= 0. || bounds.height() <= 0.
                    {
                        return Err("Use positive selection dimensions".into());
                    }
                    if ((width / bounds.width()) - (height / bounds.height())).abs() > 0.002 {
                        return Err("Selection scaling preserves proportions; use frame geometry for independent width/height".into());
                    }
                    let degrees = v
                        .get("rotation")
                        .map(|_| number(v, "rotation"))
                        .transpose()?
                        .unwrap_or(0.);
                    for e in &mut originals {
                        crate::production::apply_style(e, &p.production, true)?;
                        e.style_id = None;
                    }
                    document_ops::transform_selection(
                        &mut p.pages[i],
                        &originals,
                        bounds,
                        egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(width, height)),
                        degrees,
                    );
                }
                "selection_group" => {
                    if ids.len() < 2 {
                        return Err("Select at least two layers".into());
                    }
                    let group = p.alloc_id();
                    for e in &mut p.pages[i].elements {
                        if ids.contains(&e.id) {
                            e.group_id = Some(group);
                        }
                    }
                }
                "selection_ungroup" => {
                    for e in &mut p.pages[i].elements {
                        if ids.contains(&e.id) {
                            e.group_id = None;
                        }
                    }
                }
                "selection_duplicate" => {
                    let copies = document_ops::fresh_copies(p, &originals, 24., 24.);
                    p.pages[i].elements.extend(copies);
                }
                "selection_delete" => {
                    p.pages[i].elements.retain(|e| !ids.contains(&e.id));
                    for id in &ids {
                        p.production.studio.colors.remove(id);
                    }
                }
                "selection_align" | "selection_distribute" => {
                    // Treat an existing group as one layout unit, including rotated bounds.
                    let mut groups = BTreeMap::<(bool, u64), Vec<CanvasElement>>::new();
                    for e in &originals {
                        groups
                            .entry((e.group_id.is_some(), e.group_id.unwrap_or(e.id)))
                            .or_default()
                            .push(e.clone());
                    }
                    let mut units = groups
                        .into_values()
                        .map(|e| {
                            let b = document_ops::bounds(&e).unwrap();
                            (e, b)
                        })
                        .collect::<Vec<_>>();
                    let direction = text(v, "direction")?;
                    let target = if units.len() == 1 {
                        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(p.width, p.height))
                    } else {
                        bounds
                    };
                    let mut deltas = Vec::new();
                    if action == "selection_align" {
                        for (elements, b) in units {
                            let (dx, dy) = match direction {
                                "left" => (target.left() - b.left(), 0.),
                                "center" => (target.center().x - b.center().x, 0.),
                                "right" => (target.right() - b.right(), 0.),
                                "top" => (0., target.top() - b.top()),
                                "middle" => (0., target.center().y - b.center().y),
                                "bottom" => (0., target.bottom() - b.bottom()),
                                _ => {
                                    return Err(
                                        "Choose left, center, right, top, middle or bottom".into()
                                    )
                                }
                            };
                            for e in elements {
                                deltas.push((e.id, dx, dy));
                            }
                        }
                    } else {
                        if units.len() < 3 {
                            return Err(
                                "Select at least three independent layers or groups to distribute"
                                    .into(),
                            );
                        }
                        let horizontal = match direction {
                            "horizontal" => true,
                            "vertical" => false,
                            _ => return Err("Use horizontal or vertical distribution".into()),
                        };
                        units.sort_by(|a, b| {
                            let a = if horizontal { a.1.left() } else { a.1.top() };
                            let b = if horizontal { b.1.left() } else { b.1.top() };
                            a.total_cmp(&b)
                        });
                        let total: f32 = units
                            .iter()
                            .map(|(_, b)| if horizontal { b.width() } else { b.height() })
                            .sum();
                        let gap = ((if horizontal {
                            bounds.width()
                        } else {
                            bounds.height()
                        }) - total)
                            / (units.len() - 1) as f32;
                        let mut cursor = if horizontal {
                            bounds.left()
                        } else {
                            bounds.top()
                        };
                        for (elements, b) in units {
                            let d = cursor - if horizontal { b.left() } else { b.top() };
                            for e in elements {
                                deltas.push((
                                    e.id,
                                    if horizontal { d } else { 0. },
                                    if horizontal { 0. } else { d },
                                ));
                            }
                            cursor += (if horizontal { b.width() } else { b.height() }) + gap;
                        }
                    }
                    for (id, dx, dy) in deltas {
                        if let Some(e) = p.pages[i].elements.iter_mut().find(|e| e.id == id) {
                            e.x += dx;
                            e.y += dy;
                        }
                    }
                }
                _ => return Err("Unsupported selection command".into()),
            }
        }
    }
    campaign::validate(p)?;
    Ok(())
}
