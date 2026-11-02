//! Seeded word searches with exhaustive target-word validation and editable output.
use crate::model::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Difficulty {
    #[default]
    Easy,
    Medium,
    Hard,
}
impl Difficulty {
    pub fn directions(self) -> &'static [(i32, i32)] {
        match self {
            Self::Easy => &[(1, 0), (0, 1)],
            Self::Medium => &[(1, 0), (0, 1), (1, 1), (-1, 1)],
            Self::Hard => ALL,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Easy => "Easy: across/down",
            Self::Medium => "Medium: add diagonals",
            Self::Hard => "Hard: all eight directions",
        }
    }
    pub fn instructions(self) -> &'static str {
        match self {
            Self::Easy => "Find the words from left to right or top to bottom.",
            Self::Medium => "Find the words across, down or diagonally downward.",
            Self::Hard => "Find the words in any direction, including backwards.",
        }
    }
}
const ALL: &[(i32, i32)] = &[
    (1, 0),
    (0, 1),
    (1, 1),
    (-1, 1),
    (-1, 0),
    (0, -1),
    (-1, -1),
    (1, -1),
];
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WordSearchConfig {
    pub title: String,
    pub words: Vec<String>,
    pub size: usize,
    pub difficulty: Difficulty,
    pub seed: u64,
}
impl Default for WordSearchConfig {
    fn default() -> Self {
        Self {
            title: "In the garden".into(),
            words: [
                "BLOSSOM", "GARDEN", "PETAL", "ROSE", "SEED", "SOIL", "STEM", "SUNSHINE", "WATER",
                "LEAF",
            ]
            .into_iter()
            .map(str::to_string)
            .collect(),
            size: 12,
            difficulty: Difficulty::Easy,
            seed: 2026,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PuzzleStyle {
    pub family: FontFamily,
    pub ink: ColorRgba,
    pub accent: ColorRgba,
    pub title_pt: f32,
    pub letter_pt: f32,
    pub word_pt: f32,
    pub grid_lines: bool,
}
impl Default for PuzzleStyle {
    fn default() -> Self {
        Self {
            family: FontFamily::Sans,
            ink: rgb(30, 40, 48),
            accent: rgb(35, 99, 86),
            title_pt: 25.,
            letter_pt: 18.,
            word_pt: 12.,
            grid_lines: false,
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Placement {
    pub word: String,
    pub x: usize,
    pub y: usize,
    pub dx: i32,
    pub dy: i32,
}
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WordSearchProduct {
    pub config: WordSearchConfig,
    pub style: PuzzleStyle,
    pub grid: Vec<String>,
    pub placements: Vec<Placement>,
    pub puzzle_page: u64,
    pub answer_page: Option<u64>,
    pub generated_ids: Vec<u64>,
    pub signature: String,
}
struct Random(u64);
impl Random {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9e3779b97f4a7c15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
        z ^ (z >> 31)
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % (n as u64)) as usize
    }
}

pub fn normalise(word: &str) -> Result<String, String> {
    let mut out = String::new();
    for c in word.trim().chars() {
        if c.is_ascii_alphabetic() {
            out.push(c.to_ascii_uppercase());
        } else if !c.is_whitespace() && !['-', '\'', '’'].contains(&c) {
            return Err(format!(
                "'{word}': use English A–Z letters, spaces, hyphens or apostrophes"
            ));
        }
    }
    if out.len() < 2 {
        return Err("Each word needs at least two letters".into());
    }
    Ok(out)
}
fn words(config: &WordSearchConfig) -> Result<Vec<String>, String> {
    if !(8..=24).contains(&config.size)
        || config.words.is_empty()
        || config.words.len() > 40
        || config.title.trim().is_empty()
        || config.title.chars().count() > 100
    {
        return Err("Use an 8–24 grid, 1–40 words and a title of 1–100 characters".into());
    }
    let mut seen = HashSet::new();
    let mut result = Vec::new();
    for word in &config.words {
        let word = normalise(word)?;
        if word.len() > config.size {
            return Err(format!("'{word}' is longer than the grid"));
        }
        let reverse: String = word.chars().rev().collect();
        if seen.contains(&word) || seen.contains(&reverse) {
            return Err(format!("Duplicate or reversed duplicate word: {word}"));
        }
        seen.insert(word.clone());
        result.push(word);
    }
    Ok(result)
}
fn cells(p: &Placement) -> Vec<(usize, usize)> {
    (0..p.word.len())
        .map(|i| {
            (
                (p.x as i32 + i as i32 * p.dx) as usize,
                (p.y as i32 + i as i32 * p.dy) as usize,
            )
        })
        .collect()
}
fn fits(p: &Placement, n: usize) -> bool {
    let x = p.x as i32 + (p.word.len() as i32 - 1) * p.dx;
    let y = p.y as i32 + (p.word.len() as i32 - 1) * p.dy;
    p.x < n && p.y < n && x >= 0 && y >= 0 && x < n as i32 && y < n as i32
}
/// Independent exhaustive scan, across all eight directions, including accidental duplicates.
pub fn occurrences(grid: &[String], word: &str) -> Vec<Vec<(usize, usize)>> {
    let n = grid.len();
    let mut found = HashSet::new();
    for y in 0..n {
        for x in 0..n {
            for &(dx, dy) in ALL {
                let p = Placement {
                    word: word.into(),
                    x,
                    y,
                    dx,
                    dy,
                };
                if !fits(&p, n) {
                    continue;
                }
                let positions = cells(&p);
                if positions
                    .iter()
                    .zip(word.bytes())
                    .all(|(&(cx, cy), c)| grid[cy].as_bytes().get(cx) == Some(&c))
                {
                    let mut reverse = positions.clone();
                    reverse.reverse();
                    found.insert(if reverse < positions {
                        reverse
                    } else {
                        positions
                    });
                }
            }
        }
    }
    let mut result: Vec<_> = found.into_iter().collect();
    result.sort();
    result
}
pub fn validate(
    config: &WordSearchConfig,
    grid: &[String],
    placements: &[Placement],
) -> Result<(), String> {
    let expected = words(config)?;
    let n = config.size;
    if grid.len() != n
        || grid
            .iter()
            .any(|r| r.len() != n || !r.bytes().all(|b| b.is_ascii_uppercase()))
        || placements.len() != expected.len()
    {
        return Err("Invalid grid dimensions, letters or placement count".into());
    }
    let mut seen = HashSet::new();
    for p in placements {
        if !expected.contains(&p.word)
            || !seen.insert(&p.word)
            || !config.difficulty.directions().contains(&(p.dx, p.dy))
            || !fits(p, n)
        {
            return Err("Invalid or duplicate solution placement".into());
        }
        let found = occurrences(grid, &p.word);
        let positions = cells(p);
        if found.len() != 1
            || !positions
                .iter()
                .zip(p.word.bytes())
                .all(|(&(x, y), c)| grid[y].as_bytes()[x] == c)
        {
            return Err(format!(
                "'{}' has {} occurrences; exactly one is required",
                p.word,
                found.len()
            ));
        }
    }
    Ok(())
}
pub fn generate(config: &WordSearchConfig) -> Result<(Vec<String>, Vec<Placement>), String> {
    let mut targets = words(config)?;
    targets.sort_by_key(|w| std::cmp::Reverse(w.len()));
    let n = config.size;
    let mut random = Random(config.seed);
    for _ in 0..80 {
        let mut grid = vec![b' '; n * n];
        let mut placements = Vec::new();
        for word in &targets {
            let mut choices = Vec::new();
            for y in 0..n {
                for x in 0..n {
                    for &(dx, dy) in config.difficulty.directions() {
                        let p = Placement {
                            word: word.clone(),
                            x,
                            y,
                            dx,
                            dy,
                        };
                        if fits(&p, n)
                            && cells(&p).iter().zip(word.bytes()).all(|(&(cx, cy), c)| {
                                grid[cy * n + cx] == b' ' || grid[cy * n + cx] == c
                            })
                        {
                            choices.push(p);
                        }
                    }
                }
            }
            if choices.is_empty() {
                break;
            }
            let p = choices.swap_remove(random.below(choices.len()));
            for ((x, y), c) in cells(&p).into_iter().zip(word.bytes()) {
                grid[y * n + x] = c;
            }
            placements.push(p);
        }
        if placements.len() != targets.len() {
            continue;
        }
        for _ in 0..48 {
            let filled: Vec<u8> = grid
                .iter()
                .map(|&c| {
                    if c == b' ' {
                        b'A' + random.below(26) as u8
                    } else {
                        c
                    }
                })
                .collect();
            let rows: Vec<_> = filled
                .chunks(n)
                .map(|r| String::from_utf8(r.to_vec()).unwrap())
                .collect();
            if validate(config, &rows, &placements).is_ok() {
                return Ok((rows, placements));
            }
        }
    }
    Err("Could not make an unambiguous grid. Increase the grid size, change the seed or use fewer/longer words.".into())
}
fn base(id: u64, name: &str, rect: egui::Rect, kind: ElementKind) -> CanvasElement {
    CanvasElement {
        id,
        name: name.into(),
        x: rect.min.x,
        y: rect.min.y,
        width: rect.width(),
        height: rect.height(),
        rotation: 0.,
        opacity: 1.,
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
        kind,
    }
}
fn text(
    project: &mut Project,
    name: &str,
    rect: egui::Rect,
    content: String,
    family: FontFamily,
    size: f32,
    color: ColorRgba,
    bold: bool,
    align: TextAlign,
) -> CanvasElement {
    base(
        project.alloc_id(),
        name,
        rect,
        ElementKind::Text {
            content,
            font_family: family,
            font_size: size,
            color,
            bold,
            italic: false,
            underline: false,
            uppercase: false,
            align,
            letter_spacing: 0.,
            line_height: 1.,
            effect: TextEffect::None,
            list_style: ListStyle::None,
        },
    )
}
fn rectangle(
    project: &mut Project,
    name: &str,
    rect: egui::Rect,
    color: ColorRgba,
) -> CanvasElement {
    base(
        project.alloc_id(),
        name,
        rect,
        ElementKind::Shape {
            shape_type: ShapeType::Rectangle,
            fill: FillStyle::Solid(color),
            stroke_color: color,
            stroke_width: 0.,
            corner_radius: 0.,
        },
    )
}
fn rect(x: f32, y: f32, w: f32, h: f32) -> egui::Rect {
    egui::Rect::from_min_size(egui::pos2(x, y), egui::vec2(w, h))
}
fn build_page(
    project: &mut Project,
    product: &WordSearchProduct,
    index: usize,
    answer: bool,
) -> Result<CanvasPage, String> {
    let style = &product.style;
    let config = &product.config;
    let n = config.size;
    if [style.title_pt, style.letter_pt, style.word_pt]
        .iter()
        .any(|v| !v.is_finite() || *v < 10. || *v > 48.)
        || style.ink[3] != 255
        || style.accent[3] != 255
    {
        return Err("Puzzle sizes must be 10–48 pt and ink colours fully opaque".into());
    }
    let area = crate::book_production::content_rect(project, index)?;
    let unit = project.effective_units_per_inch() / 72.;
    let title = style.title_pt * unit;
    let body = style.word_pt * unit;
    let columns = if area.width() > 450. { 3 } else { 2 };
    let rows = config.words.len().div_ceil(columns);
    let list_h = rows as f32 * body * 1.65 + body * 2.;
    let top = area.min.y + title * 1.65 + body * 2.;
    let side = area.width().min(area.max.y - top - list_h);
    let cell = side / n as f32;
    let size = (style.letter_pt * unit).min(cell * 0.62);
    if size / unit < 10. || side < 100. {
        return Err("The puzzle is too dense for this page/margins. Use fewer words, a smaller grid or a larger page.".into());
    }
    let left = area.center().x - side / 2.;
    let id = if answer {
        product.answer_page.unwrap()
    } else {
        product.puzzle_page
    };
    let suffix = if answer { " · Answers" } else { "" };
    let mut page = CanvasPage::new(id, format!("{}{suffix}", config.title));
    page.notes=format!("Seed {}; {}. Recipe-backed generated layers are checked before export. Restyle from Puzzles to preserve validation.",config.seed,config.difficulty.label());
    page.elements.push(rectangle(
        project,
        "Puzzle accent rule",
        rect(area.min.x, area.min.y, 42., 3.),
        style.accent,
    ));
    page.elements.push(text(
        project,
        "Puzzle title",
        rect(area.min.x, area.min.y + 12., area.width(), title * 1.3),
        format!("{}{suffix}", config.title),
        style.family,
        title,
        style.accent,
        true,
        TextAlign::Left,
    ));
    page.elements.push(text(
        project,
        "Puzzle instructions",
        rect(
            area.min.x,
            area.min.y + title * 1.65,
            area.width(),
            body * 1.5,
        ),
        if answer {
            "Answer key · match each marked path to the word list.".into()
        } else {
            config.difficulty.instructions().into()
        },
        style.family,
        body,
        style.ink,
        false,
        TextAlign::Left,
    ));
    if answer {
        for p in &product.placements {
            let points = cells(p);
            let (sx, sy) = points[0];
            let (ex, ey) = *points.last().unwrap();
            let x1 = left + (sx as f32 + 0.5) * cell;
            let y1 = top + (sy as f32 + 0.5) * cell;
            let x2 = left + (ex as f32 + 0.5) * cell;
            let y2 = top + (ey as f32 + 0.5) * cell;
            let length = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt() + cell * 0.8;
            let h = cell * 0.78;
            let tint = [
                style.accent[0] / 4 + 190,
                style.accent[1] / 4 + 190,
                style.accent[2] / 4 + 190,
                255,
            ];
            let mut marker = rectangle(
                project,
                "Answer path",
                rect(
                    (x1 + x2) / 2. - length / 2.,
                    (y1 + y2) / 2. - h / 2.,
                    length,
                    h,
                ),
                tint,
            );
            marker.rotation = (y2 - y1).atan2(x2 - x1).to_degrees();
            page.elements.push(marker);
        }
    }
    if style.grid_lines {
        for i in 0..=n {
            page.elements.push(rectangle(
                project,
                "Grid rule",
                rect(left + i as f32 * cell, top, 0.45, side),
                rgb(210, 216, 219),
            ));
            page.elements.push(rectangle(
                project,
                "Grid rule",
                rect(left, top + i as f32 * cell, side, 0.45),
                rgb(210, 216, 219),
            ));
        }
    }
    for (y, row) in product.grid.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            page.elements.push(text(
                project,
                "Puzzle letter",
                rect(
                    left + x as f32 * cell,
                    top + y as f32 * cell + (cell - size) / 2. - size * 0.08,
                    cell,
                    size * 1.2,
                ),
                c.to_string(),
                style.family,
                size,
                style.ink,
                false,
                TextAlign::Center,
            ));
        }
    }
    let mut labels = config.words.clone();
    labels.sort_by_key(|w| w.to_ascii_uppercase());
    let list_top = top + side + body;
    let column_width = area.width() / columns as f32;
    for (i, word) in labels.iter().enumerate() {
        page.elements.push(text(
            project,
            "Word list",
            rect(
                area.min.x + (i % columns) as f32 * column_width,
                list_top + (i / columns) as f32 * body * 1.65,
                column_width - 8.,
                body * 1.45,
            ),
            word.trim().to_uppercase(),
            style.family,
            body,
            style.ink,
            false,
            TextAlign::Left,
        ));
    }
    // Reject overflow rather than silently shrinking titles or words.
    for element in &page.elements {
        if let ElementKind::Text {
            content,
            font_family,
            font_size,
            bold,
            italic,
            uppercase,
            align,
            letter_spacing,
            line_height,
            list_style,
            ..
        } = &element.kind
        {
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
                element.width,
                element.height,
                &element.text_format,
            );
            if layout.is_overflowing() {
                return Err(format!(
                    "'{}' does not fit; shorten the text or reduce its size",
                    element.name
                ));
            }
        }
    }
    Ok(page)
}
pub(crate) fn signature(project: &Project, product: &WordSearchProduct) -> Result<String, String> {
    let mut content = Vec::new();
    let mut seen = HashSet::new();
    for id in std::iter::once(product.puzzle_page).chain(product.answer_page) {
        let page = project.pages.iter().find(|p| p.id == id).ok_or(
            "A puzzle/answer page is missing; remove its recipe if you deleted it intentionally",
        )?;
        for e in &page.elements {
            if product.generated_ids.contains(&e.id) {
                if !seen.insert(e.id) {
                    return Err("Duplicate generated puzzle layer".into());
                }
                content.push(e);
            }
        }
    }
    if seen.len() != product.generated_ids.len() {
        return Err(
            "Generated puzzle layers are missing; rebuild from Puzzles or detach the recipe".into(),
        );
    }
    let bytes = serde_json::to_vec(&content).map_err(|e| e.to_string())?;
    Ok(crate::model::asset_store::AssetStore::compute_hash(&bytes))
}
pub fn validate_project(project: &Project) -> Result<(), String> {
    if project.production.puzzles.len() > 1000 {
        return Err("Too many puzzle recipes".into());
    }
    for product in &project.production.puzzles {
        validate(&product.config, &product.grid, &product.placements)?;
        if signature(project, product)? != product.signature {
            return Err(format!("Generated layers for '{}' changed. Rebuild/restyle in Puzzles, or detach its recipe to edit manually.",product.config.title));
        }
    }
    Ok(())
}
pub fn append_series(
    project: &mut Project,
    config: &WordSearchConfig,
    style: &PuzzleStyle,
    count: usize,
    answers: bool,
) -> Result<usize, String> {
    if !(1..=50).contains(&count) {
        return Err("Make 1–50 puzzles per job".into());
    }
    words(config)?;
    let mut candidate = project.clone();
    candidate.production.book.enabled = true;
    if candidate.pages.len() == 1
        && candidate.pages[0].elements.is_empty()
        && candidate.pages[0].title == "Page 1"
    {
        candidate.pages.clear();
    }
    let first = candidate.pages.len();
    let mut products = Vec::new();
    for i in 0..count {
        let mut config = config.clone();
        config.seed = config.seed.wrapping_add(i as u64);
        if count > 1 {
            config.title = format!("{} · {}", config.title, i + 1);
        }
        let (grid, placements) = generate(&config)?;
        products.push(WordSearchProduct {
            config,
            style: style.clone(),
            grid,
            placements,
            puzzle_page: candidate.alloc_id(),
            answer_page: answers.then(|| candidate.alloc_id()),
            generated_ids: Vec::new(),
            signature: String::new(),
        });
    }
    for answer in [false, true] {
        if answer && !answers {
            continue;
        }
        for product in &mut products {
            let index = candidate.pages.len();
            let page = build_page(&mut candidate, product, index, answer)?;
            product
                .generated_ids
                .extend(page.elements.iter().map(|e| e.id));
            candidate.pages.push(page);
        }
    }
    for product in &mut products {
        product.signature = signature(&candidate, product)?;
    }
    candidate.production.puzzles.extend(products);
    validate_project(&candidate)?;
    *project = candidate;
    Ok(first)
}
pub fn restyle(project: &mut Project, index: usize, style: &PuzzleStyle) -> Result<(), String> {
    let mut candidate = project.clone();
    let mut product = candidate
        .production
        .puzzles
        .get(index)
        .ok_or("Recipe not found")?
        .clone();
    validate(&product.config, &product.grid, &product.placements)?;
    let old = product.generated_ids.clone();
    product.style = style.clone();
    product.generated_ids.clear();
    for answer in [false, true] {
        let Some(id) = (if answer {
            product.answer_page
        } else {
            Some(product.puzzle_page)
        }) else {
            continue;
        };
        let index = candidate
            .pages
            .iter()
            .position(|p| p.id == id)
            .ok_or("Puzzle page was deleted")?;
        let mut page = build_page(&mut candidate, &product, index, answer)?;
        product
            .generated_ids
            .extend(page.elements.iter().map(|e| e.id));
        page.elements.extend(
            candidate.pages[index]
                .elements
                .iter()
                .filter(|e| !old.contains(&e.id))
                .cloned(),
        );
        candidate.pages[index].elements = page.elements;
    }
    product.signature = signature(&candidate, &product)?;
    candidate.production.puzzles[index] = product;
    validate_project(&candidate)?;
    *project = candidate;
    Ok(())
}
