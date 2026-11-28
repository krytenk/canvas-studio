//! Native generation workspace, deliberately separate from the print canvas and video edit.
use super::*;
use serde_json::{json, Value};
pub(crate) struct GenerationUi {
    pub(crate) task: String,
    model: String,
    prompt: String,
    negative: String,
    content: String,
    commercial: bool,
    ack: bool,
    width: u32,
    height: u32,
    steps: u32,
    guidance: f32,
    seed: u32,
    precision: String,
    quantization: String,
    offload: bool,
    cpu: bool,
    limit: u32,
    frames: u32,
    fps: u32,
    reference: bool,
    reference_ids: Vec<String>,
    reference_strength: f32,
    reference_sets: Vec<Value>,
    selected_set: String,
    set_name: String,
    reference_document: String,
    reference_fit: String,
    reference_textures: Vec<egui::TextureHandle>,
    adapter: String,
    search: String,
    provider: String,
    mature: bool,
    kaggle_user: String,
    kaggle_token: String,
    hf_token: String,
    civitai_token: String,
    cache: String,
    initialized: bool,
    jobs: Vec<Value>,
    poll: f64,
    previews: std::collections::BTreeMap<String, egui::TextureHandle>,
}
impl Default for GenerationUi {
    fn default() -> Self {
        Self {
            task: "image".into(),
            model: "SG161222/RealVisXL_V5.0".into(),
            prompt: String::new(),
            negative: "watermark, distorted hands, unreadable text".into(),
            content: "flexible".into(),
            commercial: true,
            ack: false,
            width: 768,
            height: 1024,
            steps: 28,
            guidance: 5.,
            seed: 20261127,
            precision: "fp16".into(),
            quantization: "none".into(),
            offload: true,
            cpu: false,
            limit: 20,
            frames: 33,
            fps: 8,
            reference: false,
            reference_ids: Vec::new(),
            reference_strength: 1.,
            reference_sets: Vec::new(),
            selected_set: String::new(),
            set_name: String::new(),
            reference_document: String::new(),
            reference_fit: "contain".into(),
            reference_textures: Vec::new(),
            adapter: String::new(),
            search: String::new(),
            provider: "huggingface".into(),
            mature: false,
            kaggle_user: String::new(),
            kaggle_token: String::new(),
            hf_token: String::new(),
            civitai_token: String::new(),
            cache: String::new(),
            initialized: false,
            jobs: Vec::new(),
            poll: 0.,
            previews: Default::default(),
        }
    }
}
impl CanvasStudioApp {
    fn generation_set_references(
        &mut self,
        ids: Vec<String>,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        let mut textures = Vec::new();
        for id in &ids {
            let bytes = crate::generation::reference_bytes(&self.project.document_id, id)?;
            let image = image::load_from_memory(&bytes)
                .map_err(|e| e.to_string())?
                .thumbnail(88, 110)
                .to_rgba8();
            let pixels = egui::ColorImage::from_rgba_unmultiplied(
                [image.width() as usize, image.height() as usize],
                image.as_raw(),
            );
            textures.push(ctx.load_texture(
                format!("reference-{id}"),
                pixels,
                egui::TextureOptions::LINEAR,
            ));
        }
        self.generation_ui.reference_ids = ids;
        self.generation_ui.reference_textures = textures;
        self.generation_ui.reference = false;
        Ok(())
    }
    fn generation_select_model(&mut self, m: &Value) {
        let g = &mut self.generation_ui;
        g.model = m["id"].as_str().unwrap().into();
        g.steps = m["steps"].as_u64().unwrap_or(28) as u32;
        g.guidance = m["guidance"].as_f64().unwrap_or(5.) as f32;
        g.precision = m["preferred_precision"].as_str().unwrap_or("fp16").into();
        g.limit = m["recommended_download_limit_gb"].as_u64().unwrap_or(20) as u32;
        g.width = m["default_width"].as_u64().unwrap_or(768) as u32;
        g.height = m["default_height"]
            .as_u64()
            .unwrap_or(if g.task == "video" { 448 } else { 1024 }) as u32;
        g.frames = m["default_frames"].as_u64().unwrap_or(33) as u32;
        g.fps = m["default_fps"].as_u64().unwrap_or(8) as u32;
        if let Some(easy) = m.get("easy") {
            g.width = easy["width"].as_u64().unwrap_or(g.width as u64) as u32;
            g.height = easy["height"].as_u64().unwrap_or(g.height as u64) as u32;
            g.steps = easy["steps"].as_u64().unwrap_or(g.steps as u64) as u32;
            g.guidance = easy["guidance"].as_f64().unwrap_or(g.guidance as f64) as f32;
            g.adapter = easy["adapter"].as_str().unwrap_or("").into();
        } else {
            g.adapter.clear();
        }
        g.ack = false;
    }
    fn generation_action(&mut self, value: Value) {
        match crate::generation::start(&self.project, self.document_revision, value) {
            Ok(_) => {
                self.status_banner = "Studio task queued · edit your project while it runs".into();
                self.generation_ui.poll = 0.;
            }
            Err(e) => self.status_banner = e,
        }
    }
    fn generation_request(&self, operation: &str) -> Value {
        let g = &self.generation_ui;
        let mut v = json!({"operation":operation,"model_id":g.model,"prompt":g.prompt,"negative_prompt":g.negative,"width":g.width,"height":g.height,"steps":g.steps,"guidance":g.guidance,"seed":g.seed,"precision":g.precision,"quantization":g.quantization,"offload":g.offload,"device":if g.cpu{"cpu"}else{"cuda"},"content_mode":g.content,"use":if g.commercial{"commercial"}else{"research"},"license_acknowledged":g.ack,"download_limit_gb":g.limit,"frames":g.frames,"fps":g.fps,"timeout":1800});
        v["reference_fit"] = json!(g.reference_fit);
        v["reference_strength"] = json!(g.reference_strength);
        let supports = crate::generation::catalog()["models"]
            .as_array()
            .unwrap()
            .iter()
            .find(|m| m["id"] == g.model)
            .is_some_and(|m| {
                [
                    "Flux2KleinPipeline",
                    "QwenImage21Pipeline",
                    "WanImageToVideoPipeline",
                    "LTXImageToVideoPipeline",
                    "WanVACEPipeline",
                ]
                .contains(&m["pipeline"].as_str().unwrap_or(""))
            });
        if supports && !g.reference_ids.is_empty() {
            v["reference_ids"] = json!(g.reference_ids);
        } else if supports && g.reference {
            v["reference_page"] = json!(self.current_page_idx);
        }
        {
            v["adapter"] = json!(g.adapter);
        }
        v
    }
    pub(crate) fn render_generation_studio(&mut self, ui: &mut Ui) {
        let state = crate::generation::state();
        let models = state["catalog"]["models"]
            .as_array()
            .cloned()
            .unwrap_or_default();
        let connection = &state["connections"];
        if self.generation_ui.reference_document != self.project.document_id {
            let g = &mut self.generation_ui;
            g.reference_document = self.project.document_id.clone();
            g.reference_ids.clear();
            g.reference_textures.clear();
            g.selected_set.clear();
            g.reference = false;
            g.previews.clear();
        }
        if !self.generation_ui.initialized {
            self.generation_ui.kaggle_user =
                connection["kaggle_username"].as_str().unwrap_or("").into();
            self.generation_ui.cache = connection["cache_dir"].as_str().unwrap_or("").into();
            if let Some(model) = models.iter().find(|m| m["id"] == self.generation_ui.model) {
                self.generation_select_model(model);
            }
            self.generation_ui.initialized = true;
        }
        let now = ui.input(|i| i.time);
        if now - self.generation_ui.poll > 1. || self.generation_ui.poll == 0. {
            self.generation_ui.jobs = crate::media_service::jobs(&self.project.document_id)
                .ok()
                .and_then(|v| v["jobs"].as_array().cloned())
                .unwrap_or_default();
            self.generation_ui.reference_sets = crate::generation::reference_sets()["sets"]
                .as_array()
                .cloned()
                .unwrap_or_default();
            self.generation_ui.poll = now;
        }
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_secs(1));
        let connected = connection["kaggle_configured"] == true
            && connection["kaggle_username"]
                .as_str()
                .is_some_and(|v| !v.is_empty());
        let busy = self.generation_ui.jobs.iter().any(|j| {
            j["state"] == "running"
                && ["ai_image", "ai_video"].contains(&j["mode"].as_str().unwrap_or(""))
        });
        ScrollArea::vertical().show(ui,|ui|{
            ui.set_max_width(900.);
            egui::Frame::none().fill(Color32::from_rgb(26,43,41)).rounding(14.).inner_margin(20.).show(ui,|ui|{
                ui.label(RichText::new("CANVAS STUDIO / CREATE").small().color(Color32::from_rgb(177,220,197)));
                ui.heading(RichText::new("What would you like to make?").size(26.).color(Color32::from_rgb(235,243,233)));
                ui.label("Choose a model, describe your idea, and generate.");
                ui.horizontal_wrapped(|ui|{
                    for (kind,label) in [("image","Images"),("video","Video")] {
                        if ui.selectable_label(self.generation_ui.task==kind,label).clicked(){self.generation_ui.task=kind.into();if let Some(m)=models.iter().find(|m|m["task"]==kind && m["runnable"]==true && m["tier"]=="practical" && m["reference_required"]!=true){self.generation_select_model(m);}}
                    }
                    if ui.button("Video editor").clicked(){self.screen=WorkspaceScreen::StudioEditor;self.active_dock=DockTab::Film;}
                });
            });
            ui.add_space(18.);ui.label(RichText::new("1 · Choose a model").strong().size(16.));
            let selected_task=self.generation_ui.task.clone();egui::ComboBox::from_id_salt("simple-generation-model").width(420.).selected_text(models.iter().find(|m|m["id"]==self.generation_ui.model).and_then(|m|m["name"].as_str()).unwrap_or("Choose a model")).show_ui(ui,|ui|{
                for m in models.iter().filter(|m|m["task"]==selected_task && m["runnable"]==true){if ui.selectable_value(&mut self.generation_ui.model,m["id"].as_str().unwrap_or("").into(),m["name"].as_str().unwrap_or("Model")).clicked(){self.generation_select_model(m);}}
            });
            let model=models.iter().find(|m|m["id"]==self.generation_ui.model).cloned().unwrap_or(json!({}));
            ui.collapsing("About this model",|ui|{ui.label(model["notes"].as_str().unwrap_or(""));ui.hyperlink_to("Model details and licence",model["url"].as_str().unwrap_or("https://huggingface.co"));});
            ui.add_space(12.);ui.label(RichText::new("2 · Describe your idea").strong().size(16.));
            ui.add(egui::TextEdit::multiline(&mut self.generation_ui.prompt).desired_rows(4).desired_width(f32::INFINITY).hint_text("A cinematic portrait, a beautiful product shot, a new story…"));
            let supports=["Flux2KleinPipeline","QwenImage21Pipeline","WanImageToVideoPipeline","LTXImageToVideoPipeline","WanVACEPipeline"].contains(&model["pipeline"].as_str().unwrap_or(""));
            let maximum=model["reference_max"].as_u64().unwrap_or(1) as usize;
            if supports {
                ui.add_space(12.);ui.group(|ui|{
                    ui.strong("Reference photos");ui.small(if maximum>1{"Use photos of the same character."}else{"Choose the photo you want to animate or edit."});
                    let mut load=None;egui::ComboBox::from_id_salt("simple-saved-character").width(300.).selected_text(self.generation_ui.reference_sets.iter().find(|v|v["id"]==self.generation_ui.selected_set).and_then(|v|v["name"].as_str()).unwrap_or("Use saved photos…")).show_ui(ui,|ui|{for set in &self.generation_ui.reference_sets{if ui.selectable_label(set["id"]==self.generation_ui.selected_set,set["name"].as_str().unwrap_or("Character")).clicked(){load=Some(set.clone());}}});
                    if let Some(set)=load{let result=(||{if set["count"].as_u64().unwrap_or(0)>maximum as u64{return Err(format!("This model accepts up to {maximum} photos. Choose a multi-reference model for this set."));}let value=crate::generation::reference_set_action(&self.project.document_id,&json!({"operation":"load","id":set["id"]}))?;let ids=value["references"].as_array().unwrap().iter().map(|r|r["id"].as_str().unwrap().into()).collect();self.generation_set_references(ids,ui.ctx())?;self.generation_ui.selected_set=set["id"].as_str().unwrap().into();Ok(())})();if let Err(e)=result{self.status_banner=e;}}
                    ui.horizontal(|ui|{for (append,label) in [(false,"Choose photos…"),(true,"Add more…")]{if (append && self.generation_ui.reference_ids.is_empty()) || append && self.generation_ui.reference_ids.len()>=maximum{continue;}if ui.button(label).clicked(){if let Some(paths)=rfd::FileDialog::new().add_filter("Photos",&["png","jpg","jpeg","webp"]).pick_files(){let result=(||{let mut ids=if append{self.generation_ui.reference_ids.clone()}else{self.generation_ui.reference_ids.clear();self.generation_ui.reference_textures.clear();Vec::new()};if paths.len()+ids.len()>maximum{return Err(format!("Choose up to {maximum} photos for this model"));}for path in paths{if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>25_000_000{return Err("Choose photos smaller than 25 MB".into());}let info=crate::generation::upload_reference(&self.project.document_id,&std::fs::read(path).map_err(|e|e.to_string())?)?;let id=info["id"].as_str().unwrap().to_string();if ids.contains(&id){return Err("This photo is already selected".into());}ids.push(id);}self.generation_set_references(ids,ui.ctx())?;self.generation_ui.selected_set.clear();Ok(())})();if let Err(e)=result{self.status_banner=e;}}}}});
                    let mut remove=None;ui.horizontal_wrapped(|ui|{for (i,texture) in self.generation_ui.reference_textures.iter().enumerate(){ui.vertical(|ui|{ui.image(texture);if ui.small_button("Remove").clicked(){remove=Some(i);}});}});if let Some(i)=remove{self.generation_ui.reference_ids.remove(i);let _=self.generation_ui.reference_textures.remove(i);self.generation_ui.selected_set.clear();}
                    ui.collapsing("Photo options",|ui|{
                        if ui.checkbox(&mut self.generation_ui.reference,"Use current design page instead").changed() && self.generation_ui.reference {self.generation_ui.reference_ids.clear();self.generation_ui.reference_textures.clear();}
                        egui::ComboBox::from_label("Framing").selected_text(&self.generation_ui.reference_fit).show_ui(ui,|ui|{ui.selectable_value(&mut self.generation_ui.reference_fit,"contain".into(),"Keep the whole photo");ui.selectable_value(&mut self.generation_ui.reference_fit,"cover".into(),"Fill the frame");});
                        if maximum>1{ui.add(egui::Slider::new(&mut self.generation_ui.reference_strength,0.5..=2.).text("Influence"));}
                        ui.add(egui::TextEdit::singleline(&mut self.generation_ui.set_name).hint_text("Name these photos"));if ui.button("Save these photos").clicked(){match crate::generation::reference_set_action(&self.project.document_id,&json!({"operation":"save","name":self.generation_ui.set_name,"reference_ids":self.generation_ui.reference_ids})){Ok(_)=>{self.generation_ui.poll=0.;self.status_banner="Photos saved for next time".into();},Err(e)=>self.status_banner=e}}
                    });
                });
            }
            ui.add_space(12.);ui.collapsing("More options",|ui|{
                ui.label("Recommended settings are already selected.");
                ui.label("Negative prompt");ui.text_edit_multiline(&mut self.generation_ui.negative);
                ui.horizontal_wrapped(|ui|{for (label,value) in [("Width",&mut self.generation_ui.width),("Height",&mut self.generation_ui.height),("Steps",&mut self.generation_ui.steps)]{ui.label(label);ui.add(egui::DragValue::new(value));}ui.label("Guidance");ui.add(egui::DragValue::new(&mut self.generation_ui.guidance).speed(0.1));});
                ui.horizontal(|ui|{ui.label("Seed");ui.add(egui::DragValue::new(&mut self.generation_ui.seed));});
                if model["task"]=="video"{ui.horizontal(|ui|{ui.label("Frames");ui.add(egui::DragValue::new(&mut self.generation_ui.frames));ui.label("FPS");egui::ComboBox::from_id_salt("simple-fps").selected_text(self.generation_ui.fps.to_string()).show_ui(ui,|ui|{for fps in [8,16,24]{ui.selectable_value(&mut self.generation_ui.fps,fps,fps.to_string());}});});}
                egui::ComboBox::from_label("Precision").selected_text(&self.generation_ui.precision).show_ui(ui,|ui|{for v in ["fp16","bf16","fp32"]{ui.selectable_value(&mut self.generation_ui.precision,v.into(),v);}});
                egui::ComboBox::from_label("Quantization").selected_text(&self.generation_ui.quantization).show_ui(ui,|ui|{for v in ["none","nf4"]{ui.selectable_value(&mut self.generation_ui.quantization,v.into(),v);}});
                egui::ComboBox::from_label("Adapter").selected_text(if self.generation_ui.adapter.is_empty(){"None"}else{"SDXL Lightning"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.generation_ui.adapter,String::new(),"None");ui.selectable_value(&mut self.generation_ui.adapter,"sdxl-lightning-4".into(),"SDXL Lightning");});
                ui.checkbox(&mut self.generation_ui.offload,"Automatic memory offload");ui.checkbox(&mut self.generation_ui.cpu,"Use local CPU");ui.horizontal(|ui|{ui.label("Download limit · GiB");ui.add(egui::DragValue::new(&mut self.generation_ui.limit));});
                ui.checkbox(&mut self.generation_ui.commercial,"Commercial / advertising use");ui.checkbox(&mut self.generation_ui.ack,"I have reviewed this model's terms and confirmed my rights");
                egui::ComboBox::from_label("Output check").selected_text(&self.generation_ui.content).show_ui(ui,|ui|{ui.selectable_value(&mut self.generation_ui.content,"flexible".into(),"Model defaults");ui.selectable_value(&mut self.generation_ui.content,"standard".into(),"Additional output check");});
                ui.collapsing("Find more models",|ui|{egui::ComboBox::from_label("Provider").selected_text(&self.generation_ui.provider).show_ui(ui,|ui|{for v in ["huggingface","civitai"]{ui.selectable_value(&mut self.generation_ui.provider,v.into(),v);}});ui.text_edit_singleline(&mut self.generation_ui.search);ui.checkbox(&mut self.generation_ui.mature,"Include mature model listings");if ui.button("Search models").clicked(){self.generation_action(json!({"operation":"search","provider":self.generation_ui.provider,"query":self.generation_ui.search,"task":self.generation_ui.task,"include_mature":self.generation_ui.mature}));}for job in self.generation_ui.jobs.clone(){if let Some(rows)=job["result"]["models"].as_array(){for m in rows{ui.horizontal_wrapped(|ui|{ui.hyperlink_to(m["name"].as_str().unwrap_or("Model"),m["url"].as_str().unwrap_or("https://huggingface.co"));if ui.small_button("Add compatible recipe").clicked(){self.generation_action(json!({"operation":"register","provider":m["provider"],"id":m["id"],"version_id":m["version_id"]}));}});}}}});
                ui.collapsing("Developer tools",|ui|{for (operation,label) in [("prepare","Prepare notebook"),("submit","Submit only"),("generate","Generate locally")]{if ui.button(label).clicked(){self.generation_action(self.generation_request(operation));}}});
            });
            if model["commercial"]=="research" && self.generation_ui.commercial {ui.small("This model is for personal research. Choose a commercial model or change project use in More options.");}
            ui.add_space(12.);let photos_ready=model["reference_required"]!=true || self.generation_ui.reference || !self.generation_ui.reference_ids.is_empty() && self.generation_ui.reference_ids.len()<=maximum;
            let valid=connected && !busy && photos_ready && !self.generation_ui.prompt.trim().is_empty() && model["runnable"]==true;
            if ui.add_enabled(valid,egui::Button::new(RichText::new(if busy{"Generating…"}else{"Generate"}).size(19.).strong().color(Color32::from_rgb(21,59,45))).fill(Color32::from_rgb(198,229,201)).min_size(egui::vec2(220.,48.))).clicked(){self.generation_action(self.generation_request("create"));}
            ui.small(if connected{"Kaggle runs in the background. Your finished creation appears here automatically."}else{"Connect Kaggle once below to get started."});
            ui.add_space(20.);ui.heading("Your creations");
            let creations=self.generation_ui.jobs.clone().into_iter().filter(|j|["ai_image","ai_video"].contains(&j["mode"].as_str().unwrap_or(""))).collect::<Vec<_>>();
            if creations.is_empty(){ui.label("Your next creation starts with an idea.");}
            for job in creations.iter().take(8){let id=job["id"].as_str().unwrap_or("");ui.push_id(id,|ui|{ui.group(|ui|{
                if job["state"]=="running" {ui.horizontal(|ui|{ui.spinner();ui.strong(job["phase"].as_str().unwrap_or("Getting ready"));if let Some(seconds)=job["elapsed_seconds"].as_u64(){ui.small(format!("{}m {}s",seconds/60,seconds%60));}});}
                else if job["state"]=="complete" {
                    ui.strong(if job["mode"]=="ai_video"{"Your video is ready"}else{"Your image is ready"});
                    if job["mode"]=="ai_image" {
                        if !self.generation_ui.previews.contains_key(id){if let Ok(path)=crate::media_service::output(id,&self.project.document_id){if let Ok(image)=image::open(path){let image=image.thumbnail(500,250).to_rgba8();let pixels=egui::ColorImage::from_rgba_unmultiplied([image.width() as usize,image.height() as usize],image.as_raw());self.generation_ui.previews.insert(id.into(),ui.ctx().load_texture(format!("creation-{id}"),pixels,egui::TextureOptions::LINEAR));}}}
                        if let Some(texture)=self.generation_ui.previews.get(id){ui.image(texture);}
                    }
                    ui.horizontal_wrapped(|ui|{
                        if ui.button(if job["mode"]=="ai_video"{"Use in video"}else{"Use in design"}).clicked(){let mut next=self.project.clone();match crate::generation::import(&mut next,&json!({"id":id,"page":self.current_page_idx})){Ok(())=>{self.record_mutation();self.project=next;self.screen=WorkspaceScreen::StudioEditor;if job["mode"]=="ai_video"{self.active_dock=DockTab::Film;}self.status_banner="Creation added".into();},Err(e)=>self.status_banner=e}}
                        if job["mode"]=="ai_video" && ui.button("Play video").clicked(){if let Ok(path)=crate::media_service::output(id,&self.project.document_id){if let Err(e)=std::process::Command::new("ffplay").args(["-autoexit","-loglevel","error"]).arg(path).spawn(){self.status_banner=format!("Could not open the video player: {e}");}}}
                        if ui.button("Save…").clicked(){if let Some(path)=rfd::FileDialog::new().set_file_name(if job["mode"]=="ai_video"{"creation.mp4"}else{"creation.png"}).save_file(){self.status_banner=match crate::media_service::output(id,&self.project.document_id).and_then(|source|fs_copy(&source,&path)){Ok(())=>"Creation saved".into(),Err(e)=>e};}}
                    });
                } else {ui.strong("Generation needs attention");ui.collapsing("Show details",|ui|{ui.label(job["error"].as_str().unwrap_or("Generation was stopped"));});if let Some(run)=job["generation"]["run_id"].as_str(){if ui.button("Continue").clicked(){self.generation_action(json!({"operation":"create","run_id":run}));}}}
            });});}
            ui.add_space(14.);egui::CollapsingHeader::new(if connected{"Connection settings"}else{"Connect Kaggle"}).default_open(!connected).show(ui,|ui|{
                ui.small("Your saved connection is reused each time. Use an API token, not your Google password.");
                ui.label("Kaggle username");ui.text_edit_singleline(&mut self.generation_ui.kaggle_user);
                for (label,value) in [("Kaggle API token",&mut self.generation_ui.kaggle_token),("Hugging Face token",&mut self.generation_ui.hf_token),("Civitai API key",&mut self.generation_ui.civitai_token)]{ui.label(label);ui.add(egui::TextEdit::singleline(value).password(true).hint_text("Leave blank to keep saved value"));}
                if ui.button("Save connection").clicked(){match crate::generation::save_connections(json!({"kaggle_username":self.generation_ui.kaggle_user,"kaggle_token":self.generation_ui.kaggle_token,"hf_token":self.generation_ui.hf_token,"civitai_token":self.generation_ui.civitai_token})){Ok(())=>{self.generation_ui.kaggle_token.clear();self.generation_ui.hf_token.clear();self.generation_ui.civitai_token.clear();self.status_banner="Connection saved".into();},Err(e)=>self.status_banner=e}}
                ui.hyperlink_to("Get a Kaggle API token","https://www.kaggle.com/settings/api");
            });
            ui.collapsing("Technical history",|ui|{
                if ui.button("Clear finished generation tasks").clicked(){match crate::media_service::clear_finished_kind(&self.project.document_id,true){Ok(_)=>{self.generation_ui.poll=0.;self.generation_ui.previews.clear();},Err(e)=>self.status_banner=e}}
                for job in self.generation_ui.jobs.clone().iter().filter(|j|j["mode"].as_str().is_some_and(|m|m.starts_with("ai_"))){ui.push_id(job["id"].to_string(),|ui|{ui.collapsing(format!("{} · {}",job["mode"],job["state"]),|ui|{if job["state"]=="running" && ui.button("Stop local waiting").clicked(){let _=crate::media_service::cancel(job["id"].as_str().unwrap_or(""),&self.project.document_id);}if let Some(error)=job["error"].as_str(){ui.label(error);}if let Some(link)=job["result"]["url"].as_str(){ui.hyperlink_to("Kaggle details",link);}if job["result"]["source_available"]==true && ui.button("Save notebook ZIP").clicked(){if let Some(path)=rfd::FileDialog::new().set_file_name("inference.zip").save_file(){if let Ok(source)=crate::media_service::source_output(job["id"].as_str().unwrap_or(""),&self.project.document_id){let _=fs_copy(&source,&path);}}}});});}
            });
        });
    }
}
fn fs_copy(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::copy(src, dest)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
