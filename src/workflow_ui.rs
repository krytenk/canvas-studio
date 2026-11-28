use super::*;

impl CanvasStudioApp {
    pub(crate) fn go_home(&mut self) {
        if self.screen != WorkspaceScreen::HomeHub {
            self.home_return = self.screen;
        }
        self.screen = WorkspaceScreen::HomeHub;
    }

    pub(crate) fn open_workflow(&mut self, id: &str) {
        self.dock_open = true;
        match id {
            "product" => {
                self.screen = WorkspaceScreen::StudioEditor;
                self.active_dock = DockTab::Templates;
                self.studio_search.clear();
                self.studio_category = "All templates".into();
            }
            "campaign" => {
                self.screen = WorkspaceScreen::StudioEditor;
                self.active_dock = DockTab::BrandKit;
            }
            "video" => {
                self.screen = WorkspaceScreen::StudioEditor;
                self.active_dock = DockTab::Film;
            }
            "research" => {
                self.screen = WorkspaceScreen::ResearchStudio;
            }
            "image" if crate::workflows::generation_available() => {
                self.generation_ui.task = "image".into();
                self.screen = WorkspaceScreen::GenerationStudio;
            }
            _ => return,
        }
        if let Some(task) = crate::workflows::TASKS.iter().find(|task| task.id == id) {
            self.status_banner = task.next.into();
        }
    }

    pub(crate) fn open_design_picker(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Canvas Project", &["canvas", "json"])
            .pick_file()
        {
            match load_project_json(&path) {
                Ok(project) => self.request_replace_document(
                    project,
                    Some(path),
                    "Opened editable design".into(),
                ),
                Err(error) => self.status_banner = format!("Open error: {error}"),
            }
        }
    }

    pub(crate) fn render_start_screen(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(14.);
            ui.label(RichText::new("YOUR CREATIVE WORKSPACE").size(11.).color(Color32::from_rgb(126,218,190)));
            ui.heading(RichText::new("What would you like to make?").size(30.).color(Color32::WHITE));
            ui.label("Choose a starting point. Your current project stays here while you explore.");
            ui.add_space(18.);
            let columns = if ui.available_width() >= 620. { 2 } else { 1 };
            let tasks = crate::workflows::TASKS;
            for row in tasks.chunks(columns) {
                ui.columns(columns, |cols| {
                    for (col, task) in cols.iter_mut().zip(row) {
                        egui::Frame::none().fill(Color32::from_rgb(28,37,44)).rounding(12.)
                            .inner_margin(Margin::same(18.)).stroke(Stroke::new(1_f32,Color32::from_rgb(61,82,85)))
                            .show(col, |ui| {
                                ui.set_min_width(ui.available_width()); ui.set_min_height(126.);
                                ui.label(RichText::new(task.title).size(20.).strong().color(Color32::WHITE));
                                ui.add_space(6.); ui.label(task.description); ui.add_space(10.);
                                if ui.add(egui::Button::new(RichText::new(task.title).color(Color32::WHITE)).fill(Color32::from_rgb(30,113,92)).min_size(Vec2::new(0.,36.))).on_hover_text(task.next).clicked() { self.open_workflow(task.id); }
                            });
                    }
                });
                ui.add_space(12.);
            }
            ui.add_space(6.);
            ui.strong(format!("Current project: {}", self.project.title));
            ui.label(format!("{} page(s) · {}", self.project.pages.len(), if self.dirty {"Unsaved changes - save an editable copy to keep them."} else if self.current_project_path.is_some() {"Saved file open."} else {"Ready for your next idea."}));
            ui.horizontal_wrapped(|ui| {
                if ui.button("Continue current project").clicked() { self.screen = self.home_return; }
                if ui.button("Open design...").clicked() { self.open_design_picker(); }
            });
            ui.add_space(12.);
            ui.collapsing("More tools and formats", |ui| {
                ui.label("Book setup, puzzle creation and CSV batch production are in the desktop tools. Existing designs remain editable across desktop and browser.");
                ui.horizontal_wrapped(|ui| {
                    for (label,dock) in [("Books",DockTab::Book),("Puzzles",DockTab::Puzzles),("Batch production",DockTab::Production)] {
                        if ui.button(label).clicked() { self.screen=WorkspaceScreen::StudioEditor; self.active_dock=dock; self.dock_open=true; }
                    }
                });
            });
            ui.add_space(12.); ui.label(RichText::new(format!("Canvas Studio {} · on this computer",env!("CARGO_PKG_VERSION"))).small());
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_starting_task_preserves_the_open_document_and_unsaved_edits() {
        let dir = std::env::temp_dir().join(format!(
            "canvas-workflows-{}",
            model::generate_document_id()
        ));
        let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(dir));
        app.project.title = "My unsaved design".into();
        app.dirty = true;
        let original = serde_json::to_value(&app.project).unwrap();
        for task in crate::workflows::TASKS {
            app.open_workflow(task.id);
            assert_eq!(serde_json::to_value(&app.project).unwrap(), original);
            assert!(app.dirty);
            let screen = app.screen;
            app.go_home();
            assert_eq!(app.home_return, screen);
            app.screen = app.home_return;
            match task.id {
                "research" => assert_eq!(app.screen, WorkspaceScreen::ResearchStudio),
                "video" => assert_eq!(app.active_dock, DockTab::Film),
                "campaign" => assert_eq!(app.active_dock, DockTab::BrandKit),
                _ => assert_eq!(app.active_dock, DockTab::Templates),
            }
        }
    }
}
