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
    image_to_video: bool,
    reference_ids: Vec<String>,
    multi_reference: bool,
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
}
impl Default for GenerationUi {
    fn default() -> Self {
        Self{task:"image".into(),model:"SG161222/RealVisXL_V5.0".into(),prompt:"Editorial photograph of an adult fashion creator, natural skin texture, considered styling, soft window light, authentic expression, 85mm portrait lens".into(),negative:"watermark, distorted hands, unreadable text".into(),content:"flexible".into(),commercial:true,ack:false,width:768,height:1024,steps:28,guidance:5.,seed:20261127,precision:"fp16".into(),quantization:"none".into(),offload:true,cpu:false,limit:20,frames:33,fps:8,reference:false,image_to_video:false,reference_ids:Vec::new(),multi_reference:false,reference_strength:1.,reference_sets:Vec::new(),selected_set:String::new(),set_name:String::new(),reference_document:String::new(),reference_fit:"contain".into(),reference_textures:Vec::new(),adapter:String::new(),search:String::new(),provider:"huggingface".into(),mature:false,kaggle_user:String::new(),kaggle_token:String::new(),hf_token:String::new(),civitai_token:String::new(),cache:String::new(),initialized:false,jobs:Vec::new(),poll:0.}
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
                .thumbnail(112, 150)
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
        if !g.adapter.is_empty() {
            v["adapter"] = json!(g.adapter);
        }
        v
    }
    pub(crate) fn render_generation_studio(&mut self, ui: &mut Ui) {
        let info = crate::generation::state();
        let catalog = &info["catalog"];
        let models = catalog["models"].as_array().cloned().unwrap_or_default();
        let connections = &info["connections"];
        if self.generation_ui.reference_document != self.project.document_id {
            self.generation_ui.reference_document = self.project.document_id.clone();
            self.generation_ui.selected_set.clear();
            self.generation_ui.reference_ids.clear();
            self.generation_ui.reference_textures.clear();
            self.generation_ui.reference = false;
        }
        if !models
            .iter()
            .any(|m| m["id"] == self.generation_ui.model && m["task"] == self.generation_ui.task)
        {
            if let Some(m) = models.iter().find(|m| {
                m["task"] == self.generation_ui.task
                    && m["tier"] == "practical"
                    && m["reference_required"].as_bool().unwrap_or(false)
                        == self.generation_ui.image_to_video
                    && (m["reference_max"].as_u64().unwrap_or(1) > 1)
                        == self.generation_ui.multi_reference
            }) {
                self.generation_select_model(m);
            }
        }

        if let Some(selected_model) = models.iter().find(|m| m["id"] == self.generation_ui.model) {
            if catalog["adapters"].as_array().is_some_and(|items| {
                items.iter().any(|a| {
                    a["id"] == self.generation_ui.adapter
                        && a["family"] != selected_model["pipeline"]
                })
            }) {
                self.generation_ui.adapter.clear();
            }
        }
        if !self.generation_ui.initialized {
            self.generation_ui.kaggle_user =
                connections["kaggle_username"].as_str().unwrap_or("").into();
            self.generation_ui.cache = connections["cache_dir"].as_str().unwrap_or("").into();
            self.generation_ui.initialized = true;
        }
        ScrollArea::vertical().show(ui,|ui|{
   egui::Frame::none().fill(Color32::from_rgb(26,43,41)).rounding(16.).inner_margin(24.).show(ui,|ui|{ui.label(RichText::new("CANVAS STUDIO / CREATE").small().color(Color32::from_rgb(177,220,197)));ui.heading(RichText::new("From an idea to an image. Or a scene.").size(27.));ui.label("Explore free model weights, direct your generation, then bring the result into your design or video edit.");ui.horizontal_wrapped(|ui|{for (kind,reference,multi,label) in [("image",false,false,"Image studio"),("video",false,false,"Text to video"),("video",true,false,"Image to video"),("video",true,true,"Reference video")]{if ui.selectable_label(self.generation_ui.task==kind&&self.generation_ui.image_to_video==reference&&self.generation_ui.multi_reference==multi,label).clicked(){self.generation_ui.task=kind.into();self.generation_ui.image_to_video=reference;self.generation_ui.multi_reference=multi;if let Some(m)=models.iter().find(|m|m["task"]==kind&&m["runnable"]==true&&m["tier"]=="practical"&&m["reference_required"].as_bool().unwrap_or(false)==reference&&(m["reference_max"].as_u64().unwrap_or(1)>1)==multi){self.generation_select_model(m);}}}

    if ui.button("Open video editor").clicked(){self.screen=WorkspaceScreen::StudioEditor;self.active_dock=DockTab::Film;}});
   });ui.add_space(16.);
   ui.columns(2,|cols|{
    cols[0].heading("The model library");cols[0].small("Reviewed recipes + live Hugging Face / Civitai discovery. Checked 27 November 2026.");
    let visible=models.iter().filter(|m|m["task"]==self.generation_ui.task&&(self.generation_ui.task!="video"||m["reference_required"].as_bool().unwrap_or(false)==self.generation_ui.image_to_video&&(m["reference_max"].as_u64().unwrap_or(1)>1)==self.generation_ui.multi_reference)).collect::<Vec<_>>();
    egui::ComboBox::from_label("Selected model").width(300.).selected_text(models.iter().find(|m|m["id"]==self.generation_ui.model).and_then(|m|m["name"].as_str()).unwrap_or("Choose a model")).show_ui(&mut cols[0],|ui|{for m in visible{let id=m["id"].as_str().unwrap();if ui.selectable_value(&mut self.generation_ui.model,id.into(),m["name"].as_str().unwrap_or(id)).clicked(){self.generation_select_model(m);}}});
    if let Some(model)=models.iter().find(|m|m["id"]==self.generation_ui.model){cols[0].group(|ui|{ui.strong(model["name"].as_str().unwrap_or("Model"));ui.label(model["notes"].as_str().unwrap_or(""));ui.small(format!("Licence: {} · profile: {}",model["license"].as_str().unwrap_or("Review required"),model["safety_profile"].as_str().unwrap_or("unknown")));if let Some(gb)=model["vram_gb"].as_f64(){ui.small(format!("Planning target: {gb} GB VRAM · offload may reduce GPU use"));}ui.hyperlink_to("Model card & licence",model["url"].as_str().unwrap_or("https://huggingface.co"));if model["commercial"]=="research"{ui.colored_label(Color32::LIGHT_YELLOW,"Research licence: use another model for advertising.");}if model["runnable"]==false{ui.colored_label(Color32::LIGHT_YELLOW,"Discovery only: this model needs a dedicated inference recipe.");}});}
    cols[0].separator();cols[0].checkbox(&mut self.generation_ui.commercial,"Commercial / advertising use");cols[0].checkbox(&mut self.generation_ui.ack,"I have reviewed the selected model's rights");
    egui::ComboBox::from_label("Content mode").selected_text(&self.generation_ui.content).show_ui(&mut cols[0],|ui|{ui.selectable_value(&mut self.generation_ui.content,"standard".into(),"Standard · additional output check");ui.selectable_value(&mut self.generation_ui.content,"flexible".into(),"Flexible · model defaults");});
    cols[0].small("Flexible mode adds no app-level content check. It preserves model/provider rules; an NSFW tag alone does not establish censorship behaviour. Publisher-declared uncensored models are marked in the library.");
    cols[0].collapsing("Find more models & weights",|ui|{egui::ComboBox::from_label("Source").selected_text(&self.generation_ui.provider).show_ui(ui,|ui|{ui.selectable_value(&mut self.generation_ui.provider,"huggingface".into(),"Hugging Face");ui.selectable_value(&mut self.generation_ui.provider,"civitai".into(),"Civitai");});ui.text_edit_singleline(&mut self.generation_ui.search);ui.checkbox(&mut self.generation_ui.mature,"Include mature Civitai listings (metadata only)");if ui.button("Search live models").clicked(){self.generation_action(json!({"operation":"search","provider":self.generation_ui.provider,"query":self.generation_ui.search,"task":self.generation_ui.task,"include_mature":self.generation_ui.mature}));}});
    cols[1].heading("Art direction");cols[1].label("Describe the image or shot");cols[1].add(egui::TextEdit::multiline(&mut self.generation_ui.prompt).desired_rows(5).desired_width(f32::INFINITY));cols[1].label("Negative prompt (supported models)");cols[1].text_edit_multiline(&mut self.generation_ui.negative);
    cols[1].horizontal(|ui|{ui.label("Width");ui.add(egui::DragValue::new(&mut self.generation_ui.width).range(256..=1536).speed(64.));ui.label("Height");ui.add(egui::DragValue::new(&mut self.generation_ui.height).range(256..=1536).speed(64.));});
    cols[1].horizontal(|ui|{ui.label("Steps");ui.add(egui::DragValue::new(&mut self.generation_ui.steps).range(1..=60));ui.label("Guidance");ui.add(egui::DragValue::new(&mut self.generation_ui.guidance).range(0.0..=12.0).speed(0.1));});
    cols[1].horizontal(|ui|{ui.label("Seed");ui.add(egui::DragValue::new(&mut self.generation_ui.seed));});
    if self.generation_ui.task=="video"{cols[1].horizontal(|ui|{let multiple=models.iter().find(|m|m["id"]==self.generation_ui.model).and_then(|m|m["frame_multiple"].as_u64()).unwrap_or(4);ui.label(format!("Frames · {multiple}n+1"));ui.add(egui::DragValue::new(&mut self.generation_ui.frames).range(9..=121).speed(4.));ui.label("FPS");egui::ComboBox::from_id_salt("ai-fps").selected_text(self.generation_ui.fps.to_string()).show_ui(ui,|ui|{for v in [8,16,24]{ui.selectable_value(&mut self.generation_ui.fps,v,v.to_string());}});});}
    let reference_enabled=models.iter().find(|m|m["id"]==self.generation_ui.model).is_some_and(|m|["Flux2KleinPipeline","QwenImage21Pipeline","WanImageToVideoPipeline","LTXImageToVideoPipeline","WanVACEPipeline"].contains(&m["pipeline"].as_str().unwrap_or("")));
    let maximum=models.iter().find(|m|m["id"]==self.generation_ui.model).and_then(|m|m["reference_max"].as_u64()).unwrap_or(1) as usize;
    cols[1].group(|ui|{ui.heading(if maximum>1{"Subject reference set"}else{"First frame / reference"});ui.small("Reuse views of the SAME person across shots. Reference conditioning helps consistency; it does not lock identity.");
     ui.add_enabled_ui(reference_enabled,|ui|{
      for (append,label) in [(false,"Choose reference images…"),(true,"Add images…")]{if ui.button(label).clicked(){if let Some(paths)=rfd::FileDialog::new().add_filter("Reference images",&["png","jpg","jpeg","webp"]).pick_files(){let mut ids=if append{self.generation_ui.reference_ids.clone()}else{self.generation_ui.reference_ids.clear();self.generation_ui.reference_textures.clear();Vec::new()};let result=(||{if paths.len()+ids.len()>maximum{return Err(format!("This model accepts at most {maximum} references"));}for path in paths{if std::fs::metadata(&path).map_err(|e|e.to_string())?.len()>25_000_000{return Err("Choose images up to 25 MB each".into());}let info=crate::generation::upload_reference(&self.project.document_id,&std::fs::read(path).map_err(|e|e.to_string())?)?;let id=info["id"].as_str().unwrap().to_string();if ids.contains(&id){return Err("That image is already in the set".into());}ids.push(id);}self.generation_set_references(ids,ui.ctx())})();self.status_banner=match result{Ok(())=>"Reference set ready · no GPU submission yet".into(),Err(e)=>e};}}}
      if ui.checkbox(&mut self.generation_ui.reference,"Use the current design page as a reference").changed()&&self.generation_ui.reference{self.generation_ui.reference_ids.clear();self.generation_ui.reference_textures.clear();}
      let mut remove=None;let mut earlier=None;ui.horizontal_wrapped(|ui|{for (index,texture) in self.generation_ui.reference_textures.iter().enumerate(){ui.vertical(|ui|{ui.label(format!("Reference {}",index+1));ui.image(texture);if ui.small_button("Remove").clicked(){remove=Some(index);}if index>0&&ui.small_button("Move earlier").clicked(){earlier=Some(index);}});}});
      if let Some(index)=remove{self.generation_ui.reference_ids.remove(index);let _=self.generation_ui.reference_textures.remove(index);self.generation_ui.selected_set.clear();}if let Some(index)=earlier{self.generation_ui.reference_ids.swap(index,index-1);self.generation_ui.reference_textures.swap(index,index-1);self.generation_ui.selected_set.clear();}
      if !self.generation_ui.reference_ids.is_empty()&&ui.button("Clear reference set").clicked(){self.generation_ui.reference_ids.clear();self.generation_ui.reference_textures.clear();self.generation_ui.selected_set.clear();}
      ui.label(format!("{} / {maximum} reference images",self.generation_ui.reference_ids.len()));
      if maximum>1{ui.add(egui::Slider::new(&mut self.generation_ui.reference_strength,0.5..=2.).text("Reference influence"));}
      egui::ComboBox::from_label("Reference framing").selected_text(&self.generation_ui.reference_fit).show_ui(ui,|ui|{ui.selectable_value(&mut self.generation_ui.reference_fit,"contain".into(),"Keep whole image · pad edges");ui.selectable_value(&mut self.generation_ui.reference_fit,"cover".into(),"Fill frame · crop edges");});
      ui.separator();ui.label("Saved reference sets");let sets=self.generation_ui.reference_sets.clone();egui::ComboBox::from_id_salt("reference-set").selected_text(sets.iter().find(|v|v["id"]==self.generation_ui.selected_set).and_then(|v|v["name"].as_str()).unwrap_or("Choose a saved set")).show_ui(ui,|ui|{for set in &sets{ui.selectable_value(&mut self.generation_ui.selected_set,set["id"].as_str().unwrap_or("").into(),set["name"].as_str().unwrap_or("Reference set"));}});
      if ui.button("Load reference set").clicked(){let result=(||{let set=sets.iter().find(|v|v["id"]==self.generation_ui.selected_set).ok_or("Choose a saved reference set")?;if set["count"].as_u64().unwrap_or(0)>maximum as u64{return Err("Select Reference video for a multi-image set".into());}let result=crate::generation::reference_set_action(&self.project.document_id,&json!({"operation":"load","id":self.generation_ui.selected_set}))?;let ids=result["references"].as_array().unwrap().iter().map(|r|r["id"].as_str().unwrap().to_string()).collect();self.generation_set_references(ids,ui.ctx())})();self.status_banner=match result{Ok(())=>"Saved reference set applied to this document".into(),Err(e)=>e};}
      ui.text_edit_singleline(&mut self.generation_ui.set_name);if ui.button("Save named reference set").clicked(){match crate::generation::reference_set_action(&self.project.document_id,&json!({"operation":"save","name":self.generation_ui.set_name,"reference_ids":self.generation_ui.reference_ids})){Ok(_)=>{self.generation_ui.poll=0.;self.status_banner="Reusable reference set saved privately".into();},Err(e)=>self.status_banner=e}}
     });if !reference_enabled{ui.small("Text-only model. Choose Image to video or Reference video.");}else if self.generation_ui.reference_ids.is_empty()&&!self.generation_ui.reference{ui.colored_label(Color32::LIGHT_YELLOW,"Choose reference images before generating.");}
    });
    cols[1].collapsing("Weights, adapters & memory",|ui|{egui::ComboBox::from_label("Weight precision").selected_text(&self.generation_ui.precision).show_ui(ui,|ui|{for v in ["fp16","bf16","fp32"]{ui.selectable_value(&mut self.generation_ui.precision,v.into(),v);}});egui::ComboBox::from_label("Quantization").selected_text(&self.generation_ui.quantization).show_ui(ui,|ui|{ui.selectable_value(&mut self.generation_ui.quantization,"none".into(),"Original weights");ui.selectable_value(&mut self.generation_ui.quantization,"nf4".into(),"NF4 · transformer weights");});ui.checkbox(&mut self.generation_ui.offload,"Offload components to CPU RAM");ui.checkbox(&mut self.generation_ui.cpu,"Use local CPU explicitly (FP32, very slow)");ui.horizontal(|ui|{ui.label("Download limit · GiB");ui.add(egui::DragValue::new(&mut self.generation_ui.limit).range(2..=100));});egui::ComboBox::from_label("Adapter").selected_text(if self.generation_ui.adapter.is_empty(){"None"}else{"SDXL Lightning · 4 steps"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.generation_ui.adapter,String::new(),"None");ui.selectable_value(&mut self.generation_ui.adapter,"sdxl-lightning-4".into(),"SDXL Lightning · compatible SDXL models only");});});
    cols[1].separator();if cols[1].button("Prepare private Kaggle notebook").clicked(){self.generation_action(self.generation_request("prepare"));}
    if cols[1].add_enabled(connections["kaggle_configured"]==true&&connections["kaggle_username"].as_str().is_some_and(|v|!v.is_empty()),egui::Button::new("Submit private Kaggle GPU job")).clicked(){self.generation_action(self.generation_request("submit"));}
    if cols[1].button("Generate locally").clicked(){self.generation_action(self.generation_request("generate"));}
    cols[1].small("Submitting sends this prompt and any selected reference to your private Kaggle notebook. Default run limit: 30 minutes. GPU quota/availability belong to your Kaggle account. Local generation needs the optional inference tools and suitable hardware.");
   });
   ui.add_space(12.);ui.collapsing("Connections & storage",|ui|{ui.label("Credentials are stored privately on this computer and excluded from projects, source packages and Git.");ui.small("Google sign-in is fine. Enter your Kaggle profile handle and a separate API token, never your Google password.");ui.hyperlink_to("Create a Kaggle API token", "https://www.kaggle.com/settings/api");ui.horizontal(|ui|{ui.label("Kaggle username");ui.text_edit_singleline(&mut self.generation_ui.kaggle_user);});for(label,key)in[("Kaggle API token",&mut self.generation_ui.kaggle_token),("Hugging Face token",&mut self.generation_ui.hf_token),("Civitai API key",&mut self.generation_ui.civitai_token)]{ui.horizontal(|ui|{ui.label(label);ui.add(egui::TextEdit::singleline(key).password(true).hint_text("Leave blank to keep saved value"));});}ui.horizontal(|ui|{ui.label("Model cache folder");ui.text_edit_singleline(&mut self.generation_ui.cache);});if ui.button("Save connections").clicked(){match crate::generation::save_connections(json!({"kaggle_username":self.generation_ui.kaggle_user,"kaggle_token":self.generation_ui.kaggle_token,"hf_token":self.generation_ui.hf_token,"civitai_token":self.generation_ui.civitai_token,"cache_dir":self.generation_ui.cache})){Ok(())=>{self.generation_ui.kaggle_token.clear();self.generation_ui.hf_token.clear();self.generation_ui.civitai_token.clear();self.status_banner="Connections saved privately".into();},Err(e)=>self.status_banner=e}}ui.small("Install SDK: python3 scripts/setup_generation.py · local inference: add --local-inference. Add HF_TOKEN / CIVITAI_API_KEY as Kaggle secrets for gated/authenticated weight downloads; local tokens are never embedded in notebooks.");});
   let now=ui.input(|i|i.time);if now-self.generation_ui.poll>1.5{self.generation_ui.jobs=crate::media_service::jobs(&self.project.document_id).ok().and_then(|v|v["jobs"].as_array().cloned()).unwrap_or_default();self.generation_ui.reference_sets=crate::generation::reference_sets()["sets"].as_array().cloned().unwrap_or_default();self.generation_ui.poll=now;}ui.ctx().request_repaint_after(std::time::Duration::from_secs(1));
   ui.separator();ui.heading("Studio tasks & results");
   ui.small("Save outputs first. Clearing finished generation tasks removes local review files, preserving imported project assets, model caches and remote notebooks.");
   if ui.button("Clear finished generation tasks").clicked(){match crate::media_service::clear_finished_kind(&self.project.document_id,true){Ok(n)=>{self.status_banner=format!("Cleared {n} generation tasks; imported assets preserved");self.generation_ui.poll=0.;},Err(e)=>self.status_banner=e}}
   for job in self.generation_ui.jobs.clone().into_iter().filter(|j|j["mode"].as_str().is_some_and(|v|v.starts_with("ai_"))){ui.group(|ui|{let id=job["id"].as_str().unwrap_or("");let result=&job["result"];let label=if job["state"]=="complete"&&result["remote_state"].is_string(){format!("GPU · {}",result["remote_state"].as_str().unwrap())}else if job["state"]=="complete"&&result["submitted"].is_boolean(){format!("GPU · {}",if result["submitted"]==true{"submitted"}else{"unconfirmed"})}else{format!("{} · {}",job["mode"].as_str().unwrap_or("Task"),job["state"].as_str().unwrap_or(""))};ui.strong(label);if let Some(model)=result["model"].as_str(){ui.small(model);}if let Some(error)=job["error"].as_str(){ui.colored_label(Color32::LIGHT_YELLOW,error);}if job["state"]=="running"{if ui.button("Cancel local task").clicked(){let _=crate::media_service::cancel(id,&self.project.document_id);}}if job["state"]=="complete"{let result=&job["result"];if let Some(rows)=result["models"].as_array(){ui.label(format!("{} / {} results for {}", result["provider"].as_str().unwrap_or("Catalog"), rows.len(), result["query"].as_str().unwrap_or("")));for m in rows{ui.horizontal_wrapped(|ui|{ui.hyperlink_to(m["name"].as_str().unwrap_or("Model"),m["url"].as_str().unwrap_or("https://huggingface.co"));if ui.small_button("Add reviewed recipe").clicked(){self.generation_action(json!({"operation":"register","provider":m["provider"],"id":m["id"],"version_id":m["version_id"]}));}});}}
    if let Some(link)=result["url"].as_str(){ui.hyperlink_to("Open Kaggle notebook",link);}if let Some(run)=result["run_id"].as_str(){ui.horizontal(|ui|{if ui.button("Check GPU run").clicked(){self.generation_action(json!({"operation":"remote_status","run_id":run}));}if ui.button("Download finished result").clicked(){self.generation_action(json!({"operation":"download","run_id":run,"task":self.generation_ui.task}));}});}if let Some(remote)=result["remote_state"].as_str(){ui.label(format!("Kaggle: {remote}"));}if let Some(log)=result["log_tail"].as_str(){ui.collapsing("GPU execution log",|ui|{ui.monospace(log);});}
    if result["source_available"]==true&&ui.button("Save inference notebook ZIP…").clicked(){if let Some(dest)=rfd::FileDialog::new().set_file_name("private-inference.zip").save_file(){self.status_banner=match crate::media_service::source_output(id,&self.project.document_id).and_then(|p|fs_copy(&p,&dest)){Ok(())=>"Inference package saved".into(),Err(e)=>e};}}
    if ["ai_image","ai_video"].contains(&job["mode"].as_str().unwrap_or("")){if ui.button("Import into design / video").clicked(){let mut p=self.project.clone();match crate::generation::import(&mut p,&json!({"id":id,"page":self.current_page_idx})){Ok(())=>{self.record_mutation();self.project=p;self.screen=WorkspaceScreen::StudioEditor;if job["mode"]=="ai_video"{self.active_dock=DockTab::Film;}self.status_banner="Generated asset imported · editable project preserved".into();},Err(e)=>self.status_banner=e}}}
   }});}
  });
    }
}
fn fs_copy(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::copy(src, dest)
        .map(|_| ())
        .map_err(|e| e.to_string())
}
