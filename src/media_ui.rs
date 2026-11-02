//! Native timeline controls use the same commands and snapshot jobs as browser/MCP.
use super::*;
use serde_json::{json, Value};
#[derive(Default)]
pub(crate) struct FilmUi {
    selected: Option<String>,
    head: u64,
    revision: Option<(String, u64)>,
    clip: Option<crate::timeline::Clip>,
    jobs: Vec<Value>,
    last_poll: f64,
    texture: Option<egui::TextureHandle>,
    frame_job: Option<String>,
    frame_revision: Option<u64>,
    width: u32,
    height: u32,
    fps: u32,
    duration: u64,
}
impl CanvasStudioApp {
    pub(crate) fn film_command(&mut self, v: Value) {
        let mut p = self.project.clone();
        match crate::timeline::apply(&mut p, &v) {
            Ok(()) => {
                self.record_mutation();
                self.project = p;
                self.status_banner = "Film edit applied · build a new preview to review".into();
            }
            Err(e) => self.status_banner = e,
        }
    }
    fn film_job(&mut self, mode: &str) {
        match crate::media_service::start(
            &self.project,
            self.document_revision,
            mode,
            if mode == "frame" {
                Some(self.film_ui.head)
            } else {
                None
            },
        ) {
            Ok(id) => {
                if mode == "frame" {
                    self.film_ui.frame_job = Some(id);
                }
                self.status_banner = format!("{mode} queued · editing remains available");
                self.film_ui.last_poll = 0.;
            }
            Err(e) => self.status_banner = e,
        }
    }
    pub(crate) fn render_film_dock(&mut self, ui: &mut Ui) {
        ui.heading("Film studio");
        ui.label("MOTION · SOUND · STORY");
        ui.add_space(12.);
        let identity = (self.project.document_id.clone(), self.document_revision);
        if self.film_ui.revision.as_ref() != Some(&identity) {
            let t = &self.project.production.timeline;
            self.film_ui.width = t.width;
            self.film_ui.height = t.height;
            self.film_ui.fps = t.fps;
            self.film_ui.duration = t.duration;
            self.film_ui.clip = t
                .clips
                .iter()
                .find(|c| Some(&c.id) == self.film_ui.selected.as_ref())
                .cloned();
            self.film_ui.revision = Some(identity);
        }
        if ui.button("Import footage, stills or sound…").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Media", crate::media_store::extensions())
                .pick_file()
            {
                let name = path
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                match crate::media_store::import(&path, &name) {
                    Ok(a) => {
                        let mut p = self.project.clone();
                        p.production.timeline.assets.insert(a.id.clone(), a.clone());
                        match crate::timeline::validate(&p.production.timeline) {
                            Ok(()) => {
                                self.record_mutation();
                                self.project = p;
                                self.status_banner =
                                    "Original imported · build a proxy for lightweight review"
                                        .into();
                            }
                            Err(e) => self.status_banner = e,
                        }
                    }
                    Err(e) => self.status_banner = e,
                }
            }
        }
        ui.collapsing("Movie settings",|ui|{
            for (label,value) in [("Width",&mut self.film_ui.width),("Height",&mut self.film_ui.height),("FPS",&mut self.film_ui.fps)]{ui.horizontal(|ui|{ui.label(label);ui.add(egui::DragValue::new(value));});}
            ui.horizontal(|ui|{ui.label("Duration · frames");ui.add(egui::DragValue::new(&mut self.film_ui.duration));});
            if ui.button("Apply movie settings").clicked(){self.film_command(json!({"action":"film_setup","width":self.film_ui.width,"height":self.film_ui.height,"fps":self.film_ui.fps,"duration":self.film_ui.duration}));}
            ui.small("Canvas and FPS stay fixed after adding clips.");
        });
        ui.separator();
        let assets = self
            .project
            .production
            .timeline
            .assets
            .values()
            .cloned()
            .collect::<Vec<_>>();
        for a in assets {
            ui.group(|ui| {
                ui.strong(&a.name);
                ui.small(format!(
                    "{} · {:.1} MB · {:.2}s",
                    a.kind,
                    a.bytes as f64 / 1048576.,
                    a.duration_us as f64 / 1e6
                ));
                ui.horizontal_wrapped(|ui| {
                    if ui.small_button("+ At playhead").clicked() {
                        self.film_command(
                            json!({"action":"film_add","asset":a.id,"start":self.film_ui.head}),
                        );
                    }
                    if ui.small_button("Build proxy").clicked() {
                        match crate::media_service::proxy(
                            &a,
                            &self.project.document_id,
                            self.document_revision,
                        ) {
                            Ok(_) => {
                                self.status_banner = "Proxy queued".into();
                                self.film_ui.last_poll = 0.;
                            }
                            Err(e) => self.status_banner = e,
                        }
                    }
                });
            });
        }
        ui.separator();
        if ui.button("+ Current design overlay").clicked() {
            let t = &self.project.production.timeline;
            self.film_command(json!({"action":"film_design","page":self.current_page_idx,"start":self.film_ui.head,"duration":(3*t.fps as u64).min(t.duration.saturating_sub(self.film_ui.head)),"transparent":true}));
        }
        if ui.button("+ Caption at playhead").clicked() {
            let t = &self.project.production.timeline;
            self.film_command(json!({"action":"film_caption","text":"Your next line.","start":self.film_ui.head,"duration":(3*t.fps as u64).min(t.duration.saturating_sub(self.film_ui.head))}));
        }
        ui.horizontal(|ui|{for (kind,label) in [("visual","+ Visual track"),("audio","+ Audio track")]{if ui.small_button(label).clicked(){self.film_command(json!({"action":"film_track","kind":kind,"name":format!("{} {}",kind,self.project.production.timeline.tracks.len()+1)}));}}});
        ui.horizontal(|ui| {
            if ui.button("Import SRT").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("Subtitles", &["srt"])
                    .pick_file()
                {
                    let mut p = self.project.clone();
                    match std::fs::read_to_string(path)
                        .map_err(|e| e.to_string())
                        .and_then(|s| crate::timeline::import_srt(&mut p, &s))
                    {
                        Ok(()) => {
                            self.record_mutation();
                            self.project = p;
                        }
                        Err(e) => self.status_banner = e,
                    }
                }
            }
            if ui.button("Save SRT").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_file_name("captions.srt")
                    .save_file()
                {
                    match std::fs::write(
                        path,
                        crate::timeline::srt(&self.project.production.timeline),
                    ) {
                        Ok(()) => self.status_banner = "Captions saved".into(),
                        Err(e) => self.status_banner = e.to_string(),
                    }
                }
            }
        });
        ui.collapsing("Local storage",|ui|{let usage=crate::media_service::storage();ui.small(format!("Originals {:.1} MB · proxies {:.1} MB · renders {:.1} MB",usage["originals_bytes"].as_u64().unwrap_or(0) as f64/1048576.,usage["cache_bytes"].as_u64().unwrap_or(0) as f64/1048576.,usage["jobs_bytes"].as_u64().unwrap_or(0) as f64/1048576.));ui.small("Save movies you need first. Clearing finished jobs removes local review/render files for this document; originals and edits stay intact.");if ui.button("Clear finished render history").clicked(){match crate::media_service::clear_finished(&self.project.document_id){Ok(n)=>{self.status_banner=format!("Cleared {n} finished jobs; original media and timeline preserved");self.film_ui.last_poll=0.;},Err(e)=>self.status_banner=e}}});
        if let Some(mut c) = self.film_ui.clip.clone() {
            ui.separator();
            ui.heading("Selected clip");
            ui.text_edit_singleline(&mut c.name);
            ui.horizontal(|ui| {
                ui.label("Start · frames");
                ui.add(egui::DragValue::new(&mut c.start));
            });
            ui.horizontal(|ui| {
                ui.label("Length · frames");
                ui.add(egui::DragValue::new(&mut c.duration));
            });
            let mut source = c.source_in_us as f64 / 1e6;
            ui.horizontal(|ui| {
                ui.label("Source in · seconds");
                ui.add(
                    egui::DragValue::new(&mut source)
                        .speed(0.01)
                        .range(0.0..=7200.),
                );
            });
            c.source_in_us = (source * 1e6).round() as u64;
            egui::ComboBox::from_id_salt("film-track")
                .selected_text(
                    self.project
                        .production
                        .timeline
                        .tracks
                        .iter()
                        .find(|t| t.id == c.track)
                        .map(|t| t.name.as_str())
                        .unwrap_or("Track"),
                )
                .show_ui(ui, |ui| {
                    for tr in &self.project.production.timeline.tracks {
                        ui.selectable_value(&mut c.track, tr.id.clone(), &tr.name);
                    }
                });
            if ui.button("Apply timing").clicked() {
                self.film_command(json!({"action":"film_clip","id":c.id,"name":c.name,"start":c.start,"duration":c.duration,"source_in_us":c.source_in_us,"track":c.track}));
            }
            ui.horizontal(|ui| {
                if ui.button("Split at playhead").clicked() {
                    self.film_command(
                        json!({"action":"film_split","id":c.id,"at":self.film_ui.head}),
                    );
                }
                if ui.button("Delete").clicked() {
                    self.film_command(json!({"action":"film_delete","id":c.id}));
                    self.film_ui.selected = None;
                    self.film_ui.clip = None;
                }
            });
            if let Some(text) = &mut c.text {
                ui.label("Caption words");
                ui.text_edit_multiline(text);
                if let Some(st) = &mut c.caption {
                    ui.add(egui::DragValue::new(&mut st.size).prefix("Size "));
                    for (label, value) in [
                        ("X ", &mut st.x),
                        ("Y ", &mut st.y),
                        ("Width ", &mut st.width),
                        ("Height ", &mut st.height),
                    ] {
                        ui.add(egui::DragValue::new(value).prefix(label));
                    }
                }
                if ui.button("Apply caption").clicked() {
                    let st = c.caption.as_ref().unwrap();
                    self.film_command(json!({"action":"film_clip","id":c.id,"text":c.text,"size":st.size,"x":st.x,"y":st.y,"width":st.width,"height":st.height}));
                }
            } else {
                ui.collapsing("Picture framing",|ui|{for (i,label) in ["X ","Y ","Width ","Height "].iter().enumerate(){ui.add(egui::DragValue::new(&mut c.rect[i]).prefix(*label));}egui::ComboBox::from_id_salt("film-fit").selected_text(&c.fit).show_ui(ui,|ui|{ui.selectable_value(&mut c.fit,"cover".into(),"Fill & crop");ui.selectable_value(&mut c.fit,"contain".into(),"Fit inside");});ui.add(egui::Slider::new(&mut c.opacity,0.0..=1.).text("Opacity"));if ui.button("Apply framing").clicked(){self.film_command(json!({"action":"film_clip","id":c.id,"rect":c.rect,"fit":c.fit,"opacity":c.opacity}));}});
            }
            if c.asset
                .as_ref()
                .and_then(|id| self.project.production.timeline.assets.get(id))
                .is_some_and(|a| a.has_audio)
            {
                ui.collapsing("Sound & fades",|ui|{ui.checkbox(&mut c.audio.enabled,"Source audio enabled");ui.add(egui::Slider::new(&mut c.audio.gain_db,-60.0..=12.).text("Gain dB"));if ui.button("Apply mix").clicked(){self.film_command(json!({"action":"film_clip","id":c.id,"gain_db":c.audio.gain_db,"audio_enabled":c.audio.enabled}));}ui.add(egui::DragValue::new(&mut c.audio.fade_in).prefix("Fade in · frames "));ui.add(egui::DragValue::new(&mut c.audio.fade_out).prefix("Fade out · frames "));if ui.button("Set fades for this clip").clicked(){self.film_command(json!({"action":"film_clip","id":c.id,"fade_in":c.audio.fade_in,"fade_out":c.audio.fade_out}));}ui.small("Split clips preserve the original fade envelope. Setting fades rebases it to this clip.");if ui.button("Detach source audio").clicked(){self.film_command(json!({"action":"film_detach_audio","id":c.id}));}});
            }
            if self.film_ui.selected.is_some() {
                self.film_ui.clip = Some(c);
            }
        }
    }
    pub(crate) fn render_film_workspace(&mut self, ui: &mut Ui) {
        let now = ui.input(|i| i.time);
        if now - self.film_ui.last_poll > 1. || self.film_ui.last_poll == 0. {
            self.film_ui.last_poll = now;
            if let Ok(v) = crate::media_service::jobs(&self.project.document_id) {
                self.film_ui.jobs = v["jobs"].as_array().cloned().unwrap_or_default();
            }
        }
        if let Some(id) = self.film_ui.frame_job.clone() {
            if let Some(j) = self.film_ui.jobs.iter().find(|j| j["id"] == id) {
                if j["state"] == "complete" {
                    if let Ok(path) = crate::media_service::output(&id, &self.project.document_id) {
                        if let Ok(image) = image::open(path) {
                            let im = image.to_rgba8();
                            self.film_ui.texture = Some(ui.ctx().load_texture(
                                "film-frame",
                                egui::ColorImage::from_rgba_unmultiplied(
                                    [im.width() as usize, im.height() as usize],
                                    im.as_raw(),
                                ),
                                egui::TextureOptions::LINEAR,
                            ));
                            self.film_ui.frame_revision = j["source_revision"].as_u64();
                        }
                    }
                    self.film_ui.frame_job = None;
                } else if j["state"] == "failed" || j["state"] == "cancelled" {
                    self.status_banner =
                        j["error"].as_str().unwrap_or("Frame review stopped").into();
                    self.film_ui.frame_job = None;
                }
            }
        }
        egui::ScrollArea::vertical().show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{ui.heading("Film studio");ui.label("MOTION · SOUND · STORY");if ui.button("Review frame").clicked(){self.film_job("frame");}if ui.button("Build preview").clicked(){self.film_job("preview");}if ui.button("Render MP4").clicked(){self.film_job("export");}});ui.add_space(12.);
            if let Some(texture)=&self.film_ui.texture{let size=texture.size_vec2();let scale=(ui.available_width()/size.x).min(330./size.y);ui.add(egui::Image::new(texture).fit_to_exact_size(size*scale));ui.small(format!("Frame review · revision {}{}",self.film_ui.frame_revision.unwrap_or(0),if self.film_ui.frame_revision==Some(self.document_revision){""}else{" · EDIT CHANGED"}));}else{egui::Frame::none().fill(Color32::from_rgb(12,19,27)).rounding(12.).inner_margin(Margin::same(35.)).show(ui,|ui|{ui.set_min_height(180.);ui.heading("A place for your next story.");ui.label("Import footage, arrange the edit, then build a preview.");ui.small("Picture and sound are reviewed together in your system video player.");});}
            let t=self.project.production.timeline.clone();self.film_ui.head=self.film_ui.head.min(t.duration-1);ui.add(egui::Slider::new(&mut self.film_ui.head,0..=t.duration-1).text("Playhead · frames"));ui.small(format!("{:.2}s / {:.2}s · {} fps",self.film_ui.head as f64/t.fps as f64,t.duration as f64/t.fps as f64,t.fps));
            for tr in &t.tracks{ui.horizontal(|ui|{let mut enabled=tr.enabled;if ui.checkbox(&mut enabled,&tr.name).changed(){self.film_command(json!({"action":"film_track_update","id":tr.id,"enabled":enabled}));}ui.small(&tr.kind);});let (rect,response)=ui.allocate_exact_size(egui::vec2(ui.available_width(),48.),egui::Sense::click());ui.painter().rect_filled(rect,4.,Color32::from_rgb(29,42,54));let cursor=rect.left()+rect.width()*self.film_ui.head as f32/t.duration as f32;ui.painter().line_segment([egui::pos2(cursor,rect.top()),egui::pos2(cursor,rect.bottom())],Stroke::new(1_f32,Color32::from_rgb(245,200,121)));for c in t.clips.iter().filter(|c|c.track==tr.id){let r=egui::Rect::from_min_max(egui::pos2(rect.left()+rect.width()*c.start as f32/t.duration as f32,rect.top()+4.),egui::pos2(rect.left()+rect.width()*(c.start+c.duration) as f32/t.duration as f32,rect.bottom()-4.));ui.painter().rect_filled(r,4.,if Some(&c.id)==self.film_ui.selected.as_ref(){Color32::from_rgb(48,125,105)}else{Color32::from_rgb(48,82,104)});ui.painter().with_clip_rect(r).text(r.left_center()+egui::vec2(6.,0.),egui::Align2::LEFT_CENTER,c.text.as_deref().unwrap_or(&c.name),egui::FontId::proportional(11.),Color32::WHITE);if response.clicked()&&response.interact_pointer_pos().is_some_and(|p|r.contains(p)){self.film_ui.selected=Some(c.id.clone());self.film_ui.clip=Some(c.clone());}}}
            ui.separator();ui.label("RENDER JOBS · snapshots of the edit");for j in self.film_ui.jobs.clone().iter().take(6){ui.horizontal_wrapped(|ui|{ui.small(format!("{} · r{} · {} · {}%",j["mode"].as_str().unwrap_or(""),j["source_revision"],j["state"].as_str().unwrap_or(""),(j["progress"].as_f64().unwrap_or(0.)*100.).round()));let id=j["id"].as_str().unwrap_or("");if j["state"]=="running"&&ui.small_button("Cancel").clicked(){let _=crate::media_service::cancel(id,&self.project.document_id);}if j["state"]=="complete"&&(j["mode"]=="preview"||j["mode"]=="export"){if ui.small_button("Play in video player").clicked(){match crate::media_service::output(id,&self.project.document_id).and_then(|path|std::process::Command::new("ffplay").args(["-autoexit","-loglevel","error"]).arg(path).spawn().map(|_|()).map_err(|e|format!("Install ffplay or save MP4 and use your video player: {e}"))){Ok(())=>self.status_banner="Preview opened in video player".into(),Err(e)=>self.status_banner=e}}if ui.small_button("Save MP4…").clicked(){if let Some(path)=rfd::FileDialog::new().set_file_name("film.mp4").save_file(){match crate::media_service::output(id,&self.project.document_id).and_then(|p|std::fs::copy(p,path).map(|_|()).map_err(|e|e.to_string())){Ok(())=>self.status_banner="Film saved".into(),Err(e)=>self.status_banner=e}}}}});if let Some(error)=j["error"].as_str(){ui.colored_label(Color32::LIGHT_RED,error);}}
        });
    }
}
