use serde::{Deserialize, Serialize};

#[allow(dead_code)]
#[path = "asset_store.rs"]
pub mod asset_store;
#[allow(dead_code)]
#[path = "font_registry.rs"]
pub mod font_registry;
#[allow(dead_code)]
#[path = "import_layout.rs"]
pub mod import_layout;
#[allow(dead_code)]
#[path = "pdf_export.rs"]
pub mod pdf_export;
#[allow(dead_code)]
#[path = "render_scene.rs"]
pub mod render_scene;
#[allow(dead_code)]
#[path = "text_layout.rs"]
pub mod text_layout;

pub type ColorRgba = [u8; 4];

pub fn rgba(r: u8, g: u8, b: u8, a: u8) -> ColorRgba {
    [r, g, b, a]
}

pub fn rgb(r: u8, g: u8, b: u8) -> ColorRgba {
    [r, g, b, 255]
}

pub fn to_egui_color(c: ColorRgba) -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], c[3])
}

pub fn with_opacity(c: ColorRgba, opacity: f32) -> egui::Color32 {
    let a = ((c[3] as f32) * opacity.clamp(0.0, 1.0)).round() as u8;
    egui::Color32::from_rgba_unmultiplied(c[0], c[1], c[2], a)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DesignFormat {
    Presentation,     // 1920 x 1080
    InstagramPost,    // 1080 x 1080
    InstagramStory,   // 1080 x 1920
    YouTubeThumbnail, // 1280 x 720
    Poster,           // 1587 x 2245
    DocumentA4,       // 210 x 297 mm (793.7008 x 1122.5197 @ 96 dpi)
    DocumentLetter,   // 8.5 x 11 in / 215.9 x 279.4 mm (816 x 1056 @ 96 dpi)
    Stationery5x7,    // 5 x 7 in / 127.0 x 177.8 mm (480 x 672 @ 96 dpi)
    Logo,             // 500 x 500
    SocialBanner,     // 1600 x 400
    Infographic,      // 800 x 2000
    Whiteboard,       // 2400 x 1600
    Custom(u32, u32),
}

impl DesignFormat {
    pub fn dimensions(&self) -> (f32, f32) {
        match self {
            DesignFormat::Presentation => (1920.0, 1080.0),
            DesignFormat::InstagramPost => (1080.0, 1080.0),
            DesignFormat::InstagramStory => (1080.0, 1920.0),
            DesignFormat::YouTubeThumbnail => (1280.0, 720.0),
            DesignFormat::Poster => (1587.0, 2245.0),
            DesignFormat::DocumentA4 => (793.7008, 1122.5197),
            DesignFormat::DocumentLetter => (816.0, 1056.0),
            DesignFormat::Stationery5x7 => (480.0, 672.0),
            DesignFormat::Logo => (500.0, 500.0),
            DesignFormat::SocialBanner => (1600.0, 400.0),
            DesignFormat::Infographic => (800.0, 2000.0),
            DesignFormat::Whiteboard => (2400.0, 1600.0),
            DesignFormat::Custom(w, h) => ((*w).max(64) as f32, (*h).max(64) as f32),
        }
    }

    pub fn physical_dimensions_mm(&self) -> (f32, f32) {
        match self {
            DesignFormat::DocumentA4 => (210.0, 297.0),
            DesignFormat::DocumentLetter => (215.9, 279.4),
            DesignFormat::Stationery5x7 => (127.0, 177.8),
            DesignFormat::Poster => (420.0, 594.0),
            _ => {
                let (w, h) = self.dimensions();
                ((w / 96.0) * 25.4, (h / 96.0) * 25.4)
            }
        }
    }

    #[allow(dead_code)]
    pub fn pixel_dimensions_at_dpi(&self, dpi: u32) -> (u32, u32) {
        let (w_mm, h_mm) = self.physical_dimensions_mm();
        let d = dpi.max(36) as f32;
        let px_w = ((w_mm / 25.4) * d).round().max(1.0) as u32;
        let px_h = ((h_mm / 25.4) * d).round().max(1.0) as u32;
        (px_w, px_h)
    }

    pub fn label(&self) -> &'static str {
        match self {
            DesignFormat::Presentation => "Presentation (16:9)",
            DesignFormat::InstagramPost => "Instagram Post (Square)",
            DesignFormat::InstagramStory => "Story / Reel (9:16)",
            DesignFormat::YouTubeThumbnail => "YouTube Thumbnail",
            DesignFormat::Poster => "Poster (Print A2)",
            DesignFormat::DocumentA4 => "A4 Print (210 × 297 mm)",
            DesignFormat::DocumentLetter => "US Letter Print (8.5 × 11 in)",
            DesignFormat::Stationery5x7 => "5×7 in Stationery (127 × 177.8 mm)",
            DesignFormat::Logo => "Logo (Square)",
            DesignFormat::SocialBanner => "Social Banner",
            DesignFormat::Infographic => "Infographic",
            DesignFormat::Whiteboard => "Whiteboard",
            DesignFormat::Custom(_, _) => "Custom Size",
        }
    }

    pub fn all_presets() -> Vec<DesignFormat> {
        vec![
            DesignFormat::DocumentA4,
            DesignFormat::DocumentLetter,
            DesignFormat::Stationery5x7,
            DesignFormat::Presentation,
            DesignFormat::InstagramPost,
            DesignFormat::InstagramStory,
            DesignFormat::YouTubeThumbnail,
            DesignFormat::Poster,
            DesignFormat::Logo,
            DesignFormat::SocialBanner,
            DesignFormat::Infographic,
            DesignFormat::Whiteboard,
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum PageBackground {
    Solid(ColorRgba),
    LinearGradient {
        start: ColorRgba,
        end: ColorRgba,
        angle_deg: f32,
    },
    RadialGradient {
        center: ColorRgba,
        outer: ColorRgba,
    },
    GridPattern {
        bg: ColorRgba,
        line: ColorRgba,
        spacing: f32,
    },
}

impl Default for PageBackground {
    fn default() -> Self {
        PageBackground::Solid(rgb(255, 255, 255))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum PageTransition {
    None,
    Dissolve,
    Slide,
    Pan,
    CircleWipe,
    MatchAndMove,
}

impl PageTransition {
    pub fn all() -> &'static [PageTransition] {
        &[
            PageTransition::None,
            PageTransition::Dissolve,
            PageTransition::Slide,
            PageTransition::Pan,
            PageTransition::CircleWipe,
            PageTransition::MatchAndMove,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            PageTransition::None => "None",
            PageTransition::Dissolve => "Dissolve",
            PageTransition::Slide => "Slide",
            PageTransition::Pan => "Pan",
            PageTransition::CircleWipe => "Circle Wipe",
            PageTransition::MatchAndMove => "Match & Move",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ElementAnimation {
    None,
    Rise,
    Pan,
    Fade,
    Pop,
    Wipe,
    Breathe,
    Tectonic,
}

impl ElementAnimation {
    pub fn all() -> &'static [ElementAnimation] {
        &[
            ElementAnimation::None,
            ElementAnimation::Rise,
            ElementAnimation::Pan,
            ElementAnimation::Fade,
            ElementAnimation::Pop,
            ElementAnimation::Wipe,
            ElementAnimation::Breathe,
            ElementAnimation::Tectonic,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ElementAnimation::None => "None",
            ElementAnimation::Rise => "Rise",
            ElementAnimation::Pan => "Pan",
            ElementAnimation::Fade => "Fade",
            ElementAnimation::Pop => "Pop",
            ElementAnimation::Wipe => "Wipe",
            ElementAnimation::Breathe => "Breathe",
            ElementAnimation::Tectonic => "Tectonic",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct DropShadow {
    pub enabled: bool,
    pub offset_x: f32,
    pub offset_y: f32,
    pub blur: f32,
    pub color: ColorRgba,
}

impl Default for DropShadow {
    fn default() -> Self {
        Self {
            enabled: false,
            offset_x: 6.0,
            offset_y: 8.0,
            blur: 14.0,
            color: rgba(0, 0, 0, 110),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ShapeType {
    Rectangle,
    RoundedRect,
    Circle,
    Triangle,
    RightTriangle,
    Diamond,
    Pentagon,
    Hexagon,
    Octagon,
    Star5,
    Star8,
    Burst12,
    Heart,
    ArrowRight,
    ArrowLeft,
    SpeechBubble,
    BadgeShield,
    Cross,
    Pill,
    LineHorizontal,
    Parallelogram,
    Trapezoid,
    BannerRibbon,
}

impl ShapeType {
    pub fn all() -> &'static [ShapeType] {
        &[
            ShapeType::Rectangle,
            ShapeType::RoundedRect,
            ShapeType::Circle,
            ShapeType::Triangle,
            ShapeType::RightTriangle,
            ShapeType::Diamond,
            ShapeType::Pentagon,
            ShapeType::Hexagon,
            ShapeType::Octagon,
            ShapeType::Star5,
            ShapeType::Star8,
            ShapeType::Burst12,
            ShapeType::Heart,
            ShapeType::ArrowRight,
            ShapeType::ArrowLeft,
            ShapeType::SpeechBubble,
            ShapeType::BadgeShield,
            ShapeType::Cross,
            ShapeType::Pill,
            ShapeType::LineHorizontal,
            ShapeType::Parallelogram,
            ShapeType::Trapezoid,
            ShapeType::BannerRibbon,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ShapeType::Rectangle => "Rectangle",
            ShapeType::RoundedRect => "Rounded Rect",
            ShapeType::Circle => "Circle / Oval",
            ShapeType::Triangle => "Triangle",
            ShapeType::RightTriangle => "Right Triangle",
            ShapeType::Diamond => "Diamond",
            ShapeType::Pentagon => "Pentagon",
            ShapeType::Hexagon => "Hexagon",
            ShapeType::Octagon => "Octagon",
            ShapeType::Star5 => "5-Point Star",
            ShapeType::Star8 => "8-Point Star",
            ShapeType::Burst12 => "12-Point Burst",
            ShapeType::Heart => "Heart",
            ShapeType::ArrowRight => "Arrow Right",
            ShapeType::ArrowLeft => "Arrow Left",
            ShapeType::SpeechBubble => "Speech Bubble",
            ShapeType::BadgeShield => "Shield Badge",
            ShapeType::Cross => "Cross / Plus",
            ShapeType::Pill => "Capsule Pill",
            ShapeType::LineHorizontal => "Divider Line",
            ShapeType::Parallelogram => "Parallelogram",
            ShapeType::Trapezoid => "Trapezoid",
            ShapeType::BannerRibbon => "Ribbon Tag",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum FillStyle {
    Solid(ColorRgba),
    LinearGradient {
        start: ColorRgba,
        end: ColorRgba,
        vertical: bool,
    },
}

impl FillStyle {
    pub fn primary_color(&self) -> ColorRgba {
        match self {
            FillStyle::Solid(c) => *c,
            FillStyle::LinearGradient { start, .. } => *start,
        }
    }

    pub fn set_primary_color(&mut self, new_c: ColorRgba) {
        match self {
            FillStyle::Solid(c) => *c = new_c,
            FillStyle::LinearGradient { start, .. } => *start = new_c,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FontFamily {
    #[serde(
        alias = "CanvaSans",
        alias = "NotoSans",
        alias = "Montserrat",
        alias = "Inter",
        alias = "Roboto"
    )]
    Sans,
    #[serde(alias = "NotoSerif", alias = "PlayfairDisplay")]
    DisplaySerif,
    #[serde(alias = "NotoSansMono", alias = "SpaceMono")]
    MonoCode,
    #[serde(alias = "NotoSansBold", alias = "Oswald")]
    PosterHeavy,
    #[serde(alias = "NotoSerifItalic", alias = "Pacifico")]
    EditorialItalic,
    Imported(u64),
}

impl FontFamily {
    pub fn all() -> &'static [FontFamily] {
        &[
            FontFamily::Sans,
            FontFamily::DisplaySerif,
            FontFamily::MonoCode,
            FontFamily::PosterHeavy,
            FontFamily::EditorialItalic,
        ]
    }

    pub fn label(&self) -> String {
        match self {
            FontFamily::Imported(id) => return format!("Imported {:016x}", id),
            FontFamily::Sans => "Noto Sans",
            FontFamily::DisplaySerif => "Noto Serif",
            FontFamily::MonoCode => "Noto Sans Mono",
            FontFamily::PosterHeavy => "Noto Sans Bold",
            FontFamily::EditorialItalic => "Noto Serif Italic",
        }
        .to_string()
    }

    #[allow(dead_code)]
    pub fn css_stack(&self) -> &'static str {
        match self {
            FontFamily::Imported(_) => "sans-serif",
            FontFamily::Sans => "'Noto Sans', 'Inter', 'Helvetica Neue', Arial, sans-serif",
            FontFamily::DisplaySerif => "'Noto Serif', 'Playfair Display', Georgia, serif",
            FontFamily::MonoCode => "'Noto Sans Mono', 'JetBrains Mono', monospace",
            FontFamily::PosterHeavy => "'Noto Sans', 'Oswald', Impact, sans-serif",
            FontFamily::EditorialItalic => "'Noto Serif', Georgia, serif",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ListStyle {
    None,
    Bullet,
    Numbered,
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum TextEffect {
    None,
    DropShadow {
        offset: f32,
        color: ColorRgba,
    },
    Lift {
        intensity: f32,
    },
    HollowOutline {
        thickness: f32,
        color: ColorRgba,
    },
    NeonGlow {
        glow_color: ColorRgba,
        intensity: f32,
    },
    Splice {
        offset: f32,
        fill_color: ColorRgba,
    },
    Echo {
        color: ColorRgba,
    },
    BackgroundPill {
        bg_color: ColorRgba,
        radius: f32,
        padding: f32,
    },
}

impl TextEffect {
    pub fn label(&self) -> &'static str {
        match self {
            TextEffect::None => "None",
            TextEffect::DropShadow { .. } => "Shadow",
            TextEffect::Lift { .. } => "Lift",
            TextEffect::HollowOutline { .. } => "Hollow",
            TextEffect::NeonGlow { .. } => "Neon Glow",
            TextEffect::Splice { .. } => "Splice",
            TextEffect::Echo { .. } => "Echo",
            TextEffect::BackgroundPill { .. } => "Background",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub enum ImageMask {
    None,
    Rounded(f32),
    Circle,
    Diamond,
    Hexagon,
    Pill,
}

impl ImageMask {
    pub fn all() -> &'static [ImageMask] {
        &[
            ImageMask::None,
            ImageMask::Rounded(24.0),
            ImageMask::Circle,
            ImageMask::Diamond,
            ImageMask::Hexagon,
            ImageMask::Pill,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ImageMask::None => "Full Frame",
            ImageMask::Rounded(_) => "Rounded Card",
            ImageMask::Circle => "Circle Frame",
            ImageMask::Diamond => "Diamond Frame",
            ImageMask::Hexagon => "Hexagon Frame",
            ImageMask::Pill => "Capsule Frame",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImageFit {
    Contain,
    Cover,
    Stretch,
}

impl Default for ImageFit {
    fn default() -> Self {
        ImageFit::Cover
    }
}

impl ImageFit {
    pub fn all() -> &'static [ImageFit] {
        &[ImageFit::Cover, ImageFit::Contain, ImageFit::Stretch]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ImageFit::Contain => "Contain (Letterbox)",
            ImageFit::Cover => "Cover (Fill Frame)",
            ImageFit::Stretch => "Stretch (Exact Box)",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ImageCrop {
    #[serde(default)]
    pub left: f32,
    #[serde(default)]
    pub top: f32,
    #[serde(default)]
    pub right: f32,
    #[serde(default)]
    pub bottom: f32,
}

impl ImageCrop {
    pub fn sanitized(self) -> Self {
        let left = self.left.clamp(0.0, 0.90);
        let top = self.top.clamp(0.0, 0.90);
        let right = self.right.clamp(0.0, (0.95 - left).max(0.0));
        let bottom = self.bottom.clamp(0.0, (0.95 - top).max(0.0));
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    #[allow(dead_code)]
    pub fn is_trivial(&self) -> bool {
        self.left.abs() < 1e-4
            && self.top.abs() < 1e-4
            && self.right.abs() < 1e-4
            && self.bottom.abs() < 1e-4
    }
}

fn default_focal_point() -> (f32, f32) {
    (0.5, 0.5)
}

#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
pub struct ImageAdjustments {
    pub brightness: f32, // -1.0 .. 1.0
    pub contrast: f32,   // -1.0 .. 1.0
    pub saturation: f32, // -1.0 .. 1.0
    pub warmth: f32,     // -1.0 .. 1.0
    pub blur: f32,       // 0.0 .. 10.0
    pub sharpen: f32,    // 0.0 .. 2.0
    pub grayscale: bool,
    pub sepia: bool,
    pub invert: bool,
    pub posterize: bool,
    pub duotone: Option<(ColorRgba, ColorRgba)>,
    pub bg_removed: bool,
    pub bg_tolerance: f32, // 0.05 .. 0.60
    #[serde(default)]
    pub fit: ImageFit,
    #[serde(default)]
    pub crop: ImageCrop,
    #[serde(default = "default_focal_point")]
    pub focal_point: (f32, f32),
}

impl Default for ImageAdjustments {
    fn default() -> Self {
        Self {
            brightness: 0.0,
            contrast: 0.0,
            saturation: 0.0,
            warmth: 0.0,
            blur: 0.0,
            sharpen: 0.0,
            grayscale: false,
            sepia: false,
            invert: false,
            posterize: false,
            duotone: None,
            bg_removed: false,
            bg_tolerance: 0.22,
            fit: ImageFit::Cover,
            crop: ImageCrop::default(),
            focal_point: (0.5, 0.5),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BrushType {
    Pen,
    Marker,
    Highlighter,
    NeonGlow,
    Dashed,
}

impl BrushType {
    pub fn label(&self) -> &'static str {
        match self {
            BrushType::Pen => "Fine Pen",
            BrushType::Marker => "Chisel Marker",
            BrushType::Highlighter => "Highlighter",
            BrushType::NeonGlow => "Neon Brush",
            BrushType::Dashed => "Dashed Sketch",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ChartType {
    VerticalBar,
    HorizontalBar,
    Pie,
    Donut,
    LineArea,
}

impl ChartType {
    pub fn all() -> &'static [ChartType] {
        &[
            ChartType::VerticalBar,
            ChartType::HorizontalBar,
            ChartType::Pie,
            ChartType::Donut,
            ChartType::LineArea,
        ]
    }

    pub fn label(&self) -> &'static str {
        match self {
            ChartType::VerticalBar => "Vertical Bar",
            ChartType::HorizontalBar => "Horizontal Bar",
            ChartType::Pie => "Pie Chart",
            ChartType::Donut => "Donut Ring",
            ChartType::LineArea => "Line & Area",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum ElementKind {
    Shape {
        shape_type: ShapeType,
        fill: FillStyle,
        stroke_color: ColorRgba,
        stroke_width: f32,
        corner_radius: f32,
    },
    Text {
        content: String,
        font_family: FontFamily,
        font_size: f32,
        color: ColorRgba,
        bold: bool,
        italic: bool,
        underline: bool,
        uppercase: bool,
        align: TextAlign,
        letter_spacing: f32,
        line_height: f32,
        effect: TextEffect,
        list_style: ListStyle,
    },
    Image {
        source_name: String,
        width_px: u32,
        height_px: u32,
        #[serde(with = "crate::model::asset_store::serde_rgba_asset")]
        raw_rgba: Vec<u8>,
        mask: ImageMask,
        #[serde(default)]
        adjustments: ImageAdjustments,
        border_color: ColorRgba,
        border_width: f32,
        #[serde(skip)]
        revision: u64,
    },
    FreehandPath {
        points: Vec<(f32, f32)>, // Normalized 0.0..1.0 within bounding box
        color: ColorRgba,
        stroke_width: f32,
        brush: BrushType,
    },
    Chart {
        chart_type: ChartType,
        title: String,
        labels: Vec<String>,
        values: Vec<f32>,
        palette: Vec<ColorRgba>,
        show_values: bool,
        show_grid: bool,
        text_color: ColorRgba,
    },
    Table {
        rows: usize,
        cols: usize,
        cells: Vec<Vec<String>>,
        header_bg: ColorRgba,
        row_bg: ColorRgba,
        alt_row_bg: ColorRgba,
        border_color: ColorRgba,
        text_color: ColorRgba,
        font_size: f32,
    },
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextRun {
    pub start: usize,
    pub end: usize,
    pub bold: Option<bool>,
    pub italic: Option<bool>,
    pub underline: Option<bool>,
    pub font_family: Option<FontFamily>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct TextFormatting {
    pub space_before: f32,
    pub space_after: f32,
    pub first_line_indent: f32,
    pub inset_left: f32,
    pub inset_right: f32,
    pub inset_top: f32,
    pub inset_bottom: f32,
    pub runs: Vec<TextRun>,
}

impl TextFormatting {
    pub fn validate(&self, content: &str) -> Result<(), String> {
        let values = [
            self.space_before,
            self.space_after,
            self.first_line_indent,
            self.inset_left,
            self.inset_right,
            self.inset_top,
            self.inset_bottom,
        ];
        if values
            .iter()
            .any(|v| !v.is_finite() || *v < 0. || *v > 100_000.)
        {
            return Err("Paragraph spacing and insets must be finite non-negative values".into());
        }
        let count = content.chars().count();
        if self.runs.iter().any(|r| r.start >= r.end || r.end > count) {
            return Err("A character style range is outside its text".into());
        }
        Ok(())
    }

    pub fn apply_run(&mut self, mut run: TextRun, count: usize) {
        run.start = run.start.min(count);
        run.end = run.end.min(count);
        if run.start < run.end {
            self.runs.push(run);
        }
    }

    pub fn reconcile_edit(&mut self, old: &str, new: &str) {
        let a: Vec<_> = old.chars().collect();
        let b: Vec<_> = new.chars().collect();
        let prefix = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        let suffix = a[prefix..]
            .iter()
            .rev()
            .zip(b[prefix..].iter().rev())
            .take_while(|(x, y)| x == y)
            .count();
        let end = a.len() - suffix;
        let inserted = b.len() - prefix - suffix;
        let shift = |v: usize| {
            ((v as isize) + (inserted as isize) - (end - prefix) as isize).max(0) as usize
        };
        for run in &mut self.runs {
            if run.end <= prefix {
                continue;
            }
            if run.start >= end {
                run.start = shift(run.start);
                run.end = shift(run.end);
            } else {
                if run.start >= prefix {
                    run.start = prefix;
                }
                run.end = if run.end >= end {
                    shift(run.end)
                } else {
                    prefix + inserted
                };
            }
            run.start = run.start.min(b.len());
            run.end = run.end.min(b.len());
        }
        self.runs.retain(|r| r.start < r.end);
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasElement {
    pub id: u64,
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub rotation: f32, // Degrees
    pub opacity: f32,  // 0.0 .. 1.0
    pub locked: bool,
    pub visible: bool,
    pub flip_h: bool,
    pub flip_v: bool,
    pub shadow: DropShadow,
    pub animation: ElementAnimation,
    pub group_id: Option<u64>,
    #[serde(default)]
    pub text_format: TextFormatting,
    pub kind: ElementKind,
}

impl CanvasElement {
    pub fn with_kind(&self, kind: ElementKind) -> Self {
        Self {
            id: self.id,
            name: self.name.clone(),
            x: self.x,
            y: self.y,
            width: self.width,
            height: self.height,
            rotation: self.rotation,
            opacity: self.opacity,
            locked: self.locked,
            visible: self.visible,
            flip_h: self.flip_h,
            flip_v: self.flip_v,
            shadow: self.shadow,
            animation: self.animation,
            group_id: self.group_id,
            text_format: self.text_format.clone(),
            kind,
        }
    }

    pub fn center(&self) -> (f32, f32) {
        (self.x + self.width * 0.5, self.y + self.height * 0.5)
    }

    pub fn rect(&self) -> egui::Rect {
        egui::Rect::from_min_size(
            egui::pos2(self.x, self.y),
            egui::vec2(self.width.max(4.0), self.height.max(4.0)),
        )
    }

    /// Hit-tests a canvas-coordinate point `(px, py)` against this element, accounting for rotation.
    pub fn contains_point(&self, px: f32, py: f32) -> bool {
        if !self.visible {
            return false;
        }
        let (cx, cy) = self.center();
        let rad = -self.rotation.to_radians();
        let dx = px - cx;
        let dy = py - cy;
        let lx = dx * rad.cos() - dy * rad.sin() + cx;
        let ly = dx * rad.sin() + dy * rad.cos() + cy;
        let pad = 4.0;
        lx >= self.x - pad
            && lx <= self.x + self.width + pad
            && ly >= self.y - pad
            && ly <= self.y + self.height + pad
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CanvasPage {
    pub id: u64,
    pub title: String,
    pub notes: String,
    pub background: PageBackground,
    pub transition: PageTransition,
    pub elements: Vec<CanvasElement>,
}

impl CanvasPage {
    pub fn new(id: u64, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            notes: String::new(),
            background: PageBackground::Solid(rgb(255, 255, 255)),
            transition: PageTransition::None,
            elements: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrandPalette {
    pub name: String,
    pub colors: Vec<ColorRgba>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BrandKit {
    pub brand_name: String,
    pub primary_font: FontFamily,
    pub secondary_font: FontFamily,
    pub palettes: Vec<BrandPalette>,
    pub active_palette_idx: usize,
}

impl Default for BrandKit {
    fn default() -> Self {
        Self {
            brand_name: "Canvas Studio Brand".to_string(),
            primary_font: FontFamily::Sans,
            secondary_font: FontFamily::DisplaySerif,
            active_palette_idx: 0,
            palettes: vec![
                BrandPalette {
                    name: "Studio Electric Gradient".to_string(),
                    colors: vec![
                        rgb(0, 196, 204),   // Studio Teal #00C4CC
                        rgb(125, 42, 232),  // Studio Purple #7D2AE8
                        rgb(20, 24, 36),    // Deep Ink
                        rgb(255, 92, 138),  // Coral Pink
                        rgb(245, 247, 252), // Soft Cloud
                    ],
                },
                BrandPalette {
                    name: "Midnight Cyber Neon".to_string(),
                    colors: vec![
                        rgb(13, 17, 28),
                        rgb(0, 245, 212),
                        rgb(155, 93, 229),
                        rgb(241, 91, 181),
                        rgb(254, 228, 64),
                    ],
                },
                BrandPalette {
                    name: "Warm Terracotta Editorial".to_string(),
                    colors: vec![
                        rgb(250, 243, 235),
                        rgb(212, 106, 78),
                        rgb(43, 58, 52),
                        rgb(233, 184, 114),
                        rgb(122, 145, 131),
                    ],
                },
                BrandPalette {
                    name: "Executive Fintech Blue".to_string(),
                    colors: vec![
                        rgb(15, 23, 42),
                        rgb(37, 99, 235),
                        rgb(56, 189, 248),
                        rgb(16, 185, 129),
                        rgb(248, 250, 252),
                    ],
                },
                BrandPalette {
                    name: "Sunset Pop Studio".to_string(),
                    colors: vec![
                        rgb(255, 89, 94),
                        rgb(255, 202, 58),
                        rgb(138, 201, 38),
                        rgb(25, 130, 196),
                        rgb(106, 76, 147),
                    ],
                },
            ],
        }
    }
}

fn default_schema_version() -> u32 {
    2
}

fn default_units_per_inch() -> f32 {
    96.0
}

pub fn generate_document_id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let seq = COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let pid = std::process::id();
    format!("doc-{pid:x}-{nanos:x}-{seq:x}")
}

pub const MIN_CUSTOM_DIMENSION_MM: f32 = 5.0;
pub const MAX_CUSTOM_DIMENSION_MM: f32 = 1270.0; // 50 inches
pub const MAX_EXPORT_PIXEL_DIM: u32 = 18_000;
pub const MAX_EXPORT_TOTAL_PIXELS: u64 = 100_000_000;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct OutputPreset {
    pub name: String,
    pub dpi: u32,
    pub bleed_mm: f32,
    pub crop_marks: bool,
    pub transparent: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct PrintSettings {
    pub safe_margin_mm: f32,
    pub show_guides: bool,
    pub guides_x_mm: Vec<f32>,
    pub guides_y_mm: Vec<f32>,
    pub presets: Vec<OutputPreset>,
}
impl Default for PrintSettings {
    fn default() -> Self {
        Self {
            safe_margin_mm: 5.,
            show_guides: true,
            guides_x_mm: Vec::new(),
            guides_y_mm: Vec::new(),
            presets: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    #[serde(default = "default_schema_version")]
    pub schema_version: u32,
    #[serde(default = "generate_document_id")]
    pub document_id: String,
    #[serde(default)]
    pub recovery_origin_path: Option<std::path::PathBuf>,
    pub title: String,
    pub format: DesignFormat,
    pub width: f32,
    pub height: f32,
    #[serde(default)]
    pub width_mm: f32,
    #[serde(default)]
    pub height_mm: f32,
    #[serde(default = "default_units_per_inch")]
    pub units_per_inch: f32,
    #[serde(default)]
    pub bleed_mm: f32,
    #[serde(default)]
    pub crop_marks: bool,
    pub pages: Vec<CanvasPage>,
    pub brand_kit: BrandKit,
    #[serde(default)]
    pub imported_fonts: Vec<crate::imported_fonts::ImportedFont>,
    #[serde(default)]
    pub print_settings: PrintSettings,
    pub next_id: u64,
}

impl Default for Project {
    fn default() -> Self {
        let format = DesignFormat::DocumentA4;
        let (width, height) = format.dimensions();
        let (width_mm, height_mm) = format.physical_dimensions_mm();
        Self {
            schema_version: 2,
            document_id: generate_document_id(),
            recovery_origin_path: None,
            title: "Untitled Design".to_string(),
            format,
            width,
            height,
            width_mm,
            height_mm,
            units_per_inch: 96.0,
            bleed_mm: 0.0,
            crop_marks: false,
            pages: vec![CanvasPage::new(1, "Page 1")],
            brand_kit: BrandKit::default(),
            imported_fonts: Vec::new(),
            print_settings: Default::default(),
            next_id: 100,
        }
    }
}

impl Project {
    pub fn alloc_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    pub fn effective_units_per_inch(&self) -> f32 {
        if self.units_per_inch > 0.0 {
            self.units_per_inch
        } else {
            96.0
        }
    }

    pub fn effective_width_mm(&self) -> f32 {
        if self.width_mm > 0.0 {
            self.width_mm
        } else {
            match self.format {
                DesignFormat::DocumentA4
                | DesignFormat::DocumentLetter
                | DesignFormat::Stationery5x7 => self.format.physical_dimensions_mm().0,
                _ => (self.width / self.effective_units_per_inch()) * 25.4,
            }
        }
    }

    pub fn effective_height_mm(&self) -> f32 {
        if self.height_mm > 0.0 {
            self.height_mm
        } else {
            match self.format {
                DesignFormat::DocumentA4
                | DesignFormat::DocumentLetter
                | DesignFormat::Stationery5x7 => self.format.physical_dimensions_mm().1,
                _ => (self.height / self.effective_units_per_inch()) * 25.4,
            }
        }
    }

    pub fn pixel_dimensions_for_dpi(&self, dpi: u32) -> (u32, u32) {
        let d = dpi.max(36) as f32;
        let w_px = ((self.effective_width_mm() / 25.4) * d).round().max(1.0) as u32;
        let h_px = ((self.effective_height_mm() / 25.4) * d).round().max(1.0) as u32;
        (w_px, h_px)
    }

    pub fn validate_export_dimensions(&self, dpi: u32) -> Result<(u32, u32), String> {
        if !self.width.is_finite()
            || !self.height.is_finite()
            || self.width <= 0.
            || self.height <= 0.
        {
            return Err("Logical page dimensions must be positive finite numbers".into());
        }
        let w_mm = self.effective_width_mm();
        let h_mm = self.effective_height_mm();
        if !w_mm.is_finite()
            || !h_mm.is_finite()
            || w_mm < MIN_CUSTOM_DIMENSION_MM
            || h_mm < MIN_CUSTOM_DIMENSION_MM
        {
            return Err(format!(
                "Invalid physical page dimensions ({w_mm:.2} x {h_mm:.2} mm): minimum supported dimension is {MIN_CUSTOM_DIMENSION_MM:.1} mm"
            ));
        }
        if w_mm > MAX_CUSTOM_DIMENSION_MM || h_mm > MAX_CUSTOM_DIMENSION_MM {
            return Err(format!(
                "Physical page dimensions ({w_mm:.2} x {h_mm:.2} mm) exceed maximum supported size ({MAX_CUSTOM_DIMENSION_MM:.1} mm / 50 in)"
            ));
        }
        let (w_px, h_px) = self.pixel_dimensions_for_dpi(dpi);
        let total = (w_px as u64).saturating_mul(h_px as u64);
        if w_px == 0
            || h_px == 0
            || w_px > MAX_EXPORT_PIXEL_DIM
            || h_px > MAX_EXPORT_PIXEL_DIM
            || total > MAX_EXPORT_TOTAL_PIXELS
        {
            return Err(format!(
                "Output raster dimensions {w_px} x {h_px} px at {dpi} DPI exceed safe allocation limits (max {MAX_EXPORT_PIXEL_DIM} px per side, {MAX_EXPORT_TOTAL_PIXELS} total pixels)"
            ));
        }
        Ok((w_px, h_px))
    }

    /// Converts `(w, h)` from `from_unit` (`0 = px`, `1 = mm`, `2 = in`) to `to_unit`,
    /// preserving physical size using `effective_units_per_inch()`.
    pub fn convert_dimension_units(
        &self,
        w: f32,
        h: f32,
        from_unit: u8,
        to_unit: u8,
    ) -> (f32, f32) {
        if from_unit == to_unit {
            return (w, h);
        }
        let upi = self.effective_units_per_inch();
        let (w_mm, h_mm) = match from_unit {
            1 => (w, h),
            2 => (w * 25.4, h * 25.4),
            _ => ((w / upi) * 25.4, (h / upi) * 25.4),
        };
        match to_unit {
            1 => (w_mm, h_mm),
            2 => (w_mm / 25.4, h_mm / 25.4),
            _ => ((w_mm / 25.4) * upi, (h_mm / 25.4) * upi),
        }
    }

    /// Validates and applies custom dimensions in `unit` (`0 = px`, `1 = mm`, `2 = in`),
    /// preserving exact fractional `width`, `height`, `width_mm`, and `height_mm` without silent clamping.
    pub fn apply_custom_dimensions(
        &mut self,
        w_input: f32,
        h_input: f32,
        unit: u8,
        scale_elements: bool,
    ) -> Result<(), String> {
        if !w_input.is_finite() || !h_input.is_finite() || w_input <= 0.0 || h_input <= 0.0 {
            return Err("Custom width and height must be positive finite numbers.".to_string());
        }
        let upi = self.effective_units_per_inch();
        let (new_w, new_h, w_mm, h_mm) = match unit {
            1 => (
                (w_input / 25.4) * upi,
                (h_input / 25.4) * upi,
                w_input,
                h_input,
            ),
            2 => (w_input * upi, h_input * upi, w_input * 25.4, h_input * 25.4),
            _ => (
                w_input,
                h_input,
                (w_input / upi) * 25.4,
                (h_input / upi) * 25.4,
            ),
        };

        if w_mm < MIN_CUSTOM_DIMENSION_MM || h_mm < MIN_CUSTOM_DIMENSION_MM {
            return Err(format!(
                "Custom size ({w_mm:.2} × {h_mm:.2} mm) is below the minimum supported size ({MIN_CUSTOM_DIMENSION_MM:.1} mm / {:.2} in).",
                MIN_CUSTOM_DIMENSION_MM / 25.4
            ));
        }
        if w_mm > MAX_CUSTOM_DIMENSION_MM || h_mm > MAX_CUSTOM_DIMENSION_MM {
            return Err(format!(
                "Custom size ({w_mm:.2} × {h_mm:.2} mm) exceeds the maximum supported size ({MAX_CUSTOM_DIMENSION_MM:.1} mm / {:.1} in).",
                MAX_CUSTOM_DIMENSION_MM / 25.4
            ));
        }

        let w_300 = ((w_mm / 25.4) * 300.0).round().max(1.0) as u32;
        let h_300 = ((h_mm / 25.4) * 300.0).round().max(1.0) as u32;
        let total_300 = (w_300 as u64).saturating_mul(h_300 as u64);
        if w_300 > MAX_EXPORT_PIXEL_DIM
            || h_300 > MAX_EXPORT_PIXEL_DIM
            || total_300 > MAX_EXPORT_TOTAL_PIXELS
        {
            return Err(format!(
                "Custom size would require {w_300} × {h_300} px at 300 DPI, exceeding safe limits."
            ));
        }

        if scale_elements && self.width > 0.0 && self.height > 0.0 {
            let sx = new_w / self.width;
            let sy = new_h / self.height;
            let s_font = sx.min(sy);
            for page in &mut self.pages {
                for el in &mut page.elements {
                    el.x *= sx;
                    el.y *= sy;
                    el.width = (el.width * sx).max(8.0);
                    el.height = (el.height * sy).max(8.0);
                    if let ElementKind::Text { font_size, .. } = &mut el.kind {
                        *font_size = (*font_size * s_font).clamp(8.0, 260.0);
                    }
                }
            }
        }

        self.format =
            DesignFormat::Custom(new_w.round().max(1.0) as u32, new_h.round().max(1.0) as u32);
        self.width = new_w;
        self.height = new_h;
        self.width_mm = w_mm;
        self.height_mm = h_mm;
        Ok(())
    }

    pub fn pdf_points_dimensions(&self) -> (f32, f32) {
        let w_pt = (self.effective_width_mm() / 25.4) * 72.0;
        let h_pt = (self.effective_height_mm() / 25.4) * 72.0;
        (w_pt, h_pt)
    }
}

const DEHYDRATED_MARKER_PREFIX: &[u8] = b"__canvas_asset__:";

/// Reuses immutable pixel registrations while comparing/snapshotting document metadata.
/// Image mutations must increment `revision`; replacing buffers also changes their address.
#[derive(Clone, Debug, Default)]
pub struct SnapshotCache {
    document_id: String,
    images: std::collections::HashMap<u64, (u64, usize, usize, u32, u32, String)>,
}

impl SnapshotCache {
    pub fn snapshot(&mut self, project: &Project) -> Project {
        if self.document_id != project.document_id {
            self.images.clear();
            self.document_id = project.document_id.clone();
        }
        let pages = project
            .pages
            .iter()
            .map(|page| CanvasPage {
                id: page.id,
                title: page.title.clone(),
                notes: page.notes.clone(),
                background: page.background.clone(),
                transition: page.transition,
                elements: page
                    .elements
                    .iter()
                    .map(|el| {
                        let kind = match &el.kind {
                            ElementKind::Image {
                                source_name,
                                width_px,
                                height_px,
                                raw_rgba,
                                mask,
                                adjustments,
                                border_color,
                                border_width,
                                revision,
                            } => {
                                let pixels = if raw_rgba.len() <= 64
                                    || raw_rgba.starts_with(DEHYDRATED_MARKER_PREFIX)
                                {
                                    raw_rgba.clone()
                                } else {
                                    let key = (
                                        *revision,
                                        raw_rgba.as_ptr() as usize,
                                        raw_rgba.len(),
                                        *width_px,
                                        *height_px,
                                    );
                                    let id = match self.images.get(&el.id) {
                                        Some(v) if (v.0, v.1, v.2, v.3, v.4) == key => v.5.clone(),
                                        _ => {
                                            let id =
                                                crate::model::asset_store::AssetStore::global()
                                                    .register_rgba(raw_rgba, *width_px, *height_px)
                                                    .asset_id;
                                            self.images.insert(
                                                el.id,
                                                (key.0, key.1, key.2, key.3, key.4, id.clone()),
                                            );
                                            id
                                        }
                                    };
                                    [DEHYDRATED_MARKER_PREFIX, id.as_bytes()].concat()
                                };
                                ElementKind::Image {
                                    source_name: source_name.clone(),
                                    width_px: *width_px,
                                    height_px: *height_px,
                                    raw_rgba: pixels,
                                    mask: *mask,
                                    adjustments: adjustments.clone(),
                                    border_color: *border_color,
                                    border_width: *border_width,
                                    revision: *revision,
                                }
                            }
                            other => other.clone(),
                        };
                        el.with_kind(kind)
                    })
                    .collect(),
            })
            .collect();
        Project {
            schema_version: project.schema_version,
            document_id: project.document_id.clone(),
            recovery_origin_path: project.recovery_origin_path.clone(),
            title: project.title.clone(),
            format: project.format,
            width: project.width,
            height: project.height,
            width_mm: project.width_mm,
            height_mm: project.height_mm,
            units_per_inch: project.units_per_inch,
            bleed_mm: project.bleed_mm,
            crop_marks: project.crop_marks,
            pages,
            brand_kit: project.brand_kit.clone(),
            imported_fonts: project.imported_fonts.clone(),
            print_settings: project.print_settings.clone(),
            next_id: project.next_id,
        }
    }
}

pub fn dehydrate_project_assets(project: &Project) -> Project {
    SnapshotCache::default().snapshot(project)
}

pub fn hydrate_project_assets(project: &mut Project) {
    let store = crate::model::asset_store::AssetStore::global();
    for page in &mut project.pages {
        for elem in &mut page.elements {
            if let ElementKind::Image { raw_rgba, .. } = &mut elem.kind {
                if raw_rgba.starts_with(DEHYDRATED_MARKER_PREFIX) {
                    if let Ok(asset_id) =
                        std::str::from_utf8(&raw_rgba[DEHYDRATED_MARKER_PREFIX.len()..])
                    {
                        if let Some(stored) = store.get(asset_id) {
                            *raw_rgba = (*stored.decoded_rgba).clone();
                        }
                    }
                }
            }
        }
    }
}

/// Undo / Redo snapshot history stack (stores asset-dehydrated snapshots and their revision numbers)
#[derive(Clone, Debug)]
pub struct HistoryManager {
    undo_stack: Vec<(Project, u64)>,
    redo_stack: Vec<(Project, u64)>,
    max_entries: usize,
    asset_cache: SnapshotCache,
}

impl Default for HistoryManager {
    fn default() -> Self {
        Self {
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            max_entries: 60,
            asset_cache: SnapshotCache::default(),
        }
    }
}

impl HistoryManager {
    pub fn record_with_revision(&mut self, snapshot: &Project, revision: u64) -> bool {
        let dehydrated = self.asset_cache.snapshot(snapshot);
        if self
            .undo_stack
            .last()
            .map(|(last_proj, _)| last_proj == &dehydrated)
            .unwrap_or(false)
        {
            return false;
        }
        self.undo_stack.push((dehydrated, revision));
        if self.undo_stack.len() > self.max_entries {
            self.undo_stack.remove(0);
        }
        self.redo_stack.clear();
        true
    }

    #[allow(dead_code)] // Backward-compatible convenience API; the editor uses revision-aware variants.
    pub fn record(&mut self, snapshot: &Project) {
        let next_rev = self.undo_stack.last().map(|(_, r)| r + 1).unwrap_or(0);
        let _ = self.record_with_revision(snapshot, next_rev);
    }

    pub fn undo_with_revision(
        &mut self,
        current: &mut Project,
        current_revision: &mut u64,
    ) -> bool {
        if let Some((mut prev, prev_rev)) = self.undo_stack.pop() {
            self.redo_stack
                .push((self.asset_cache.snapshot(current), *current_revision));
            hydrate_project_assets(&mut prev);
            *current = prev;
            *current_revision = prev_rev;
            true
        } else {
            false
        }
    }

    pub fn redo_with_revision(
        &mut self,
        current: &mut Project,
        current_revision: &mut u64,
    ) -> bool {
        if let Some((mut next, next_rev)) = self.redo_stack.pop() {
            self.undo_stack
                .push((self.asset_cache.snapshot(current), *current_revision));
            hydrate_project_assets(&mut next);
            *current = next;
            *current_revision = next_rev;
            true
        } else {
            false
        }
    }

    #[allow(dead_code)] // Backward-compatible convenience API; the editor uses revision-aware variants.
    pub fn undo(&mut self, current: &mut Project) -> bool {
        let mut dummy_rev = 0;
        self.undo_with_revision(current, &mut dummy_rev)
    }

    #[allow(dead_code)] // Backward-compatible convenience API; the editor uses revision-aware variants.
    pub fn redo(&mut self, current: &mut Project) -> bool {
        let mut dummy_rev = 0;
        self.redo_with_revision(current, &mut dummy_rev)
    }

    pub fn can_undo(&self) -> bool {
        !self.undo_stack.is_empty()
    }

    pub fn can_redo(&self) -> bool {
        !self.redo_stack.is_empty()
    }

    pub fn clear(&mut self) {
        self.undo_stack.clear();
        self.redo_stack.clear();
    }

    #[allow(dead_code)]
    pub fn approximate_heap_bytes(&self) -> usize {
        let sum_stack = |stack: &[(Project, u64)]| -> usize {
            stack
                .iter()
                .flat_map(|(p, _)| p.pages.iter())
                .flat_map(|pg| pg.elements.iter())
                .map(|e| match &e.kind {
                    ElementKind::Image { raw_rgba, .. } => raw_rgba.capacity(),
                    ElementKind::Text { content, .. } => content.capacity(),
                    _ => 64,
                })
                .sum()
        };
        sum_stack(&self.undo_stack) + sum_stack(&self.redo_stack)
    }
}

/// Smart snap guide line displayed in magenta on the canvas
#[derive(Clone, Copy, Debug)]
pub struct SnapGuide {
    pub vertical: bool,
    pub position: f32, // Canvas X (if vertical) or Canvas Y (if horizontal)
    pub label: &'static str,
}

/// Computes smart alignment snap guides and snapped `(x, y)` for a dragged element
pub fn compute_smart_snap(
    target_id: u64,
    proposed_x: f32,
    proposed_y: f32,
    width: f32,
    height: f32,
    canvas_w: f32,
    canvas_h: f32,
    others: &[CanvasElement],
    snap_threshold: f32,
) -> (f32, f32, Vec<SnapGuide>) {
    let mut snapped_x = proposed_x;
    let mut snapped_y = proposed_y;
    let mut guides = Vec::new();

    let cx = proposed_x + width * 0.5;
    let cy = proposed_y + height * 0.5;
    let right = proposed_x + width;
    let bottom = proposed_y + height;

    // 1. Snap to canvas center X / edges
    let canvas_cx = canvas_w * 0.5;
    let canvas_cy = canvas_h * 0.5;

    let mut x_snapped = false;
    if (cx - canvas_cx).abs() <= snap_threshold {
        snapped_x = canvas_cx - width * 0.5;
        guides.push(SnapGuide {
            vertical: true,
            position: canvas_cx,
            label: "Center X",
        });
        x_snapped = true;
    } else if proposed_x.abs() <= snap_threshold {
        snapped_x = 0.0;
        guides.push(SnapGuide {
            vertical: true,
            position: 0.0,
            label: "Left Edge",
        });
        x_snapped = true;
    } else if (right - canvas_w).abs() <= snap_threshold {
        snapped_x = canvas_w - width;
        guides.push(SnapGuide {
            vertical: true,
            position: canvas_w,
            label: "Right Edge",
        });
        x_snapped = true;
    }

    let mut y_snapped = false;
    if (cy - canvas_cy).abs() <= snap_threshold {
        snapped_y = canvas_cy - height * 0.5;
        guides.push(SnapGuide {
            vertical: false,
            position: canvas_cy,
            label: "Center Y",
        });
        y_snapped = true;
    } else if proposed_y.abs() <= snap_threshold {
        snapped_y = 0.0;
        guides.push(SnapGuide {
            vertical: false,
            position: 0.0,
            label: "Top Edge",
        });
        y_snapped = true;
    } else if (bottom - canvas_h).abs() <= snap_threshold {
        snapped_y = canvas_h - height;
        guides.push(SnapGuide {
            vertical: false,
            position: canvas_h,
            label: "Bottom Edge",
        });
        y_snapped = true;
    }

    // 2. Snap to other visible elements on the page
    for other in others {
        if other.id == target_id || !other.visible {
            continue;
        }
        let (ocx, ocy) = other.center();
        let oright = other.x + other.width;
        let obottom = other.y + other.height;

        if !x_snapped {
            if (proposed_x - other.x).abs() <= snap_threshold {
                snapped_x = other.x;
                guides.push(SnapGuide {
                    vertical: true,
                    position: other.x,
                    label: "Align Left",
                });
                x_snapped = true;
            } else if (cx - ocx).abs() <= snap_threshold {
                snapped_x = ocx - width * 0.5;
                guides.push(SnapGuide {
                    vertical: true,
                    position: ocx,
                    label: "Align Center",
                });
                x_snapped = true;
            } else if (right - oright).abs() <= snap_threshold {
                snapped_x = oright - width;
                guides.push(SnapGuide {
                    vertical: true,
                    position: oright,
                    label: "Align Right",
                });
                x_snapped = true;
            }
        }

        if !y_snapped {
            if (proposed_y - other.y).abs() <= snap_threshold {
                snapped_y = other.y;
                guides.push(SnapGuide {
                    vertical: false,
                    position: other.y,
                    label: "Align Top",
                });
                y_snapped = true;
            } else if (cy - ocy).abs() <= snap_threshold {
                snapped_y = ocy - height * 0.5;
                guides.push(SnapGuide {
                    vertical: false,
                    position: ocy,
                    label: "Align Middle",
                });
                y_snapped = true;
            } else if (bottom - obottom).abs() <= snap_threshold {
                snapped_y = obottom - height;
                guides.push(SnapGuide {
                    vertical: false,
                    position: obottom,
                    label: "Align Bottom",
                });
                y_snapped = true;
            }
        }
    }

    (snapped_x, snapped_y, guides)
}
