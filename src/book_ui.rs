use super::*;

pub(crate) struct MasterEditSession {
    pub original: CanvasPage,
}

impl CanvasStudioApp {
    pub(crate) fn duplicate_book_page(&mut self, index: usize) {
        let Some(mut page) = self.project.pages.get(index).cloned() else {
            return;
        };
        self.record_mutation();
        page.id = self.project.alloc_id();
        page.title.push_str(" Copy");
        page.elements = document_ops::fresh_copies(&mut self.project, &page.elements, 0., 0.);
        self.project.pages.insert(index + 1, page);
        self.current_page_idx = index + 1;
        self.selected_id = None;
        self.canvas_state.reset_for_document();
    }

    pub(crate) fn move_book_page(&mut self, index: usize, up: bool) {
        let target = if up {
            index.saturating_sub(1)
        } else {
            (index + 1).min(self.project.pages.len() - 1)
        };
        if target == index {
            return;
        }
        let selected = self.project.pages[self.current_page_idx].id;
        self.record_mutation();
        self.project.pages.swap(index, target);
        self.current_page_idx = self
            .project
            .pages
            .iter()
            .position(|p| p.id == selected)
            .unwrap_or(0);
    }

    pub(crate) fn move_selection_to_master(&mut self) {
        let selected = self
            .current_page()
            .map(|p| {
                document_ops::selected_elements(
                    p,
                    &self.canvas_state.selected_ids,
                    self.selected_id,
                )
            })
            .unwrap_or_default();
        if selected.is_empty() || selected.iter().any(|e| e.locked) {
            self.status_banner = "Select unlocked layers to move to the book master".into();
            return;
        }
        self.record_mutation();
        let ids: Vec<_> = selected.iter().map(|e| e.id).collect();
        self.project.production.book.enabled = true;
        self.project.production.book.master_elements = selected;
        if let Some(page) = self.current_page_mut() {
            page.elements.retain(|e| !ids.contains(&e.id));
            page.master_enabled = true;
        }
        self.selected_id = None;
        self.canvas_state.reset_for_document();
        self.status_banner = "Moved selected layers to the shared book master".into();
    }

    pub(crate) fn begin_master_edit(&mut self) {
        if self.master_edit_session.is_some() {
            return;
        }
        let elements = self.project.production.book.master_elements.clone();
        if elements.is_empty() {
            return;
        }
        let original = self.project.pages[self.current_page_idx].clone();
        self.record_mutation();
        let copies = document_ops::fresh_copies(&mut self.project, &elements, 0., 0.);
        let ids: Vec<_> = copies.iter().map(|e| e.id).collect();
        if let Some(page) = self.current_page_mut() {
            page.master_enabled = false;
            page.elements.extend(copies);
        }
        self.selected_id = ids.last().copied();
        self.canvas_state.selected_ids = ids;
        self.master_edit_session = Some(MasterEditSession { original });
    }

    pub(crate) fn finish_master_edit(&mut self, save: bool) {
        let Some(session) = self.master_edit_session.take() else {
            return;
        };
        let Some(index) = self
            .project
            .pages
            .iter()
            .position(|p| p.id == session.original.id)
        else {
            return;
        };
        self.record_mutation();
        if save {
            let ids: Vec<_> = session.original.elements.iter().map(|e| e.id).collect();
            let master = self.project.pages[index]
                .elements
                .iter()
                .filter(|e| !ids.contains(&e.id))
                .cloned()
                .collect::<Vec<_>>();
            if master.is_empty() {
                self.master_edit_session = Some(session);
                self.status_banner =
                    "The edited master has no layers; cancel or add content".into();
                return;
            }
            self.project.production.book.master_elements = master;
            self.project.pages[index]
                .elements
                .retain(|e| ids.contains(&e.id));
            self.project.pages[index].master_enabled = session.original.master_enabled;
        } else {
            self.project.pages[index] = session.original;
        }
        self.selected_id = None;
        self.canvas_state.reset_for_document();
    }

    pub(crate) fn render_book(&mut self, ui: &mut Ui) {
        ui.collapsing("3D product / book mockups", |ui| {
            self.render_mockup_tools(ui)
        });
        ui.heading("Book Production");
        ui.checkbox(
            &mut self.project.production.book.enabled,
            "Enable book master and margins",
        );
        ui.collapsing("Margins, headers and numbering", |ui| {
            let book = &mut self.project.production.book;
            ui.checkbox(
                &mut book.facing_pages,
                "Facing pages: alternate inner/outer margins",
            );
            egui::Grid::new("book-margins")
                .num_columns(2)
                .show(ui, |ui| {
                    for (label, value) in [
                        ("Inner mm", &mut book.inner_mm),
                        ("Outer mm", &mut book.outer_mm),
                        ("Top mm", &mut book.top_mm),
                        ("Bottom mm", &mut book.bottom_mm),
                    ] {
                        ui.label(label);
                        ui.add(egui::DragValue::new(value).range(0.0..=150.0).speed(0.5));
                        ui.end_row();
                    }
                });
            ui.checkbox(&mut book.folios, "Automatic page numbers");
            ui.horizontal(|ui| {
                ui.label("First number");
                ui.add(egui::DragValue::new(&mut book.first_number).range(1..=1_000_000));
            });
            ui.horizontal(|ui| {
                ui.label("Skip first pages");
                ui.add(egui::DragValue::new(&mut book.start_after_pages).range(0..=1000));
            });
            ui.add(
                egui::TextEdit::singleline(&mut book.header)
                    .hint_text("Header; {{page_title}} is the current page title"),
            );
            ui.add(
                egui::TextEdit::singleline(&mut book.footer)
                    .hint_text("Footer; {{page}} and {{pages}} are available"),
            );
            ui.horizontal(|ui| {
                ui.label("Header/footer pt");
                ui.add(egui::DragValue::new(&mut book.type_size_pt).range(6.0..=24.0));
            });
        });
        ui.collapsing("Reusable master artwork",|ui| {
            ui.label("Selected layers move from this page into a shared master. They remain editable in the saved document and appear on pages with Master enabled.");
            if ui.add_enabled(self.master_edit_session.is_none(),egui::Button::new("Move / Replace Master From Selected Layers")).clicked(){self.move_selection_to_master();}
            ui.label(format!("{} master layer(s)",self.project.production.book.master_elements.len()));
            if self.master_edit_session.is_some() {
                ui.label("Master editing is active. Save all edited/new master layers or cancel this edit.");
                ui.horizontal(|ui| {if ui.button("Save Edited Master").clicked(){self.finish_master_edit(true);}if ui.button("Cancel Master Edit").clicked(){self.finish_master_edit(false);}});
            }else if ui.button("Edit Master on This Page").clicked(){self.begin_master_edit();}
            if ui.button("Clear Master Artwork").clicked(){self.record_mutation();self.project.production.book.master_elements.clear();}
        });
        ui.collapsing("Batch artwork placement", |ui| {
            ui.checkbox(
                &mut self.project.production.book.blank_backs,
                "Add a blank back after each artwork page",
            );
            ui.horizontal(|ui| {
                ui.label("Artwork fit");
                for fit in [ImageFit::Contain, ImageFit::Cover] {
                    ui.selectable_value(
                        &mut self.project.production.book.artwork_fit,
                        fit,
                        fit.label(),
                    );
                }
            });
            if ui
                .button("Refit Imported Artwork to Current Margins")
                .clicked()
            {
                let mut candidate = self.project.clone();
                match crate::book_production::refit_artwork(&mut candidate) {
                    Ok(()) => {
                        self.record_mutation();
                        self.project = candidate;
                    }
                    Err(error) => self.status_banner = error,
                }
            }
            if ui
                .add_enabled(
                    self.production_job.is_none(),
                    egui::Button::new("Import Artwork Images as Pages…"),
                )
                .clicked()
            {
                if let Some(paths) = rfd::FileDialog::new()
                    .add_filter("Artwork", &["png", "jpg", "jpeg", "webp"])
                    .pick_files()
                {
                    match crate::production::fingerprint(&self.project) {
                        Ok(source_fingerprint) => {
                            let mut project = self.project.clone();
                            let (sender, receiver) = std::sync::mpsc::channel();
                            self.production_job = Some(receiver);
                            self.status_banner = "Artwork import running in the background".into();
                            std::thread::spawn(move || {
                                let result =
                                    crate::book_production::append_artwork(&mut project, &paths)
                                        .map(|_| {
                                            crate::production_ui::ProductionOutcome::Artwork {
                                                project,
                                                source_fingerprint,
                                            }
                                        });
                                let _ = sender.send(result);
                            });
                        }
                        Err(error) => self.status_banner = error,
                    }
                }
            }
            if self.production_job.is_some() {
                ui.spinner();
                ui.label("Production job running…");
            }
        });
        if self.completed_artwork.is_some() {
            ui.label(
                "Completed production was preserved separately because this document changed.",
            );
            if ui
                .button("Save Completed Production as Portable Project…")
                .clicked()
            {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("imported-book.canvas")
                    .save_file()
                {
                    let result = crate::project_package::save(
                        self.completed_artwork.as_ref().unwrap(),
                        &path,
                    );
                    match result {
                        Ok(()) => {
                            self.completed_artwork = None;
                            self.status_banner =
                                format!("Saved separate imported book to {}", path.display());
                        }
                        Err(error) => self.status_banner = error,
                    }
                }
            }
        }
        if self.master_edit_session.is_some() {
            ui.label("Save or cancel the master edit before organising pages.");
            return;
        }
        ui.separator();
        ui.strong("Page organiser");
        ui.horizontal(|ui| {
            if ui.button("Insert Blank After Current").clicked() {
                self.record_mutation();
                let mut page = CanvasPage::new(self.project.alloc_id(), "Blank page");
                page.master_enabled = false;
                self.current_page_idx += 1;
                self.project.pages.insert(self.current_page_idx, page);
                self.selected_id = None;
                self.canvas_state.reset_for_document();
            }
        });
        let count = self.project.pages.len();
        self.book_page_offset = self.book_page_offset.min(count.saturating_sub(1) / 20 * 20);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(self.book_page_offset > 0, egui::Button::new("Previous"))
                .clicked()
            {
                self.book_page_offset = self.book_page_offset.saturating_sub(20);
            }
            ui.label(format!(
                "{}–{} of {count}",
                self.book_page_offset + 1,
                (self.book_page_offset + 20).min(count)
            ));
            if ui
                .add_enabled(
                    self.book_page_offset + 20 < count,
                    egui::Button::new("Next"),
                )
                .clicked()
            {
                self.book_page_offset += 20;
            }
        });
        if self.book_preview_cache.len() > 64 {
            self.book_preview_cache.clear();
        }
        let mut action = None;
        let mut rename = None;
        let mut master_change = None;
        for index in self.book_page_offset..(self.book_page_offset + 20).min(count) {
            let page = &self.project.pages[index];
            let key = (
                self.project.document_id.clone(),
                page.id,
                self.document_revision,
            );
            if !self.book_preview_cache.contains_key(&key) {
                if let Ok(page) = crate::production::resolve_page(&self.project, index, false) {
                    let w = 90u32;
                    let h = (w as f32 * self.project.height / self.project.width).clamp(16., 300.)
                        as u32;
                    let image = crate::exporter::render_page_to_rgba_image(
                        &page,
                        self.project.width,
                        self.project.height,
                        w,
                        h,
                        false,
                    );
                    let texture = ui.ctx().load_texture(
                        format!("page-{}-{}", key.1, key.2),
                        egui::ColorImage::from_rgba_unmultiplied(
                            [w as usize, h as usize],
                            image.as_raw(),
                        ),
                        egui::TextureOptions::LINEAR,
                    );
                    self.book_preview_cache.insert(key.clone(), texture);
                }
            }
            let mut title = self.project.pages[index].title.clone();
            let mut master = self.project.pages[index].master_enabled;
            egui::Frame::group(ui.style()).show(ui, |ui| {
                ui.horizontal(|ui| {
                    if let Some(texture) = self.book_preview_cache.get(&key) {
                        ui.add(egui::Image::new(texture).max_size(Vec2::new(70., 100.)));
                    }
                    ui.vertical(|ui| {
                        if ui
                            .selectable_label(
                                index == self.current_page_idx,
                                format!("Page {}", index + 1),
                            )
                            .clicked()
                        {
                            action = Some((index, 0));
                        }
                        if ui
                            .add(egui::TextEdit::singleline(&mut title).desired_width(150.))
                            .changed()
                        {
                            rename = Some((index, title.clone()));
                        }
                        if ui.checkbox(&mut master, "Master").changed() {
                            master_change = Some((index, master));
                        }
                        ui.horizontal(|ui| {
                            if ui.add_enabled(index > 0, egui::Button::new("↑")).clicked() {
                                action = Some((index, 1));
                            }
                            if ui
                                .add_enabled(index + 1 < count, egui::Button::new("↓"))
                                .clicked()
                            {
                                action = Some((index, 2));
                            }
                            if ui.small_button("Copy").clicked() {
                                action = Some((index, 3));
                            }
                            if ui
                                .add_enabled(count > 1, egui::Button::new("Delete"))
                                .clicked()
                            {
                                action = Some((index, 4));
                            }
                        });
                    });
                });
            });
        }
        if let Some((index, title)) = rename {
            self.project.pages[index].title = title;
        }
        if let Some((index, enabled)) = master_change {
            self.project.pages[index].master_enabled = enabled;
        }
        if let Some((index, command)) = action {
            match command {
                0 => {
                    self.current_page_idx = index;
                    self.selected_id = None;
                    self.canvas_state.reset_for_document();
                }
                1 => self.move_book_page(index, true),
                2 => self.move_book_page(index, false),
                3 => self.duplicate_book_page(index),
                _ => {
                    self.record_mutation();
                    self.project.pages.remove(index);
                    if self.current_page_idx > index {
                        self.current_page_idx -= 1;
                    }
                    self.reconcile_selection_and_page();
                }
            }
        }
    }
}
