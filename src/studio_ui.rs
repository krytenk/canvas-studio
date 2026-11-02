use super::*;
impl CanvasStudioApp {
    pub(crate) fn studio_template_texture(
        &mut self,
        ui: &Ui,
        id: &str,
    ) -> Option<egui::TextureHandle> {
        if let Some(texture) = self.studio_previews.get(id) {
            return Some(texture.clone());
        }
        let bytes = crate::studio_catalog::template_preview(id).ok()?;
        let image = image::load_from_memory(&bytes).ok()?.to_rgba8();
        let texture = ui.ctx().load_texture(
            format!("studio-template-{id}"),
            egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            ),
            egui::TextureOptions::LINEAR,
        );
        self.studio_previews.insert(id.into(), texture.clone());
        Some(texture)
    }
    pub(crate) fn render_design_gallery(&mut self, ui: &mut Ui) {
        ui.add(
            egui::TextEdit::singleline(&mut self.studio_search)
                .hint_text("Search templates and formats")
                .desired_width(f32::INFINITY),
        );
        ui.add_space(7.);
        egui::ComboBox::from_id_salt("studio-category")
            .width(240.)
            .selected_text(&self.studio_category)
            .show_ui(ui, |ui| {
                for category in [
                    "All templates",
                    "Instagram",
                    "YouTube",
                    "Print",
                    "Presentation",
                    "Identity",
                    "Data",
                ] {
                    ui.selectable_value(&mut self.studio_category, category.to_string(), category);
                }
            });
        ui.add_space(12.);
        let search = self.studio_search.to_lowercase();
        let templates: Vec<_> = available_templates()
            .into_iter()
            .filter(|t| {
                (self.studio_category == "All templates" || self.studio_category == t.category)
                    && format!("{} {} {}", t.name, t.category, t.subtitle)
                        .to_lowercase()
                        .contains(&search)
            })
            .collect();
        ui.label(
            RichText::new(format!("{} considered starting points", templates.len()))
                .size(10.5)
                .color(Color32::from_rgb(139, 151, 163)),
        );
        for tpl in templates {
            ui.add_space(10.);
            let texture = self.studio_template_texture(ui, tpl.id);
            egui::Frame::none()
                .fill(Color32::from_rgb(32, 39, 47))
                .rounding(10.)
                .stroke(Stroke::new(1_f32, Color32::from_rgb(52, 62, 73)))
                .inner_margin(Margin::same(11.))
                .show(ui, |ui| {
                    if let Some(texture) = texture {
                        let width = ui.available_width();
                        ui.horizontal(|ui| {
                            let size = texture.size_vec2();
                            let scale = (width / size.x).min(145. / size.y);
                            let size = size * scale;
                            ui.add_space(((width - size.x) * 0.5).max(0.));
                            if ui
                                .add(
                                    egui::Image::new(&texture)
                                        .fit_to_exact_size(size)
                                        .sense(egui::Sense::click()),
                                )
                                .clicked()
                            {
                                self.request_replace_document(
                                    build_template_project(tpl.id),
                                    None,
                                    format!("Opened {}", tpl.name),
                                );
                            }
                        });
                    }
                    ui.add_space(10.);
                    ui.label(
                        RichText::new(tpl.name)
                            .strong()
                            .size(14.)
                            .color(Color32::from_rgb(235, 241, 245)),
                    );
                    ui.label(
                        RichText::new(tpl.subtitle)
                            .size(10.5)
                            .color(Color32::from_rgb(148, 160, 173)),
                    );
                    ui.add_space(7.);
                    ui.horizontal(|ui| {
                        if ui
                            .add(
                                egui::Button::new("Use template")
                                    .fill(Color32::from_rgb(35, 111, 94)),
                            )
                            .clicked()
                        {
                            self.request_replace_document(
                                build_template_project(tpl.id),
                                None,
                                format!("Opened {}", tpl.name),
                            );
                        }
                        if ui.small_button("Insert pages").clicked() {
                            self.insert_template_pages(&build_template_project(tpl.id));
                        }
                    });
                });
        }
    }
}
pub(crate) fn tool_button(ui: &mut Ui, tab: DockTab, active: bool) -> egui::Response {
    let (_, label) = tab.icon_and_label();
    let label = match tab {
        DockTab::Elements => "Shapes",
        DockTab::Uploads => "Media",
        _ => label,
    };
    let (rect, response) = ui.allocate_exact_size(Vec2::new(56., 48.), egui::Sense::click());
    response.widget_info(|| {
        egui::WidgetInfo::labeled(egui::WidgetType::Button, ui.is_enabled(), label)
    });
    let bg = if active {
        Color32::from_rgb(37, 67, 62)
    } else if response.hovered() {
        Color32::from_rgb(34, 42, 51)
    } else {
        Color32::TRANSPARENT
    };
    let color = if active {
        Color32::from_rgb(133, 223, 195)
    } else {
        Color32::from_rgb(155, 168, 181)
    };
    let painter = ui.painter();
    painter.rect_filled(rect, 8., bg);
    let c = egui::pos2(rect.center().x, rect.min.y + 14.);
    let stroke = Stroke::new(1.3_f32, color);
    match tab {
        DockTab::Templates | DockTab::Puzzles => {
            for y in 0..2 {
                for x in 0..2 {
                    painter.rect_stroke(
                        egui::Rect::from_min_size(
                            c + Vec2::new(x as f32 * 9. - 8., y as f32 * 9. - 8.),
                            Vec2::splat(6.),
                        ),
                        1.5,
                        stroke,
                    );
                }
            }
        }
        DockTab::Text => {
            painter.line_segment([c + Vec2::new(-8., -7.), c + Vec2::new(8., -7.)], stroke);
            painter.line_segment([c + Vec2::new(0., -7.), c + Vec2::new(0., 8.)], stroke);
            painter.line_segment([c + Vec2::new(-5., 8.), c + Vec2::new(5., 8.)], stroke);
        }
        DockTab::Book => {
            painter.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::new(17., 16.)),
                2.,
                stroke,
            );
            painter.line_segment([c + Vec2::new(-3., -8.), c + Vec2::new(-3., 8.)], stroke);
        }
        DockTab::Layers => {
            for dy in [-5., 0., 5.] {
                painter.line_segment([c + Vec2::new(-9., dy), c + Vec2::new(9., dy)], stroke);
            }
        }
        DockTab::Uploads => {
            painter.rect_stroke(
                egui::Rect::from_center_size(c, Vec2::new(19., 15.)),
                2.,
                stroke,
            );
            painter.circle_stroke(c + Vec2::new(4., -3.), 2., stroke);
            painter.line_segment([c + Vec2::new(-8., 6.), c + Vec2::new(-2., 0.)], stroke);
            painter.line_segment([c + Vec2::new(-2., 0.), c + Vec2::new(5., 6.)], stroke);
        }
        DockTab::ChartsAndTables => {
            for (x, h) in [(-6., 6.), (0., 14.), (6., 10.)] {
                painter.line_segment(
                    [c + Vec2::new(x, 8.), c + Vec2::new(x, 8. - h)],
                    Stroke::new(3_f32, color),
                );
            }
        }
        DockTab::Draw => {
            painter.line_segment(
                [c + Vec2::new(-7., 7.), c + Vec2::new(7., -7.)],
                Stroke::new(3_f32, color),
            );
        }
        _ => {
            painter.circle_stroke(c, 8., stroke);
            painter.line_segment([c + Vec2::new(-4., 0.), c + Vec2::new(4., 0.)], stroke);
            painter.line_segment([c + Vec2::new(0., -4.), c + Vec2::new(0., 4.)], stroke);
        }
    }
    painter.text(
        egui::pos2(rect.center().x, rect.min.y + 36.),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(9.5),
        color,
    );
    response
}
