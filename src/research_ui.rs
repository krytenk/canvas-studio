use super::*;
use crate::reference_desk::{Entry, Source};
use serde_json::json;

#[derive(Default)]
pub(crate) struct ResearchUi {
    entries: Vec<Entry>,
    preview: Option<Source>,
    selected: Option<String>,
    cue: Option<usize>,
    filter: String,
    initialized: bool,
    document: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn reading_research_at_small_sizes_never_dirties_or_replaces_the_document() {
        let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(
            std::env::temp_dir().join(model::generate_document_id()),
        ));
        let source =
            crate::reference_desk::parse("notes.txt", b"A personal research note.").unwrap();
        app.research_ui.initialized = true;
        app.research_ui.document = app.project.document_id.clone();
        app.research_ui.selected = Some(source.id.clone());
        crate::reference_desk::import_source(&mut app.project, source).unwrap();
        app.screen = WorkspaceScreen::ResearchStudio;
        let before = app.project.clone();
        let ctx = Context::default();
        for size in [Vec2::new(1024., 680.), Vec2::new(700., 900.)] {
            let _ = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                    ..Default::default()
                },
                |ctx| app.update_ui(ctx),
            );
            assert_eq!(app.project, before);
            assert!(!app.dirty);
            assert_eq!(app.screen, WorkspaceScreen::ResearchStudio);
        }
    }
}
impl CanvasStudioApp {
    fn research_commit(
        &mut self,
        mut candidate: Project,
        operation: impl FnOnce(&mut Project) -> Result<String, String>,
    ) -> bool {
        match operation(&mut candidate) {
            Ok(id) => {
                self.record_mutation();
                self.project = candidate;
                self.research_ui.selected = Some(id);
                self.research_ui.preview = None;
                self.research_ui.cue = self
                    .project
                    .production
                    .research
                    .iter()
                    .find(|s| Some(&s.id) == self.research_ui.selected.as_ref())
                    .filter(|s| !s.cues.is_empty())
                    .map(|_| 0);
                self.status_banner =
                    "Research copied into this project. Original files stay in Reference Desk."
                        .into();
                true
            }
            Err(e) => {
                self.status_banner = e;
                false
            }
        }
    }
    fn research_media(&mut self, path: PathBuf, name: String) {
        let candidate = self.project.clone();
        if !self.research_commit(candidate, |p| {
            crate::reference_desk::import_media(p, &path, &name)
        }) {
            return;
        }
        if let Some(asset) = self
            .research_ui
            .selected
            .as_ref()
            .and_then(|id| {
                self.project
                    .production
                    .research
                    .iter()
                    .find(|s| &s.id == id)
            })
            .and_then(|s| s.media_asset.as_ref())
            .and_then(|id| self.project.production.timeline.assets.get(id))
            .cloned()
        {
            let _ = crate::media_service::proxy(
                &asset,
                &self.project.document_id,
                self.document_revision,
            );
            self.screen = WorkspaceScreen::StudioEditor;
            self.active_dock = DockTab::Film;
            self.dock_open = true;
        }
    }
    fn research_action(&mut self, action: &str, id: &str, cue: Option<usize>) {
        let mut v = json!({"action":action,"id":id,"page":self.current_page_idx});
        if let Some(cue) = cue {
            v["cue"] = json!(cue);
        }
        let mut candidate = self.project.clone();
        match crate::reference_desk::apply(&mut candidate, &v) {
            Ok(()) => {
                self.record_mutation();
                self.project = candidate;
                self.status_banner = if action == "research_captions" {
                    "Captions added. Open Video studio to review their timing."
                } else {
                    "Research updated. Undo is available."
                }
                .into();
                if action == "research_remove" {
                    self.research_ui.selected = None;
                    self.research_ui.cue = None;
                }
                if action == "research_captions" {
                    self.screen = WorkspaceScreen::StudioEditor;
                    self.active_dock = DockTab::Film;
                    self.dock_open = true;
                }
            }
            Err(e) => self.status_banner = e,
        }
    }
    pub(crate) fn render_research_studio(&mut self, ui: &mut Ui) {
        if self.research_ui.document != self.project.document_id {
            self.research_ui.document = self.project.document_id.clone();
            self.research_ui.preview = None;
            self.research_ui.selected = None;
            self.research_ui.cue = None;
        }
        if !self.research_ui.initialized {
            self.research_ui.initialized = true;
            match crate::reference_desk::list_at(&crate::reference_desk::root()) {
                Ok(entries) => self.research_ui.entries = entries,
                Err(e) => self.status_banner = e,
            }
        }
        ScrollArea::vertical().show(ui,|ui| {
            ui.heading("Research");
            ui.label("Bring Reference Desk transcripts, captions, audio and video into your project.");
            ui.label("Imported sources travel with your editable project. Original files stay in Reference Desk.");
            ui.horizontal_wrapped(|ui| {
                if ui.add_enabled(crate::reference_desk::launcher().is_file(),egui::Button::new("Open Reference Desk")).clicked(){if let Err(e)=crate::reference_desk::launch(){self.status_banner=e;}}
                if ui.button("Refresh saved files").clicked(){match crate::reference_desk::list_at(&crate::reference_desk::root()){Ok(entries)=>self.research_ui.entries=entries,Err(e)=>self.status_banner=e}}
                if ui.button("Import transcript / SRT...").clicked(){
                    if let Some(path)=rfd::FileDialog::new().add_filter("Transcripts and captions",&["txt","srt"]).pick_file(){
                        match crate::reference_desk::parse_file(&path){
                            Ok(source)=>{let candidate=self.project.clone();self.research_commit(candidate,|p|crate::reference_desk::import_source(p,source));},Err(e)=>self.status_banner=e,
                        }
                    }
                }
                if ui.button("Import audio / video...").clicked(){if let Some(path)=rfd::FileDialog::new().add_filter("Audio and video",&["mp3","wav","m4a","ogg","flac","mp4","mov","webm","mkv"]).pick_file(){let name=path.file_name().and_then(|s|s.to_str()).unwrap_or("media.mp4").to_string();self.research_media(path,name);}}
            });
            ui.add_space(10.);ui.add(egui::TextEdit::singleline(&mut self.research_ui.filter).hint_text("Find a saved file or project source").desired_width(450.));
            let search=self.research_ui.filter.to_lowercase();
            ui.columns(2,|columns| {
                let left=&mut columns[0];left.heading("Reference Desk files");left.small("Latest 250 saved files on this computer");
                if self.research_ui.entries.is_empty(){left.label("No saved files yet. Open Reference Desk, or import a file above.");}
                ScrollArea::vertical().id_salt("research-files").max_height(210.).show(left,|ui|{
                    for entry in self.research_ui.entries.clone().into_iter().filter(|e|e.name.to_lowercase().contains(&search)) {
                        ui.push_id(&entry.id,|ui|{
                            ui.label(&entry.name);ui.small(format!("{} · {:.1} KB",entry.kind,entry.bytes as f64/1024.));
                            if matches!(entry.kind.as_str(),"txt"|"srt") {
                                if ui.button("Preview transcript").clicked(){match crate::reference_desk::preview_at(&crate::reference_desk::root(),&entry.id){Ok(source)=>{self.research_ui.preview=Some(source);self.research_ui.selected=None;self.research_ui.cue=None;},Err(e)=>self.status_banner=e}}
                            }else if ui.button("Import to video library").clicked(){match crate::reference_desk::path_at(&crate::reference_desk::root(),&entry.id){Ok(path)=>self.research_media(path,entry.name),Err(e)=>self.status_banner=e}}
                            ui.separator();
                        });
                    }
                });
                left.add_space(14.);left.heading("Sources in this project");
                if self.project.production.research.is_empty(){left.label("Import a source to keep a portable research copy here.");}
                let sources=self.project.production.research.iter().map(|s|(s.id.clone(),s.title.clone(),s.filename.clone(),s.kind.clone(),!s.cues.is_empty())).collect::<Vec<_>>();
                for (id,title,_filename,kind,has_cues) in sources.into_iter().filter(|s|format!("{} {}",s.1,s.2).to_lowercase().contains(&search)) {
                    if left.selectable_label(self.research_ui.selected.as_ref()==Some(&id),format!("{} · {}",title,kind)).clicked(){self.research_ui.cue=if has_cues{Some(0)}else{None};self.research_ui.selected=Some(id);self.research_ui.preview=None;}
                }
                let right=&mut columns[1];right.heading("Read and use");
                let saved=self.research_ui.selected.as_ref().and_then(|id|self.project.production.research.iter().find(|s|&s.id==id)).cloned();
                if let Some(source)=saved.or_else(||self.research_ui.preview.clone()) {
                    right.strong(&source.title);
                    if let Some(url)=&source.source_url{right.hyperlink_to("Open original source",url);}
                    if let Some(method)=&source.method{right.small(method);}
                    if self.research_ui.preview.is_some(){
                        right.label("Preview only. Copy this transcript into the project to use passages.");
                        if right.button("Copy source into project").clicked(){let candidate=self.project.clone();let source=source.clone();self.research_commit(candidate,|p|crate::reference_desk::import_source(p,source));}
                    }else{
                        right.horizontal_wrapped(|ui|{
                            if !source.text.is_empty()&&ui.button("Add passage to page notes").clicked(){self.research_action("research_note",&source.id,self.research_ui.cue);}
                            if source.kind=="srt"&&ui.button("Add captions to video").clicked(){self.research_action("research_captions",&source.id,None);}
                            if source.media_asset.is_some()&&ui.button("Open video library").clicked(){self.screen=WorkspaceScreen::StudioEditor;self.active_dock=DockTab::Film;}
                            if ui.button("Remove project copy").clicked(){self.research_action("research_remove",&source.id,None);}
                        });
                    }
                    right.add_space(8.);
                    ScrollArea::vertical().id_salt("research-reader").max_height(300.).show(right,|ui|{
                        if source.cues.is_empty(){ui.label(&source.text);}else{
                            for (index,cue) in source.cues.iter().enumerate(){if ui.selectable_label(self.research_ui.cue==Some(index),format!("{:02}:{:02}  {}",cue.start_ms/60_000,cue.start_ms/1000%60,cue.text)).clicked(){self.research_ui.cue=Some(index);}}
                        }
                    });
                }else{right.label("Preview a saved transcript, or select a source already in this project.");}
                right.add_space(12.);right.strong(format!("Notes for page {}",self.current_page_idx+1));
                if let Some(page)=self.project.pages.get(self.current_page_idx){ScrollArea::vertical().id_salt("research-page-notes").max_height(150.).show(right,|ui|{ui.label(if page.notes.is_empty(){"Choose a passage, then add it to page notes."}else{&page.notes});});}
            });
        });
    }
}
