//! Native art direction and render review. WebGL live preview is in the browser surface.
use super::*;
use serde_json::{json, Value};
#[derive(Default)]
pub(crate) struct MockupUi {
    selected: Option<String>,
    identity: Option<(String, u64)>,
    draft: Option<crate::mockups::Mockup>,
    jobs: Vec<Value>,
    poll: f64,
    poster: Option<egui::TextureHandle>,
}
impl CanvasStudioApp {
    fn mockup_command(&mut self, v: Value) {
        let mut p = self.project.clone();
        match crate::mockups::apply(&mut p, &v) {
            Ok(()) => {
                self.record_mutation();
                self.project = p;
                self.status_banner =
                    "Product scene updated · render a fresh snapshot for delivery".into();
            }
            Err(e) => self.status_banner = e,
        }
    }
    pub(crate) fn render_mockup_tools(&mut self, ui: &mut Ui) {
        ui.heading("Product studio · 3D");
        ui.small("Books · product boxes · stationery");
        ui.horizontal_wrapped(|ui| {
            for (kind, label) in [("book", "+ Book"), ("box", "+ Box"), ("cards", "+ Cards")] {
                if ui.button(label).clicked() {
                    self.mockup_command(
                        json!({"action":"mockup_create","page":self.current_page_idx,"kind":kind}),
                    );
                    self.mockup_ui.selected =
                        self.project.production.mockups.last().map(|m| m.id.clone());
                }
            }
        });
        let identity = (self.project.document_id.clone(), self.document_revision);
        if self.mockup_ui.identity.as_ref() != Some(&identity) {
            if self
                .mockup_ui
                .identity
                .as_ref()
                .is_some_and(|v| v.0 != identity.0)
            {
                self.mockup_ui.poster = None;
            }
            self.mockup_ui.draft = self
                .project
                .production
                .mockups
                .iter()
                .find(|m| Some(&m.id) == self.mockup_ui.selected.as_ref())
                .or(self.project.production.mockups.first())
                .cloned();
            self.mockup_ui.selected = self.mockup_ui.draft.as_ref().map(|m| m.id.clone());
            self.mockup_ui.identity = Some(identity);
        }
        let scenes = self.project.production.mockups.clone();
        for m in scenes {
            if ui
                .selectable_label(self.mockup_ui.selected.as_ref() == Some(&m.id), &m.name)
                .clicked()
            {
                self.mockup_ui.selected = Some(m.id.clone());
                self.mockup_ui.draft = Some(m);
            }
        }
        let mut apply = None;
        let mut render = None;
        let mut delete = None;
        if let Some(m) = &mut self.mockup_ui.draft {
            ui.separator();
            ui.text_edit_singleline(&mut m.name);
            egui::ComboBox::from_label("Front artwork")
                .selected_text(
                    self.project
                        .pages
                        .iter()
                        .find(|p| p.id.to_string() == m.page_id)
                        .map(|p| p.title.as_str())
                        .unwrap_or("Missing page"),
                )
                .show_ui(ui, |ui| {
                    for p in &self.project.pages {
                        ui.selectable_value(&mut m.page_id, p.id.to_string(), &p.title);
                    }
                });
            egui::ComboBox::from_label("Back artwork")
                .selected_text(
                    m.back_page
                        .as_ref()
                        .and_then(|id| self.project.pages.iter().find(|p| p.id.to_string() == *id))
                        .map(|p| p.title.as_str())
                        .unwrap_or("Plain back"),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut m.back_page, None, "Plain back");
                    for p in &self.project.pages {
                        ui.selectable_value(&mut m.back_page, Some(p.id.to_string()), &p.title);
                    }
                });
            for (label, v, a, b) in [
                ("Width", &mut m.width, 0.2, 5.),
                ("Height", &mut m.height, 0.2, 5.),
                ("Depth", &mut m.depth, 0.08, 2.),
                ("Turn °", &mut m.yaw, -75., 75.),
                ("Roll °", &mut m.tilt, -20., 20.),
                ("Elevation °", &mut m.elevation, 0., 45.),
                ("Zoom", &mut m.zoom, 0.6, 1.4),
                ("Reveal sweep °", &mut m.sweep, -90., 90.),
                ("Roughness", &mut m.roughness, 0.25, 1.),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.add(egui::DragValue::new(v).range(a..=b).speed(0.05));
                });
            }
            for (label, value) in [
                ("Backdrop", &mut m.background),
                ("Spine / sides", &mut m.cover_color),
            ] {
                ui.horizontal(|ui| {
                    ui.label(label);
                    ui.text_edit_singleline(value);
                });
            }
            egui::ComboBox::from_label("Light")
                .selected_text(&m.light)
                .show_ui(ui, |ui| {
                    for v in ["warm", "neutral"] {
                        ui.selectable_value(&mut m.light, v.into(), v);
                    }
                });
            ui.horizontal(|ui| {
                ui.label("Output width");
                ui.add(egui::DragValue::new(&mut m.output_width).range(256..=2400));
            });
            ui.horizontal(|ui| {
                ui.label("Output height");
                ui.add(egui::DragValue::new(&mut m.output_height).range(256..=2400));
            });
            if ui.button("Apply product scene").clicked() {
                let mut v = serde_json::to_value(&m).unwrap();
                v["action"] = json!("mockup_update");
                v["page"] = json!(self
                    .project
                    .pages
                    .iter()
                    .position(|p| p.id.to_string() == m.page_id)
                    .unwrap_or(self.current_page_idx));
                apply = Some(v);
            }
            let saved = self
                .project
                .production
                .mockups
                .iter()
                .find(|s| s.id == m.id);
            let changed = saved != Some(m);
            ui.add_enabled_ui(!changed, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("Render listing image").clicked() {
                        render = Some((m.id.clone(), false));
                    }
                    if ui.button("Render 6s reveal").clicked() {
                        render = Some((m.id.clone(), true));
                    }
                });
            });
            if changed {
                ui.small("Apply scene edits before rendering.");
            }
            if ui.small_button("Delete scene").clicked() {
                delete = Some(m.id.clone());
            }
        }
        if let Some(v) = apply {
            self.mockup_command(v)
        }
        if let Some(id) = delete {
            self.mockup_command(json!({"action":"mockup_delete","id":id}))
        }
        if let Some((id, movie)) = render {
            match crate::mockups::start(&self.project, self.document_revision, &id, movie) {
                Ok(_) => {
                    self.status_banner = "Product render queued · editing remains available".into();
                    self.mockup_ui.poll = 0.;
                }
                Err(e) => self.status_banner = e,
            }
        }
        ui.small("Live 3D runs in the browser editor. Native/print use saved poster snapshots. Match the scene proportions to your cover page.");
        let now = ui.input(|i| i.time);
        if now - self.mockup_ui.poll > 1. {
            self.mockup_ui.jobs = crate::media_service::jobs(&self.project.document_id)
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
            self.mockup_ui.poll = now;
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
        for job in self
            .mockup_ui
            .jobs
            .clone()
            .into_iter()
            .filter(|j| j["mode"].as_str().is_some_and(|v| v.starts_with("mockup_")))
        {
            ui.separator();
            ui.label(format!(
                "{} · {}",
                job["mode"].as_str().unwrap_or("Product"),
                job["state"].as_str().unwrap_or("")
            ));
            if let Some(e) = job["error"].as_str() {
                ui.small(e);
            }
            let id = job["id"].as_str().unwrap_or("");
            if job["state"] == "running" {
                if ui.button("Cancel product render").clicked() {
                    let _ = crate::media_service::cancel(id, &self.project.document_id);
                }
            }
            if job["state"] == "complete" {
                let movie = job["mode"] == "mockup_movie";
                if !movie && ui.button("Review poster").clicked() {
                    if let Ok(path) = crate::media_service::output(id, &self.project.document_id) {
                        if let Ok(image) = image::open(path) {
                            let rgba = image.to_rgba8();
                            self.mockup_ui.poster = Some(ui.ctx().load_texture(
                                "product-poster",
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [rgba.width() as usize, rgba.height() as usize],
                                    &rgba,
                                ),
                                Default::default(),
                            ));
                        }
                    }
                }
                if ui
                    .button(if movie {
                        "Add reveal to Film"
                    } else {
                        "Place poster on current page"
                    })
                    .clicked()
                {
                    self.mockup_command(json!({"action":if movie{"mockup_film"}else{"mockup_place"},"id":job["result"]["scene_id"],"job":id,"page":self.current_page_idx,"start":0}));
                }
                if ui
                    .button(if movie { "Save MP4…" } else { "Save PNG…" })
                    .clicked()
                {
                    if let Some(dest) = rfd::FileDialog::new()
                        .set_file_name(if movie {
                            "product-reveal.mp4"
                        } else {
                            "product-poster.png"
                        })
                        .save_file()
                    {
                        self.status_banner =
                            match crate::media_service::output(id, &self.project.document_id)
                                .and_then(|p| {
                                    std::fs::copy(p, dest)
                                        .map(|_| ())
                                        .map_err(|e| e.to_string())
                                }) {
                                Ok(()) => "Product render saved".into(),
                                Err(e) => e,
                            };
                    }
                }
                if ui.small_button("Save editable source ZIP…").clicked() {
                    if let Some(dest) = rfd::FileDialog::new()
                        .set_file_name("product-source.zip")
                        .save_file()
                    {
                        self.status_banner = match crate::media_service::source_output(
                            id,
                            &self.project.document_id,
                        )
                        .and_then(|p| {
                            std::fs::copy(p, dest)
                                .map(|_| ())
                                .map_err(|e| e.to_string())
                        }) {
                            Ok(()) => "Editable 3D source saved".into(),
                            Err(e) => e,
                        };
                    }
                }
            }
        }
        if let Some(texture) = &self.mockup_ui.poster {
            ui.add(
                egui::Image::new(texture)
                    .fit_to_exact_size(egui::vec2(ui.available_width(), ui.available_width())),
            );
        }
    }
}
