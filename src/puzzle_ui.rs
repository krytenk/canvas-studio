use super::*;
use crate::puzzle_production::{self as puzzles, Difficulty, PuzzleStyle, WordSearchConfig};

pub(crate) struct PuzzleDraft {
    pub config: WordSearchConfig,
    pub words: String,
    pub style: PuzzleStyle,
    pub count: usize,
    pub answers: bool,
}
impl Default for PuzzleDraft {
    fn default() -> Self {
        let config = WordSearchConfig::default();
        Self {
            words: config.words.join("\n"),
            config,
            style: Default::default(),
            count: 1,
            answers: true,
        }
    }
}
impl CanvasStudioApp {
    pub(crate) fn render_puzzles(&mut self, ui: &mut Ui) {
        ui.heading("Word-search studio");
        ui.label("Create reproducible puzzles with checked answer keys. Review the vocabulary for your readers; difficulty controls directions, not reading age.");
        let draft = &mut self.puzzle_draft;
        ui.label("Title");
        ui.text_edit_singleline(&mut draft.config.title);
        ui.label("Words: one per line, A–Z (spaces/hyphens are omitted in the grid)");
        ui.add(
            egui::TextEdit::multiline(&mut draft.words)
                .desired_rows(8)
                .desired_width(f32::INFINITY),
        );
        ui.horizontal(|ui| {
            ui.label("Grid");
            ui.add(egui::DragValue::new(&mut draft.config.size).range(8..=24));
            ui.label("× same");
        });
        egui::ComboBox::from_id_salt("puzzle-difficulty")
            .selected_text(draft.config.difficulty.label())
            .show_ui(ui, |ui| {
                for level in [Difficulty::Easy, Difficulty::Medium, Difficulty::Hard] {
                    ui.selectable_value(&mut draft.config.difficulty, level, level.label());
                }
            });
        ui.horizontal(|ui| {
            ui.label("Seed");
            ui.add(egui::DragValue::new(&mut draft.config.seed));
        });
        ui.horizontal(|ui| {
            ui.label("Puzzles");
            ui.add(egui::DragValue::new(&mut draft.count).range(1..=50));
        });
        ui.checkbox(&mut draft.answers, "Include answer pages at the back");
        ui.collapsing("Print styling", |ui| {
            egui::ComboBox::from_id_salt("puzzle-font")
                .selected_text(draft.style.family.label())
                .show_ui(ui, |ui| {
                    for family in [FontFamily::Sans, FontFamily::DisplaySerif] {
                        ui.selectable_value(&mut draft.style.family, family, family.label());
                    }
                    for font in &self.project.imported_fonts {
                        ui.selectable_value(
                            &mut draft.style.family,
                            FontFamily::Imported(font.id),
                            &font.name,
                        );
                    }
                });
            for (label, value) in [
                ("Title pt", &mut draft.style.title_pt),
                ("Letter pt", &mut draft.style.letter_pt),
                ("Word-list pt", &mut draft.style.word_pt),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.add(egui::DragValue::new(value).range(10.0..=48.0));
                });
            }
            ui.horizontal(|ui| {
                ui.label("Ink");
                ui.color_edit_button_srgba_unmultiplied(&mut draft.style.ink);
                ui.label("Accent");
                ui.color_edit_button_srgba_unmultiplied(&mut draft.style.accent);
            });
            ui.checkbox(&mut draft.style.grid_lines, "Light grid rules");
        });
        if ui
            .add_enabled(
                self.production_job.is_none() && self.master_edit_session.is_none(),
                egui::Button::new("Create Checked Puzzle Pages…"),
            )
            .clicked()
        {
            let mut config = draft.config.clone();
            config.words = draft
                .words
                .lines()
                .map(str::trim)
                .filter(|w| !w.is_empty())
                .map(str::to_string)
                .collect();
            let style = draft.style.clone();
            let count = draft.count;
            let answers = draft.answers;
            match production::fingerprint(&self.project) {
                Ok(source_fingerprint) => {
                    let mut project = self.project.clone();
                    let (sender, receiver) = std::sync::mpsc::channel();
                    self.production_job = Some(receiver);
                    self.status_banner =
                        "Generating and checking puzzles in the background…".into();
                    std::thread::spawn(move || {
                        let result =
                            puzzles::append_series(&mut project, &config, &style, count, answers)
                                .map(|_| crate::production_ui::ProductionOutcome::Artwork {
                                    project,
                                    source_fingerprint,
                                });
                        let _ = sender.send(result);
                    });
                }
                Err(error) => self.status_banner = error,
            }
        }
        if self.production_job.is_some() {
            ui.spinner();
            ui.label("Production job running…");
        }
        ui.separator();
        ui.strong("Saved recipes");
        ui.label("Rebuild styling keeps the same grid and answers, and preserves added artwork. It replaces generated layers. Editing those layers by hand blocks checked export until rebuilt or detached. Detaching keeps the pages but ends automatic answer validation.");
        let mut action = None;
        for (index, product) in self.project.production.puzzles.iter().enumerate() {
            ui.group(|ui| {
                ui.strong(&product.config.title);
                ui.label(format!(
                    "{}×{} · {} words · seed {}",
                    product.config.size,
                    product.config.size,
                    product.config.words.len(),
                    product.config.seed
                ));
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("View").clicked() {
                        action = Some((index, 0));
                    }
                    if ui.small_button("Load controls").clicked() {
                        action = Some((index, 1));
                    }
                    if ui.small_button("Rebuild with current styling").clicked() {
                        action = Some((index, 2));
                    }
                    if ui.small_button("Detach recipe").clicked() {
                        action = Some((index, 3));
                    }
                });
            });
        }
        if let Some((index, command)) = action {
            match command {
                0 => {
                    let id = self.project.production.puzzles[index].puzzle_page;
                    if let Some(page) = self.project.pages.iter().position(|p| p.id == id) {
                        self.current_page_idx = page;
                        self.selected_id = None;
                        self.canvas_state.reset_for_document();
                    }
                }
                1 => {
                    let product = &self.project.production.puzzles[index];
                    self.puzzle_draft.config = product.config.clone();
                    self.puzzle_draft.words = product.config.words.join("\n");
                    self.puzzle_draft.style = product.style.clone();
                }
                2 => {
                    let mut candidate = self.project.clone();
                    match puzzles::restyle(&mut candidate, index, &self.puzzle_draft.style) {
                        Ok(()) => {
                            self.record_mutation();
                            self.project = candidate;
                            self.reconcile_selection_and_page();
                            self.status_banner =
                                "Puzzle styling rebuilt; the checked grid and answer are unchanged"
                                    .into();
                        }
                        Err(error) => self.status_banner = error,
                    }
                }
                _ => {
                    self.record_mutation();
                    self.project.production.puzzles.remove(index);
                    self.status_banner =
                        "Detached recipe; the editable pages remain and need manual answer review"
                            .into();
                }
            }
        }
    }
}
