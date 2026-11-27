//! Purpose-built editable systems: creator campaigns, commerce, events and workbooks.
use super::*;
struct Spec {
    id: &'static str,
    name: &'static str,
    category: &'static str,
    description: &'static str,
    format: DesignFormat,
    headline: &'static str,
    accent: ColorRgba,
    system: u8,
    sections: &'static [&'static str],
}
fn specs() -> Vec<Spec> {
    vec![
        Spec {
            id: "creator_media_kit",
            name: "The Creator Portfolio",
            category: "Creator",
            description: "3 pages · Introduction, audience and partnership media kit",
            format: DesignFormat::DocumentA4,
            headline: "A point\nof view.",
            accent: [197, 180, 154, 255],
            system: 0,
            sections: &["Meet the creator", "Audience & impact", "Let's collaborate"],
        },
        Spec {
            id: "ugc_rate_card",
            name: "Worth Creating",
            category: "Creator",
            description: "2 pages · UGC packages, deliverables and usage terms",
            format: DesignFormat::DocumentA4,
            headline: "Made for\nyour brand.",
            accent: CORAL,
            system: 2,
            sections: &["Partnership packages", "Deliverables & usage"],
        },
        Spec {
            id: "reel_cover_series",
            name: "The Cover Edit",
            category: "Instagram",
            description: "3 pages · Cohesive 9:16 reel cover series",
            format: DesignFormat::InstagramStory,
            headline: "The daily\nedit.",
            accent: LILAC,
            system: 0,
            sections: &["The daily edit", "Behind the process", "Worth saving"],
        },
        Spec {
            id: "fashion_lookbook",
            name: "Objects & Outfits",
            category: "Creator",
            description: "3 pages · Fashion, beauty and product lookbook",
            format: DesignFormat::Custom(1080, 1350),
            headline: "Considered\nfrom every angle.",
            accent: [176, 195, 177, 255],
            system: 0,
            sections: &["The new collection", "The details", "Style your own story"],
        },
        Spec {
            id: "case_study_carousel",
            name: "From Brief to Better",
            category: "Instagram",
            description: "5 pages · Problem, process, result and CTA",
            format: DesignFormat::Custom(1080, 1350),
            headline: "From brief\nto better.",
            accent: BLUE,
            system: 0,
            sections: &[
                "The brief",
                "The challenge",
                "The approach",
                "The outcome",
                "Your next step",
            ],
        },
        Spec {
            id: "content_week",
            name: "A Week With Purpose",
            category: "Planning",
            description: "2 pages · Weekly content calendar and channel planner",
            format: DesignFormat::DocumentA4,
            headline: "Make room\nfor good ideas.",
            accent: [154, 181, 171, 255],
            system: 1,
            sections: &["The weekly plan", "Ideas & intentions"],
        },
        Spec {
            id: "video_shot_list",
            name: "On the Call Sheet",
            category: "Video",
            description: "2 pages · Shot list, props and production notes",
            format: DesignFormat::DocumentA4,
            headline: "Every shot\nhas a purpose.",
            accent: CORAL,
            system: 1,
            sections: &["The shot list", "Production notes"],
        },
        Spec {
            id: "video_storyboard",
            name: "Frame by Frame",
            category: "Video",
            description: "3 pages · Six-frame storyboard with audio and timing notes",
            format: DesignFormat::Presentation,
            headline: "Frame by frame.",
            accent: LIME,
            system: 1,
            sections: &["Opening sequence", "The turning point", "The payoff"],
        },
        Spec {
            id: "creative_moodboard",
            name: "A Visual Direction",
            category: "Identity",
            description: "2 pages · Art direction, palette and reference grid",
            format: DesignFormat::Presentation,
            headline: "A feeling.\nA direction.",
            accent: [190, 136, 111, 255],
            system: 1,
            sections: &["Visual references", "Colour & material"],
        },
        Spec {
            id: "habit_tracker",
            name: "Small Things, Often",
            category: "Planning",
            description: "1 page · Four-week habit tracker with reflection area",
            format: DesignFormat::DocumentA4,
            headline: "Small things.\nOften.",
            accent: LILAC,
            system: 1,
            sections: &["Four weeks of progress"],
        },
        Spec {
            id: "learning_worksheet",
            name: "Room to Discover",
            category: "Education",
            description: "3 pages · Lesson, practice and reflection worksheet",
            format: DesignFormat::DocumentA4,
            headline: "Room to\ndiscover.",
            accent: [232, 187, 84, 255],
            system: 1,
            sections: &["The lesson", "Try it yourself", "Reflect & remember"],
        },
        Spec {
            id: "service_price_guide",
            name: "The Service Edit",
            category: "Commerce",
            description: "3 pages · Services, packages and booking guide",
            format: DesignFormat::DocumentA4,
            headline: "Thoughtful work.\nClear choices.",
            accent: [159, 179, 159, 255],
            system: 2,
            sections: &["Our services", "Find your fit", "How to book"],
        },
        Spec {
            id: "product_detail_cards",
            name: "Made With Intention",
            category: "Commerce",
            description: "3 pages · Features, care and product story",
            format: DesignFormat::Custom(1080, 1350),
            headline: "Made with\nintention.",
            accent: [199, 153, 118, 255],
            system: 2,
            sections: &["Why you'll love it", "The finer details", "Made to last"],
        },
        Spec {
            id: "thank_you_insert",
            name: "A Little Thank You",
            category: "Commerce",
            description: "2 pages · Order insert, care note and repeat-order CTA",
            format: DesignFormat::Stationery5x7,
            headline: "Good things\ninside.",
            accent: [176, 195, 177, 255],
            system: 2,
            sections: &["A note of thanks", "Made to be kept"],
        },
        Spec {
            id: "gift_voucher",
            name: "Something to Look Forward To",
            category: "Commerce",
            description: "2 pages · Gift certificate and redemption details",
            format: DesignFormat::Stationery5x7,
            headline: "A little\nsomething.",
            accent: LILAC,
            system: 2,
            sections: &["A gift for you", "Your next good thing"],
        },
        Spec {
            id: "restaurant_menu",
            name: "At the Table",
            category: "Commerce",
            description: "2 pages · Food and drinks menu with editable prices",
            format: DesignFormat::DocumentA4,
            headline: "At the\ntable.",
            accent: [181, 90, 63, 255],
            system: 2,
            sections: &["Seasonal favourites", "Drinks & something sweet"],
        },
        Spec {
            id: "wedding_paper",
            name: "The Wedding Edition",
            category: "Events",
            description: "4 pages · Couple story, schedule, people and guest notes",
            format: DesignFormat::DocumentA4,
            headline: "The beginning\nof a good story.",
            accent: [174, 167, 141, 255],
            system: 3,
            sections: &[
                "The wedding edition",
                "Our story so far",
                "The people we love",
                "A day to remember",
            ],
        },
        Spec {
            id: "retirement_edition",
            name: "The Next Chapter",
            category: "Events",
            description: "4 pages · Career tribute, memories, messages and new plans",
            format: DesignFormat::DocumentA4,
            headline: "The next\nchapter.",
            accent: [114, 142, 149, 255],
            system: 3,
            sections: &[
                "A remarkable chapter",
                "The highlights",
                "In good company",
                "What's next?",
            ],
        },
        Spec {
            id: "recipe_collection",
            name: "The Kitchen Notebook",
            category: "Books",
            description: "3 pages · Recipe cover, ingredients and method",
            format: DesignFormat::DocumentA4,
            headline: "Good food.\nGood company.",
            accent: [163, 174, 125, 255],
            system: 3,
            sections: &[
                "From our kitchen",
                "Ingredients & preparation",
                "The method",
            ],
        },
        Spec {
            id: "book_chapter",
            name: "Between the Lines",
            category: "Books",
            description: "3 pages · Book title page, chapter opener and reading page",
            format: DesignFormat::DocumentA4,
            headline: "Between\nthe lines.",
            accent: [194, 176, 154, 255],
            system: 3,
            sections: &["The title page", "Chapter one", "A thought worth keeping"],
        },
        Spec {
            id: "lead_magnet",
            name: "The Useful Guide",
            category: "Education",
            description: "4 pages · Downloadable guide, checklist and next steps",
            format: DesignFormat::DocumentA4,
            headline: "A useful\nplace to start.",
            accent: BLUE,
            system: 3,
            sections: &[
                "Your field guide",
                "Three things to know",
                "Put it into practice",
                "Where to go next",
            ],
        },
        Spec {
            id: "event_program",
            name: "An Occasion to Remember",
            category: "Events",
            description: "3 pages · Event programme, speakers and running order",
            format: DesignFormat::DocumentA4,
            headline: "An occasion\nto remember.",
            accent: [161, 136, 168, 255],
            system: 3,
            sections: &["You're invited", "The running order", "Meet the people"],
        },
        Spec {
            id: "brand_guidelines",
            name: "A Brand With Character",
            category: "Identity",
            description: "4 pages · Brand story, palette, typography and applications",
            format: DesignFormat::Presentation,
            headline: "A brand with\ncharacter.",
            accent: LIME,
            system: 3,
            sections: &[
                "The point of view",
                "Colour & character",
                "Words & typography",
                "Out in the world",
            ],
        },
        Spec {
            id: "youtube_end_screen",
            name: "Keep the Conversation Going",
            category: "YouTube",
            description: "2 pages · 16:9 end screen and channel membership CTA",
            format: DesignFormat::Presentation,
            headline: "There's more\nto the story.",
            accent: CORAL,
            system: 1,
            sections: &["Watch this next", "Join the conversation"],
        },
    ]
}
pub(super) fn catalog() -> Vec<TemplateDescriptor> {
    specs()
        .into_iter()
        .map(|s| descriptor(s.id, s.name, s.category, s.description, s.format, s.accent))
        .collect()
}
fn label(
    p: &mut Project,
    a: &mut CanvasPage,
    x: f32,
    y: f32,
    w: f32,
    h: f32,
    value: &str,
    size: f32,
    ink: ColorRgba,
    serif: bool,
) {
    text(p, a, value, x, y, w, h, value, size, ink, serif)
}
fn purpose(id: &str, page: usize) -> &'static str {
    let copy: &[&str] = match id {
        "creator_media_kit" => &["CREATOR / LOCATION / SPECIALISM\nIntroduce the perspective your audience returns for.", "AUDIENCE / REACH / ENGAGEMENT\nAdd dated platform insights and your audience profile.", "PARTNERSHIPS / CONTACT\nFeature selected collaborations and your enquiry address."],
        "fashion_lookbook" => &["COLLECTION / SEASON / CREATOR\nReplace the editable artwork with your campaign photograph.", "FABRIC / FIT / FINISH\nCredit the pieces, materials and details in this look.", "THE COMPLETE LOOK\nAdd product references, availability and your next step."],
        "reel_cover_series" => &["EPISODE 01 / THE DAILY EDIT\nKeep your headline inside the central crop-safe area.", "EPISODE 02 / BEHIND THE PROCESS\nMatch the colour system; change the story.", "EPISODE 03 / WORTH SAVING\nGive viewers one useful reason to watch."],
        "case_study_carousel" => &["01 / THE BRIEF\nName the audience and the outcome you set out to achieve.", "02 / THE CHALLENGE\nShow the specific obstacle, with an honest starting point.", "03 / THE APPROACH\nShare the decision that changed the result.", "04 / THE OUTCOME\nReplace with a verified result and its measurement period.", "05 / YOUR NEXT STEP\nGive readers one clear action and a destination."],
        "product_detail_cards" => &["THE PRODUCT / THE PROMISE\nShow the item clearly and name the problem it solves.", "MATERIALS / DIMENSIONS / CARE\nAdd accurate product details before publishing.", "WHAT IS INCLUDED\nExplain delivery, compatibility and the next step."],
        "thank_you_insert" => &["THANK YOU FOR YOUR ORDER\nA personal note from the maker, with your contact details.", "CARE / SUPPORT / YOUR NEXT VISIT\nAdd care instructions and a working support address."],
        "brand_guidelines" => &["THE BRAND / THE POINT OF VIEW\nA short description of who you serve and how you sound.", "COLOUR / TYPOGRAPHY\nAdd colour values, font names and readable combinations.", "LOGO / CLEAR SPACE\nShow approved variants and minimum reproduction sizes.", "IN PRACTICE / CONTACT\nInclude a good application and the asset-library location."],
        "wedding_paper" => &["AVERY & JORDAN / 12 JUNE 2027\nA sample celebration edition. Replace all names and details.", "THE DAY AT A GLANCE\nCeremony 14:00 / reception 16:00 / dinner 18:00.", "THE STORY SO FAR\nShare how you met, favourite moments and a photograph.", "WITH LOVE & THANKS\nCredit your people and share the practical guest details."],
        "retirement_edition" => &["ALEX MORGAN / THE NEXT CHAPTER\nA sample retirement edition. Make every memory personal.", "THE YEARS THAT MADE THE STORY\nAdd a timeline of roles, milestones and favourite moments.", "FROM THE PEOPLE WHO KNOW YOU\nCollect short messages, names and approved photographs.", "WHAT COMES NEXT\nShare the plans, the celebration details and a final toast."],
        "recipe_collection" => &["AT THE TABLE / YOUR RECIPE COLLECTION\nA seasonal collection for sharing and keeping.", "INGREDIENTS / SERVES / PREP TIME\nList measured ingredients and relevant allergen information.", "METHOD / COOK TIME / NOTES\nWrite clear numbered steps and your serving suggestion."],
        "book_chapter" => &["PART ONE / A NEW PERSPECTIVE\nSet the scene with a short chapter introduction.", "THE CENTRAL IDEA\nBuild one clear argument with supporting examples.", "REFLECTION / WHAT COMES NEXT\nClose with a useful prompt and a chapter takeaway."],
        "lead_magnet" => &["A PRACTICAL GUIDE / YOUR SPECIALISM\nMake one useful promise you can deliver in these pages.", "START HERE / THE FIRST STEP\nExplain the problem and the reader's starting point.", "THE WORKSHEET / PUT IT INTO PRACTICE\nGive the reader a small action and space to record it.", "YOUR NEXT STEP / KEEP GOING\nAdd a relevant resource, contact and clear call to action."],
        "event_program" => &["THE GATHERING / DATE & VENUE\nWelcome your guests and introduce the occasion.", "THE RUNNING ORDER\nAdd times, sessions, speakers and useful locations.", "PEOPLE / PRACTICAL DETAILS\nCredit contributors and explain access and support."],
        _ => &["Replace the sample details with your own before publishing."],
    };
    copy[page.min(copy.len() - 1)]
}
fn cell(id: &str, i: usize) -> &'static str {
    let labels: &[&str] = match id {
        "content_week" => &[
            "MON / EDUCATE",
            "TUE / BEHIND THE SCENES",
            "WED / COMMUNITY",
            "THU / PRODUCT STORY",
            "FRI / PROOF & RESULTS",
            "WEEKEND / REFLECT",
        ],
        "video_shot_list" => &[
            "SHOT / FRAMING",
            "SUBJECT / ACTION",
            "CAMERA / MOVEMENT",
            "LIGHT / LOCATION",
            "AUDIO / DIALOGUE",
            "TAKES / NOTES",
        ],
        "video_storyboard" => &[
            "HOOK / FIRST FRAME",
            "SETUP / THE NEED",
            "ACTION / WHAT CHANGES",
            "DETAIL / THE PROOF",
            "PAYOFF / THE RESULT",
            "CTA / NEXT STEP",
        ],
        "creative_moodboard" => &[
            "COLOUR / PALETTE",
            "MATERIAL / TEXTURE",
            "LIGHT / ATMOSPHERE",
            "TYPE / CHARACTER",
            "COMPOSITION / SPACE",
            "REFERENCE / CREDIT",
        ],
        "learning_worksheet" => &[
            "WHAT I KNOW",
            "WHAT I NOTICE",
            "MY QUESTION",
            "TRY IT / SHOW YOUR WORK",
            "WHAT I LEARNED",
            "MY NEXT QUESTION",
        ],
        "youtube_end_screen" => &[
            "WATCH NEXT / FEATURED",
            "EXPLORE / PLAYLIST",
            "SUBSCRIBE / CHANNEL",
            "NEW EPISODES / SCHEDULE",
        ],
        _ => &["YOUR NOTES"],
    };
    labels[i.min(labels.len() - 1)]
}
fn offer(id: &str, i: usize) -> (&'static str, &'static str) {
    let rows = match id {
        "ugc_rate_card" => [
            ("Single story", "1 edited video / agreed usage / your rate"),
            (
                "Campaign set",
                "3 creative concepts / agreed revisions / your rate",
            ),
            (
                "Monthly partnership",
                "Deliverables / licensing period / enquiry details",
            ),
        ],
        "restaurant_menu" => [
            (
                "The seasonal plate",
                "Ingredients / dietary details / price",
            ),
            (
                "Something to share",
                "Ingredients / dietary details / price",
            ),
            ("A sweet finish", "Ingredients / dietary details / price"),
        ],
        "service_price_guide" => [
            (
                "The consultation",
                "Duration / deliverables / starting price",
            ),
            (
                "The complete service",
                "Scope / turnaround / starting price",
            ),
            (
                "The ongoing partnership",
                "Monthly scope / review cycle / enquiry details",
            ),
        ],
        "gift_voucher" => [
            ("A gift for", "Recipient name / personal message"),
            (
                "From someone who knows you",
                "Sender name / value or experience",
            ),
            (
                "How to redeem",
                "Voucher code / expiry / booking contact / terms",
            ),
        ],
        _ => [
            ("The essentials", "Scope / inclusions / price"),
            ("A little more", "Scope / inclusions / price"),
            ("The complete experience", "Scope / inclusions / price"),
        ],
    };
    rows[i.min(2)]
}
pub(super) fn build(id: &str) -> Option<Project> {
    let s = specs().into_iter().find(|s| s.id == id)?;
    let mut p = new_template_project(s.name, s.format);
    let w = p.width;
    let h = p.height;
    let unit = w.min(h);
    let margin = w * 0.065;
    for (index, section) in s.sections.iter().enumerate() {
        let dark = s.system == 0 && index % 2 == 0;
        let bg = if dark { INK } else { PAPER };
        let ink = if dark { PAPER } else { INK };
        let mut a = page(&mut p, section, bg);
        label(
            &mut p,
            &mut a,
            margin,
            h * 0.045,
            w * 0.85,
            h * 0.04,
            &format!("{} / {:02}", s.category.to_uppercase(), index + 1),
            unit * 0.015,
            ink,
            false,
        );
        match s.system {
            0 => {
                let title = if index == 0 { s.headline } else { section };
                label(
                    &mut p,
                    &mut a,
                    margin,
                    h * 0.14,
                    w * 0.86,
                    h * 0.28,
                    title,
                    (w * 0.087).min(h * 0.105),
                    ink,
                    index % 2 == 0,
                );
                let y = h * 0.48;
                art(&mut p, &mut a, margin, y, w * 0.87, h * 0.31, s.accent);
                label(
                    &mut p,
                    &mut a,
                    margin,
                    h * 0.82,
                    w * 0.84,
                    h * 0.07,
                    purpose(s.id, index),
                    unit * 0.02,
                    ink,
                    false,
                );
            }
            1 => {
                label(
                    &mut p,
                    &mut a,
                    margin,
                    h * 0.12,
                    w * 0.87,
                    h * 0.14,
                    section,
                    unit * 0.058,
                    INK,
                    true,
                );
                let columns = if s.id == "habit_tracker" {
                    7
                } else if s.id == "youtube_end_screen" {
                    2
                } else {
                    3
                };
                let rows = if s.id == "habit_tracker" { 4 } else { 2 };
                let top = h * 0.32;
                let gap = unit * 0.022;
                let cw = (w - 2. * margin - gap * (columns - 1) as f32) / columns as f32;
                let ch = (h * 0.46 - gap * (rows - 1) as f32) / rows as f32;
                for row in 0..rows {
                    for col in 0..columns {
                        let x = margin + col as f32 * (cw + gap);
                        let y = top + row as f32 * (ch + gap);
                        rect(
                            &mut p,
                            &mut a,
                            "Editable planning cell",
                            x,
                            y,
                            cw,
                            ch,
                            if (row + col) % 2 == 0 {
                                s.accent
                            } else {
                                [225, 222, 212, 255]
                            },
                        );
                        label(
                            &mut p,
                            &mut a,
                            x + cw * 0.07,
                            y + ch * 0.07,
                            cw * 0.86,
                            ch * 0.25,
                            &format!("{:02}", row * columns + col + 1),
                            unit * 0.018,
                            INK,
                            false,
                        );
                        if columns < 4 {
                            label(
                                &mut p,
                                &mut a,
                                x + cw * 0.07,
                                y + ch * 0.5,
                                cw * 0.86,
                                ch * 0.4,
                                cell(s.id, row * columns + col),
                                unit * 0.018,
                                INK,
                                false,
                            );
                        }
                    }
                }
                label(
                    &mut p,
                    &mut a,
                    margin,
                    h * 0.82,
                    w * 0.86,
                    h * 0.06,
                    "NOTES / WHAT WILL MAKE THIS USEFUL?",
                    unit * 0.015,
                    INK,
                    false,
                );
            }
            2 => {
                label(
                    &mut p,
                    &mut a,
                    margin,
                    h * 0.12,
                    w * 0.87,
                    h * 0.25,
                    if index == 0 { s.headline } else { section },
                    (unit * 0.073).min(h * 0.085),
                    INK,
                    true,
                );
                for i in 0..3 {
                    let y = h * (0.43 + i as f32 * 0.14);
                    rect(
                        &mut p,
                        &mut a,
                        "Package divider",
                        margin,
                        y,
                        w * 0.87,
                        unit * 0.002,
                        INK,
                    );
                    circle(
                        &mut p,
                        &mut a,
                        "Package marker",
                        margin,
                        y + h * 0.025,
                        unit * 0.025,
                        s.accent,
                    );
                    let (title, detail) = offer(s.id, i);
                    label(
                        &mut p,
                        &mut a,
                        margin + unit * 0.05,
                        y + h * 0.02,
                        w * 0.64,
                        h * 0.045,
                        title,
                        unit * 0.027,
                        INK,
                        false,
                    );
                    label(
                        &mut p,
                        &mut a,
                        margin + unit * 0.05,
                        y + h * 0.07,
                        w * 0.75,
                        h * 0.05,
                        detail,
                        unit * 0.018,
                        INK,
                        false,
                    );
                }
            }
            _ => {
                label(
                    &mut p,
                    &mut a,
                    margin,
                    h * 0.12,
                    w * 0.87,
                    h * 0.25,
                    if index == 0 { s.headline } else { section },
                    (unit * 0.078).min(h * 0.10),
                    INK,
                    true,
                );
                rect(
                    &mut p,
                    &mut a,
                    "Editorial rule",
                    margin,
                    h * 0.39,
                    w * 0.87,
                    unit * 0.003,
                    INK,
                );
                if index == 0 {
                    art(
                        &mut p,
                        &mut a,
                        margin,
                        h * 0.44,
                        w * 0.56,
                        h * 0.40,
                        s.accent,
                    );
                    label(
                        &mut p,
                        &mut a,
                        w * 0.66,
                        h * 0.46,
                        w * 0.27,
                        h * 0.12,
                        "A STORY\nWORTH KEEPING",
                        unit * 0.026,
                        INK,
                        false,
                    );
                    label(
                        &mut p,
                        &mut a,
                        w * 0.66,
                        h * 0.64,
                        w * 0.27,
                        h * 0.17,
                        purpose(s.id, index),
                        unit * 0.019,
                        INK,
                        false,
                    );
                } else {
                    for col in 0..2 {
                        let x = margin + col as f32 * w * 0.455;
                        label(
                            &mut p,
                            &mut a,
                            x,
                            h * 0.45,
                            w * 0.40,
                            h * 0.075,
                            if col == 0 {
                                "The details"
                            } else {
                                "Your own perspective"
                            },
                            unit * 0.03,
                            INK,
                            true,
                        );
                        label(
                            &mut p,
                            &mut a,
                            x,
                            h * 0.56,
                            w * 0.40,
                            h * 0.27,
                            purpose(s.id, index),
                            unit * 0.022,
                            INK,
                            false,
                        );
                    }
                }
            }
        }
        footer(&mut p, &mut a, index, dark);
        p.pages.push(a);
    }
    Some(production_defaults(p))
}
