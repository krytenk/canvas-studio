use std::collections::HashMap;

use egui::{
    epaint::{Mesh, PathShape, Vertex},
    Align2, Color32, ColorImage, Context, CursorIcon, FontFamily as EguiFontFamily, FontId, Id,
    Pos2, Rect, Rounding, Sense, Shape, Stroke, TextureHandle, TextureOptions, Ui, Vec2,
};

use crate::exporter::compute_shape_polygon_points;
use crate::magic_studio::apply_image_pipeline;
use crate::model::font_registry::register_egui_fonts;
use crate::model::pdf_export::sample_bilinear_rgba;
use crate::model::render_scene::{compute_image_placement, mask_coverage};
use crate::model::{
    compute_smart_snap, to_egui_color, with_opacity, BrushType, CanvasElement, ChartType,
    ElementAnimation, ElementKind, FillStyle, HistoryManager, ImageMask, PageBackground, Project,
    ShapeType, SnapGuide, TextEffect,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResizeHandle {
    TopLeft,
    Top,
    TopRight,
    Right,
    BottomRight,
    Bottom,
    BottomLeft,
    Left,
    Rotate,
}

pub struct CanvasInteractionState {
    pub zoom: f32,
    pub pan: Vec2,
    pub show_grid: bool,
    pub show_rulers: bool,
    pub snap_enabled: bool,
    pub active_handle: Option<ResizeHandle>,
    pub dragging_element: bool,
    pub drag_start_canvas_pos: Pos2,
    pub drag_orig_rect: Rect,
    pub drag_orig_rotation: f32,
    pub drag_transform_elements: Vec<CanvasElement>,
    pub drag_group_origins: Vec<(u64, f32, f32)>,
    pub selected_ids: Vec<u64>,
    pub marquee_start: Option<Pos2>,
    pub marquee_current: Option<Pos2>,
    pub active_guides: Vec<SnapGuide>,
    pub drawing_mode: bool,
    pub draw_brush: BrushType,
    pub draw_color: [u8; 4],
    pub draw_width: f32,
    pub current_stroke_pts: Vec<Pos2>,
    pub inline_edit_id: Option<u64>,
    pub text_selection: Option<(u64, usize, usize)>,
    pub texture_cache: HashMap<(u64, u64), TextureHandle>,
    pub anim_start_time: f64,
}

impl CanvasInteractionState {
    pub fn new() -> Self {
        Self {
            zoom: 0.55,
            pan: Vec2::ZERO,
            show_grid: false,
            show_rulers: true,
            snap_enabled: true,
            active_handle: None,
            dragging_element: false,
            drag_start_canvas_pos: Pos2::ZERO,
            drag_orig_rect: Rect::NOTHING,
            drag_orig_rotation: 0.0,
            drag_transform_elements: Vec::new(),
            drag_group_origins: Vec::new(),
            selected_ids: Vec::new(),
            marquee_start: None,
            marquee_current: None,
            active_guides: Vec::new(),
            drawing_mode: false,
            draw_brush: BrushType::Pen,
            draw_color: [0, 196, 204, 255],
            draw_width: 5.0,
            current_stroke_pts: Vec::new(),
            inline_edit_id: None,
            text_selection: None,
            texture_cache: HashMap::new(),
            anim_start_time: 0.0,
        }
    }

    pub fn reset_for_document(&mut self) {
        self.active_handle = None;
        self.dragging_element = false;
        self.drag_group_origins.clear();
        self.drag_transform_elements.clear();
        self.selected_ids.clear();
        self.marquee_start = None;
        self.marquee_current = None;
        self.active_guides.clear();
        self.current_stroke_pts.clear();
        self.inline_edit_id = None;
        self.text_selection = None;
        self.texture_cache.clear();
    }
}

pub enum CanvasAction {
    None,
    DuplicateSelected,
    DeleteSelected,
    BringForward,
    SendBackward,
    ToggleLock,
}

#[allow(clippy::too_many_arguments)]
pub fn render_interactive_canvas(
    ctx: &Context,
    ui: &mut Ui,
    project: &mut Project,
    current_page_idx: usize,
    selected_id: &mut Option<u64>,
    state: &mut CanvasInteractionState,
    _history: &mut HistoryManager,
    presentation_mode: bool,
) -> CanvasAction {
    register_egui_fonts(ctx);
    if state.texture_cache.len() > 256 {
        state.texture_cache.clear();
    }
    let mut triggered_action = CanvasAction::None;
    let avail = ui.available_rect_before_wrap();

    // Workspace surround backdrop
    let workspace_bg = if presentation_mode {
        Color32::from_rgb(8, 10, 15)
    } else {
        Color32::from_rgb(24, 25, 28)
    };
    ui.painter().rect_filled(avail, 0.0, workspace_bg);

    let (response, painter) = ui.allocate_painter(avail.size(), Sense::click_and_drag());
    let viewport_rect = response.rect;

    // Handle Pan (Middle mouse or Space + drag) & Scroll Wheel Zoom
    if !presentation_mode && response.hovered() {
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta);
        let zoom_delta = ui.input(|i| i.zoom_delta());
        if (zoom_delta - 1.0).abs() > 0.001 {
            state.zoom = (state.zoom * zoom_delta).clamp(0.12, 3.5);
        } else if scroll_delta.y.abs() > 0.1
            && ui.input(|i| i.modifiers.ctrl || i.modifiers.command)
        {
            let factor = 1.0 + scroll_delta.y * 0.0015;
            state.zoom = (state.zoom * factor).clamp(0.12, 3.5);
        } else if scroll_delta.length_sq() > 0.1 {
            state.pan += scroll_delta * 0.6;
        }

        if response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged() && ui.input(|i| i.key_down(egui::Key::Space)))
        {
            state.pan += response.drag_delta();
        }
    }

    // Calculate screen rect of the page canvas
    let effective_zoom = if presentation_mode {
        let zx = (viewport_rect.width() - 48.0) / project.width.max(1.0);
        let zy = (viewport_rect.height() - 80.0) / project.height.max(1.0);
        zx.min(zy).max(0.1)
    } else {
        state.zoom
    };

    let page_screen_size = Vec2::new(
        project.width * effective_zoom,
        project.height * effective_zoom,
    );
    let page_center = viewport_rect.center()
        + if presentation_mode {
            Vec2::new(0.0, -16.0)
        } else {
            state.pan
        };
    let page_rect = Rect::from_center_size(page_center, page_screen_size);

    let to_screen = |cx: f32, cy: f32| -> Pos2 {
        Pos2::new(
            page_rect.min.x + cx * effective_zoom,
            page_rect.min.y + cy * effective_zoom,
        )
    };
    let to_canvas = |sp: Pos2| -> Pos2 {
        Pos2::new(
            (sp.x - page_rect.min.x) / effective_zoom,
            (sp.y - page_rect.min.y) / effective_zoom,
        )
    };

    // Page drop shadow
    painter.rect_filled(
        page_rect.translate(Vec2::new(0.0, 10.0)).expand(4.0),
        4.0,
        Color32::from_black_alpha(110),
    );

    let preview = crate::production::resolve_page(project, current_page_idx, false);
    let Ok(page) = preview else {
        return CanvasAction::None;
    };

    // 1. Paint Page Background
    let page_painter = painter.with_clip_rect(page_rect.intersect(viewport_rect));
    paint_page_background(&page_painter, page_rect, &page.background, effective_zoom);

    // Optional design grid overlay
    if state.show_grid && !presentation_mode {
        let step = (50.0 * effective_zoom).max(10.0);
        let grid_stroke = Stroke::new(1.0_f32, Color32::from_rgba_unmultiplied(148, 163, 184, 42));
        let mut x = page_rect.min.x;
        while x <= page_rect.max.x {
            page_painter.line_segment(
                [Pos2::new(x, page_rect.min.y), Pos2::new(x, page_rect.max.y)],
                grid_stroke,
            );
            x += step;
        }
        let mut y = page_rect.min.y;
        while y <= page_rect.max.y {
            page_painter.line_segment(
                [Pos2::new(page_rect.min.x, y), Pos2::new(page_rect.max.x, y)],
                grid_stroke,
            );
            y += step;
        }
    }

    // Animation progress in presentation mode
    let now = ctx.input(|i| i.time);
    let elapsed = (now - state.anim_start_time).max(0.0) as f32;
    if presentation_mode && elapsed < 1.6 {
        ctx.request_repaint();
    }

    // 2. Paint all elements on the current page
    for (idx, el) in page.elements.iter().enumerate() {
        if !el.visible {
            continue;
        }
        let (anim_offset, anim_scale, anim_alpha) = if presentation_mode {
            compute_animation_transform(el.animation, elapsed, idx)
        } else {
            (Vec2::ZERO, 1.0, 1.0)
        };

        paint_canvas_element(
            ctx,
            &page_painter,
            el,
            &to_screen,
            effective_zoom,
            anim_offset,
            anim_scale,
            anim_alpha,
            &mut state.texture_cache,
        );
    }

    // Top & Left Rulers
    if project.print_settings.show_guides && !presentation_mode {
        let settings = &project.print_settings;
        let mm_x = project.width / project.effective_width_mm();
        let mm_y = project.height / project.effective_height_mm();
        let margin = Vec2::new(
            settings.safe_margin_mm * mm_x * effective_zoom,
            settings.safe_margin_mm * mm_y * effective_zoom,
        );
        if margin.x * 2. < page_rect.width() && margin.y * 2. < page_rect.height() {
            painter.rect_stroke(
                Rect::from_min_max(page_rect.min + margin, page_rect.max - margin),
                0.,
                Stroke::new(1.0_f32, Color32::from_rgb(40, 170, 220)),
            );
        }
        let bleed = Vec2::new(
            project.bleed_mm * mm_x * effective_zoom,
            project.bleed_mm * mm_y * effective_zoom,
        );
        if project.bleed_mm > 0. {
            painter.rect_stroke(
                Rect::from_min_max(page_rect.min - bleed, page_rect.max + bleed),
                0.,
                Stroke::new(1.0_f32, Color32::LIGHT_RED),
            );
        }
        if project.production.book.enabled {
            if let Ok(content) = crate::book_production::content_rect(project, current_page_idx) {
                painter.rect_stroke(
                    Rect::from_min_max(
                        to_screen(content.min.x, content.min.y),
                        to_screen(content.max.x, content.max.y),
                    ),
                    0.,
                    Stroke::new(1.0_f32, Color32::from_rgb(170, 110, 220)),
                );
            }
        }
        let stroke = Stroke::new(1.0_f32, Color32::from_rgb(230, 180, 60));
        for value in &settings.guides_x_mm {
            let x = to_screen(value * mm_x, 0.).x;
            painter.line_segment(
                [Pos2::new(x, page_rect.min.y), Pos2::new(x, page_rect.max.y)],
                stroke,
            );
        }
        for value in &settings.guides_y_mm {
            let y = to_screen(0., value * mm_y).y;
            painter.line_segment(
                [Pos2::new(page_rect.min.x, y), Pos2::new(page_rect.max.x, y)],
                stroke,
            );
        }
    }
    if state.show_rulers && !presentation_mode {
        paint_rulers(&painter, viewport_rect, page_rect, effective_zoom);
    }

    if presentation_mode {
        return CanvasAction::None;
    }

    // 3. Freehand Draw Mode handling
    if state.drawing_mode {
        if response.hovered() {
            ctx.set_cursor_icon(CursorIcon::Crosshair);
        }
        if response.drag_started() {
            state.current_stroke_pts.clear();
            if let Some(pos) = response.interact_pointer_pos() {
                state.current_stroke_pts.push(to_canvas(pos));
            }
        } else if response.dragged() {
            if let Some(pos) = response.interact_pointer_pos() {
                let cpos = to_canvas(pos);
                if state
                    .current_stroke_pts
                    .last()
                    .map_or(true, |last| last.distance(cpos) > 2.0)
                {
                    state.current_stroke_pts.push(cpos);
                }
            }
        } else if response.drag_stopped() && state.current_stroke_pts.len() >= 2 {
            let mut min_x = f32::MAX;
            let mut min_y = f32::MAX;
            let mut max_x = f32::MIN;
            let mut max_y = f32::MIN;
            for pt in &state.current_stroke_pts {
                min_x = min_x.min(pt.x);
                min_y = min_y.min(pt.y);
                max_x = max_x.max(pt.x);
                max_y = max_y.max(pt.y);
            }
            let w = (max_x - min_x).max(12.0);
            let h = (max_y - min_y).max(12.0);
            let norm_pts: Vec<(f32, f32)> = state
                .current_stroke_pts
                .iter()
                .map(|p| {
                    (
                        ((p.x - min_x) / w).clamp(0.0, 1.0),
                        ((p.y - min_y) / h).clamp(0.0, 1.0),
                    )
                })
                .collect();
            let new_id = project.alloc_id();
            if let Some(p) = project.pages.get_mut(current_page_idx) {
                p.elements.push(CanvasElement {
                    id: new_id,
                    name: format!("{} Stroke", state.draw_brush.label()),
                    x: min_x,
                    y: min_y,
                    width: w,
                    height: h,
                    rotation: 0.0,
                    opacity: if state.draw_brush == BrushType::Highlighter {
                        0.45
                    } else {
                        1.0
                    },
                    locked: false,
                    visible: true,
                    flip_h: false,
                    flip_v: false,
                    shadow: Default::default(),
                    animation: ElementAnimation::None,
                    group_id: None,
                    text_format: Default::default(),
                    style_id: None,
                    binding: None,
                    kind: ElementKind::FreehandPath {
                        points: norm_pts,
                        color: state.draw_color,
                        stroke_width: state.draw_width,
                        brush: state.draw_brush,
                    },
                });
                *selected_id = Some(new_id);
            }
            state.current_stroke_pts.clear();
        }

        // Paint live stroke preview
        if state.current_stroke_pts.len() >= 2 {
            let screen_pts: Vec<Pos2> = state
                .current_stroke_pts
                .iter()
                .map(|p| to_screen(p.x, p.y))
                .collect();
            page_painter.add(PathShape::line(
                screen_pts,
                Stroke::new(
                    state.draw_width * effective_zoom,
                    to_egui_color(state.draw_color),
                ),
            ));
        }

        return CanvasAction::None;
    }

    // 4. Direct-Manipulation Selection, Resize Handles, Rotation & Smart Snap Guides
    let pointer_pos = response.interact_pointer_pos();

    // Check if user started dragging a resize/rotate handle on the currently selected element
    if response.drag_started_by(egui::PointerButton::Primary) {
        if let Some(spos) = pointer_pos {
            let cpos = to_canvas(spos);
            state.active_handle = None;
            state.dragging_element = false;

            if let Some(sel_id) = *selected_id {
                if let Some(sel_el) = project.pages[current_page_idx]
                    .elements
                    .iter()
                    .find(|e| e.id == sel_id)
                {
                    let members = crate::document_ops::selected_elements(
                        &project.pages[current_page_idx],
                        &state.selected_ids,
                        Some(sel_id),
                    );
                    if !members.iter().any(|e| e.locked) {
                        let bounds = if members.len() > 1 {
                            crate::document_ops::bounds(&members).unwrap()
                        } else {
                            sel_el.rect()
                        };
                        let screen_rect = Rect::from_min_max(
                            to_screen(bounds.min.x, bounds.min.y),
                            to_screen(bounds.max.x, bounds.max.y),
                        );
                        if let Some(handle) = hit_test_handles(screen_rect, spos) {
                            state.active_handle = Some(handle);
                            state.drag_start_canvas_pos = cpos;
                            state.drag_orig_rect = bounds;
                            state.drag_orig_rotation = sel_el.rotation;
                            state.drag_transform_elements = members.clone();
                        }
                    }
                }
            }

            // If no handle was hit, hit-test elements top-to-bottom
            if state.active_handle.is_none() {
                let hit_id = project.pages[current_page_idx]
                    .elements
                    .iter()
                    .rev()
                    .find(|e| e.contains_point(cpos.x, cpos.y))
                    .map(|e| e.id);

                let shift_held = ui.input(|i| i.modifiers.shift);
                if let Some(hid) = hit_id {
                    if shift_held {
                        if !state.selected_ids.contains(&hid) {
                            state.selected_ids.push(hid);
                        }
                    } else if !state.selected_ids.contains(&hid) {
                        state.selected_ids = vec![hid];
                    }
                    // Also include all members sharing the same group_id
                    if let Some(gid) = project.pages[current_page_idx]
                        .elements
                        .iter()
                        .find(|e| e.id == hid)
                        .and_then(|e| e.group_id)
                    {
                        for member in &project.pages[current_page_idx].elements {
                            if member.group_id == Some(gid)
                                && !state.selected_ids.contains(&member.id)
                            {
                                state.selected_ids.push(member.id);
                            }
                        }
                    }
                } else if !shift_held {
                    state.selected_ids.clear();
                }

                *selected_id = hit_id;
                state.inline_edit_id = None;

                if let Some(hid) = hit_id {
                    state.marquee_start = None;
                    state.marquee_current = None;
                    if let Some(el) = project.pages[current_page_idx]
                        .elements
                        .iter()
                        .find(|e| e.id == hid)
                    {
                        if !el.locked
                            && !project.pages[current_page_idx]
                                .elements
                                .iter()
                                .any(|m| state.selected_ids.contains(&m.id) && m.locked)
                        {
                            state.dragging_element = true;
                            state.drag_start_canvas_pos = cpos;
                            state.drag_orig_rect = el.rect();
                            state.drag_group_origins = project.pages[current_page_idx]
                                .elements
                                .iter()
                                .filter(|m| state.selected_ids.contains(&m.id) && !m.locked)
                                .map(|m| (m.id, m.x, m.y))
                                .collect();
                        }
                    }
                } else {
                    state.marquee_start = Some(cpos);
                    state.marquee_current = Some(cpos);
                }
            }
        }
    } else if response.clicked_by(egui::PointerButton::Primary) {
        if let Some(spos) = pointer_pos {
            let cpos = to_canvas(spos);
            let hit_id = project.pages[current_page_idx]
                .elements
                .iter()
                .rev()
                .find(|e| e.contains_point(cpos.x, cpos.y))
                .map(|e| e.id);
            let shift_held = ui.input(|i| i.modifiers.shift);
            if let Some(hid) = hit_id {
                if shift_held {
                    if let Some(pos) = state.selected_ids.iter().position(|&x| x == hid) {
                        state.selected_ids.remove(pos);
                    } else {
                        state.selected_ids.push(hid);
                    }
                } else {
                    state.selected_ids = vec![hid];
                }
                if let Some(gid) = project.pages[current_page_idx]
                    .elements
                    .iter()
                    .find(|e| e.id == hid)
                    .and_then(|e| e.group_id)
                {
                    for member in &project.pages[current_page_idx].elements {
                        if member.group_id == Some(gid) && !state.selected_ids.contains(&member.id)
                        {
                            state.selected_ids.push(member.id);
                        }
                    }
                }
            } else if !shift_held {
                state.selected_ids.clear();
            }
            *selected_id = hit_id.or_else(|| state.selected_ids.last().copied());
            if response.double_clicked() {
                state.inline_edit_id = hit_id;
            } else {
                state.inline_edit_id = None;
            }
        }
    }

    // Apply active drag, handle resize, or marquee rubber-band selection
    if response.dragged_by(egui::PointerButton::Primary) {
        if let (Some(spos), Some(m_start)) = (pointer_pos, state.marquee_start) {
            let cpos = to_canvas(spos);
            state.marquee_current = Some(cpos);
            let marquee_rect = Rect::from_two_pos(m_start, cpos);
            let mut hits = Vec::new();
            for el in &project.pages[current_page_idx].elements {
                if el.visible && el.rect().intersects(marquee_rect) {
                    hits.push(el.id);
                }
            }
            state.selected_ids = hits;
            *selected_id = state.selected_ids.last().copied();
        } else if let (Some(spos), Some(sel_id)) = (pointer_pos, *selected_id) {
            let cpos = to_canvas(spos);
            let delta = cpos - state.drag_start_canvas_pos;
            let orig = state.drag_orig_rect;

            if let Some(handle) = state.active_handle {
                if state.drag_transform_elements.len() > 1 {
                    let (target, degrees) = if handle == ResizeHandle::Rotate {
                        let center = orig.center();
                        let mut angle =
                            (cpos.y - center.y).atan2(cpos.x - center.x).to_degrees() + 90.;
                        if ui.input(|i| i.modifiers.shift) {
                            angle = (angle / 15.).round() * 15.;
                        }
                        (orig, angle)
                    } else {
                        (group_resize_rect(orig, handle, delta), 0.)
                    };
                    crate::document_ops::transform_selection(
                        &mut project.pages[current_page_idx],
                        &state.drag_transform_elements,
                        orig,
                        target,
                        degrees,
                    );
                } else if let Some(el) = project.pages[current_page_idx]
                    .elements
                    .iter_mut()
                    .find(|e| e.id == sel_id)
                {
                    match handle {
                        ResizeHandle::Rotate => {
                            let (cx, cy) = el.center();
                            let angle = (cpos.y - cy).atan2(cpos.x - cx).to_degrees() + 90.0;
                            let snapped_angle = if ui.input(|i| i.modifiers.shift) {
                                (angle / 15.0).round() * 15.0
                            } else {
                                angle
                            };
                            el.rotation = snapped_angle.rem_euclid(360.0);
                        }
                        ResizeHandle::BottomRight => {
                            el.width = (orig.width() + delta.x).max(16.0);
                            el.height = (orig.height() + delta.y).max(16.0);
                        }
                        ResizeHandle::Right => {
                            el.width = (orig.width() + delta.x).max(16.0);
                        }
                        ResizeHandle::Bottom => {
                            el.height = (orig.height() + delta.y).max(16.0);
                        }
                        ResizeHandle::TopLeft => {
                            let nw = (orig.width() - delta.x).max(16.0);
                            let nh = (orig.height() - delta.y).max(16.0);
                            el.x = orig.max.x - nw;
                            el.y = orig.max.y - nh;
                            el.width = nw;
                            el.height = nh;
                        }
                        ResizeHandle::TopRight => {
                            let nw = (orig.width() + delta.x).max(16.0);
                            let nh = (orig.height() - delta.y).max(16.0);
                            el.y = orig.max.y - nh;
                            el.width = nw;
                            el.height = nh;
                        }
                        ResizeHandle::BottomLeft => {
                            let nw = (orig.width() - delta.x).max(16.0);
                            let nh = (orig.height() + delta.y).max(16.0);
                            el.x = orig.max.x - nw;
                            el.width = nw;
                            el.height = nh;
                        }
                        ResizeHandle::Top => {
                            let nh = (orig.height() - delta.y).max(16.0);
                            el.y = orig.max.y - nh;
                            el.height = nh;
                        }
                        ResizeHandle::Left => {
                            let nw = (orig.width() - delta.x).max(16.0);
                            el.x = orig.max.x - nw;
                            el.width = nw;
                        }
                    }
                }
            } else if state.dragging_element {
                let proposed_x = orig.min.x + delta.x;
                let proposed_y = orig.min.y + delta.y;
                let w = orig.width();
                let h = orig.height();

                let (mut final_x, mut final_y, mut guides) = if state.snap_enabled {
                    compute_smart_snap(
                        sel_id,
                        proposed_x,
                        proposed_y,
                        w,
                        h,
                        project.width,
                        project.height,
                        &project.pages[current_page_idx].elements,
                        10.0 / effective_zoom.max(0.25),
                    )
                } else {
                    (proposed_x, proposed_y, Vec::new())
                };

                if state.snap_enabled && project.print_settings.show_guides {
                    let tolerance = 10. / effective_zoom.max(0.25);
                    for guide in &project.print_settings.guides_x_mm {
                        let position = guide * project.width / project.effective_width_mm();
                        if let Some(anchor) = [0., w / 2., w]
                            .into_iter()
                            .find(|anchor| (proposed_x + anchor - position).abs() < tolerance)
                        {
                            final_x = position - anchor;
                            guides.push(SnapGuide {
                                vertical: true,
                                position,
                                label: "Custom guide",
                            });
                            break;
                        }
                    }
                    for guide in &project.print_settings.guides_y_mm {
                        let position = guide * project.height / project.effective_height_mm();
                        if let Some(anchor) = [0., h / 2., h]
                            .into_iter()
                            .find(|anchor| (proposed_y + anchor - position).abs() < tolerance)
                        {
                            final_y = position - anchor;
                            guides.push(SnapGuide {
                                vertical: false,
                                position,
                                label: "Custom guide",
                            });
                            break;
                        }
                    }
                }

                let applied_dx = final_x - orig.min.x;
                let applied_dy = final_y - orig.min.y;
                state.active_guides = guides;

                if state.drag_group_origins.len() > 1 {
                    for &(mid, ox, oy) in &state.drag_group_origins {
                        if let Some(member) = project.pages[current_page_idx]
                            .elements
                            .iter_mut()
                            .find(|e| e.id == mid)
                        {
                            member.x = ox + applied_dx;
                            member.y = oy + applied_dy;
                        }
                    }
                } else if let Some(el) = project.pages[current_page_idx]
                    .elements
                    .iter_mut()
                    .find(|e| e.id == sel_id)
                {
                    el.x = final_x;
                    el.y = final_y;
                }
            }
        }
    } else if response.drag_stopped() {
        state.active_handle = None;
        state.dragging_element = false;
        state.drag_group_origins.clear();
        state.active_guides.clear();
        state.marquee_start = None;
        state.marquee_current = None;
    }

    // Paint rubber-band marquee selection box if active
    if let (Some(m_start), Some(m_cur)) = (state.marquee_start, state.marquee_current) {
        let s_rect =
            Rect::from_two_pos(to_screen(m_start.x, m_start.y), to_screen(m_cur.x, m_cur.y));
        painter.rect_filled(
            s_rect,
            2.0,
            Color32::from_rgba_unmultiplied(56, 189, 248, 36),
        );
        painter.rect_stroke(
            s_rect,
            2.0,
            Stroke::new(1.5_f32, Color32::from_rgb(56, 189, 248)),
        );
    }

    // 5. Paint Smart Alignment Snap Guides in Studio Magenta
    let guide_stroke = Stroke::new(1.5_f32, Color32::from_rgb(236, 72, 153));
    for g in &state.active_guides {
        if g.vertical {
            let sx = to_screen(g.position, 0.0).x;
            painter.line_segment(
                [
                    Pos2::new(sx, page_rect.min.y),
                    Pos2::new(sx, page_rect.max.y),
                ],
                guide_stroke,
            );
            painter.text(
                Pos2::new(sx + 6.0, page_rect.min.y + 14.0),
                Align2::LEFT_TOP,
                g.label,
                FontId::proportional(11.0),
                Color32::from_rgb(244, 114, 182),
            );
        } else {
            let sy = to_screen(0.0, g.position).y;
            painter.line_segment(
                [
                    Pos2::new(page_rect.min.x, sy),
                    Pos2::new(page_rect.max.x, sy),
                ],
                guide_stroke,
            );
            painter.text(
                Pos2::new(page_rect.min.x + 8.0, sy - 14.0),
                Align2::LEFT_TOP,
                g.label,
                FontId::proportional(11.0),
                Color32::from_rgb(244, 114, 182),
            );
        }
    }

    // Paint secondary multi-selection / group outlines
    for &mid in &state.selected_ids {
        if Some(mid) != *selected_id {
            if let Some(mel) = project.pages[current_page_idx]
                .elements
                .iter()
                .find(|e| e.id == mid)
            {
                let mrect = Rect::from_min_max(
                    to_screen(mel.x, mel.y),
                    to_screen(mel.x + mel.width, mel.y + mel.height),
                );
                painter.rect_stroke(
                    mrect,
                    2.0,
                    Stroke::new(1.5_f32, Color32::from_rgb(56, 189, 248)),
                );
            }
        }
    }

    // 6. Paint Selection Bounding Box, 8 Handles, Rotation Handle & Floating Action Pill
    if let Some(sel_id) = *selected_id {
        if let Some(sel_el) = project.pages[current_page_idx]
            .elements
            .iter()
            .find(|e| e.id == sel_id)
        {
            let members = crate::document_ops::selection_ids(
                &project.pages[current_page_idx],
                &state.selected_ids,
                Some(sel_id),
            );
            let locked = project.pages[current_page_idx]
                .elements
                .iter()
                .any(|e| members.contains(&e.id) && e.locked);
            let bounds = if members.len() > 1 {
                crate::document_ops::selection_bounds(&project.pages[current_page_idx], &members)
                    .unwrap()
            } else {
                sel_el.rect()
            };
            let srect = Rect::from_min_max(
                to_screen(bounds.min.x, bounds.min.y),
                to_screen(bounds.max.x, bounds.max.y),
            );

            let border_col = if locked {
                Color32::from_rgb(245, 158, 11)
            } else {
                Color32::from_rgb(139, 92, 246) // Studio Purple #8B5CF6
            };

            painter.rect_stroke(srect, 2.0, Stroke::new(2.0_f32, border_col));

            if !locked {
                for (_, hpos) in handle_positions(srect) {
                    painter.circle_filled(hpos, 5.5, Color32::WHITE);
                    painter.circle_stroke(hpos, 5.5, Stroke::new(2.0_f32, border_col));
                }
                // Connector line to rotate handle
                let rot_pos = Pos2::new(srect.center().x, srect.min.y - 24.0);
                painter.line_segment(
                    [Pos2::new(srect.center().x, srect.min.y), rot_pos],
                    Stroke::new(1.5_f32, border_col),
                );
                painter.circle_filled(rot_pos, 7.5, Color32::WHITE);
                painter.circle_stroke(rot_pos, 7.5, Stroke::new(2.0_f32, border_col));
            }

            // Dimensions badge at bottom-center of selection
            let dim_label = format!(
                "{} x {}{}",
                bounds.width().round() as i32,
                bounds.height().round() as i32,
                if sel_el.rotation.abs() > 0.5 {
                    format!(" • {:.0}°", sel_el.rotation)
                } else {
                    String::new()
                }
            );
            let badge_center = Pos2::new(srect.center().x, srect.max.y + 18.0);
            let badge_rect = Rect::from_center_size(badge_center, Vec2::new(98.0, 20.0));
            painter.rect_filled(badge_rect, 4.0, Color32::from_rgb(30, 41, 59));
            painter.text(
                badge_center,
                Align2::CENTER_CENTER,
                dim_label,
                FontId::proportional(11.0),
                Color32::WHITE,
            );

            // Floating Quick-Action Pill above the selected element
            let pill_pos = Pos2::new(
                (srect.center().x - 105.0)
                    .clamp(viewport_rect.min.x + 8.0, viewport_rect.max.x - 220.0),
                (srect.min.y - 62.0).max(viewport_rect.min.y + 8.0),
            );
            egui::Area::new(Id::new("canvas_floating_pill"))
                .fixed_pos(pill_pos)
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(24, 26, 32))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(63, 66, 78)))
                        .rounding(Rounding::same(18.0))
                        .inner_margin(egui::Margin::symmetric(10.0, 5.0))
                        .show(ui, |ui| {
                            ui.horizontal(|ui| {
                                if ui
                                    .small_button("➕ Duplicate")
                                    .on_hover_text("Duplicate Element (Ctrl+D)")
                                    .clicked()
                                {
                                    triggered_action = CanvasAction::DuplicateSelected;
                                }
                                if ui
                                    .small_button("⬆")
                                    .on_hover_text("Bring Forward")
                                    .clicked()
                                {
                                    triggered_action = CanvasAction::BringForward;
                                }
                                if ui
                                    .small_button("⬇")
                                    .on_hover_text("Send Backward")
                                    .clicked()
                                {
                                    triggered_action = CanvasAction::SendBackward;
                                }
                                let lock_lbl = if locked { "🔒" } else { "🔓" };
                                if ui
                                    .small_button(lock_lbl)
                                    .on_hover_text("Toggle Lock")
                                    .clicked()
                                {
                                    triggered_action = CanvasAction::ToggleLock;
                                }
                                if ui.small_button("🗑").on_hover_text("Delete (Del)").clicked() {
                                    triggered_action = CanvasAction::DeleteSelected;
                                }
                            });
                        });
                });
        }
    }

    // 7. Inline Text Editor Overlay when double-clicking a Text element on the canvas
    if let Some(edit_id) = state.inline_edit_id {
        if let Some(edit_el) = project.pages[current_page_idx]
            .elements
            .iter_mut()
            .find(|e| e.id == edit_id)
        {
            let srect = Rect::from_min_max(
                to_screen(edit_el.x, edit_el.y),
                to_screen(edit_el.x + edit_el.width, edit_el.y + edit_el.height),
            );
            if edit_el.locked {
                state.inline_edit_id = None;
                return triggered_action;
            }
            if let ElementKind::Text { content, .. } = &mut edit_el.kind {
                let mut close_inline = false;
                egui::Area::new(Id::new(("canvas_inline_text_edit", edit_id)))
                    .fixed_pos(srect.min)
                    .order(egui::Order::Foreground)
                    .show(ctx, |ui| {
                        egui::Frame::none()
                            .fill(Color32::from_rgba_unmultiplied(15, 23, 42, 235))
                            .stroke(Stroke::new(2.0_f32, Color32::from_rgb(0, 245, 212)))
                            .rounding(Rounding::same(6.0))
                            .inner_margin(egui::Margin::same(6.0))
                            .show(ui, |ui| {
                                ui.set_min_size(Vec2::new(
                                    srect.width().max(180.0),
                                    srect.height().max(54.0),
                                ));
                                let before = content.clone();
                                let output = egui::TextEdit::multiline(&mut *content)
                                    .desired_width(srect.width().max(180.))
                                    .desired_rows(2)
                                    .show(ui);
                                if output.response.changed() {
                                    edit_el.text_format.reconcile_edit(&before, content);
                                }
                                if let Some(range) = output.cursor_range {
                                    let selected = range.as_sorted_char_range();
                                    let start = selected.start;
                                    let end = selected.end;
                                    state.text_selection = if start < end {
                                        Some((edit_id, start, end))
                                    } else {
                                        None
                                    };
                                }
                                if ui.small_button("✔ Done Editing").clicked() {
                                    close_inline = true;
                                }
                            });
                    });
                if close_inline {
                    state.inline_edit_id = None;
                    state.text_selection = None;
                }
            } else {
                state.inline_edit_id = None;
            }
        }
    }

    triggered_action
}

fn handle_positions(rect: Rect) -> [(ResizeHandle, Pos2); 8] {
    let cx = rect.center().x;
    let cy = rect.center().y;
    [
        (ResizeHandle::TopLeft, rect.left_top()),
        (ResizeHandle::Top, Pos2::new(cx, rect.min.y)),
        (ResizeHandle::TopRight, rect.right_top()),
        (ResizeHandle::Right, Pos2::new(rect.max.x, cy)),
        (ResizeHandle::BottomRight, rect.right_bottom()),
        (ResizeHandle::Bottom, Pos2::new(cx, rect.max.y)),
        (ResizeHandle::BottomLeft, rect.left_bottom()),
        (ResizeHandle::Left, Pos2::new(rect.min.x, cy)),
    ]
}

fn hit_test_handles(rect: Rect, pointer: Pos2) -> Option<ResizeHandle> {
    let rot_pos = Pos2::new(rect.center().x, rect.min.y - 24.0);
    if pointer.distance(rot_pos) <= 10.0 {
        return Some(ResizeHandle::Rotate);
    }
    for (h, pos) in handle_positions(rect) {
        if pointer.distance(pos) <= 9.0 {
            return Some(h);
        }
    }
    None
}

fn paint_page_background(painter: &egui::Painter, page_rect: Rect, bg: &PageBackground, zoom: f32) {
    match bg {
        PageBackground::Solid(c) => {
            painter.rect_filled(page_rect, 0.0, to_egui_color(*c));
        }
        PageBackground::LinearGradient { start, end, .. } => {
            let mut mesh = Mesh::default();
            let c_start = to_egui_color(*start);
            let c_end = to_egui_color(*end);
            let c_mid = Color32::from_rgba_unmultiplied(
                ((start[0] as u16 + end[0] as u16) / 2) as u8,
                ((start[1] as u16 + end[1] as u16) / 2) as u8,
                ((start[2] as u16 + end[2] as u16) / 2) as u8,
                255,
            );
            mesh.vertices.push(Vertex {
                pos: page_rect.left_top(),
                uv: Pos2::ZERO,
                color: c_start,
            });
            mesh.vertices.push(Vertex {
                pos: page_rect.right_top(),
                uv: Pos2::ZERO,
                color: c_mid,
            });
            mesh.vertices.push(Vertex {
                pos: page_rect.right_bottom(),
                uv: Pos2::ZERO,
                color: c_end,
            });
            mesh.vertices.push(Vertex {
                pos: page_rect.left_bottom(),
                uv: Pos2::ZERO,
                color: c_mid,
            });
            mesh.add_triangle(0, 1, 2);
            mesh.add_triangle(0, 2, 3);
            painter.add(Shape::mesh(mesh));
        }
        PageBackground::RadialGradient { center, outer } => {
            painter.rect_filled(page_rect, 0.0, to_egui_color(*outer));
            let steps = 12;
            let max_r = page_rect.width().max(page_rect.height()) * 0.6;
            for i in (1..=steps).rev() {
                let t = i as f32 / steps as f32;
                let col = Color32::from_rgba_unmultiplied(
                    (center[0] as f32 * (1.0 - t) + outer[0] as f32 * t) as u8,
                    (center[1] as f32 * (1.0 - t) + outer[1] as f32 * t) as u8,
                    (center[2] as f32 * (1.0 - t) + outer[2] as f32 * t) as u8,
                    255,
                );
                painter.circle_filled(page_rect.center(), max_r * t, col);
            }
        }
        PageBackground::GridPattern { bg, line, spacing } => {
            painter.rect_filled(page_rect, 0.0, to_egui_color(*bg));
            let step = (*spacing * zoom).max(8.0);
            let stroke = Stroke::new(1.0_f32, to_egui_color(*line));
            let mut x = page_rect.min.x;
            while x <= page_rect.max.x {
                painter.line_segment(
                    [Pos2::new(x, page_rect.min.y), Pos2::new(x, page_rect.max.y)],
                    stroke,
                );
                x += step;
            }
            let mut y = page_rect.min.y;
            while y <= page_rect.max.y {
                painter.line_segment(
                    [Pos2::new(page_rect.min.x, y), Pos2::new(page_rect.max.x, y)],
                    stroke,
                );
                y += step;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_canvas_element(
    ctx: &Context,
    painter: &egui::Painter,
    el: &CanvasElement,
    to_screen: &dyn Fn(f32, f32) -> Pos2,
    zoom: f32,
    anim_offset: Vec2,
    anim_scale: f32,
    anim_alpha: f32,
    texture_cache: &mut HashMap<(u64, u64), TextureHandle>,
) {
    let base_min = to_screen(el.x, el.y) + anim_offset;
    let base_max = to_screen(el.x + el.width, el.y + el.height) + anim_offset;
    let raw_rect = Rect::from_min_max(base_min, base_max);
    let srect = Rect::from_center_size(raw_rect.center(), raw_rect.size() * anim_scale);
    let opacity = (el.opacity * anim_alpha).clamp(0.0, 1.0);

    // Optional drop shadow
    if el.shadow.enabled {
        let sh_offset = Vec2::new(el.shadow.offset_x * zoom, el.shadow.offset_y * zoom);
        let sh_col = with_opacity(el.shadow.color, opacity * 0.7);
        painter.rect_filled(
            srect
                .translate(sh_offset)
                .expand(el.shadow.blur * zoom * 0.25),
            8.0 * zoom,
            sh_col,
        );
    }

    match &el.kind {
        ElementKind::Shape {
            shape_type,
            fill,
            stroke_color,
            stroke_width,
            corner_radius,
        } => {
            if matches!(fill, FillStyle::LinearGradient { .. }) {
                use std::hash::{Hash, Hasher};
                let scale = (zoom * anim_scale)
                    .min(2048. / el.width.max(1.))
                    .min(2048. / el.height.max(1.))
                    .max(0.01);
                let w = (el.width * scale).ceil().max(1.) as u32;
                let h = (el.height * scale).ceil().max(1.) as u32;
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&el.kind)
                    .unwrap_or_default()
                    .hash(&mut hash);
                (w, h, el.width.to_bits(), el.height.to_bits()).hash(&mut hash);
                let key = (el.id, hash.finish());
                let tex = texture_cache.entry(key).or_insert_with(|| {
                    let mut local = el.clone();
                    local.x = 0.;
                    local.y = 0.;
                    local.rotation = 0.;
                    local.flip_h = false;
                    local.flip_v = false;
                    local.opacity = 1.;
                    local.shadow.enabled = false;
                    let mut page = crate::model::CanvasPage::new(0, "Gradient preview");
                    page.elements.push(local);
                    let pixels = crate::exporter::render_page_to_rgba_image(
                        &page, el.width, el.height, w, h, true,
                    );
                    ctx.load_texture(
                        format!("shape_{}_{}", key.0, key.1),
                        ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize],
                            pixels.as_raw(),
                        ),
                        TextureOptions::LINEAR,
                    )
                });
                let tf = crate::model::render_scene::LocalToPageTransform::from_element(el);
                let center = to_screen(el.center().0, el.center().1);
                let mut mesh = Mesh::with_texture(tex.id());
                for ((x, y), uv) in [
                    ((0., 0.), Pos2::new(0., 0.)),
                    ((el.width, 0.), Pos2::new(1., 0.)),
                    ((el.width, el.height), Pos2::new(1., 1.)),
                    ((0., el.height), Pos2::new(0., 1.)),
                ] {
                    let (px, py) = tf.local_to_page(x, y);
                    let pos = center + (to_screen(px, py) - center) * anim_scale + anim_offset;
                    mesh.vertices.push(Vertex {
                        pos,
                        uv,
                        color: Color32::from_white_alpha((opacity * 255.) as u8),
                    });
                }
                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                painter.add(Shape::mesh(mesh));
                return;
            }
            let fill_col = with_opacity(fill.primary_color(), opacity);
            let stroke = if *stroke_width > 0.1 {
                Stroke::new(*stroke_width * zoom, with_opacity(*stroke_color, opacity))
            } else {
                Stroke::NONE
            };

            match shape_type {
                ShapeType::Rectangle | ShapeType::RoundedRect | ShapeType::Pill
                    if el.rotation.abs() <= 0.1 =>
                {
                    let r = match shape_type {
                        ShapeType::Rectangle => 0.0,
                        ShapeType::RoundedRect => *corner_radius * zoom,
                        ShapeType::Pill => srect.width().min(srect.height()) * 0.5,
                        _ => 0.0,
                    };
                    if let FillStyle::LinearGradient {
                        start,
                        end,
                        vertical,
                    } = fill
                    {
                        paint_gradient_rect(
                            painter,
                            srect,
                            with_opacity(*start, opacity),
                            with_opacity(*end, opacity),
                            *vertical,
                        );
                    } else {
                        painter.rect_filled(srect, r, fill_col);
                    }
                    if stroke.width > 0.0 {
                        painter.rect_stroke(srect, r, stroke);
                    }
                }
                ShapeType::Circle => {
                    let center = srect.center();
                    let rx = srect.width() * 0.5;
                    let ry = srect.height() * 0.5;
                    if (rx - ry).abs() < 1.5 {
                        painter.circle_filled(center, rx, fill_col);
                        if stroke.width > 0.0 {
                            painter.circle_stroke(center, rx, stroke);
                        }
                    } else {
                        let (sin, cos) = el.rotation.to_radians().sin_cos();
                        let pts: Vec<_> = (0..64)
                            .map(|i| {
                                let a = i as f32 / 64. * std::f32::consts::TAU;
                                let dx = a.cos() * rx;
                                let dy = a.sin() * ry;
                                (
                                    center.x + dx * cos - dy * sin,
                                    center.y + dx * sin + dy * cos,
                                )
                            })
                            .collect();
                        paint_fan_polygon(painter, srect.center(), &pts, fill_col, stroke);
                    }
                }
                _ => {
                    let pts = compute_shape_polygon_points(
                        *shape_type,
                        srect.min.x,
                        srect.min.y,
                        srect.width(),
                        srect.height(),
                    );
                    let rot_pts: Vec<(f32, f32)> =
                        if el.rotation.abs() > 0.1 || el.flip_h || el.flip_v {
                            let c = srect.center();
                            let rad = el.rotation.to_radians();
                            pts.into_iter()
                                .map(|(px, py)| {
                                    let dx = (px - c.x) * if el.flip_h { -1. } else { 1. };
                                    let dy = (py - c.y) * if el.flip_v { -1. } else { 1. };
                                    (
                                        c.x + dx * rad.cos() - dy * rad.sin(),
                                        c.y + dx * rad.sin() + dy * rad.cos(),
                                    )
                                })
                                .collect()
                        } else {
                            pts
                        };
                    paint_fan_polygon(painter, srect.center(), &rot_pts, fill_col, stroke);
                }
            }
        }
        ElementKind::Text {
            content,
            font_family,
            font_size,
            bold,
            italic,
            underline,
            color,
            uppercase,
            align,
            letter_spacing,
            line_height,
            effect,
            list_style,
        } => {
            let layout = crate::paragraph_layout::layout_formatted(
                content,
                *font_family,
                *font_size,
                *bold,
                *italic,
                *uppercase,
                *align,
                *letter_spacing,
                *line_height,
                *list_style,
                el.width,
                el.height,
                &el.text_format,
            );
            if matches!(effect, TextEffect::None) {
                use std::hash::{Hash, Hasher};
                let pad = *font_size + 2.;
                let lw = el.width.max(layout.measured_width) + 2. * pad;
                let lh = el.height.max(layout.measured_height) + 2. * pad;
                let scale = (zoom * anim_scale)
                    .min(4096. / lw.max(1.))
                    .min(4096. / lh.max(1.))
                    .max(0.01);
                let w = (lw * scale).ceil().max(1.) as u32;
                let h = (lh * scale).ceil().max(1.) as u32;
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                serde_json::to_string(&el.kind)
                    .unwrap_or_default()
                    .hash(&mut hash);
                serde_json::to_string(&el.text_format)
                    .unwrap_or_default()
                    .hash(&mut hash);
                (w, h, el.width.to_bits(), el.height.to_bits()).hash(&mut hash);
                let key = (el.id, hash.finish());
                let tex = texture_cache.entry(key).or_insert_with(|| {
                    let mut rgba = image::RgbaImage::new(w, h);
                    crate::model::text_layout::rasterize_text_layout(
                        &mut rgba,
                        &layout,
                        pad * scale,
                        pad * scale,
                        scale,
                        scale,
                        0.,
                        false,
                        false,
                        *underline,
                        *color,
                        1.,
                    );
                    ctx.load_texture(
                        format!("text_{}_{}", key.0, key.1),
                        ColorImage::from_rgba_unmultiplied([w as usize, h as usize], rgba.as_raw()),
                        TextureOptions::LINEAR,
                    )
                });
                let tf = crate::model::render_scene::LocalToPageTransform::from_element(el);
                let corners = [
                    (-pad, -pad),
                    (lw - pad, -pad),
                    (lw - pad, lh - pad),
                    (-pad, lh - pad),
                ];
                let uvs = [
                    Pos2::new(0., 0.),
                    Pos2::new(1., 0.),
                    Pos2::new(1., 1.),
                    Pos2::new(0., 1.),
                ];
                let mut mesh = Mesh::with_texture(tex.id());
                for (i, (x, y)) in corners.iter().enumerate() {
                    let (px, py) = tf.local_to_page(*x, *y);
                    let pos = to_screen(px, py);
                    let pos = to_screen(el.center().0, el.center().1)
                        + (pos - to_screen(el.center().0, el.center().1)) * anim_scale
                        + anim_offset;
                    mesh.vertices.push(Vertex {
                        pos,
                        uv: uvs[i],
                        color: Color32::from_white_alpha((opacity * 255.) as u8),
                    });
                }
                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                painter.add(Shape::mesh(mesh));
                if layout.is_overflowing() {
                    painter.rect_stroke(srect, 2., Stroke::new(1.5_f32, Color32::RED));
                }
                return;
            }
            let scaled_size = (*font_size * zoom * anim_scale).max(4.0);
            let egui_family =
                EguiFontFamily::Name(layout.primary_face_id.egui_family_name().into());
            let font_id = FontId::new(scaled_size, egui_family);

            // Text effect background/underlays
            match effect {
                TextEffect::BackgroundPill {
                    bg_color,
                    radius,
                    padding,
                } => {
                    let pad = *padding * zoom;
                    painter.rect_filled(
                        srect.expand2(Vec2::new(pad, pad * 0.5)),
                        *radius * zoom,
                        with_opacity(*bg_color, opacity),
                    );
                }
                TextEffect::Lift { intensity } => {
                    painter.rect_filled(
                        srect.translate(Vec2::new(0.0, 4.0 * zoom)),
                        8.0 * zoom,
                        Color32::from_black_alpha((60.0 * intensity * opacity) as u8),
                    );
                }
                _ => {}
            }

            let text_col = with_opacity(*color, opacity);
            let center = srect.center();
            let rot_rad = el.rotation.to_radians();
            let (sin_r, cos_r) = rot_rad.sin_cos();
            let map_local_pt = |lx: f32, ly: f32, extra_offset: Vec2| -> Pos2 {
                let raw_p =
                    Pos2::new(srect.min.x + lx * zoom, srect.min.y + ly * zoom) + extra_offset;
                if el.rotation.abs() > 0.1 {
                    let d = raw_p - center;
                    Pos2::new(
                        center.x + d.x * cos_r - d.y * sin_r,
                        center.y + d.x * sin_r + d.y * cos_r,
                    )
                } else {
                    raw_p
                }
            };

            let paint_line_glyphs = |line: &crate::model::text_layout::TextLineLayout,
                                     extra_offset: Vec2,
                                     col: Color32| {
                if line.glyphs.is_empty() {
                    return;
                }
                if letter_spacing.abs() <= 0.05
                    && el.rotation.abs() <= 0.1
                    && line
                        .glyphs
                        .iter()
                        .all(|g| g.face_id == layout.primary_face_id)
                {
                    let pos = map_local_pt(line.x_offset, line.y_top, extra_offset);
                    painter.text(pos, Align2::LEFT_TOP, &line.text, font_id.clone(), col);
                } else {
                    let mut buf = [0u8; 4];
                    for pg in &line.glyphs {
                        if pg.ch.is_control() {
                            continue;
                        }
                        let pos = map_local_pt(pg.x, line.y_top, extra_offset);
                        let g_family = EguiFontFamily::Name(pg.face_id.egui_family_name().into());
                        let g_font = FontId::new(scaled_size, g_family);
                        let s = pg.ch.encode_utf8(&mut buf);
                        painter.text(pos, Align2::LEFT_TOP, s, g_font, col);
                    }
                }
            };

            for line in &layout.lines {
                match effect {
                    TextEffect::DropShadow {
                        offset,
                        color: sh_col,
                    } => {
                        let d = *offset * zoom;
                        paint_line_glyphs(line, Vec2::new(d, d), with_opacity(*sh_col, opacity));
                    }
                    TextEffect::NeonGlow {
                        glow_color,
                        intensity,
                    } => {
                        let gc =
                            with_opacity(*glow_color, (0.32 * intensity * opacity).clamp(0.0, 1.0));
                        for (dx, dy) in [
                            (-3.0, 0.0),
                            (3.0, 0.0),
                            (0.0, -3.0),
                            (0.0, 3.0),
                            (-2.0, -2.0),
                            (2.0, 2.0),
                        ] {
                            paint_line_glyphs(line, Vec2::new(dx * zoom, dy * zoom), gc);
                        }
                    }
                    TextEffect::Splice { offset, fill_color } => {
                        let d = *offset * zoom;
                        paint_line_glyphs(
                            line,
                            Vec2::new(d, d),
                            with_opacity(*fill_color, opacity),
                        );
                    }
                    TextEffect::Echo { color: echo_col } => {
                        for step in [2, 1] {
                            let d = (step as f32) * 4.0 * zoom;
                            paint_line_glyphs(
                                line,
                                Vec2::new(-d, d * 0.6),
                                with_opacity(*echo_col, opacity * (0.45 / step as f32)),
                            );
                        }
                    }
                    TextEffect::HollowOutline {
                        thickness,
                        color: out_col,
                    } => {
                        let t = (*thickness * zoom).max(1.0);
                        let oc = with_opacity(*out_col, opacity);
                        for (dx, dy) in [(-t, 0.0), (t, 0.0), (0.0, -t), (0.0, t)] {
                            paint_line_glyphs(line, Vec2::new(dx, dy), oc);
                        }
                    }
                    _ => {}
                }

                paint_line_glyphs(line, Vec2::ZERO, text_col);

                if *underline && !line.text.is_empty() {
                    let uy = line.y_baseline + *font_size * 0.1;
                    let p0 = map_local_pt(line.x_offset, uy, Vec2::ZERO);
                    let p1 = map_local_pt(line.x_offset + line.width, uy, Vec2::ZERO);
                    painter.line_segment(
                        [p0, p1],
                        Stroke::new((*font_size * 0.06 * zoom).max(1.0), text_col),
                    );
                }
            }

            if layout.is_overflowing() {
                let warn_col = Color32::from_rgb(239, 68, 68);
                painter.rect_stroke(srect, 2.0, Stroke::new(1.5_f32, warn_col));
                let badge_rect = Rect::from_min_size(
                    Pos2::new((srect.max.x - 74.0).max(srect.min.x), srect.max.y - 16.0),
                    Vec2::new(74.0, 16.0),
                );
                painter.rect_filled(badge_rect, 3.0, warn_col);
                painter.text(
                    badge_rect.center(),
                    Align2::CENTER_CENTER,
                    "OVERFLOW",
                    FontId::proportional(9.5),
                    Color32::WHITE,
                );
            }
        }
        ElementKind::Image {
            width_px,
            height_px,
            raw_rgba,
            mask,
            adjustments,
            border_color,
            border_width,
            revision,
            ..
        } => {
            if *width_px > 0 && *height_px > 0 && !raw_rgba.is_empty() {
                let target_w = (el.width.round() as u32).clamp(32, 768);
                let target_h = (el.height.round() as u32).clamp(32, 768);
                let mask_disc = match mask {
                    ImageMask::None => 0u64,
                    ImageMask::Rounded(r) => 1u64 ^ ((r.to_bits() as u64) << 8),
                    ImageMask::Circle => 2u64,
                    ImageMask::Pill => 3u64,
                    ImageMask::Diamond => 4u64,
                    ImageMask::Hexagon => 5u64,
                };
                let fit_disc = adjustments.fit as u64;
                let crop_sig = (adjustments.crop.left.to_bits() as u64)
                    .wrapping_mul(31)
                    .wrapping_add(adjustments.crop.top.to_bits() as u64)
                    .wrapping_mul(31)
                    .wrapping_add(adjustments.crop.right.to_bits() as u64)
                    .wrapping_mul(31)
                    .wrapping_add(adjustments.crop.bottom.to_bits() as u64);
                let cache_rev = ((*revision)
                    .wrapping_mul(1_000_003)
                    .wrapping_add(((target_w as u64) << 16) ^ (target_h as u64))
                    .wrapping_add(mask_disc << 32)
                    .wrapping_add(fit_disc << 48))
                    ^ crop_sig;
                let key = (el.id, cache_rev);
                let tex = texture_cache.entry(key).or_insert_with(|| {
                    let processed =
                        apply_image_pipeline(raw_rgba, *width_px, *height_px, adjustments);
                    let placement = compute_image_placement(
                        *width_px,
                        *height_px,
                        target_w as f32,
                        target_h as f32,
                        adjustments.fit,
                        adjustments.crop,
                        adjustments.focal_point,
                    );
                    let mut composed = vec![0u8; (target_w * target_h * 4) as usize];
                    for py in 0..target_h {
                        for px in 0..target_w {
                            let lx = px as f32 + 0.5;
                            let ly = py as f32 + 0.5;
                            let m_cov =
                                mask_coverage(*mask, lx, ly, target_w as f32, target_h as f32);
                            if m_cov <= 0.0 {
                                continue;
                            }
                            if let Some((sx, sy)) = placement.local_to_source_px(lx, ly) {
                                let mut rgba =
                                    sample_bilinear_rgba(&processed, *width_px, *height_px, sx, sy);
                                rgba[3] =
                                    ((rgba[3] as f32) * m_cov).round().clamp(0.0, 255.0) as u8;
                                let dst = ((py * target_w + px) * 4) as usize;
                                composed[dst..dst + 4].copy_from_slice(&rgba);
                            }
                        }
                    }
                    let color_img = ColorImage::from_rgba_unmultiplied(
                        [target_w as usize, target_h as usize],
                        &composed,
                    );
                    ctx.load_texture(
                        format!("img_{}_{}", el.id, cache_rev),
                        color_img,
                        TextureOptions::LINEAR,
                    )
                });

                let uv = Rect::from_min_max(
                    Pos2::new(
                        if el.flip_h { 1.0 } else { 0.0 },
                        if el.flip_v { 1.0 } else { 0.0 },
                    ),
                    Pos2::new(
                        if el.flip_h { 0.0 } else { 1.0 },
                        if el.flip_v { 0.0 } else { 1.0 },
                    ),
                );
                let tint = Color32::from_white_alpha((opacity * 255.0) as u8);
                let mut mesh = Mesh::with_texture(tex.id());
                let (sin, cos) = el.rotation.to_radians().sin_cos();
                for (point, uvp) in [
                    (srect.left_top(), uv.left_top()),
                    (srect.right_top(), uv.right_top()),
                    (srect.right_bottom(), uv.right_bottom()),
                    (srect.left_bottom(), uv.left_bottom()),
                ] {
                    let d = point - srect.center();
                    let pos =
                        srect.center() + Vec2::new(d.x * cos - d.y * sin, d.x * sin + d.y * cos);
                    mesh.vertices.push(Vertex {
                        pos,
                        uv: uvp,
                        color: tint,
                    });
                }
                mesh.indices.extend_from_slice(&[0, 1, 2, 0, 2, 3]);
                painter.add(Shape::mesh(mesh));

                let rounding = match mask {
                    ImageMask::None => 0.0,
                    ImageMask::Rounded(r) => *r * zoom,
                    ImageMask::Circle | ImageMask::Pill => srect.width().min(srect.height()) * 0.5,
                    _ => 12.0 * zoom,
                };
                if *border_width > 0.1 {
                    painter.rect_stroke(
                        srect,
                        rounding,
                        Stroke::new(*border_width * zoom, with_opacity(*border_color, opacity)),
                    );
                }
            }
        }
        ElementKind::FreehandPath {
            points,
            color,
            stroke_width,
            brush,
        } => {
            if points.len() >= 2 {
                let pts: Vec<Pos2> = points
                    .iter()
                    .map(|(nx, ny)| {
                        Pos2::new(
                            srect.min.x + nx * srect.width(),
                            srect.min.y + ny * srect.height(),
                        )
                    })
                    .collect();
                let col = with_opacity(*color, opacity);
                let w = (*stroke_width * zoom).max(1.5);
                if *brush == BrushType::NeonGlow {
                    painter.add(PathShape::line(
                        pts.clone(),
                        Stroke::new(w * 2.4, col.gamma_multiply(0.35)),
                    ));
                }
                painter.add(PathShape::line(pts, Stroke::new(w, col)));
            }
        }
        ElementKind::Chart {
            chart_type,
            title,
            labels,
            values,
            palette,
            show_values,
            show_grid,
            text_color,
        } => {
            paint_chart_element(
                painter,
                srect,
                zoom,
                opacity,
                *chart_type,
                title,
                labels,
                values,
                palette,
                *show_values,
                *show_grid,
                *text_color,
            );
        }
        ElementKind::Table { rows, cols, .. } => {
            if *rows == 0 || *cols == 0 || rows.saturating_mul(*cols) > 10000 {
                return;
            }
            let mut page = crate::model::CanvasPage::new(0, "Table preview");
            page.elements.push(el.clone());
            for child in crate::model::render_scene::expand_page_elements(&page) {
                paint_canvas_element(
                    ctx,
                    painter,
                    &child,
                    to_screen,
                    zoom,
                    anim_offset,
                    anim_scale,
                    anim_alpha,
                    texture_cache,
                );
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn paint_chart_element(
    painter: &egui::Painter,
    srect: Rect,
    zoom: f32,
    opacity: f32,
    chart_type: ChartType,
    title: &str,
    labels: &[String],
    values: &[f32],
    palette: &[[u8; 4]],
    show_values: bool,
    show_grid: bool,
    text_color: [u8; 4],
) {
    let t_col = with_opacity(text_color, opacity);
    painter.text(
        Pos2::new(srect.min.x + 12.0 * zoom, srect.min.y + 8.0 * zoom),
        Align2::LEFT_TOP,
        title,
        FontId::proportional((24.0 * zoom).max(10.0)),
        t_col,
    );

    let plot_rect = Rect::from_min_max(
        Pos2::new(srect.min.x + 18.0 * zoom, srect.min.y + 48.0 * zoom),
        Pos2::new(srect.max.x - 18.0 * zoom, srect.max.y - 28.0 * zoom),
    );
    if plot_rect.width() < 20.0 || plot_rect.height() < 20.0 || values.is_empty() {
        return;
    }

    let max_val = values.iter().copied().fold(1.0_f32, f32::max);

    if show_grid && (chart_type == ChartType::VerticalBar || chart_type == ChartType::LineArea) {
        let g_stroke = Stroke::new(1.0_f32, t_col.gamma_multiply(0.18));
        for i in 0..=4 {
            let y = plot_rect.min.y + (i as f32 / 4.0) * plot_rect.height();
            painter.line_segment(
                [Pos2::new(plot_rect.min.x, y), Pos2::new(plot_rect.max.x, y)],
                g_stroke,
            );
        }
    }

    match chart_type {
        ChartType::VerticalBar => {
            let n = values.len();
            let step = plot_rect.width() / (n as f32);
            let bar_w = (step * 0.64).max(6.0);
            for (i, val) in values.iter().enumerate() {
                let ratio = (*val / max_val).clamp(0.02, 1.0);
                let bh = ratio * plot_rect.height();
                let bx = plot_rect.min.x + (i as f32) * step + (step - bar_w) * 0.5;
                let by = plot_rect.max.y - bh;
                let col = palette
                    .get(i % palette.len().max(1))
                    .copied()
                    .unwrap_or([0, 196, 204, 255]);
                let brect = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(bar_w, bh));
                painter.rect_filled(brect, 6.0 * zoom, with_opacity(col, opacity));

                if show_values {
                    painter.text(
                        Pos2::new(brect.center().x, brect.min.y - 4.0 * zoom),
                        Align2::CENTER_BOTTOM,
                        format!("{:.0}", val),
                        FontId::proportional((14.0 * zoom).max(8.0)),
                        t_col,
                    );
                }
                if let Some(lbl) = labels.get(i) {
                    painter.text(
                        Pos2::new(brect.center().x, plot_rect.max.y + 6.0 * zoom),
                        Align2::CENTER_TOP,
                        lbl,
                        FontId::proportional((14.0 * zoom).max(8.0)),
                        t_col,
                    );
                }
            }
        }
        ChartType::HorizontalBar => {
            let n = values.len();
            let step = plot_rect.height() / (n as f32);
            let bar_h = (step * 0.62).max(6.0);
            for (i, val) in values.iter().enumerate() {
                let ratio = (*val / max_val).clamp(0.02, 1.0);
                let bw = ratio * (plot_rect.width() * 0.78);
                let by = plot_rect.min.y + (i as f32) * step + (step - bar_h) * 0.5;
                let bx = plot_rect.min.x + plot_rect.width() * 0.18;
                let col = palette
                    .get(i % palette.len().max(1))
                    .copied()
                    .unwrap_or([0, 196, 204, 255]);
                let brect = Rect::from_min_size(Pos2::new(bx, by), Vec2::new(bw, bar_h));
                painter.rect_filled(brect, 5.0 * zoom, with_opacity(col, opacity));
                if let Some(lbl) = labels.get(i) {
                    painter.text(
                        Pos2::new(bx - 8.0 * zoom, brect.center().y),
                        Align2::RIGHT_CENTER,
                        lbl,
                        FontId::proportional((14.0 * zoom).max(8.0)),
                        t_col,
                    );
                }
            }
        }
        ChartType::Pie | ChartType::Donut => {
            let sum: f32 = values.iter().copied().sum::<f32>().max(1.0);
            let center = plot_rect.center();
            let radius = plot_rect.width().min(plot_rect.height()) * 0.42;
            let mut start_angle = -std::f32::consts::FRAC_PI_2;

            for (i, val) in values.iter().enumerate() {
                let sweep = (*val / sum) * std::f32::consts::TAU;
                let steps = ((sweep * 12.0).ceil() as usize).max(3);
                let col = palette
                    .get(i % palette.len().max(1))
                    .copied()
                    .unwrap_or([0, 196, 204, 255]);
                let mut pts = Vec::with_capacity(steps + 1);
                for s in 0..=steps {
                    let a = start_angle + (s as f32 / steps as f32) * sweep;
                    pts.push((center.x + radius * a.cos(), center.y + radius * a.sin()));
                }
                paint_fan_polygon(
                    painter,
                    center,
                    &pts,
                    with_opacity(col, opacity),
                    Stroke::NONE,
                );
                start_angle += sweep;
            }
            if chart_type == ChartType::Donut {
                painter.circle_filled(center, radius * 0.54, Color32::from_rgb(15, 23, 42));
            }
        }
        ChartType::LineArea => {
            let n = values.len();
            if n >= 2 {
                let mut pts = Vec::with_capacity(n);
                for (i, val) in values.iter().enumerate() {
                    let x = plot_rect.min.x + (i as f32 / (n - 1) as f32) * plot_rect.width();
                    let y = plot_rect.max.y - (*val / max_val).clamp(0.0, 1.0) * plot_rect.height();
                    pts.push(Pos2::new(x, y));
                }
                let col = palette.first().copied().unwrap_or([0, 196, 204, 255]);
                let line_col = with_opacity(col, opacity);
                painter.add(PathShape::line(
                    pts.clone(),
                    Stroke::new(3.5 * zoom, line_col),
                ));
                for p in pts {
                    painter.circle_filled(p, 5.0 * zoom, line_col);
                }
            }
        }
    }
}

fn paint_gradient_rect(
    painter: &egui::Painter,
    rect: Rect,
    start: Color32,
    end: Color32,
    vertical: bool,
) {
    let mut mesh = Mesh::default();
    let (c_tr, c_bl) = if vertical { (start, end) } else { (end, start) };
    mesh.vertices.push(Vertex {
        pos: rect.left_top(),
        uv: Pos2::ZERO,
        color: start,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_top(),
        uv: Pos2::ZERO,
        color: c_tr,
    });
    mesh.vertices.push(Vertex {
        pos: rect.right_bottom(),
        uv: Pos2::ZERO,
        color: end,
    });
    mesh.vertices.push(Vertex {
        pos: rect.left_bottom(),
        uv: Pos2::ZERO,
        color: c_bl,
    });
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(0, 2, 3);
    painter.add(Shape::mesh(mesh));
}

/// Renders a filled polygon (supporting both convex and star-shaped concave polygons centered at `center`)
/// using a triangle fan mesh + crisp perimeter stroke.
fn paint_fan_polygon(
    painter: &egui::Painter,
    center: Pos2,
    pts: &[(f32, f32)],
    fill: Color32,
    stroke: Stroke,
) {
    if pts.len() < 3 {
        return;
    }
    let mut mesh = Mesh::default();
    mesh.vertices.push(Vertex {
        pos: center,
        uv: Pos2::ZERO,
        color: fill,
    });
    for (px, py) in pts {
        mesh.vertices.push(Vertex {
            pos: Pos2::new(*px, *py),
            uv: Pos2::ZERO,
            color: fill,
        });
    }
    let n = pts.len() as u32;
    for i in 1..=n {
        let next = if i == n { 1 } else { i + 1 };
        mesh.add_triangle(0, i, next);
    }
    painter.add(Shape::mesh(mesh));

    if stroke.width > 0.0 {
        let pos_pts: Vec<Pos2> = pts.iter().map(|(x, y)| Pos2::new(*x, *y)).collect();
        painter.add(PathShape::closed_line(pos_pts, stroke));
    }
}

fn paint_rulers(painter: &egui::Painter, viewport: Rect, page_rect: Rect, zoom: f32) {
    let ruler_bg = Color32::from_rgb(20, 22, 26);
    let tick_col = Color32::from_rgb(120, 130, 145);
    let top_ruler = Rect::from_min_max(
        viewport.min,
        Pos2::new(viewport.max.x, viewport.min.y + 20.0),
    );
    let left_ruler = Rect::from_min_max(
        viewport.min,
        Pos2::new(viewport.min.x + 24.0, viewport.max.y),
    );
    painter.rect_filled(top_ruler, 0.0, ruler_bg);
    painter.rect_filled(left_ruler, 0.0, ruler_bg);

    let step_canvas = 200.0;
    let mut cx = 0.0;
    while cx <= 3000.0 {
        let sx = page_rect.min.x + cx * zoom;
        if sx >= viewport.min.x + 24.0 && sx <= viewport.max.x {
            painter.line_segment(
                [
                    Pos2::new(sx, viewport.min.y + 12.0),
                    Pos2::new(sx, viewport.min.y + 20.0),
                ],
                Stroke::new(1.0_f32, tick_col),
            );
            painter.text(
                Pos2::new(sx + 3.0, viewport.min.y + 2.0),
                Align2::LEFT_TOP,
                format!("{:.0}", cx),
                FontId::proportional(9.5),
                tick_col,
            );
        }
        cx += step_canvas;
    }

    let mut cy = 0.0;
    while cy <= 3000.0 {
        let sy = page_rect.min.y + cy * zoom;
        if sy >= viewport.min.y + 20.0 && sy <= viewport.max.y {
            painter.line_segment(
                [
                    Pos2::new(viewport.min.x + 14.0, sy),
                    Pos2::new(viewport.min.x + 24.0, sy),
                ],
                Stroke::new(1.0_f32, tick_col),
            );
        }
        cy += step_canvas;
    }
}

fn compute_animation_transform(
    anim: ElementAnimation,
    elapsed: f32,
    idx: usize,
) -> (Vec2, f32, f32) {
    let delay = (idx as f32) * 0.08;
    let t = ((elapsed - delay) / 0.65).clamp(0.0, 1.0);
    // Smooth ease-out cubic
    let ease = 1.0 - (1.0 - t).powi(3);

    match anim {
        ElementAnimation::None => (Vec2::ZERO, 1.0, 1.0),
        ElementAnimation::Rise => (Vec2::new(0.0, (1.0 - ease) * 48.0), 1.0, ease),
        ElementAnimation::Pan => (Vec2::new(-(1.0 - ease) * 64.0, 0.0), 1.0, ease),
        ElementAnimation::Fade => (Vec2::ZERO, 1.0, ease),
        ElementAnimation::Pop => {
            let scale = 0.4 + 0.6 * ease;
            (Vec2::ZERO, scale, ease)
        }
        ElementAnimation::Wipe => (Vec2::new((1.0 - ease) * 30.0, 0.0), 1.0, ease),
        ElementAnimation::Breathe => {
            let s = 0.92 + 0.08 * (elapsed * 2.5).sin();
            (Vec2::ZERO, s, 1.0)
        }
        ElementAnimation::Tectonic => (
            Vec2::new((1.0 - ease) * 40.0, (1.0 - ease) * 20.0),
            0.85 + 0.15 * ease,
            ease,
        ),
    }
}

/// Group handles preserve aspect ratio; the opposite edge/corner remains fixed.
fn group_resize_rect(original: Rect, handle: ResizeHandle, delta: Vec2) -> Rect {
    let left = matches!(
        handle,
        ResizeHandle::Left | ResizeHandle::TopLeft | ResizeHandle::BottomLeft
    );
    let right = matches!(
        handle,
        ResizeHandle::Right | ResizeHandle::TopRight | ResizeHandle::BottomRight
    );
    let top = matches!(
        handle,
        ResizeHandle::Top | ResizeHandle::TopLeft | ResizeHandle::TopRight
    );
    let bottom = matches!(
        handle,
        ResizeHandle::Bottom | ResizeHandle::BottomLeft | ResizeHandle::BottomRight
    );
    let sx = 1.
        + if left {
            -delta.x
        } else if right {
            delta.x
        } else {
            0.
        } / original.width().max(1.);
    let sy = 1.
        + if top {
            -delta.y
        } else if bottom {
            delta.y
        } else {
            0.
        } / original.height().max(1.);
    let scale = if !(left || right) {
        sy
    } else if !(top || bottom) || (sx - 1.).abs() > (sy - 1.).abs() {
        sx
    } else {
        sy
    };
    let scale = scale.max(16. / original.width().min(original.height()).max(1.));
    let size = original.size() * scale;
    let x = if left {
        original.max.x - size.x
    } else if right {
        original.min.x
    } else {
        original.center().x - size.x / 2.
    };
    let y = if top {
        original.max.y - size.y
    } else if bottom {
        original.min.y
    } else {
        original.center().y - size.y / 2.
    };
    Rect::from_min_size(egui::pos2(x, y), size)
}
