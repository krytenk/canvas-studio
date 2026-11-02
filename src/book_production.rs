use crate::model::*;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct BookSettings {
    pub enabled: bool,
    pub facing_pages: bool,
    pub inner_mm: f32,
    pub outer_mm: f32,
    pub top_mm: f32,
    pub bottom_mm: f32,
    pub first_number: u32,
    pub start_after_pages: usize,
    pub folios: bool,
    pub header: String,
    pub footer: String,
    pub type_size_pt: f32,
    pub master_elements: Vec<CanvasElement>,
    pub blank_backs: bool,
    pub artwork_fit: ImageFit,
}
impl Default for BookSettings {
    fn default() -> Self {
        Self {
            enabled: false,
            facing_pages: true,
            inner_mm: 15.,
            outer_mm: 12.,
            top_mm: 15.,
            bottom_mm: 15.,
            first_number: 1,
            start_after_pages: 0,
            folios: true,
            header: String::new(),
            footer: String::new(),
            type_size_pt: 10.,
            master_elements: Vec::new(),
            blank_backs: false,
            artwork_fit: ImageFit::Contain,
        }
    }
}

pub fn page_number(project: &Project, index: usize) -> Result<Option<u32>, String> {
    let book = &project.production.book;
    if index < book.start_after_pages {
        return Ok(None);
    }
    let offset = u32::try_from(index - book.start_after_pages)
        .map_err(|_| "Page number exceeds supported range")?;
    Ok(Some(
        book.first_number
            .checked_add(offset)
            .ok_or("Page number overflow")?,
    ))
}

pub fn content_rect(project: &Project, index: usize) -> Result<egui::Rect, String> {
    let book = &project.production.book;
    let values = [
        book.inner_mm,
        book.outer_mm,
        book.top_mm,
        book.bottom_mm,
        book.type_size_pt,
    ];
    if values.iter().any(|v| !v.is_finite() || *v < 0.) || book.type_size_pt < 3. {
        return Err("Book margins/type size must be valid non-negative values".into());
    }
    if book.inner_mm + book.outer_mm >= project.effective_width_mm()
        || book.top_mm + book.bottom_mm >= project.effective_height_mm()
    {
        return Err("Book margins leave no content area".into());
    }
    let left = if book.facing_pages && index % 2 == 1 {
        book.outer_mm
    } else {
        book.inner_mm
    };
    let right = if book.facing_pages && index % 2 == 1 {
        book.inner_mm
    } else {
        book.outer_mm
    };
    let sx = project.width / project.effective_width_mm();
    let sy = project.height / project.effective_height_mm();
    Ok(egui::Rect::from_min_max(
        egui::pos2(left * sx, book.top_mm * sy),
        egui::pos2(
            project.width - right * sx,
            project.height - book.bottom_mm * sy,
        ),
    ))
}

fn text(
    id: u64,
    name: &str,
    value: String,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    size: f32,
    align: TextAlign,
) -> CanvasElement {
    CanvasElement {
        id,
        name: name.into(),
        x,
        y,
        width: w,
        height: h,
        rotation: 0.,
        opacity: 1.,
        locked: true,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: Default::default(),
        animation: ElementAnimation::None,
        group_id: None,
        text_format: Default::default(),
        style_id: None,
        binding: None,
        kind: ElementKind::Text {
            content: value,
            font_family: FontFamily::Sans,
            font_size: size,
            color: rgb(70, 76, 86),
            bold: false,
            italic: false,
            underline: false,
            uppercase: false,
            align,
            letter_spacing: 0.,
            line_height: 1.2,
            effect: TextEffect::None,
            list_style: ListStyle::None,
        },
    }
}

pub fn compose_page(project: &Project, index: usize) -> Result<CanvasPage, String> {
    let mut page = project
        .pages
        .get(index)
        .ok_or("Page is outside document")?
        .clone();
    let book = &project.production.book;
    if !book.enabled || !page.master_enabled {
        return Ok(page);
    }
    let rect = content_rect(project, index)?;
    let max = project
        .pages
        .iter()
        .flat_map(|p| &p.elements)
        .map(|e| e.id)
        .max()
        .unwrap_or(0)
        .max(project.next_id);
    let range = (book.master_elements.len() * 2 + 4) as u64;
    let offset = (index as u64)
        .checked_mul(range)
        .ok_or("Master layer ID overflow")?;
    max.checked_add(offset)
        .and_then(|v| v.checked_add(range))
        .ok_or("Master layer ID overflow")?;
    let mut id = max + 1 + offset;
    let mut next = || {
        let value = id;
        id += 1;
        value
    };
    let mut groups = HashMap::new();
    let mut master = Vec::new();
    for source in &book.master_elements {
        let mut copy = source.clone();
        copy.id = next();
        copy.locked = true;
        copy.group_id = source
            .group_id
            .map(|g| *groups.entry(g).or_insert_with(&mut next));
        master.push(copy);
    }
    master.extend(page.elements);
    page.elements = master;
    let size = book.type_size_pt * project.effective_units_per_inch() / 72.;
    let h = size * 1.5;
    if !book.header.is_empty() {
        page.elements.push(text(
            next(),
            "Master header",
            book.header.clone(),
            rect.min.x,
            (rect.min.y - h) / 2.,
            rect.width(),
            h,
            size,
            TextAlign::Left,
        ));
    }
    let footer_y = rect.max.y + (project.height - rect.max.y - h) / 2.;
    if !book.footer.is_empty() {
        page.elements.push(text(
            next(),
            "Master footer",
            book.footer.clone(),
            rect.min.x
                + if book.facing_pages
                    && index % 2 == 1
                    && book.folios
                    && !book.footer.contains("{{page}}")
                {
                    45.
                } else {
                    0.
                },
            footer_y,
            (rect.width()
                - if book.folios && !book.footer.contains("{{page}}") {
                    45.
                } else {
                    0.
                })
            .max(1.),
            h,
            size,
            TextAlign::Left,
        ));
    }
    if book.folios && !book.footer.contains("{{page}}") && page_number(project, index)?.is_some() {
        let (x, align) = if book.facing_pages && index % 2 == 1 {
            (rect.min.x, TextAlign::Left)
        } else {
            (rect.max.x - 40., TextAlign::Right)
        };
        page.elements.push(text(
            next(),
            "Automatic page number",
            "{{page}}".into(),
            x,
            footer_y,
            40.,
            h,
            size,
            align,
        ));
    }
    Ok(page)
}

pub fn append_artwork(project: &mut Project, paths: &[PathBuf]) -> Result<usize, String> {
    if paths.is_empty() {
        return Err("Choose at least one artwork image".into());
    }
    if paths.len() > 500 {
        return Err("Import at most 500 artwork images in one job".into());
    }
    let mut loaded = Vec::new();
    for path in paths {
        if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 25_000_000 {
            return Err(format!("Artwork '{}' exceeds 25 MB", path.display()));
        }
        let reader = image::ImageReader::open(path)
            .map_err(|e| e.to_string())?
            .with_guessed_format()
            .map_err(|e| e.to_string())?;
        let (w, h) = reader.into_dimensions().map_err(|e| e.to_string())?;
        if w > 18_000 || h > 18_000 || w as u64 * h as u64 > 100_000_000 {
            return Err("Artwork image exceeds safe dimensions".into());
        }
        loaded.push((
            path.clone(),
            crate::model::asset_store::AssetStore::global().import_image_file(path)?,
        ));
    }
    let mut candidate = project.clone();
    candidate.production.book.enabled = true;
    if candidate.pages.len() == 1
        && candidate.pages[0].elements.is_empty()
        && candidate.pages[0].title == "Page 1"
    {
        candidate.pages.clear();
    }
    let first = candidate.pages.len();
    for (path, asset) in loaded {
        let index = candidate.pages.len();
        let rect = content_rect(&candidate, index)?;
        let mut page = CanvasPage::new(
            candidate.alloc_id(),
            path.file_stem().unwrap_or_default().to_string_lossy(),
        );
        let stock = crate::templates::SampleStockAsset {
            name: "Artwork",
            subtitle: "Imported artwork",
            width: asset.width_px,
            height: asset.height_px,
            rgba: asset.decoded_rgba.into(),
        };
        let mut element = crate::templates::make_stock_image_element(
            candidate.alloc_id(),
            &stock,
            rect.min.x,
            rect.min.y,
        );
        element.name = format!(
            "Artwork: {}",
            path.file_name().unwrap_or_default().to_string_lossy()
        );
        element.width = rect.width();
        element.height = rect.height();
        if let ElementKind::Image {
            mask,
            adjustments,
            source_name,
            ..
        } = &mut element.kind
        {
            *mask = ImageMask::None;
            adjustments.fit = candidate.production.book.artwork_fit;
            *source_name = path
                .file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .into_owned();
        }
        page.elements.push(element);
        candidate.pages.push(page);
        if candidate.production.book.blank_backs {
            let mut blank = CanvasPage::new(candidate.alloc_id(), "Blank back");
            blank.master_enabled = false;
            candidate.pages.push(blank);
        }
    }
    let count = candidate.pages.len() - first;
    *project = candidate;
    Ok(count)
}

pub fn refit_artwork(project: &mut Project) -> Result<(), String> {
    let rects = (0..project.pages.len())
        .map(|i| content_rect(project, i))
        .collect::<Result<Vec<_>, _>>()?;
    for (page, rect) in project.pages.iter_mut().zip(rects) {
        for element in &mut page.elements {
            if element.name.starts_with("Artwork:")
                && matches!(element.kind, ElementKind::Image { .. })
            {
                element.x = rect.min.x;
                element.y = rect.min.y;
                element.width = rect.width();
                element.height = rect.height();
            }
        }
    }
    Ok(())
}
