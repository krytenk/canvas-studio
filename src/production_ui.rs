use super::*;

pub(crate) enum ProductionOutcome {
    Csv(crate::batch_production::BatchReport),
    Artwork {
        project: Project,
        source_fingerprint: String,
    },
}

impl CanvasStudioApp {
    pub(crate) fn poll_production_job(&mut self) {
        let outcome = self.production_job.as_ref().map(|job| job.try_recv());
        match outcome {
            Some(Ok(result)) => {
                self.production_job = None;
                match result {
                    Ok(ProductionOutcome::Csv(report)) => {
                        self.status_banner = format!(
                            "Batch finished: {} succeeded, {} need attention. Report: {}",
                            report.records.iter().filter(|r| r.success).count(),
                            report.records.iter().filter(|r| !r.success).count(),
                            report.folder.display()
                        )
                    }
                    Ok(ProductionOutcome::Artwork {
                        mut project,
                        source_fingerprint,
                    }) => {
                        if crate::production::fingerprint(&self.project).ok().as_ref()
                            == Some(&source_fingerprint)
                        {
                            self.record_mutation();
                            self.project = project;
                            self.reconcile_selection_and_page();
                            self.fit_canvas_to_viewport();
                            self.status_banner =
                                "Artwork import completed; the editable book is ready to review"
                                    .into();
                        } else {
                            project.document_id = model::generate_document_id();
                            project.recovery_origin_path = None;
                            self.completed_artwork = Some(project);
                            self.status_banner="Artwork import completed separately because the document changed. Save it from the Book panel".into();
                        }
                    }
                    Err(error) => self.status_banner = format!("Production job failed: {error}"),
                }
            }
            Some(Err(std::sync::mpsc::TryRecvError::Disconnected)) => {
                self.production_job = None;
                self.status_banner =
                    "Production worker stopped; inspect the batch folder before retrying".into();
            }
            _ => {}
        }
    }

    fn bind_selected_field(&mut self, key: &str) {
        let selected = self.selected_id;
        if let Some(element) = self
            .current_page_mut()
            .and_then(|p| p.elements.iter_mut().find(|e| Some(e.id) == selected))
        {
            if element.locked {
                return;
            }
            match &mut element.kind {
                ElementKind::Text { content, .. } => {
                    let old = content.clone();
                    *content = format!("{{{{{key}}}}}");
                    element.text_format.reconcile_edit(&old, content);
                }
                ElementKind::Image { .. } => {
                    element.binding = Some(key.into());
                    if let Some(field) = self.project.production.fields.get_mut(key) {
                        field.image = true;
                    }
                }
                _ => {
                    self.status_banner = "Select a text or image layer to bind a field".into();
                    return;
                }
            }
            self.status_banner = format!("Bound selected layer to '{key}'");
        }
    }

    pub(crate) fn render_production(&mut self, ui: &mut Ui) {
        ui.heading("Product Production");
        ui.label("Keep the master editable; fields and shared styles are resolved for preview and output.");
        ui.separator();
        ui.strong("Collection fields");
        ui.add(
            egui::TextEdit::singleline(&mut self.production_key)
                .hint_text("Field key, e.g. guest_name"),
        );
        if ui.button("Add Field").clicked() {
            let key = self.production_key.trim();
            if crate::production::valid_key(key) && !matches!(key, "page" | "pages" | "page_title")
            {
                self.project.production.fields.entry(key.into()).or_insert(
                    crate::production::DataField {
                        required: true,
                        ..Default::default()
                    },
                );
            } else {
                self.status_banner="Use letters, numbers and underscores for the field key; page/pages/page_title are reserved".into();
            }
        }
        let mut bind = None;
        let mut remove = None;
        for key in self
            .project
            .production
            .fields
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                let field = self.project.production.fields.get_mut(&key).unwrap();
                ui.strong(&key);
                ui.add_enabled(
                    !field.image,
                    egui::TextEdit::singleline(&mut field.value).hint_text("Value"),
                );
                ui.checkbox(&mut field.required, "Required");
                ui.horizontal(|ui| {
                    if ui.small_button("Bind Selected").clicked() {
                        bind = Some(key.clone());
                    }
                    if ui.small_button("Choose Photo").clicked() {
                        if let Some(path) = rfd::FileDialog::new()
                            .add_filter("Photo", &["png", "jpg", "jpeg", "webp"])
                            .pick_file()
                        {
                            if let Err(error) = crate::production::attach_image(field, &path) {
                                self.status_banner = error;
                            }
                        }
                    }
                    if ui.small_button("Remove").clicked() {
                        remove = Some(key.clone());
                    }
                });
            });
        }
        if let Some(key) = remove {
            self.project.production.fields.remove(&key);
        }
        if let Some(key) = bind {
            self.bind_selected_field(&key);
        }
        ui.label("Use {{field_key}} inside any text frame. A field may appear on multiple pages.");

        ui.separator();
        ui.strong("Shared text styles");
        ui.add(
            egui::TextEdit::singleline(&mut self.shared_style_name)
                .hint_text("Style key, e.g. collection_heading"),
        );
        if ui
            .button("Save / Replace Style From Selected Text")
            .clicked()
        {
            let key = self.shared_style_name.trim().to_string();
            if crate::production::valid_key(&key) {
                if let Some(style) = self
                    .current_page()
                    .and_then(|p| p.elements.iter().find(|e| Some(e.id) == self.selected_id))
                    .and_then(crate::production::style_from)
                {
                    self.project.production.styles.insert(key, style);
                }
            } else {
                self.status_banner =
                    "Use a valid style key containing letters, numbers or underscores".into();
            }
        }
        for key in self
            .project
            .production
            .styles
            .keys()
            .cloned()
            .collect::<Vec<_>>()
        {
            ui.horizontal(|ui| {
                ui.label(&key);
                if ui.small_button("Link Selected Text").clicked() {
                    let selected = self.selected_id;
                    if let Some(element) = self
                        .current_page_mut()
                        .and_then(|p| p.elements.iter_mut().find(|e| Some(e.id) == selected))
                    {
                        if !element.locked && matches!(element.kind, ElementKind::Text { .. }) {
                            element.style_id = Some(key.clone());
                        }
                    }
                }
            });
        }
        if ui
            .button("Unlink Selected Layer Style / Photo Binding")
            .clicked()
        {
            let selected = self.selected_id;
            if let Some(element) = self
                .current_page_mut()
                .and_then(|p| p.elements.iter_mut().find(|e| Some(e.id) == selected))
            {
                if !element.locked {
                    element.style_id = None;
                    element.binding = None;
                }
            }
        }

        ui.separator();
        ui.strong("CSV personalization");
        ui.label("Column headers must match defined field keys. Photo paths may be relative to the CSV file. Missing required data and text overflow are reported per record.");
        ui.checkbox(
            &mut self.batch_editable,
            "Also save editable .canvas working copies",
        );
        if self.production_job.is_some() {
            ui.spinner();
            ui.label("Batch running in the background…");
        }
        if ui
            .add_enabled(
                self.production_job.is_none(),
                egui::Button::new("Create Batch From CSV…"),
            )
            .clicked()
        {
            if let Some(csv) = rfd::FileDialog::new()
                .add_filter("CSV", &["csv"])
                .pick_file()
            {
                if let Some(output) = rfd::FileDialog::new().pick_folder() {
                    let (sender, receiver) = std::sync::mpsc::channel();
                    let master = self.project.clone();
                    let editable = self.batch_editable;
                    self.production_job = Some(receiver);
                    self.status_banner =
                        "CSV production started; you can continue editing the master".into();
                    std::thread::spawn(move || {
                        let _ = sender.send(
                            crate::batch_production::export_csv(&master, &csv, &output, editable)
                                .map(ProductionOutcome::Csv),
                        );
                    });
                }
            }
        }
    }
}
