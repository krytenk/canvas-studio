use super::*;
use serde_json::json;
impl CanvasStudioApp {
    fn apply_studio_command(&mut self, value: serde_json::Value) {
        let mut candidate = self.project.clone();
        match crate::layout_commands::apply(&mut candidate, &value) {
            Ok(()) => {
                self.record_mutation();
                self.project = candidate;
                self.current_page_idx = self
                    .current_page_idx
                    .min(self.project.pages.len().saturating_sub(1));
                self.selected_id = None;
                self.canvas_state.selected_ids.clear();
                self.pending_canvas_fit = true;
                self.status_banner = "Campaign updated · Undo is available".into();
            }
            Err(e) => self.status_banner = e,
        }
    }
    pub(crate) fn render_campaign_layouts(&mut self, ui: &mut Ui) {
        ui.collapsing("Campaign layouts",|ui| {
            ui.label("Independent compositions. Shared copy, colours and reusable groups.");
            if ui.button("Start a four-format campaign").clicked(){self.request_replace_document(crate::campaign::starter(),None,"Started a linked campaign".into());}
            if let Some(active)=&self.project.production.studio.active_variant{ui.strong(format!("Current: {active}"));}
            let layouts=self.project.production.studio.variants.iter().map(|(name,v)|(name.clone(),v.width,v.height)).collect::<Vec<_>>();
            for (name,w,h) in layouts{if ui.button(format!("{name} · {w:.0} × {h:.0}")).clicked(){self.apply_studio_command(json!({"action":"variant_activate","name":name}));}}
            ui.text_edit_singleline(&mut self.campaign_name);
            if ui.button("Save current as another layout").clicked(){self.apply_studio_command(json!({"action":"variant_save","name":self.campaign_name}));}
            ui.label(RichText::new("Each layout keeps its own size and pages. Save the whole campaign as a portable .canvas project.").small());
        });
    }
    pub(crate) fn render_campaign_brand(&mut self, ui: &mut Ui) {
        ui.heading("Linked brand colours");
        ui.label("Change a named colour across every campaign layout.");
        for (key, color) in self.project.production.studio.tokens.clone() {
            ui.horizontal(|ui|{let mut value=color;let mut swatch=to_egui_color(value);if ui.color_edit_button_srgba(&mut swatch).changed(){value=[swatch.r(),swatch.g(),swatch.b(),255];self.apply_studio_command(json!({"action":"brand_token","key":key,"color":crate::campaign::color_hex(value)}));}ui.label(&key);if ui.small_button("Link selection").clicked(){let ids=self.current_page().map(|p|document_ops::selection_ids(p,&self.canvas_state.selected_ids,self.selected_id)).unwrap_or_default();self.apply_studio_command(json!({"action":"brand_bind","page":self.current_page_idx,"ids":ids.iter().map(u64::to_string).collect::<Vec<_>>(),"token":key,"target":"primary"}));}});
        }
        ui.horizontal(|ui| {
            ui.text_edit_singleline(&mut self.brand_key);
            color_Edit_button(ui, &mut self.brand_value);
        });
        if ui.button("Save named colour").clicked() {
            self.apply_studio_command(json!({"action":"brand_token","key":self.brand_key,"color":crate::campaign::color_hex(self.brand_value)}));
        }
        if ui.button("Unlink selection colour").clicked() {
            let ids = self
                .current_page()
                .map(|p| {
                    document_ops::selection_ids(
                        p,
                        &self.canvas_state.selected_ids,
                        self.selected_id,
                    )
                })
                .unwrap_or_default();
            self.apply_studio_command(json!({"action":"brand_bind","page":self.current_page_idx,"ids":ids.iter().map(u64::to_string).collect::<Vec<_>>(),"token":null,"target":"primary"}));
        }
        ui.separator();
        ui.heading("Reusable groups");
        ui.label("Inserted groups keep field/colour links and independent geometry.");
        ui.text_edit_singleline(&mut self.component_key);
        if ui.button("Save selection as a reusable group").clicked() {
            let ids = self
                .current_page()
                .map(|p| {
                    document_ops::selection_ids(
                        p,
                        &self.canvas_state.selected_ids,
                        self.selected_id,
                    )
                })
                .unwrap_or_default();
            self.apply_studio_command(json!({"action":"component_save","page":self.current_page_idx,"ids":ids.iter().map(u64::to_string).collect::<Vec<_>>(),"key":self.component_key,"name":self.component_key}));
        }
        for (key, c) in self.project.production.studio.components.clone() {
            if ui.button(format!("Insert {}", c.name)).clicked() {
                self.apply_studio_command(json!({"action":"component_insert","page":self.current_page_idx,"key":key,"x":60,"y":60,"width":c.width.min(self.project.width-120.)}));
            }
        }
    }
}
