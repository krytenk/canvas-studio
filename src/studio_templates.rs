//! Editable campaign layouts. All artwork is native vector content; no stock licences or remote assets.
use super::*;
const INK: ColorRgba = [24, 30, 36, 255];
const PAPER: ColorRgba = [245, 241, 232, 255];
const LIME: ColorRgba = [213, 241, 113, 255];
const BLUE: ColorRgba = [49, 74, 230, 255];
const CORAL: ColorRgba = [244, 107, 78, 255];
const LILAC: ColorRgba = [202, 189, 241, 255];
fn descriptor(
    id: &'static str,
    name: &'static str,
    category: &'static str,
    subtitle: &'static str,
    format: DesignFormat,
    accent: ColorRgba,
) -> TemplateDescriptor {
    TemplateDescriptor {
        id,
        name,
        category,
        subtitle,
        format,
        accent,
    }
}
pub(super) fn catalog() -> Vec<TemplateDescriptor> {
    vec![
        descriptor(
            "ig_editorial",
            "The Editorial",
            "Instagram",
            "4:5 · Typography-led campaign",
            DesignFormat::Custom(1080, 1350),
            BLUE,
        ),
        descriptor(
            "ig_carousel",
            "Small Shifts",
            "Instagram",
            "5 pages · Educational carousel",
            DesignFormat::Custom(1080, 1350),
            CORAL,
        ),
        descriptor(
            "social_launch",
            "The Product Drop",
            "Instagram",
            "1:1 · New-release campaign",
            DesignFormat::InstagramPost,
            LIME,
        ),
        descriptor(
            "ig_testimonial",
            "Good Words",
            "Instagram",
            "1:1 · Customer story",
            DesignFormat::InstagramPost,
            LILAC,
        ),
        descriptor(
            "ig_offer",
            "A Better Offer",
            "Instagram",
            "4:5 · Offer announcement",
            DesignFormat::Custom(1080, 1350),
            CORAL,
        ),
        descriptor(
            "story_reel",
            "The Reveal",
            "Instagram",
            "9:16 · Story cover",
            DesignFormat::InstagramStory,
            LILAC,
        ),
        descriptor(
            "ig_story_sequence",
            "From Hook to Hello",
            "Instagram",
            "3 pages · Story sequence",
            DesignFormat::InstagramStory,
            LIME,
        ),
        descriptor(
            "yt_thumbnail",
            "The Big Idea",
            "YouTube",
            "16:9 · Creator thumbnail",
            DesignFormat::YouTubeThumbnail,
            LIME,
        ),
        descriptor(
            "yt_editorial",
            "A Different Perspective",
            "YouTube",
            "16:9 · Editorial thumbnail",
            DesignFormat::YouTubeThumbnail,
            PAPER,
        ),
        descriptor(
            "yt_tutorial",
            "Zero to Ready",
            "YouTube",
            "16:9 · Tutorial thumbnail",
            DesignFormat::YouTubeThumbnail,
            CORAL,
        ),
        descriptor(
            "yt_comparison",
            "Before / After",
            "YouTube",
            "16:9 · Comparison thumbnail",
            DesignFormat::YouTubeThumbnail,
            LILAC,
        ),
        descriptor(
            "yt_channel_art",
            "Room for Ideas",
            "YouTube",
            "2560 × 1440 · Channel banner with guides",
            DesignFormat::Custom(2560, 1440),
            LIME,
        ),
        descriptor(
            "yt_end_screen",
            "Keep Watching",
            "YouTube",
            "16:9 · End-screen artwork",
            DesignFormat::Presentation,
            BLUE,
        ),
        descriptor(
            "festival_poster",
            "After Hours",
            "Print",
            "A2 · Editorial event poster",
            DesignFormat::Poster,
            CORAL,
        ),
        descriptor(
            "print_lookbook",
            "The Collection",
            "Print",
            "3 pages · A4 lookbook",
            DesignFormat::DocumentA4,
            PAPER,
        ),
        descriptor(
            "pitch_deck",
            "A Clear Direction",
            "Presentation",
            "2 pages · Pitch and principles",
            DesignFormat::Presentation,
            LIME,
        ),
        descriptor(
            "infographic_report",
            "By the Numbers",
            "Data",
            "Portrait · Editable metric story",
            DesignFormat::Infographic,
            BLUE,
        ),
        descriptor(
            "brand_logo",
            "North / Studio",
            "Identity",
            "Square · Brand direction",
            DesignFormat::Logo,
            LILAC,
        ),
    ]
}
fn rect(
    p: &mut Project,
    page: &mut CanvasPage,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    color: ColorRgba,
) {
    let mut e = make_shape(
        p.alloc_id(),
        name,
        x,
        y,
        w,
        h,
        ShapeType::Rectangle,
        FillStyle::Solid(color),
        0.,
    );
    e.shadow = Default::default();
    e.animation = ElementAnimation::None;
    page.elements.push(e);
}
fn circle(
    p: &mut Project,
    page: &mut CanvasPage,
    name: &str,
    x: f32,
    y: f32,
    size: f32,
    color: ColorRgba,
) {
    let mut e = make_shape(
        p.alloc_id(),
        name,
        x,
        y,
        size,
        size,
        ShapeType::Circle,
        FillStyle::Solid(color),
        0.,
    );
    e.shadow = Default::default();
    page.elements.push(e);
}
fn text(
    p: &mut Project,
    page: &mut CanvasPage,
    name: &str,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    value: &str,
    size: f32,
    color: ColorRgba,
    serif: bool,
) {
    let mut e = make_text(
        p.alloc_id(),
        name,
        x,
        y,
        w,
        h,
        value,
        if serif {
            FontFamily::DisplaySerif
        } else {
            FontFamily::Sans
        },
        size,
        color,
        !serif,
        TextAlign::Left,
        TextEffect::None,
    );
    e.animation = ElementAnimation::None;
    if let ElementKind::Text {
        line_height,
        letter_spacing,
        ..
    } = &mut e.kind
    {
        *line_height = 1.08;
        *letter_spacing = if size > 50. { -size * 0.018 } else { 0.3 };
    }
    page.elements.push(e);
}
fn footer(p: &mut Project, page: &mut CanvasPage, index: usize, dark: bool) {
    let w = p.width;
    let h = p.height;
    let ink = if dark { PAPER } else { INK };
    rect(
        p,
        page,
        "Footer rule",
        w * 0.06,
        h * 0.91,
        w * 0.88,
        1.3,
        ink,
    );
    text(
        p,
        page,
        "Studio signature",
        w * 0.06,
        h * 0.935,
        w * 0.65,
        h * 0.035,
        "YOUR STUDIO / MADE WITH INTENTION",
        w * 0.012,
        ink,
        false,
    );
    text(
        p,
        page,
        "Edition",
        w * 0.83,
        h * 0.935,
        w * 0.1,
        h * 0.035,
        &format!("{:02}", index + 1),
        w * 0.014,
        ink,
        false,
    );
}
fn page(p: &mut Project, title: &str, bg: ColorRgba) -> CanvasPage {
    let mut page = CanvasPage::new(p.alloc_id(), title);
    page.background = PageBackground::Solid(bg);
    page
}
fn art(p: &mut Project, page: &mut CanvasPage, x: f32, y: f32, w: f32, h: f32, accent: ColorRgba) {
    rect(p, page, "Abstract art / dark field", x, y, w, h, INK);
    circle(
        p,
        page,
        "Abstract art / sun",
        x + w * 0.17,
        y + h * 0.16,
        w.min(h) * 0.65,
        accent,
    );
    for i in 0..7 {
        rect(
            p,
            page,
            "Abstract art / cadence",
            x + w * 0.1,
            y + h * 0.72 + i as f32 * h * 0.026,
            w * 0.8,
            h * 0.008,
            PAPER,
        );
    }
}
pub(super) fn build(id: &str) -> Option<Project> {
    let tpl = catalog().into_iter().find(|t| t.id == id)?;
    let mut p = new_template_project(tpl.name, tpl.format);
    let w = p.width;
    let h = p.height;
    match id {
        "festival_poster" => {
            let mut a = page(&mut p, "After Hours / poster", PAPER);
            rect(
                &mut p,
                &mut a,
                "Editorial Arch Card",
                w * 0.06,
                h * 0.045,
                w * 0.88,
                h * 0.88,
                INK,
            );
            circle(
                &mut p,
                &mut a,
                "Warm Sun Circle",
                w * 0.5,
                h * 0.11,
                w * 0.34,
                CORAL,
            );
            text(
                &mut p,
                &mut a,
                "Poster Header",
                w * 0.11,
                h * 0.34,
                w * 0.78,
                h * 0.38,
                "STUDIO\nAFTER\nHOURS.",
                w * 0.145,
                PAPER,
                true,
            );
            text(
                &mut p,
                &mut a,
                "Poster Details",
                w * 0.12,
                h * 0.78,
                w * 0.76,
                h * 0.09,
                "GOOD PEOPLE. BETTER IDEAS.\nAN EVENING FOR THE CREATIVE COMMUNITY.",
                w * 0.021,
                PAPER,
                false,
            );
            footer(&mut p, &mut a, 0, false);
            p.pages.push(a);
        }
        "social_launch" => {
            let mut a = page(&mut p, "Product drop", INK);
            a.background = PageBackground::LinearGradient {
                start: INK,
                end: rgb(42, 59, 54),
                angle_deg: 35.,
            };
            circle(
                &mut p,
                &mut a,
                "Orbital accent",
                w * 0.72,
                h * 0.04,
                w * 0.22,
                LIME,
            );
            rect(
                &mut p,
                &mut a,
                "Edition tab",
                w * 0.06,
                h * 0.065,
                w * 0.32,
                h * 0.048,
                LIME,
            );
            text(
                &mut p,
                &mut a,
                "Edition",
                w * 0.08,
                h * 0.073,
                w * 0.28,
                h * 0.045,
                "NEW RELEASE / NO. 01",
                w * 0.018,
                INK,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Launch headline",
                w * 0.06,
                h * 0.30,
                w * 0.88,
                h * 0.43,
                "THE FUTURE\nLOOKS\nGOOD.",
                w * 0.12,
                PAPER,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Launch caption",
                w * 0.065,
                h * 0.77,
                w * 0.78,
                h * 0.08,
                "A thoughtful new addition.\nDiscover what is next for your everyday.",
                w * 0.024,
                PAPER,
                false,
            );
            footer(&mut p, &mut a, 0, true);
            p.pages.push(a);
        }
        "ig_editorial" => {
            let mut a = page(&mut p, "The editorial", BLUE);
            text(
                &mut p,
                &mut a,
                "Issue label",
                w * 0.07,
                h * 0.06,
                w * 0.8,
                h * 0.06,
                "THE STUDIO JOURNAL / 001",
                w * 0.021,
                PAPER,
                false,
            );
            rect(
                &mut p,
                &mut a,
                "Headline bracket",
                w * 0.07,
                h * 0.18,
                7.,
                h * 0.49,
                LIME,
            );
            text(
                &mut p,
                &mut a,
                "Editorial headline",
                w * 0.11,
                h * 0.19,
                w * 0.83,
                h * 0.48,
                "A BETTER\nWAY TO\nSHOW UP.",
                w * 0.12,
                PAPER,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Editorial deck",
                w * 0.11,
                h * 0.73,
                w * 0.76,
                h * 0.11,
                "Make the useful feel beautiful.\nMake the beautiful work harder.",
                w * 0.029,
                PAPER,
                true,
            );
            footer(&mut p, &mut a, 0, true);
            p.pages.push(a);
        }
        "ig_carousel" => {
            let entries = [
                (
                    "Five small\nshifts.",
                    "A PRACTICAL GUIDE TO SHOWING UP WITH INTENTION.",
                ),
                (
                    "01 / Make\nroom.",
                    "One clear idea is stronger than five competing messages.",
                ),
                (
                    "02 / Say it\nsimply.",
                    "Write for the person who is reading, not the person who is presenting.",
                ),
                (
                    "03 / Give it\na rhythm.",
                    "Contrast, space and a repeatable system make the work feel considered.",
                ),
                (
                    "A little more\nintention.",
                    "SAVE THIS FOR YOUR NEXT CREATIVE SESSION.",
                ),
            ];
            for (i, (title, body)) in entries.iter().enumerate() {
                let bg = if i == 0 { CORAL } else { PAPER };
                let mut a = page(&mut p, &format!("{:02} / Small shifts", i + 1), bg);
                text(
                    &mut p,
                    &mut a,
                    "Series",
                    w * 0.07,
                    h * 0.065,
                    w * 0.78,
                    h * 0.05,
                    "THE FIELD NOTES / A FIVE-PART CAROUSEL",
                    w * 0.019,
                    INK,
                    false,
                );
                text(
                    &mut p,
                    &mut a,
                    "Carousel headline",
                    w * 0.07,
                    h * 0.21,
                    w * 0.88,
                    h * 0.39,
                    title,
                    w * 0.117,
                    INK,
                    true,
                );
                rect(
                    &mut p,
                    &mut a,
                    "Accent rule",
                    w * 0.075,
                    h * 0.67,
                    w * 0.15,
                    6.,
                    INK,
                );
                text(
                    &mut p,
                    &mut a,
                    "Carousel body",
                    w * 0.075,
                    h * 0.72,
                    w * 0.78,
                    h * 0.12,
                    body,
                    w * 0.032,
                    INK,
                    false,
                );
                footer(&mut p, &mut a, i, false);
                p.pages.push(a);
            }
        }
        "ig_testimonial" => {
            let mut a = page(&mut p, "Customer story", LILAC);
            text(
                &mut p,
                &mut a,
                "Story label",
                w * 0.075,
                h * 0.07,
                w * 0.8,
                h * 0.05,
                "GOOD WORDS / REAL PEOPLE",
                w * 0.019,
                INK,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Customer quote",
                w * 0.075,
                h * 0.23,
                w * 0.86,
                h * 0.42,
                "“The difference\nis in the\ndetails.”",
                w * 0.084,
                INK,
                true,
            );
            rect(
                &mut p,
                &mut a,
                "Attribution rule",
                w * 0.08,
                h * 0.73,
                w * 0.08,
                4.,
                INK,
            );
            text(
                &mut p,
                &mut a,
                "Attribution",
                w * 0.2,
                h * 0.71,
                w * 0.67,
                h * 0.09,
                "A CUSTOMER STORY\nReplace with your own verified quote.",
                w * 0.02,
                INK,
                false,
            );
            footer(&mut p, &mut a, 0, false);
            p.pages.push(a);
        }
        "ig_offer" => {
            let mut a = page(&mut p, "Offer announcement", CORAL);
            text(
                &mut p,
                &mut a,
                "Offer label",
                w * 0.07,
                h * 0.06,
                w * 0.8,
                h * 0.05,
                "A LITTLE SOMETHING / JUST FOR YOU",
                w * 0.02,
                INK,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Offer headline",
                w * 0.07,
                h * 0.19,
                w * 0.86,
                h * 0.43,
                "LESS SPEND.\nMORE\nPOSSIBILITY.",
                w * 0.1,
                INK,
                false,
            );
            rect(
                &mut p,
                &mut a,
                "Offer card",
                w * 0.07,
                h * 0.68,
                w * 0.86,
                h * 0.15,
                PAPER,
            );
            text(
                &mut p,
                &mut a,
                "Offer detail",
                w * 0.11,
                h * 0.705,
                w * 0.75,
                h * 0.09,
                "YOUR OFFER GOES HERE\nAdd the dates, details and terms.",
                w * 0.028,
                INK,
                false,
            );
            footer(&mut p, &mut a, 0, false);
            p.pages.push(a);
        }
        "story_reel" | "ig_story_sequence" => {
            let count = if id == "story_reel" { 1 } else { 3 };
            let titles = [
                "SOMETHING\nGOOD IS\nCOMING.",
                "MADE FOR\nYOUR NEXT\nCHAPTER.",
                "LET US\nMAKE IT\nHAPPEN.",
            ];
            for (i, title) in titles.iter().enumerate().take(count) {
                let bg = if i % 2 == 0 { INK } else { LILAC };
                let ink = if i % 2 == 0 { PAPER } else { INK };
                let accent = if i % 2 == 0 { LIME } else { BLUE };
                let mut a = page(&mut p, &format!("Story {:02}", i + 1), bg);
                rect(
                    &mut p,
                    &mut a,
                    "Story progress",
                    w * 0.07,
                    h * 0.06,
                    w * 0.86,
                    h * 0.004,
                    accent,
                );
                text(
                    &mut p,
                    &mut a,
                    "Story label",
                    w * 0.07,
                    h * 0.1,
                    w * 0.8,
                    h * 0.055,
                    "YOUR STUDIO / THE NEXT CHAPTER",
                    w * 0.022,
                    ink,
                    false,
                );
                text(
                    &mut p,
                    &mut a,
                    "Story headline",
                    w * 0.07,
                    h * 0.26,
                    w * 0.88,
                    h * 0.37,
                    title,
                    w * 0.111,
                    ink,
                    false,
                );
                rect(
                    &mut p,
                    &mut a,
                    "Story CTA",
                    w * 0.07,
                    h * 0.77,
                    w * 0.86,
                    h * 0.085,
                    accent,
                );
                text(
                    &mut p,
                    &mut a,
                    "CTA label",
                    w * 0.11,
                    h * 0.79,
                    w * 0.77,
                    h * 0.05,
                    if i == 2 {
                        "START THE CONVERSATION"
                    } else {
                        "TAKE A CLOSER LOOK"
                    },
                    w * 0.03,
                    if i % 2 == 0 { INK } else { PAPER },
                    false,
                );
                footer(&mut p, &mut a, i, i % 2 == 0);
                p.pages.push(a);
            }
        }
        "yt_thumbnail" | "yt_editorial" | "yt_tutorial" => {
            let (bg, ink, accent, title, tag) = match id {
                "yt_editorial" => (
                    PAPER,
                    INK,
                    CORAL,
                    "A DIFFERENT\nPERSPECTIVE.",
                    "THE CREATIVE JOURNAL",
                ),
                "yt_tutorial" => (
                    BLUE,
                    PAPER,
                    CORAL,
                    "FROM ZERO\nTO READY.",
                    "THE PRACTICAL GUIDE",
                ),
                _ => (
                    INK,
                    PAPER,
                    LIME,
                    "MAKE IT\nMAKE SENSE.",
                    "THE BIG IDEA / EXPLAINED",
                ),
            };
            let mut a = page(&mut p, "Thumbnail", bg);
            text(
                &mut p,
                &mut a,
                "Episode label",
                w * 0.055,
                h * 0.07,
                w * 0.85,
                h * 0.07,
                tag,
                w * 0.018,
                ink,
                false,
            );
            art(
                &mut p,
                &mut a,
                w * 0.69,
                h * 0.22,
                w * 0.26,
                h * 0.62,
                accent,
            );
            text(
                &mut p,
                &mut a,
                "Thumbnail headline",
                w * 0.05,
                h * 0.26,
                w * 0.63,
                h * 0.5,
                title,
                w * 0.071,
                ink,
                id == "yt_editorial",
            );
            rect(
                &mut p,
                &mut a,
                "Signature rule",
                w * 0.055,
                h * 0.87,
                w * 0.22,
                4.,
                accent,
            );
            text(
                &mut p,
                &mut a,
                "Channel signature",
                w * 0.055,
                h * 0.91,
                w * 0.8,
                h * 0.05,
                "YOUR CHANNEL / A NEW WAY TO SEE IT",
                w * 0.013,
                ink,
                false,
            );
            p.pages.push(a);
        }
        "yt_comparison" => {
            let mut a = page(&mut p, "Before and after", PAPER);
            rect(&mut p, &mut a, "After field", w * 0.5, 0., w * 0.5, h, INK);
            text(
                &mut p,
                &mut a,
                "Before label",
                w * 0.055,
                h * 0.09,
                w * 0.4,
                h * 0.08,
                "BEFORE",
                w * 0.022,
                INK,
                false,
            );
            text(
                &mut p,
                &mut a,
                "After label",
                w * 0.56,
                h * 0.09,
                w * 0.4,
                h * 0.08,
                "AFTER",
                w * 0.022,
                LIME,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Before headline",
                w * 0.055,
                h * 0.29,
                w * 0.42,
                h * 0.44,
                "SAME\nIDEA.",
                w * 0.092,
                INK,
                true,
            );
            text(
                &mut p,
                &mut a,
                "After headline",
                w * 0.555,
                h * 0.29,
                w * 0.42,
                h * 0.44,
                "BETTER\nSYSTEM.",
                w * 0.071,
                PAPER,
                false,
            );
            p.pages.push(a);
        }
        "yt_channel_art" => {
            let mut a = page(&mut p, "Channel banner", INK);
            circle(
                &mut p,
                &mut a,
                "Left orbit",
                -w * 0.11,
                h * 0.02,
                w * 0.38,
                BLUE,
            );
            circle(
                &mut p,
                &mut a,
                "Right orbit",
                w * 0.81,
                h * 0.42,
                w * 0.35,
                LILAC,
            );
            text(
                &mut p,
                &mut a,
                "Channel label",
                w * 0.26,
                h * 0.37,
                w * 0.5,
                h * 0.04,
                "YOUR CHANNEL / FILMS ABOUT GOOD IDEAS",
                w * 0.012,
                LIME,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Channel title",
                w * 0.26,
                h * 0.43,
                w * 0.5,
                h * 0.14,
                "ROOM FOR BETTER IDEAS.",
                w * 0.03,
                PAPER,
                true,
            );
            text(
                &mut p,
                &mut a,
                "Channel cadence",
                w * 0.26,
                h * 0.59,
                w * 0.5,
                h * 0.04,
                "A FRESH PERSPECTIVE. EVERY WEEK.",
                w * 0.011,
                PAPER,
                false,
            );
            let scale = 1.25;
            let safe_w = 1235. * scale;
            let safe_h = 338. * scale;
            p.print_settings.guides_x_mm = vec![
                (w - safe_w) * 0.5 / 96. * 25.4,
                (w + safe_w) * 0.5 / 96. * 25.4,
            ];
            p.print_settings.guides_y_mm = vec![
                (h - safe_h) * 0.5 / 96. * 25.4,
                (h + safe_h) * 0.5 / 96. * 25.4,
            ];
            p.pages.push(a);
        }
        "yt_end_screen" => {
            let mut a = page(&mut p, "End-screen artwork", INK);
            text(
                &mut p,
                &mut a,
                "End screen headline",
                w * 0.065,
                h * 0.08,
                w * 0.84,
                h * 0.13,
                "THERE IS MORE TO THE STORY.",
                w * 0.041,
                PAPER,
                true,
            );
            for i in 0..2 {
                let x = w * (0.065 + i as f32 * 0.48);
                rect(
                    &mut p,
                    &mut a,
                    "Video frame",
                    x,
                    h * 0.31,
                    w * 0.405,
                    h * 0.405,
                    if i == 0 { BLUE } else { LILAC },
                );
                text(
                    &mut p,
                    &mut a,
                    "Video placeholder label",
                    x + w * 0.035,
                    h * 0.47,
                    w * 0.34,
                    h * 0.1,
                    if i == 0 {
                        "WATCH NEXT"
                    } else {
                        "KEEP EXPLORING"
                    },
                    w * 0.026,
                    if i == 0 { PAPER } else { INK },
                    false,
                );
            }
            text(
                &mut p,
                &mut a,
                "Subscribe CTA",
                w * 0.065,
                h * 0.84,
                w * 0.85,
                h * 0.06,
                "SUBSCRIBE FOR THE NEXT CHAPTER / YOUR CHANNEL",
                w * 0.019,
                LIME,
                false,
            );
            p.pages.push(a);
        }
        "pitch_deck" => {
            for i in 0..2 {
                let mut a = page(
                    &mut p,
                    if i == 0 {
                        "A clear direction"
                    } else {
                        "The principles"
                    },
                    if i == 0 { INK } else { PAPER },
                );
                let ink = if i == 0 { PAPER } else { INK };
                text(
                    &mut p,
                    &mut a,
                    "Deck label",
                    w * 0.06,
                    h * 0.08,
                    w * 0.8,
                    h * 0.07,
                    "YOUR STUDIO / THE NEXT CHAPTER",
                    w * 0.016,
                    ink,
                    false,
                );
                if i == 0 {
                    circle(
                        &mut p,
                        &mut a,
                        "Pitch orbit",
                        w * 0.72,
                        h * 0.16,
                        w * 0.22,
                        LIME,
                    );
                    text(
                        &mut p,
                        &mut a,
                        "Pitch headline",
                        w * 0.06,
                        h * 0.29,
                        w * 0.67,
                        h * 0.42,
                        "GOOD IDEAS.\nCLEARLY TOLD.",
                        w * 0.067,
                        ink,
                        true,
                    );
                } else {
                    for (j, value) in [
                        "ONE CLEAR IDEA.",
                        "A SYSTEM THAT SCALES.",
                        "DETAILS THAT MATTER.",
                    ]
                    .iter()
                    .enumerate()
                    {
                        let y = h * (0.28 + j as f32 * 0.19);
                        text(
                            &mut p,
                            &mut a,
                            "Principle number",
                            w * 0.06,
                            y,
                            w * 0.08,
                            h * 0.1,
                            &format!("0{}", j + 1),
                            w * 0.024,
                            BLUE,
                            false,
                        );
                        text(
                            &mut p,
                            &mut a,
                            "Principle",
                            w * 0.17,
                            y,
                            w * 0.77,
                            h * 0.14,
                            value,
                            w * 0.035,
                            INK,
                            false,
                        );
                    }
                }
                footer(&mut p, &mut a, i, i == 0);
                p.pages.push(a);
            }
        }
        "brand_logo" => {
            let mut a = page(&mut p, "Identity direction", INK);
            rect(&mut p, &mut a, "Identity slash", 55., 55., 32., 95., LILAC);
            rect(
                &mut p,
                &mut a,
                "Identity counter",
                103.,
                55.,
                32.,
                95.,
                CORAL,
            );
            text(
                &mut p,
                &mut a,
                "Brand wordmark",
                55.,
                215.,
                400.,
                180.,
                "NORTH /\nSTUDIO",
                65.,
                PAPER,
                true,
            );
            text(
                &mut p,
                &mut a,
                "Brand purpose",
                57.,
                425.,
                390.,
                36.,
                "INDEPENDENT IDEAS. DISTINCTIVE WORK.",
                11.,
                PAPER,
                false,
            );
            p.pages.push(a);
        }
        "infographic_report" => {
            let mut a = page(&mut p, "Metric story", PAPER);
            text(
                &mut p,
                &mut a,
                "Report label",
                w * 0.08,
                h * 0.05,
                w * 0.8,
                h * 0.05,
                "THE YEAR IN FOCUS / YOUR STUDIO",
                w * 0.022,
                INK,
                false,
            );
            text(
                &mut p,
                &mut a,
                "Report headline",
                w * 0.08,
                h * 0.14,
                w * 0.86,
                h * 0.17,
                "PROGRESS,\nIN PERSPECTIVE.",
                w * 0.085,
                INK,
                true,
            );
            for (i, (number, label, ratio)) in [
                ("42%", "A METRIC THAT MATTERS", 0.68),
                ("3.2×", "A CHANGE WORTH SHARING", 0.85),
                ("86", "THE PEOPLE BEHIND IT", 0.54),
            ]
            .iter()
            .enumerate()
            {
                let y = h * (0.37 + i as f32 * 0.17);
                text(
                    &mut p,
                    &mut a,
                    "Metric value",
                    w * 0.08,
                    y,
                    w * 0.83,
                    h * 0.09,
                    number,
                    w * 0.11,
                    BLUE,
                    false,
                );
                text(
                    &mut p,
                    &mut a,
                    "Metric label",
                    w * 0.085,
                    y + h * 0.085,
                    w * 0.8,
                    h * 0.04,
                    label,
                    w * 0.018,
                    INK,
                    false,
                );
                rect(
                    &mut p,
                    &mut a,
                    "Metric track",
                    w * 0.085,
                    y + h * 0.13,
                    w * 0.83,
                    7.,
                    rgb(218, 219, 212),
                );
                rect(
                    &mut p,
                    &mut a,
                    "Metric bar",
                    w * 0.085,
                    y + h * 0.13,
                    w * 0.83 * ratio,
                    7.,
                    BLUE,
                );
            }
            footer(&mut p, &mut a, 0, false);
            p.pages.push(a);
        }
        "print_lookbook" => {
            for i in 0..3 {
                let mut a = page(&mut p, &format!("Collection / {:02}", i + 1), PAPER);
                text(
                    &mut p,
                    &mut a,
                    "Lookbook label",
                    w * 0.075,
                    h * 0.055,
                    w * 0.85,
                    h * 0.05,
                    "YOUR STUDIO / A CONSIDERED COLLECTION",
                    w * 0.016,
                    INK,
                    false,
                );
                if i == 0 {
                    text(
                        &mut p,
                        &mut a,
                        "Lookbook headline",
                        w * 0.075,
                        h * 0.16,
                        w * 0.87,
                        h * 0.25,
                        "Objects with\na point of view.",
                        w * 0.072,
                        INK,
                        true,
                    );
                    art(
                        &mut p,
                        &mut a,
                        w * 0.075,
                        h * 0.47,
                        w * 0.85,
                        h * 0.37,
                        CORAL,
                    );
                } else {
                    art(
                        &mut p,
                        &mut a,
                        w * 0.075,
                        h * 0.15,
                        w * 0.85,
                        h * 0.42,
                        if i == 1 { LILAC } else { LIME },
                    );
                    text(
                        &mut p,
                        &mut a,
                        "Collection title",
                        w * 0.075,
                        h * 0.62,
                        w * 0.87,
                        h * 0.12,
                        if i == 1 {
                            "The everyday,\nreconsidered."
                        } else {
                            "Details make\nthe difference."
                        },
                        w * 0.06,
                        INK,
                        true,
                    );
                    text(&mut p,&mut a,"Collection description",w*0.075,h*0.79,w*0.8,h*0.07,"A short, thoughtful description of your work.\nReplace the artwork with your own photography.",w*0.018,INK,false);
                }
                footer(&mut p, &mut a, i, false);
                p.pages.push(a);
            }
        }
        _ => return None,
    }
    Some(production_defaults(p))
}
