use crate::model::{
    rgb, rgba, CanvasElement, CanvasPage, ChartType, ColorRgba, DesignFormat, DropShadow,
    ElementAnimation, ElementKind, FillStyle, FontFamily, ImageAdjustments, ImageMask, ListStyle,
    PageBackground, PageTransition, Project, ShapeType, TextAlign, TextEffect,
};

pub struct TemplateDescriptor {
    pub id: &'static str,
    pub name: &'static str,
    pub category: &'static str,
    pub subtitle: &'static str,
    pub format: DesignFormat,
    pub accent: ColorRgba,
}

pub fn available_templates() -> Vec<TemplateDescriptor> {
    vec![
        TemplateDescriptor {
            id: "pitch_deck",
            name: "Visual Suite Keynote Deck",
            category: "Presentation",
            subtitle: "2-Slide Presentation with Chart & Metrics",
            format: DesignFormat::Presentation,
            accent: rgb(0, 196, 204),
        },
        TemplateDescriptor {
            id: "social_launch",
            name: "Neon Product Drop Post",
            category: "Social Media",
            subtitle: "Square 1080x1080 Instagram Launch Graphic",
            format: DesignFormat::InstagramPost,
            accent: rgb(125, 42, 232),
        },
        TemplateDescriptor {
            id: "yt_thumbnail",
            name: "Bold Creator Thumbnail",
            category: "YouTube",
            subtitle: "Bold 1280x720 Thumbnail with Badge & Burst",
            format: DesignFormat::YouTubeThumbnail,
            accent: rgb(255, 71, 87),
        },
        TemplateDescriptor {
            id: "festival_poster",
            name: "Modern Design Summit Poster",
            category: "Poster",
            subtitle: "Print-Ready Editorial Event Poster",
            format: DesignFormat::Poster,
            accent: rgb(255, 165, 2),
        },
        TemplateDescriptor {
            id: "story_reel",
            name: "Vertical Flash Sale Story",
            category: "Story / Reel",
            subtitle: "9:16 Mobile Story with Promo Code Pill",
            format: DesignFormat::InstagramStory,
            accent: rgb(255, 92, 138),
        },
        TemplateDescriptor {
            id: "infographic_report",
            name: "Annual Growth Infographic",
            category: "Infographic",
            subtitle: "Data-Driven Infographic with Charts & Table",
            format: DesignFormat::Infographic,
            accent: rgb(16, 185, 129),
        },
        TemplateDescriptor {
            id: "brand_logo",
            name: "Geometric Tech Emblem Logo",
            category: "Logo",
            subtitle: "500x500 Clean Vector-Style Brand Mark",
            format: DesignFormat::Logo,
            accent: rgb(56, 189, 248),
        },
    ]
}

fn make_shape(
    id: u64,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    shape_type: ShapeType,
    fill: FillStyle,
    corner_radius: f32,
) -> CanvasElement {
    CanvasElement {
        id,
        name: name.to_string(),
        x,
        y,
        width: w,
        height: h,
        rotation: 0.0,
        opacity: 1.0,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow {
            enabled: true,
            offset_x: 0.0,
            offset_y: 8.0,
            blur: 18.0,
            color: rgba(0, 0, 0, 75),
        },
        animation: ElementAnimation::Rise,
        group_id: None,
        kind: ElementKind::Shape {
            shape_type,
            fill,
            stroke_color: rgba(255, 255, 255, 45),
            stroke_width: 0.0,
            corner_radius,
        },
    }
}

#[allow(clippy::too_many_arguments)]
fn make_text(
    id: u64,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    content: &str,
    font_family: FontFamily,
    font_size: f32,
    color: ColorRgba,
    bold: bool,
    align: TextAlign,
    effect: TextEffect,
) -> CanvasElement {
    CanvasElement {
        id,
        name: name.to_string(),
        x,
        y,
        width: w,
        height: h,
        rotation: 0.0,
        opacity: 1.0,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow::default(),
        animation: ElementAnimation::Fade,
        group_id: None,
        kind: ElementKind::Text {
            content: content.to_string(),
            font_family,
            font_size,
            color,
            bold,
            italic: false,
            underline: false,
            uppercase: false,
            align,
            letter_spacing: 0.5,
            line_height: 1.2,
            effect,
            list_style: ListStyle::None,
        },
    }
}

pub fn build_template_project(template_id: &str) -> Project {
    let project = match template_id {
        "pitch_deck" => build_pitch_deck_template(),
        "social_launch" => build_social_launch_template(),
        "yt_thumbnail" => build_yt_thumbnail_template(),
        "festival_poster" => build_poster_template(),
        "story_reel" => build_story_template(),
        "infographic_report" => build_infographic_template(),
        "brand_logo" => build_logo_template(),
        _ => build_pitch_deck_template(),
    };
    production_defaults(project)
}

fn new_template_project(title: &str, format: DesignFormat) -> Project {
    let (width, height) = format.dimensions();
    let (width_mm, height_mm) = format.physical_dimensions_mm();
    Project {
        schema_version: 2,
        document_id: crate::model::generate_document_id(),
        recovery_origin_path: None,
        title: title.to_string(),
        format,
        width,
        height,
        width_mm,
        height_mm,
        units_per_inch: 96.0,
        bleed_mm: 0.0,
        crop_marks: false,
        pages: Vec::new(),
        brand_kit: Default::default(),
        next_id: 500,
    }
}

fn build_pitch_deck_template() -> Project {
    let mut proj = new_template_project("Visual Suite 2026 Keynote", DesignFormat::Presentation);

    // Slide 1: Cover
    let mut slide1 = CanvasPage {
        id: 1,
        title: "01 • Executive Cover".to_string(),
        notes: "Welcome the audience and highlight the unified visual studio architecture."
            .to_string(),
        background: PageBackground::LinearGradient {
            start: rgb(14, 19, 36),
            end: rgb(35, 18, 68),
            angle_deg: 135.0,
        },
        transition: PageTransition::Dissolve,
        elements: Vec::new(),
    };

    // Decorative glow orbs & glass card
    let mut orb1 = make_shape(
        10,
        "Teal Glow Sphere",
        1220.0,
        120.0,
        520.0,
        520.0,
        ShapeType::Circle,
        FillStyle::LinearGradient {
            start: rgba(0, 196, 204, 210),
            end: rgba(125, 42, 232, 180),
            vertical: true,
        },
        0.0,
    );
    orb1.animation = ElementAnimation::Breathe;
    slide1.elements.push(orb1);

    slide1.elements.push(make_shape(
        11,
        "Accent Star Badge",
        1540.0,
        520.0,
        220.0,
        220.0,
        ShapeType::Star8,
        FillStyle::Solid(rgb(255, 202, 58)),
        0.0,
    ));

    slide1.elements.push(make_shape(
        12,
        "Hero Feature Card",
        120.0,
        140.0,
        1040.0,
        800.0,
        ShapeType::RoundedRect,
        FillStyle::Solid(rgba(24, 30, 52, 225)),
        32.0,
    ));

    slide1.elements.push(make_text(
        13,
        "Category Pill",
        180.0,
        210.0,
        420.0,
        54.0,
        "CANVAS VISUAL SUITE • RUST EDITION",
        FontFamily::MonoCode,
        22.0,
        rgb(15, 23, 42),
        true,
        TextAlign::Center,
        TextEffect::BackgroundPill {
            bg_color: rgb(0, 245, 212),
            radius: 14.0,
            padding: 12.0,
        },
    ));

    slide1.elements.push(make_text(
        14,
        "Main Keynote Headline",
        180.0,
        310.0,
        920.0,
        240.0,
        "Design Anything.\nRun Locally in Rust.",
        FontFamily::PosterHeavy,
        84.0,
        rgb(255, 255, 255),
        true,
        TextAlign::Left,
        TextEffect::DropShadow {
            offset: 5.0,
            color: rgba(0, 196, 204, 140),
        },
    ));

    slide1.elements.push(make_text(
        15,
        "Keynote Subtitle",
        180.0,
        590.0,
        860.0,
        140.0,
        "Full vector shapes, multi-page presentations, Magic Studio image filters, smart snap guides, live charts, and instant high-res PNG/SVG export.",
        FontFamily::Sans,
        30.0,
        rgb(203, 213, 225),
        false,
        TextAlign::Left,
        TextEffect::None,
    ));

    slide1.elements.push(make_shape(
        16,
        "CTA Pill Button",
        180.0,
        770.0,
        340.0,
        82.0,
        ShapeType::Pill,
        FillStyle::LinearGradient {
            start: rgb(0, 196, 204),
            end: rgb(125, 42, 232),
            vertical: false,
        },
        41.0,
    ));

    slide1.elements.push(make_text(
        17,
        "CTA Button Text",
        200.0,
        792.0,
        300.0,
        45.0,
        "START CREATING ->",
        FontFamily::Sans,
        26.0,
        rgb(255, 255, 255),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    // Slide 2: Analytics & Live Chart
    let mut slide2 = CanvasPage {
        id: 2,
        title: "02 • Performance & Analytics".to_string(),
        notes: "Demonstrate native chart rendering and structured tables.".to_string(),
        background: PageBackground::Solid(rgb(248, 250, 252)),
        transition: PageTransition::Slide,
        elements: Vec::new(),
    };

    slide2.elements.push(make_text(
        20,
        "Slide 2 Heading",
        110.0,
        85.0,
        1100.0,
        90.0,
        "Quarterly Creative Velocity & Output",
        FontFamily::PosterHeavy,
        58.0,
        rgb(15, 23, 42),
        true,
        TextAlign::Left,
        TextEffect::None,
    ));

    slide2.elements.push(CanvasElement {
        id: 21,
        name: "Q1-Q4 Revenue Bar Chart".to_string(),
        x: 110.0,
        y: 220.0,
        width: 980.0,
        height: 720.0,
        rotation: 0.0,
        opacity: 1.0,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow {
            enabled: true,
            offset_x: 0.0,
            offset_y: 8.0,
            blur: 20.0,
            color: rgba(15, 23, 42, 30),
        },
        animation: ElementAnimation::Rise,
        group_id: None,
        kind: ElementKind::Chart {
            chart_type: ChartType::VerticalBar,
            title: "Design Assets Shipped (Thousands)".to_string(),
            labels: vec![
                "Q1".to_string(),
                "Q2".to_string(),
                "Q3".to_string(),
                "Q4".to_string(),
                "2026E".to_string(),
            ],
            values: vec![42.0, 68.0, 95.0, 134.0, 185.0],
            palette: vec![
                rgb(0, 196, 204),
                rgb(56, 189, 248),
                rgb(125, 42, 232),
                rgb(241, 91, 181),
                rgb(16, 185, 129),
            ],
            show_values: true,
            show_grid: true,
            text_color: rgb(30, 41, 59),
        },
    });

    slide2.elements.push(make_shape(
        22,
        "KPI Summary Card",
        1150.0,
        220.0,
        660.0,
        720.0,
        ShapeType::RoundedRect,
        FillStyle::Solid(rgb(15, 23, 42)),
        28.0,
    ));

    slide2.elements.push(make_text(
        23,
        "KPI Big Metric",
        1210.0,
        290.0,
        540.0,
        130.0,
        "+340%",
        FontFamily::PosterHeavy,
        112.0,
        rgb(0, 245, 212),
        true,
        TextAlign::Left,
        TextEffect::NeonGlow {
            glow_color: rgb(0, 196, 204),
            intensity: 1.0,
        },
    ));

    slide2.elements.push(make_text(
        24,
        "KPI Description",
        1210.0,
        450.0,
        540.0,
        380.0,
        "Native Rust execution eliminates browser memory overhead while offering a focused local design workflow:\n\n• Local Image Editing Tools\n• Smart Alignment Snap Guides\n• Multi-Channel Magic Resize\n• Vector SVG & High-DPI PNG Export",
        FontFamily::Sans,
        28.0,
        rgb(226, 232, 240),
        false,
        TextAlign::Left,
        TextEffect::None,
    ));

    proj.pages.push(slide1);
    proj.pages.push(slide2);
    proj
}

fn build_social_launch_template() -> Project {
    let mut proj = new_template_project(
        "Neon Product Drop Instagram Post",
        DesignFormat::InstagramPost,
    );

    let mut page = CanvasPage {
        id: 1,
        title: "Social Square Post".to_string(),
        notes: String::new(),
        background: PageBackground::LinearGradient {
            start: rgb(18, 12, 43),
            end: rgb(74, 20, 140),
            angle_deg: 120.0,
        },
        transition: PageTransition::None,
        elements: Vec::new(),
    };

    page.elements.push(make_shape(
        10,
        "Center Hexagon Frame",
        190.0,
        160.0,
        700.0,
        700.0,
        ShapeType::Hexagon,
        FillStyle::LinearGradient {
            start: rgb(0, 245, 212),
            end: rgb(241, 91, 181),
            vertical: true,
        },
        0.0,
    ));

    page.elements.push(make_shape(
        11,
        "Inner Dark Card",
        140.0,
        260.0,
        800.0,
        560.0,
        ShapeType::RoundedRect,
        FillStyle::Solid(rgba(13, 17, 28, 235)),
        36.0,
    ));

    page.elements.push(make_text(
        12,
        "Drop Tag",
        310.0,
        320.0,
        460.0,
        52.0,
        "LIMITED EDITION RELEASE",
        FontFamily::MonoCode,
        24.0,
        rgb(13, 17, 28),
        true,
        TextAlign::Center,
        TextEffect::BackgroundPill {
            bg_color: rgb(254, 228, 64),
            radius: 12.0,
            padding: 10.0,
        },
    ));

    page.elements.push(make_text(
        13,
        "Hero Neon Title",
        180.0,
        410.0,
        720.0,
        220.0,
        "FUTURE\nVISION 2.0",
        FontFamily::PosterHeavy,
        92.0,
        rgb(255, 255, 255),
        true,
        TextAlign::Center,
        TextEffect::NeonGlow {
            glow_color: rgb(0, 245, 212),
            intensity: 1.2,
        },
    ));

    page.elements.push(make_text(
        14,
        "Bottom Handle",
        240.0,
        690.0,
        600.0,
        60.0,
        "@CANVASSTUDIO • LINK IN BIO",
        FontFamily::Sans,
        28.0,
        rgb(0, 245, 212),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    proj.pages.push(page);
    proj
}

fn build_yt_thumbnail_template() -> Project {
    let mut proj =
        new_template_project("Creator YouTube Thumbnail", DesignFormat::YouTubeThumbnail);

    let mut page = CanvasPage {
        id: 1,
        title: "Thumbnail Main".to_string(),
        notes: String::new(),
        background: PageBackground::LinearGradient {
            start: rgb(15, 23, 42),
            end: rgb(30, 58, 138),
            angle_deg: 45.0,
        },
        transition: PageTransition::None,
        elements: Vec::new(),
    };

    page.elements.push(make_shape(
        10,
        "Diagonal Accent Banner",
        -40.0,
        420.0,
        1360.0,
        340.0,
        ShapeType::Parallelogram,
        FillStyle::Solid(rgb(255, 71, 87)),
        0.0,
    ));

    page.elements.push(make_shape(
        11,
        "Burst Sticker",
        920.0,
        60.0,
        280.0,
        280.0,
        ShapeType::Burst12,
        FillStyle::Solid(rgb(255, 211, 42)),
        0.0,
    ));

    page.elements.push(make_text(
        12,
        "Burst Price/Tag",
        950.0,
        165.0,
        220.0,
        80.0,
        "100%\nRUST!",
        FontFamily::PosterHeavy,
        42.0,
        rgb(15, 23, 42),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    page.elements.push(make_text(
        13,
        "Thumbnail Big Hook",
        70.0,
        90.0,
        820.0,
        290.0,
        "DESIGN YOUR\nNEXT BIG IDEA!",
        FontFamily::PosterHeavy,
        96.0,
        rgb(255, 255, 255),
        true,
        TextAlign::Left,
        TextEffect::Splice {
            offset: 6.0,
            fill_color: rgb(0, 196, 204),
        },
    ));

    page.elements.push(make_text(
        14,
        "Sub Hook Banner",
        80.0,
        490.0,
        1100.0,
        120.0,
        "ZERO CLOUD • INSTANT EXPORT • FULL MAGIC STUDIO",
        FontFamily::Sans,
        44.0,
        rgb(255, 255, 255),
        true,
        TextAlign::Left,
        TextEffect::DropShadow {
            offset: 4.0,
            color: rgba(0, 0, 0, 180),
        },
    ));

    proj.pages.push(page);
    proj
}

fn build_poster_template() -> Project {
    let mut proj = new_template_project("Design Summit Event Poster", DesignFormat::Poster);

    let mut page = CanvasPage {
        id: 1,
        title: "Event Poster".to_string(),
        notes: String::new(),
        background: PageBackground::Solid(rgb(250, 243, 235)),
        transition: PageTransition::None,
        elements: Vec::new(),
    };

    page.elements.push(make_shape(
        10,
        "Editorial Arch Card",
        140.0,
        140.0,
        1307.0,
        1965.0,
        ShapeType::RoundedRect,
        FillStyle::Solid(rgb(30, 39, 46)),
        64.0,
    ));

    page.elements.push(make_shape(
        11,
        "Warm Sun Circle",
        443.0,
        340.0,
        700.0,
        700.0,
        ShapeType::Circle,
        FillStyle::LinearGradient {
            start: rgb(255, 127, 80),
            end: rgb(255, 202, 58),
            vertical: true,
        },
        0.0,
    ));

    page.elements.push(make_text(
        12,
        "Poster Header",
        220.0,
        1140.0,
        1147.0,
        380.0,
        "INTERNATIONAL\nDESIGN SUMMIT",
        FontFamily::PosterHeavy,
        116.0,
        rgb(250, 243, 235),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    page.elements.push(make_text(
        13,
        "Poster Details",
        280.0,
        1620.0,
        1027.0,
        280.0,
        "NOVEMBER 2026 • SAN FRANCISCO & LOCAL STUDIO\nTYPOGRAPHY • VECTOR SYSTEMS • CREATIVE TOOLS",
        FontFamily::MonoCode,
        36.0,
        rgb(255, 202, 58),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    proj.pages.push(page);
    proj
}

fn build_story_template() -> Project {
    let mut proj = new_template_project("Vertical Flash Sale Story", DesignFormat::InstagramStory);

    let mut page = CanvasPage {
        id: 1,
        title: "Flash Sale Story".to_string(),
        notes: String::new(),
        background: PageBackground::LinearGradient {
            start: rgb(255, 92, 138),
            end: rgb(125, 42, 232),
            angle_deg: 160.0,
        },
        transition: PageTransition::None,
        elements: Vec::new(),
    };

    page.elements.push(make_shape(
        10,
        "Story Center Card",
        90.0,
        260.0,
        900.0,
        1400.0,
        ShapeType::RoundedRect,
        FillStyle::Solid(rgb(255, 255, 255)),
        48.0,
    ));

    page.elements.push(make_text(
        11,
        "Flash Sale Title",
        150.0,
        420.0,
        780.0,
        360.0,
        "WEEKEND\nFLASH\nSALE",
        FontFamily::PosterHeavy,
        118.0,
        rgb(125, 42, 232),
        true,
        TextAlign::Center,
        TextEffect::Echo {
            color: rgba(255, 92, 138, 110),
        },
    ));

    page.elements.push(make_text(
        12,
        "Discount Badge",
        240.0,
        960.0,
        600.0,
        120.0,
        "50% OFF EVERYTHING",
        FontFamily::Sans,
        48.0,
        rgb(255, 255, 255),
        true,
        TextAlign::Center,
        TextEffect::BackgroundPill {
            bg_color: rgb(20, 24, 36),
            radius: 24.0,
            padding: 18.0,
        },
    ));

    proj.pages.push(page);
    proj
}

fn build_infographic_template() -> Project {
    let mut proj = new_template_project("Annual Growth Infographic", DesignFormat::Infographic);

    let mut page = CanvasPage {
        id: 1,
        title: "Infographic".to_string(),
        notes: String::new(),
        background: PageBackground::Solid(rgb(15, 23, 42)),
        transition: PageTransition::None,
        elements: Vec::new(),
    };

    page.elements.push(make_text(
        10,
        "Infographic Header",
        60.0,
        70.0,
        680.0,
        160.0,
        "2026 CREATIVE TECH\nBENCHMARK REPORT",
        FontFamily::PosterHeavy,
        54.0,
        rgb(0, 245, 212),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    page.elements.push(CanvasElement {
        id: 11,
        name: "Share Donut Chart".to_string(),
        x: 80.0,
        y: 280.0,
        width: 640.0,
        height: 520.0,
        rotation: 0.0,
        opacity: 1.0,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow::default(),
        animation: ElementAnimation::Pop,
        group_id: None,
        kind: ElementKind::Chart {
            chart_type: ChartType::Donut,
            title: "Workflow Time Saved by Feature".to_string(),
            labels: vec![
                "Templates".to_string(),
                "Magic Cutout".to_string(),
                "Smart Resize".to_string(),
                "Brand Kit".to_string(),
            ],
            values: vec![35.0, 28.0, 22.0, 15.0],
            palette: vec![
                rgb(0, 196, 204),
                rgb(125, 42, 232),
                rgb(255, 92, 138),
                rgb(254, 228, 64),
            ],
            show_values: true,
            show_grid: false,
            text_color: rgb(248, 250, 252),
        },
    });

    page.elements.push(CanvasElement {
        id: 12,
        name: "Comparison Data Table".to_string(),
        x: 60.0,
        y: 880.0,
        width: 680.0,
        height: 440.0,
        rotation: 0.0,
        opacity: 1.0,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow::default(),
        animation: ElementAnimation::Rise,
        group_id: None,
        kind: ElementKind::Table {
            rows: 4,
            cols: 3,
            cells: vec![
                vec![
                    "Capability".to_string(),
                    "Cloud Web App".to_string(),
                    "Canvas Rust App".to_string(),
                ],
                vec![
                    "Startup Latency".to_string(),
                    "3.8 sec".to_string(),
                    "0.05 sec".to_string(),
                ],
                vec![
                    "Offline Privacy".to_string(),
                    "Cloud Synced".to_string(),
                    "100% Local".to_string(),
                ],
                vec![
                    "Export Formats".to_string(),
                    "Paywalled SVG".to_string(),
                    "Free PNG/JPG/SVG".to_string(),
                ],
            ],
            header_bg: rgb(0, 196, 204),
            row_bg: rgb(30, 41, 59),
            alt_row_bg: rgb(22, 30, 46),
            border_color: rgb(71, 85, 105),
            text_color: rgb(248, 250, 252),
            font_size: 20.0,
        },
    });

    proj.pages.push(page);
    proj
}

fn build_logo_template() -> Project {
    let mut proj = new_template_project("Geometric Brand Logo", DesignFormat::Logo);

    let mut page = CanvasPage {
        id: 1,
        title: "Logo Mark".to_string(),
        notes: String::new(),
        background: PageBackground::Solid(rgb(15, 23, 42)),
        transition: PageTransition::None,
        elements: Vec::new(),
    };

    page.elements.push(make_shape(
        10,
        "Outer Hexagon Shield",
        110.0,
        70.0,
        280.0,
        280.0,
        ShapeType::Hexagon,
        FillStyle::LinearGradient {
            start: rgb(0, 196, 204),
            end: rgb(125, 42, 232),
            vertical: true,
        },
        0.0,
    ));

    page.elements.push(make_shape(
        11,
        "Inner Diamond Core",
        175.0,
        135.0,
        150.0,
        150.0,
        ShapeType::Diamond,
        FillStyle::Solid(rgb(15, 23, 42)),
        0.0,
    ));

    page.elements.push(make_text(
        12,
        "Logo Monogram",
        205.0,
        172.0,
        90.0,
        80.0,
        "C",
        FontFamily::PosterHeavy,
        64.0,
        rgb(0, 245, 212),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    page.elements.push(make_text(
        13,
        "Brand Wordmark",
        70.0,
        380.0,
        360.0,
        60.0,
        "CANVAS STUDIO",
        FontFamily::Sans,
        34.0,
        rgb(255, 255, 255),
        true,
        TextAlign::Center,
        TextEffect::None,
    ));

    proj.pages.push(page);
    proj
}

/// Generates procedural high-resolution sample stock images so the user can test
/// One-Click Background Removal, Duotone, Blur/Sharpen, Magic Layers, and Frames immediately.
pub struct SampleStockAsset {
    pub name: &'static str,
    pub subtitle: &'static str,
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

pub fn generate_builtin_stock_assets() -> Vec<SampleStockAsset> {
    vec![
        generate_subject_on_studio_backdrop(),
        generate_sunset_mountain_landscape(),
        generate_neon_cyber_grid(),
        generate_botanical_minimal_art(),
    ]
}

/// Generates a crisp geometric emerald/coral emblem on a solid white studio backdrop
/// (ideal for testing One-Click Background Removal!).
fn generate_subject_on_studio_backdrop() -> SampleStockAsset {
    let w = 240u32;
    let h = 240u32;
    let mut buf = vec![255u8; (w * h * 4) as usize];
    let cx = w as f32 * 0.5;
    let cy = h as f32 * 0.5;

    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let dx = x as f32 - cx;
            let dy = y as f32 - cy;
            let dist = (dx * dx + dy * dy).sqrt();
            let angle = dy.atan2(dx);
            let star_r = 72.0 + 18.0 * (angle * 5.0).cos();

            if dist <= star_r {
                let t = (dist / star_r).clamp(0.0, 1.0);
                buf[idx] = (0.0 * (1.0 - t) + 125.0 * t) as u8;
                buf[idx + 1] = (210.0 * (1.0 - t) + 42.0 * t) as u8;
                buf[idx + 2] = (204.0 * (1.0 - t) + 232.0 * t) as u8;
                buf[idx + 3] = 255;
            } else {
                // Studio off-white background (removed cleanly by One-Click BG Remover)
                buf[idx] = 248;
                buf[idx + 1] = 249;
                buf[idx + 2] = 252;
                buf[idx + 3] = 255;
            }
        }
    }

    SampleStockAsset {
        name: "Studio Emblem (BG Cutout Ready)",
        subtitle: "Try 'Remove Background' on this studio subject",
        width: w,
        height: h,
        rgba: buf,
    }
}

fn generate_sunset_mountain_landscape() -> SampleStockAsset {
    let w = 320u32;
    let h = 200u32;
    let mut buf = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        let ty = y as f32 / h as f32;
        for x in 0..w {
            let tx = x as f32 / w as f32;
            let idx = ((y * w + x) * 4) as usize;

            // Sun disc
            let sdx = x as f32 - 160.0;
            let sdy = y as f32 - 78.0;
            let sdist = (sdx * sdx + sdy * sdy).sqrt();

            // Mountain ridges
            let ridge1 = 115.0 + 22.0 * (tx * 6.2).sin() + 12.0 * (tx * 13.0).cos();
            let ridge2 = 142.0 + 18.0 * (tx * 4.5 + 1.2).sin();

            let (r, g, b) = if (y as f32) > ridge2 {
                (22, 18, 42)
            } else if (y as f32) > ridge1 {
                (58, 32, 86)
            } else if sdist < 34.0 {
                (255, 225, 105)
            } else {
                let r = (255.0 * (1.0 - ty * 0.5)).clamp(0.0, 255.0) as u8;
                let g = (95.0 + 90.0 * (1.0 - ty)).clamp(0.0, 255.0) as u8;
                let b = (138.0 + 80.0 * ty).clamp(0.0, 255.0) as u8;
                (r, g, b)
            };

            buf[idx] = r;
            buf[idx + 1] = g;
            buf[idx + 2] = b;
            buf[idx + 3] = 255;
        }
    }

    SampleStockAsset {
        name: "Synthwave Sunset Mountains",
        subtitle: "320x200 Scenic Illustration",
        width: w,
        height: h,
        rgba: buf,
    }
}

fn generate_neon_cyber_grid() -> SampleStockAsset {
    let w = 280u32;
    let h = 200u32;
    let mut buf = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let gx = x % 28 == 0;
            let gy = y % 28 == 0;
            if gx || gy {
                buf[idx] = 0;
                buf[idx + 1] = 245;
                buf[idx + 2] = 212;
                buf[idx + 3] = 255;
            } else {
                buf[idx] = 15;
                buf[idx + 1] = 23;
                buf[idx + 2] = (42 + (x * 40 / w) as u8).min(255);
                buf[idx + 3] = 255;
            }
        }
    }

    SampleStockAsset {
        name: "Cyber Blueprint Grid",
        subtitle: "280x200 Tech Pattern",
        width: w,
        height: h,
        rgba: buf,
    }
}

fn generate_botanical_minimal_art() -> SampleStockAsset {
    let w = 240u32;
    let h = 240u32;
    let mut buf = vec![0u8; (w * h * 4) as usize];

    for y in 0..h {
        for x in 0..w {
            let idx = ((y * w + x) * 4) as usize;
            let dx = x as f32 - 120.0;
            let dy = y as f32 - 120.0;
            let d = (dx * dx + dy * dy).sqrt();
            if d < 76.0 {
                buf[idx] = 212;
                buf[idx + 1] = 106;
                buf[idx + 2] = 78;
                buf[idx + 3] = 255;
            } else {
                buf[idx] = 250;
                buf[idx + 1] = 243;
                buf[idx + 2] = 235;
                buf[idx + 3] = 255;
            }
        }
    }

    SampleStockAsset {
        name: "Warm Terracotta Sun",
        subtitle: "240x240 Minimalist Art (BG Cutout Ready)",
        width: w,
        height: h,
        rgba: buf,
    }
}

pub fn make_stock_image_element(
    id: u64,
    asset: &SampleStockAsset,
    x: f32,
    y: f32,
) -> CanvasElement {
    CanvasElement {
        id,
        name: asset.name.to_string(),
        x,
        y,
        width: asset.width as f32 * 1.4,
        height: asset.height as f32 * 1.4,
        rotation: 0.0,
        opacity: 1.0,
        locked: false,
        visible: true,
        flip_h: false,
        flip_v: false,
        shadow: DropShadow {
            enabled: false,
            offset_x: 0.0,
            offset_y: 8.0,
            blur: 18.0,
            color: rgba(0, 0, 0, 75),
        },
        animation: ElementAnimation::Fade,
        group_id: None,
        kind: ElementKind::Image {
            source_name: asset.name.to_string(),
            width_px: asset.width,
            height_px: asset.height,
            raw_rgba: asset.rgba.clone(),
            mask: ImageMask::Rounded(16.0),
            adjustments: ImageAdjustments::default(),
            border_color: rgb(255, 255, 255),
            border_width: 0.0,
            revision: 1,
        },
    }
}

/// Built-in examples use only features supported by every production exporter.
/// Chart examples become editable labelled bar graphics, retaining their data.
fn production_defaults(mut project: Project) -> Project {
    let mut next = project.next_id;
    for page in &mut project.pages {
        let mut elements = Vec::new();
        for mut el in std::mem::take(&mut page.elements) {
            el.shadow.enabled = false;
            if let ElementKind::Text {
                effect:
                    TextEffect::BackgroundPill {
                        bg_color,
                        radius,
                        padding,
                    },
                ..
            } = &el.kind
            {
                let gid = el.group_id.unwrap_or_else(|| {
                    let id = next;
                    next += 1;
                    id
                });
                let mut backdrop = make_shape(
                    next,
                    &format!("{} backdrop", el.name),
                    el.x - *padding,
                    el.y - *padding * 0.5,
                    el.width + *padding * 2.,
                    el.height + *padding,
                    ShapeType::RoundedRect,
                    FillStyle::Solid(*bg_color),
                    *radius,
                );
                next += 1;
                backdrop.shadow.enabled = false;
                backdrop.opacity = el.opacity;
                backdrop.group_id = Some(gid);
                backdrop.animation = el.animation;
                el.group_id = Some(gid);
                elements.push(backdrop);
            }
            if let ElementKind::Text { effect, .. } = &mut el.kind {
                *effect = TextEffect::None;
            }
            if let ElementKind::Chart {
                title,
                labels,
                values,
                palette,
                text_color,
                ..
            } = &el.kind
            {
                let gid = next;
                next += 1;
                let mut text =
                    |name: String, content: String, x: f32, y: f32, w: f32, h: f32, size: f32| {
                        let mut item = make_text(
                            next,
                            &name,
                            x,
                            y,
                            w,
                            h,
                            &content,
                            FontFamily::Sans,
                            size,
                            *text_color,
                            false,
                            TextAlign::Left,
                            TextEffect::None,
                        );
                        next += 1;
                        item.group_id = Some(gid);
                        item.animation = ElementAnimation::None;
                        item
                    };
                elements.push(text(
                    format!("{} title", el.name),
                    title.clone(),
                    el.x,
                    el.y,
                    el.width,
                    38.,
                    22.,
                ));
                let count = values.len().max(1);
                let row_h = (el.height - 52.).max(40.) / count as f32;
                let max = values.iter().copied().fold(1., f32::max);
                for (i, value) in values.iter().enumerate() {
                    let y = el.y + 48. + i as f32 * row_h;
                    let label = labels
                        .get(i)
                        .cloned()
                        .unwrap_or_else(|| format!("Item {}", i + 1));
                    elements.push(text(
                        format!("{} label {}", el.name, i),
                        label,
                        el.x,
                        y,
                        el.width * 0.25,
                        row_h,
                        16.,
                    ));
                    elements.push(text(
                        format!("{} value {}", el.name, i),
                        format!("{value:.0}"),
                        el.x + el.width * 0.87,
                        y,
                        el.width * 0.13,
                        row_h,
                        16.,
                    ));
                    // Allocate shapes after text construction, independently of the text ID closure.
                    let mut bar = make_shape(
                        0,
                        &format!("{} bar {}", el.name, i),
                        el.x + el.width * 0.27,
                        y + 2.,
                        el.width * 0.56 * (*value / max).clamp(0.01, 1.),
                        (row_h * 0.55).max(2.),
                        ShapeType::Rectangle,
                        FillStyle::Solid(
                            palette
                                .get(i % palette.len().max(1))
                                .copied()
                                .unwrap_or(rgb(0, 120, 160)),
                        ),
                        0.,
                    );
                    bar.shadow.enabled = false;
                    bar.animation = ElementAnimation::None;
                    bar.group_id = Some(gid);
                    elements.push(bar);
                }
                for bar in elements.iter_mut().filter(|e| e.id == 0) {
                    bar.id = next;
                    next += 1;
                }
            } else {
                elements.push(el);
            }
        }
        page.elements = elements;
    }
    project.next_id = next;
    project
}
