//! Shared first-use task names and descriptions for native and browser clients.
use serde::Serialize;

#[derive(Clone, Copy, Serialize)]
pub struct Workflow {
    pub id: &'static str,
    pub title: &'static str,
    pub description: &'static str,
    pub next: &'static str,
}

pub const TASKS: [Workflow; 4] = [
    Workflow {
        id: "product",
        title: "Design a product",
        description:
            "Start with an editable template for print, a digital download or social media.",
        next: "Choose a template, make it yours, then export.",
    },
    Workflow {
        id: "campaign",
        title: "Create a campaign",
        description:
            "Keep your print, Instagram and YouTube designs together with shared copy and colours.",
        next: "Start a linked campaign or continue your current layouts.",
    },
    Workflow {
        id: "video",
        title: "Make a video",
        description: "Bring in footage, arrange your story and add sound or captions.",
        next: "Import media, edit your timeline, then export MP4.",
    },
    Workflow {
        id: "research",
        title: "Bring in research",
        description:
            "Use your Reference Desk transcripts, captions, audio and video in this project.",
        next: "Import a source, choose a passage or add captions to your video.",
    },
];

pub fn generation_available() -> bool {
    generation_for_profile(std::env::var("CANVAS_STUDIO_PROFILE").ok().as_deref())
}
fn generation_for_profile(profile: Option<&str>) -> bool {
    matches!(profile, Some("personal"))
}

pub fn catalog() -> serde_json::Value {
    serde_json::json!({"tasks": TASKS, "generation_available": generation_available(), "version": env!("CARGO_PKG_VERSION"),
        "desktop_extra": "Book setup, puzzle creation and CSV batch production are available in the desktop app. Their projects can be opened and personalised in this browser."})
}

pub fn is_blank(p: &crate::model::Project) -> bool {
    p.pages.len() == 1
        && p.pages[0].elements.is_empty()
        && p.pages[0].notes.is_empty()
        && p.production.research.is_empty()
        && p.production.fields.is_empty()
        && !p.production.studio.active()
        && !p.production.timeline.active()
        && p.production.mockups.is_empty()
        && !p.production.book.enabled
        && p.production.puzzles.is_empty()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publishing_and_unknown_profiles_cannot_enable_generation() {
        assert!(!generation_for_profile(Some("publishing")));
        assert!(!generation_for_profile(Some("mistyped-profile")));
        assert!(generation_for_profile(Some("personal")));
        assert!(!generation_for_profile(None));
    }
    #[test]
    fn saved_content_is_never_mistaken_for_a_first_use_blank() {
        let mut p = crate::model::Project::default();
        assert!(is_blank(&p));
        p.pages[0].notes = "Keep my research".into();
        assert!(!is_blank(&p));
        p.pages[0].notes.clear();
        p.production.timeline.enabled = true;
        assert!(!is_blank(&p));
    }
}
