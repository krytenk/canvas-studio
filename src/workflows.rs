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
        id: "image",
        title: "Generate an image",
        description: "Choose a model, describe your idea and let generation run in the background.",
        next: "Generation needs a configured provider. Design tools work without keys.",
    },
];

pub fn catalog() -> serde_json::Value {
    serde_json::json!({"tasks": TASKS, "version": env!("CARGO_PKG_VERSION"),
        "desktop_extra": "Book setup, puzzle creation and CSV batch production are available in the desktop app. Their projects can be opened and personalised in this browser."})
}

pub fn is_blank(p: &crate::model::Project) -> bool {
    p.pages.len() == 1
        && p.pages[0].elements.is_empty()
        && p.pages[0].notes.is_empty()
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
