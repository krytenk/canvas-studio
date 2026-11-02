use crate::model::text_layout::{layout_text_block, TextLayoutResult};
use crate::model::{
    CanvasElement, CanvasPage, ColorRgba, ElementKind, FillStyle, ImageAdjustments, ImageCrop,
    ImageFit, ImageMask, PageBackground, ShapeType,
};

#[derive(Clone, Copy, Debug)]
pub struct LocalToPageTransform {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub cx: f32,
    pub cy: f32,
    pub rotation_deg: f32,
    pub flip_h: bool,
    pub flip_v: bool,
    cos_r: f32,
    sin_r: f32,
}

impl LocalToPageTransform {
    pub fn from_element(el: &CanvasElement) -> Self {
        let cx = el.x + el.width * 0.5;
        let cy = el.y + el.height * 0.5;
        let rad = el.rotation.to_radians();
        Self {
            x: el.x,
            y: el.y,
            width: el.width,
            height: el.height,
            cx,
            cy,
            rotation_deg: el.rotation,
            flip_h: el.flip_h,
            flip_v: el.flip_v,
            cos_r: rad.cos(),
            sin_r: rad.sin(),
        }
    }

    pub fn has_non_identity(&self) -> bool {
        self.rotation_deg.abs() > 0.01 || self.flip_h || self.flip_v
    }

    /// Transforms a local coordinate `(lx, ly)` in `0..width, 0..height` into page coordinates:
    /// 1. Translate relative to local center `(width/2, height/2)`
    /// 2. Apply horizontal/vertical reflection (`flip_h`, `flip_v`)
    /// 3. Rotate by `rotation_deg`
    /// 4. Translate to page center `(cx, cy)`
    #[inline]
    pub fn local_to_page(&self, lx: f32, ly: f32) -> (f32, f32) {
        let mut dx = lx - self.width * 0.5;
        let mut dy = ly - self.height * 0.5;
        if self.flip_h {
            dx = -dx;
        }
        if self.flip_v {
            dy = -dy;
        }
        let rx = dx * self.cos_r - dy * self.sin_r;
        let ry = dx * self.sin_r + dy * self.cos_r;
        (self.cx + rx, self.cy + ry)
    }

    /// Transforms a page coordinate `(px, py)` into local element coordinates `(0..width, 0..height)`.
    #[inline]
    pub fn page_to_local(&self, px: f32, py: f32) -> (f32, f32) {
        let rx = px - self.cx;
        let ry = py - self.cy;
        // Inverse rotation (-rotation_deg)
        let mut dx = rx * self.cos_r + ry * self.sin_r;
        let mut dy = -rx * self.sin_r + ry * self.cos_r;
        // Inverse reflection
        if self.flip_h {
            dx = -dx;
        }
        if self.flip_v {
            dy = -dy;
        }
        (dx + self.width * 0.5, dy + self.height * 0.5)
    }

    /// Computes the axis-aligned bounding box `(min_x, min_y, max_x, max_y)` in page coordinates
    /// that encloses all 4 transformed corners of the element.
    pub fn page_aabb(&self, pad: f32) -> (f32, f32, f32, f32) {
        let corners = [
            self.local_to_page(-pad, -pad),
            self.local_to_page(self.width + pad, -pad),
            self.local_to_page(self.width + pad, self.height + pad),
            self.local_to_page(-pad, self.height + pad),
        ];
        let mut min_x = f32::MAX;
        let mut min_y = f32::MAX;
        let mut max_x = f32::MIN;
        let mut max_y = f32::MIN;
        for (x, y) in corners {
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
        (min_x, min_y, max_x, max_y)
    }

    /// Emits the unified SVG transform attribute matching `local_to_page`.
    pub fn svg_transform_attr(&self) -> String {
        if !self.has_non_identity() {
            return String::new();
        }
        let sx = if self.flip_h { -1 } else { 1 };
        let sy = if self.flip_v { -1 } else { 1 };
        if self.flip_h || self.flip_v {
            format!(
                r#" transform="translate({:.2}, {:.2}) rotate({:.2}) scale({}, {}) translate({:.2}, {:.2})""#,
                self.cx, self.cy, self.rotation_deg, sx, sy, -self.cx, -self.cy
            )
        } else {
            format!(
                r#" transform="rotate({:.2} {:.2} {:.2})""#,
                self.rotation_deg, self.cx, self.cy
            )
        }
    }
}

/// Placement geometry for an image inside an element frame `(0..fw, 0..fh)` after applying
/// source crop `(left, top, right, bottom)` and `ImageFit` (`Contain` vs `Cover` vs `Stretch`).
#[derive(Clone, Copy, Debug)]
pub struct ImagePlacement {
    pub src_x0: f32,
    pub src_y0: f32,
    pub src_w: f32,
    pub src_h: f32,
    pub dest_x: f32,
    pub dest_y: f32,
    pub dest_w: f32,
    pub dest_h: f32,
    pub clip_x: f32,
    pub clip_y: f32,
    pub clip_w: f32,
    pub clip_h: f32,
}

impl ImagePlacement {
    #[inline]
    pub fn local_to_source_px(&self, lx: f32, ly: f32) -> Option<(f32, f32)> {
        if lx < self.clip_x
            || lx > self.clip_x + self.clip_w
            || ly < self.clip_y
            || ly > self.clip_y + self.clip_h
        {
            return None;
        }
        let u = (lx - self.dest_x) / self.dest_w.max(1e-5);
        let v = (ly - self.dest_y) / self.dest_h.max(1e-5);
        if !(0.0..=1.0).contains(&u) || !(0.0..=1.0).contains(&v) {
            return None;
        }
        Some((self.src_x0 + u * self.src_w, self.src_y0 + v * self.src_h))
    }
}

pub fn compute_image_placement(
    src_width_px: u32,
    src_height_px: u32,
    frame_w: f32,
    frame_h: f32,
    fit: ImageFit,
    crop: ImageCrop,
    focal_point: (f32, f32),
) -> ImagePlacement {
    let sw = (src_width_px.max(1)) as f32;
    let sh = (src_height_px.max(1)) as f32;
    let cl = crop.left.clamp(0.0, 0.95);
    let ct = crop.top.clamp(0.0, 0.95);
    let cr = crop.right.clamp(0.0, 0.95 - cl);
    let cb = crop.bottom.clamp(0.0, 0.95 - ct);

    let src_x0 = sw * cl;
    let src_y0 = sh * ct;
    let cw = (sw * (1.0 - cl - cr)).max(1.0);
    let ch = (sh * (1.0 - ct - cb)).max(1.0);

    let fw = frame_w.max(1.0);
    let fh = frame_h.max(1.0);
    let fx = focal_point.0.clamp(0.0, 1.0);
    let fy = focal_point.1.clamp(0.0, 1.0);

    match fit {
        ImageFit::Contain => {
            let scale = (fw / cw).min(fh / ch);
            let dw = cw * scale;
            let dh = ch * scale;
            let dx = (fw - dw) * fx;
            let dy = (fh - dh) * fy;
            ImagePlacement {
                src_x0,
                src_y0,
                src_w: cw,
                src_h: ch,
                dest_x: dx,
                dest_y: dy,
                dest_w: dw,
                dest_h: dh,
                clip_x: dx,
                clip_y: dy,
                clip_w: dw,
                clip_h: dh,
            }
        }
        ImageFit::Cover => {
            let scale = (fw / cw).max(fh / ch);
            let dw = cw * scale;
            let dh = ch * scale;
            let dx = (fw - dw) * fx;
            let dy = (fh - dh) * fy;
            ImagePlacement {
                src_x0,
                src_y0,
                src_w: cw,
                src_h: ch,
                dest_x: dx,
                dest_y: dy,
                dest_w: dw,
                dest_h: dh,
                clip_x: 0.0,
                clip_y: 0.0,
                clip_w: fw,
                clip_h: fh,
            }
        }
        ImageFit::Stretch => ImagePlacement {
            src_x0,
            src_y0,
            src_w: cw,
            src_h: ch,
            dest_x: 0.0,
            dest_y: 0.0,
            dest_w: fw,
            dest_h: fh,
            clip_x: 0.0,
            clip_y: 0.0,
            clip_w: fw,
            clip_h: fh,
        },
    }
}

/// Evaluates antialiased mask coverage `0.0..=1.0` for a point `(lx, ly)` in local frame `(0..fw, 0..fh)`.
pub fn mask_coverage(mask: ImageMask, lx: f32, ly: f32, fw: f32, fh: f32) -> f32 {
    if lx < 0.0 || ly < 0.0 || lx > fw || ly > fh {
        return 0.0;
    }
    match mask {
        ImageMask::None => 1.0,
        ImageMask::Circle => {
            let cx = fw * 0.5;
            let cy = fh * 0.5;
            let r = fw.min(fh) * 0.5;
            let d = (lx - cx).hypot(ly - cy) - r;
            (0.5 - d).clamp(0.0, 1.0)
        }
        ImageMask::Rounded(radius) => {
            let r = radius.min(fw * 0.5).min(fh * 0.5);
            let qx = (lx - fw * 0.5).abs() - (fw * 0.5 - r);
            let qy = (ly - fh * 0.5).abs() - (fh * 0.5 - r);
            let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r;
            (0.5 - d).clamp(0.0, 1.0)
        }
        ImageMask::Pill => {
            let r = fw.min(fh) * 0.5;
            let qx = (lx - fw * 0.5).abs() - (fw * 0.5 - r);
            let qy = (ly - fh * 0.5).abs() - (fh * 0.5 - r);
            let d = qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r;
            (0.5 - d).clamp(0.0, 1.0)
        }
        ImageMask::Diamond => {
            let nx = ((lx - fw * 0.5) / (fw * 0.5).max(1.0)).abs();
            let ny = ((ly - fh * 0.5) / (fh * 0.5).max(1.0)).abs();
            let d = (nx + ny - 1.0) * fw.min(fh) * 0.35;
            (0.5 - d).clamp(0.0, 1.0)
        }
        ImageMask::Hexagon => {
            let nx = ((lx - fw * 0.5) / (fw * 0.5).max(1.0)).abs();
            let ny = ((ly - fh * 0.5) / (fh * 0.5).max(1.0)).abs();
            let d = (nx.max(nx * 0.5 + ny * 0.866) - 0.95) * fw.min(fh) * 0.4;
            (0.5 - d).clamp(0.0, 1.0)
        }
    }
}

/// Evaluates signed distance to a local shape boundary `(0..fw, 0..fh)`.
/// Negative values are inside the shape; `0.0` is on the boundary; positive values are outside.
pub fn shape_signed_distance(
    shape_type: ShapeType,
    corner_radius: f32,
    lx: f32,
    ly: f32,
    fw: f32,
    fh: f32,
    local_poly: &[(f32, f32)],
) -> f32 {
    match shape_type {
        ShapeType::Rectangle | ShapeType::LineHorizontal => {
            let qx = (lx - fw * 0.5).abs() - fw * 0.5;
            let qy = (ly - fh * 0.5).abs() - fh * 0.5;
            qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0)
        }
        ShapeType::RoundedRect => {
            let r = corner_radius.min(fw * 0.5).min(fh * 0.5);
            let qx = (lx - fw * 0.5).abs() - (fw * 0.5 - r);
            let qy = (ly - fh * 0.5).abs() - (fh * 0.5 - r);
            qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
        }
        ShapeType::Pill => {
            let r = fw.min(fh) * 0.5;
            let qx = (lx - fw * 0.5).abs() - (fw * 0.5 - r);
            let qy = (ly - fh * 0.5).abs() - (fh * 0.5 - r);
            qx.max(0.0).hypot(qy.max(0.0)) + qx.max(qy).min(0.0) - r
        }
        ShapeType::Circle => {
            let rx = (fw * 0.5).max(0.5);
            let ry = (fh * 0.5).max(0.5);
            let dx = lx - fw * 0.5;
            let dy = ly - fh * 0.5;
            let norm = (dx / rx).hypot(dy / ry);
            (norm - 1.0) * rx.min(ry)
        }
        _ => polygon_signed_distance(lx, ly, local_poly),
    }
}

fn polygon_signed_distance(px: f32, py: f32, poly: &[(f32, f32)]) -> f32 {
    if poly.len() < 3 {
        return 1000.0;
    }
    let mut min_dist_sq = f32::MAX;
    let mut inside = false;
    let mut j = poly.len() - 1;
    for i in 0..poly.len() {
        let (x0, y0) = poly[j];
        let (x1, y1) = poly[i];
        if ((y1 > py) != (y0 > py)) && (px < (x0 - x1) * (py - y1) / (y0 - y1 + 1e-6) + x1) {
            inside = !inside;
        }
        let vx = x1 - x0;
        let vy = y1 - y0;
        let wx = px - x0;
        let wy = py - y0;
        let len_sq = (vx * vx + vy * vy).max(1e-6);
        let t = ((wx * vx + wy * vy) / len_sq).clamp(0.0, 1.0);
        let proj_x = x0 + t * vx;
        let proj_y = y0 + t * vy;
        let d_sq = (px - proj_x).powi(2) + (py - proj_y).powi(2);
        min_dist_sq = min_dist_sq.min(d_sq);
        j = i;
    }
    let dist = min_dist_sq.sqrt();
    if inside {
        -dist
    } else {
        dist
    }
}

#[derive(Clone, Debug)]
pub enum SceneItem {
    Shape {
        element_id: u64,
        transform: LocalToPageTransform,
        shape_type: ShapeType,
        fill: FillStyle,
        stroke_color: ColorRgba,
        stroke_width: f32,
        corner_radius: f32,
        opacity: f32,
    },
    Text {
        element_id: u64,
        name: String,
        transform: LocalToPageTransform,
        layout: TextLayoutResult,
        color: ColorRgba,
        underline: bool,
        opacity: f32,
    },
    Image {
        element_id: u64,
        name: String,
        transform: LocalToPageTransform,
        width_px: u32,
        height_px: u32,
        raw_rgba: Vec<u8>,
        mask: ImageMask,
        adjustments: ImageAdjustments,
        placement: ImagePlacement,
        border_color: ColorRgba,
        border_width: f32,
        opacity: f32,
    },
    FallbackElement(CanvasElement),
}

#[derive(Clone, Debug)]
pub struct RenderScene {
    pub page_id: u64,
    pub page_title: String,
    pub width: f32,
    pub height: f32,
    pub background: PageBackground,
    pub items: Vec<SceneItem>,
}

pub fn build_page_scene(page: &CanvasPage, width: f32, height: f32) -> RenderScene {
    let mut items = Vec::with_capacity(page.elements.len());

    let expanded = expand_page_elements(page);
    for el in &expanded {
        if !el.visible {
            continue;
        }
        let transform = LocalToPageTransform::from_element(el);
        let opacity = el.opacity.clamp(0.0, 1.0);

        match &el.kind {
            ElementKind::Shape {
                shape_type,
                fill,
                stroke_color,
                stroke_width,
                corner_radius,
            } => {
                items.push(SceneItem::Shape {
                    element_id: el.id,
                    transform,
                    shape_type: *shape_type,
                    fill: *fill,
                    stroke_color: *stroke_color,
                    stroke_width: *stroke_width,
                    corner_radius: *corner_radius,
                    opacity,
                });
            }
            ElementKind::Text {
                content,
                font_family,
                font_size,
                color,
                bold,
                italic,
                underline,
                uppercase,
                align,
                letter_spacing,
                line_height,
                list_style,
                ..
            } => {
                let layout = layout_text_block(
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
                );
                items.push(SceneItem::Text {
                    element_id: el.id,
                    name: el.name.clone(),
                    transform,
                    layout,
                    color: *color,
                    underline: *underline,
                    opacity,
                });
            }
            ElementKind::Image {
                width_px,
                height_px,
                raw_rgba,
                mask,
                adjustments,
                border_color,
                border_width,
                ..
            } => {
                let placement = compute_image_placement(
                    *width_px,
                    *height_px,
                    el.width,
                    el.height,
                    adjustments.fit,
                    adjustments.crop,
                    adjustments.focal_point,
                );
                items.push(SceneItem::Image {
                    element_id: el.id,
                    name: el.name.clone(),
                    transform,
                    width_px: *width_px,
                    height_px: *height_px,
                    raw_rgba: raw_rgba.clone(),
                    mask: *mask,
                    adjustments: adjustments.clone(),
                    placement,
                    border_color: *border_color,
                    border_width: *border_width,
                    opacity,
                });
            }
            _ => {
                items.push(SceneItem::FallbackElement(el.clone()));
            }
        }
    }

    RenderScene {
        page_id: page.id,
        page_title: page.title.clone(),
        width,
        height,
        background: page.background.clone(),
        items,
    }
}

/// Reduces editable tables and drawing paths to shared text/shape primitives.
pub fn expand_page_elements(page: &CanvasPage) -> Vec<CanvasElement> {
    let mut out = Vec::new();
    for el in &page.elements {
        match &el.kind {
            ElementKind::Table {
                rows,
                cols,
                cells,
                header_bg,
                row_bg,
                alt_row_bg,
                border_color,
                text_color,
                font_size,
            } if *rows > 0 && *cols > 0 && rows.saturating_mul(*cols) <= 10000 => {
                let tf = LocalToPageTransform::from_element(el);
                let cw = el.width / *cols as f32;
                let ch = el.height / *rows as f32;
                for r in 0..*rows {
                    for c in 0..*cols {
                        let bg = if r == 0 {
                            *header_bg
                        } else if r % 2 == 1 {
                            *row_bg
                        } else {
                            *alt_row_bg
                        };
                        let mut cell = el.with_kind(ElementKind::Shape {
                            shape_type: ShapeType::Rectangle,
                            fill: FillStyle::Solid(bg),
                            stroke_color: *border_color,
                            stroke_width: 0.7,
                            corner_radius: 0.,
                        });
                        let (cx, cy) =
                            tf.local_to_page((c as f32 + 0.5) * cw, (r as f32 + 0.5) * ch);
                        cell.x = cx - cw / 2.;
                        cell.y = cy - ch / 2.;
                        cell.width = cw;
                        cell.height = ch;
                        out.push(cell);
                        let pad = 4.0_f32.min(cw * 0.1).min(ch * 0.1);
                        let mut text = el.with_kind(ElementKind::Text {
                            content: cells
                                .get(r)
                                .and_then(|row| row.get(c))
                                .cloned()
                                .unwrap_or_default(),
                            font_family: crate::model::FontFamily::Sans,
                            font_size: *font_size,
                            color: *text_color,
                            bold: r == 0,
                            italic: false,
                            underline: false,
                            uppercase: false,
                            align: crate::model::TextAlign::Left,
                            letter_spacing: 0.,
                            line_height: 1.2,
                            effect: crate::model::TextEffect::None,
                            list_style: crate::model::ListStyle::None,
                        });
                        text.x = cx - cw / 2. + pad;
                        text.y = cy - ch / 2. + pad;
                        text.width = (cw - 2. * pad).max(1.);
                        text.height = (ch - 2. * pad).max(1.);
                        out.push(text);
                    }
                }
            }
            ElementKind::FreehandPath {
                points,
                color,
                stroke_width,
                ..
            } => {
                let tf = LocalToPageTransform::from_element(el);
                for pair in points.windows(2) {
                    let a = tf.local_to_page(pair[0].0 * el.width, pair[0].1 * el.height);
                    let b = tf.local_to_page(pair[1].0 * el.width, pair[1].1 * el.height);
                    let len = (b.0 - a.0).hypot(b.1 - a.1);
                    let h = stroke_width.max(0.1);
                    let mut line = el.with_kind(ElementKind::Shape {
                        shape_type: ShapeType::LineHorizontal,
                        fill: FillStyle::Solid(*color),
                        stroke_color: *color,
                        stroke_width: h,
                        corner_radius: 0.,
                    });
                    line.x = (a.0 + b.0) / 2. - len / 2.;
                    line.y = (a.1 + b.1) / 2. - h / 2.;
                    line.width = len;
                    line.height = h;
                    line.rotation = (b.1 - a.1).atan2(b.0 - a.0).to_degrees();
                    line.flip_h = false;
                    line.flip_v = false;
                    out.push(line);
                }
            }
            _ => out.push(el.clone()),
        }
    }
    out
}
