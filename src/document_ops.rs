//! Document operations shared by keyboard, canvas and library actions.
use std::collections::HashMap;

use crate::model::render_scene::LocalToPageTransform;
use crate::model::{CanvasElement, CanvasPage, ElementKind, Project};

pub fn selection_ids(page: &CanvasPage, ids: &[u64], primary: Option<u64>) -> Vec<u64> {
    let mut ids = ids.to_vec();
    if ids.is_empty() {
        ids.extend(primary);
    }
    let groups: Vec<_> = page
        .elements
        .iter()
        .filter(|e| ids.contains(&e.id))
        .filter_map(|e| e.group_id)
        .collect();
    page.elements
        .iter()
        .filter(|e| ids.contains(&e.id) || e.group_id.is_some_and(|g| groups.contains(&g)))
        .map(|e| e.id)
        .collect()
}

pub fn selected_elements(
    page: &CanvasPage,
    ids: &[u64],
    primary: Option<u64>,
) -> Vec<CanvasElement> {
    let ids = selection_ids(page, ids, primary);
    page.elements
        .iter()
        .filter(|e| ids.contains(&e.id))
        .cloned()
        .collect()
}

pub fn selection_bounds(page: &CanvasPage, ids: &[u64]) -> Option<egui::Rect> {
    page.elements
        .iter()
        .filter(|e| ids.contains(&e.id))
        .map(element_bounds)
        .reduce(|a, b| a.union(b))
}

fn element_bounds(e: &CanvasElement) -> egui::Rect {
    let (x0, y0, x1, y1) = LocalToPageTransform::from_element(e).page_aabb(0.);
    egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1))
}

pub fn fresh_copies(
    project: &mut Project,
    elements: &[CanvasElement],
    dx: f32,
    dy: f32,
) -> Vec<CanvasElement> {
    let mut groups = HashMap::new();
    elements
        .iter()
        .map(|original| {
            let mut copy = original.clone();
            copy.id = project.alloc_id();
            if let Some(colors) = project.production.studio.colors.get(&original.id).cloned() {
                project.production.studio.colors.insert(copy.id, colors);
            }
            copy.group_id = original
                .group_id
                .map(|g| *groups.entry(g).or_insert_with(|| project.alloc_id()));
            copy.x += dx;
            copy.y += dy;
            copy
        })
        .collect()
}

pub fn bounds(elements: &[CanvasElement]) -> Option<egui::Rect> {
    elements
        .iter()
        .map(|e| {
            let (x0, y0, x1, y1) = LocalToPageTransform::from_element(e).page_aabb(0.);
            egui::Rect::from_min_max(egui::pos2(x0, y0), egui::pos2(x1, y1))
        })
        .reduce(|a, b| a.union(b))
}

/// Uniform scaling and rotation preserve each member's geometry without shear.
pub fn transform_selection(
    page: &mut CanvasPage,
    originals: &[CanvasElement],
    from: egui::Rect,
    to: egui::Rect,
    degrees: f32,
) {
    if originals.iter().any(|e| e.locked) {
        return;
    }
    let scale = (to.width() / from.width().max(0.001)).max(0.001);
    let (sin, cos) = degrees.to_radians().sin_cos();
    for original in originals {
        if let Some(element) = page.elements.iter_mut().find(|e| e.id == original.id) {
            let center = egui::pos2(original.center().0, original.center().1);
            let delta = (center - from.center()) * scale;
            let center = to.center()
                + egui::vec2(delta.x * cos - delta.y * sin, delta.x * sin + delta.y * cos);
            *element = original.clone();
            scale_element(element, scale);
            element.x = center.x - element.width / 2.;
            element.y = center.y - element.height / 2.;
            element.rotation = (original.rotation + degrees).rem_euclid(360.);
        }
    }
}

pub fn scale_element(element: &mut CanvasElement, scale: f32) {
    for value in [
        &mut element.text_format.space_before,
        &mut element.text_format.space_after,
        &mut element.text_format.first_line_indent,
        &mut element.text_format.inset_left,
        &mut element.text_format.inset_right,
        &mut element.text_format.inset_top,
        &mut element.text_format.inset_bottom,
    ] {
        *value *= scale;
    }
    element.width *= scale;
    element.height *= scale;
    element.shadow.offset_x *= scale;
    element.shadow.offset_y *= scale;
    element.shadow.blur *= scale;
    match &mut element.kind {
        ElementKind::Text {
            font_size,
            letter_spacing,
            ..
        } => {
            *font_size *= scale;
            *letter_spacing *= scale;
        }
        ElementKind::Shape {
            stroke_width,
            corner_radius,
            ..
        } => {
            *stroke_width *= scale;
            *corner_radius *= scale;
        }
        ElementKind::Image { border_width, .. } => *border_width *= scale,
        ElementKind::FreehandPath { stroke_width, .. } => *stroke_width *= scale,
        ElementKind::Table { font_size, .. } => *font_size *= scale,
        _ => {}
    }
}

/// Apply inspector values in the selected display units without partial mutation.
pub fn set_geometry(
    element: &mut CanvasElement,
    mut values: [f32; 4],
    changed: [bool; 4],
    factor: f32,
    aspect_lock: bool,
) -> Result<(), String> {
    if !factor.is_finite()
        || factor <= 0.
        || values.iter().any(|v| !v.is_finite())
        || values[2] <= 0.
        || values[3] <= 0.
    {
        return Err("Geometry must contain finite values and positive dimensions".into());
    }
    if element.locked {
        return Err("Unlock the element before changing geometry".into());
    }
    if aspect_lock && changed[2] {
        values[3] = element.height * values[2] / element.width.max(0.001);
    } else if aspect_lock && changed[3] {
        values[2] = element.width * values[3] / element.height.max(0.001);
    }
    let converted = values.map(|v| v / factor);
    if converted.iter().any(|v| !v.is_finite()) {
        return Err("Geometry is outside the supported range".into());
    }
    element.x = converted[0];
    element.y = converted[1];
    element.width = converted[2];
    element.height = converted[3];
    Ok(())
}

/// Insert every source page, preserving its proportions inside the destination.
pub fn fitted_pages(destination: &mut Project, source: &Project) -> Vec<CanvasPage> {
    for (key, field) in &source.production.fields {
        destination
            .production
            .fields
            .entry(key.clone())
            .or_insert_with(|| field.clone());
    }
    for (key, style) in &source.production.styles {
        destination
            .production
            .styles
            .entry(key.clone())
            .or_insert_with(|| style.clone());
    }
    for font in &source.imported_fonts {
        if !destination.imported_fonts.iter().any(|f| f.id == font.id) {
            destination.imported_fonts.push(font.clone());
        }
    }
    let mut token_keys = std::collections::HashMap::new();
    for (key, color) in &source.production.studio.tokens {
        let mut target = key.clone();
        let mut suffix = 2;
        while destination
            .production
            .studio
            .tokens
            .get(&target)
            .is_some_and(|c| c != color)
        {
            target = format!("{key}_{suffix}");
            suffix += 1;
        }
        destination
            .production
            .studio
            .tokens
            .insert(target.clone(), *color);
        token_keys.insert(key.clone(), target);
    }
    let scale =
        (destination.width / source.width.max(1.)).min(destination.height / source.height.max(1.));
    let dx = (destination.width - source.width * scale) / 2.;
    let dy = (destination.height - source.height * scale) / 2.;
    let copies: Vec<_> = source
        .pages
        .iter()
        .map(|page| {
            let mut copy = page.clone();
            copy.id = destination.alloc_id();
            copy.elements = fresh_copies(destination, &page.elements, 0., 0.);
            if let Some(key) = source
                .production
                .studio
                .backgrounds
                .get(&page.id)
                .and_then(|k| token_keys.get(k))
            {
                destination
                    .production
                    .studio
                    .backgrounds
                    .insert(copy.id, key.clone());
            }
            for (original, element) in page.elements.iter().zip(&mut copy.elements) {
                destination.production.studio.colors.remove(&element.id);
                if let Some(bindings) = source.production.studio.colors.get(&original.id) {
                    destination.production.studio.colors.insert(
                        element.id,
                        bindings
                            .iter()
                            .filter_map(|(target, key)| {
                                token_keys.get(key).map(|k| (target.clone(), k.clone()))
                            })
                            .collect(),
                    );
                }
                element.x = element.x * scale + dx;
                element.y = element.y * scale + dy;
                scale_element(element, scale);
            }
            copy
        })
        .collect();
    // Preserve puzzle recipes when template pages are inserted with fresh IDs.
    let page_ids: std::collections::HashMap<_, _> = source
        .pages
        .iter()
        .zip(&copies)
        .map(|(a, b)| (a.id, b.id))
        .collect();
    let layer_ids: std::collections::HashMap<_, _> = source
        .pages
        .iter()
        .zip(&copies)
        .flat_map(|(a, b)| {
            a.elements
                .iter()
                .zip(&b.elements)
                .map(|(a, b)| (a.id, b.id))
        })
        .collect();
    let mut proof = destination.clone();
    proof.pages.extend(copies.clone());
    let recipes_valid = crate::puzzle_production::validate_project(source).is_ok();
    for recipe in &source.production.puzzles {
        if let Some(&page) = page_ids.get(&recipe.puzzle_page) {
            let mut recipe = recipe.clone();
            recipe.puzzle_page = page;
            recipe.answer_page = recipe.answer_page.and_then(|id| page_ids.get(&id).copied());
            recipe.generated_ids = recipe
                .generated_ids
                .iter()
                .filter_map(|id| layer_ids.get(id).copied())
                .collect();
            if let Ok(signature) = crate::puzzle_production::signature(&proof, &recipe) {
                recipe.signature = if recipes_valid {
                    signature
                } else {
                    "invalid source recipe; rebuild before export".into()
                };
                destination.production.puzzles.push(recipe);
            }
        }
    }
    copies
}
