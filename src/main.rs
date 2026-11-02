mod batch_production;
mod book_production;
#[cfg(test)]
mod book_tests;
mod book_ui;
mod browser_editor;
mod canvas_view;
mod document_ops;
mod exporter;
mod imported_fonts;
#[cfg(test)]
mod library_tests;
mod magic_studio;
mod model;
mod paragraph_layout;
#[cfg(test)]
mod print_studio_tests;
mod production;
#[cfg(test)]
mod production_tests;
mod production_ui;
mod project_package;
mod puzzle_production;
#[cfg(test)]
mod puzzle_tests;
mod puzzle_ui;
mod template_library;
mod templates;
#[cfg(test)]
mod typography_tests;
#[cfg(test)]
mod workflow_tests;

use std::path::{Path, PathBuf};

use eframe::egui::{
    self, Color32, Context, Key, Layout, Margin, RichText, Rounding, ScrollArea, Stroke,
    TopBottomPanel, Ui, Vec2,
};

use crate::canvas_view::{render_interactive_canvas, CanvasAction, CanvasInteractionState};
#[cfg(test)]
use crate::exporter::export_page_to_svg;
use crate::exporter::{
    export_project_to_html_deck, load_project_json, render_project_page_at_dpi, save_project_json,
};
use crate::magic_studio::{
    extract_magic_layers_from_image, magic_erase_patch, magic_resize_project, magic_write_generate,
    upscale_image_2x, MagicWriteTone,
};
use crate::model::import_layout::{import_layout_json, replace_element_image_from_path};
use crate::model::pdf_export::{
    export_project_to_pdf, run_export_preflight, PageSelection, PdfExportOptions,
};
use crate::model::{
    rgb, rgba, to_egui_color, BrushType, CanvasElement, CanvasPage, ChartType, ColorRgba,
    DesignFormat, DropShadow, ElementAnimation, ElementKind, FillStyle, FontFamily, HistoryManager,
    ImageAdjustments, ImageFit, ImageMask, ListStyle, PageBackground, PageTransition, Project,
    ShapeType, TextAlign, TextEffect,
};
use crate::templates::{
    available_templates, build_template_project, generate_builtin_stock_assets,
    make_stock_image_element, SampleStockAsset,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WorkspaceScreen {
    HomeHub,
    StudioEditor,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DockTab {
    Templates,
    Elements,
    Text,
    BrandKit,
    Uploads,
    MagicStudio,
    ChartsAndTables,
    Draw,
    Layers,
    Production,
    Book,
    Puzzles,
    AppInfo,
}

impl DockTab {
    fn all() -> &'static [DockTab] {
        &[
            DockTab::Templates,
            DockTab::Elements,
            DockTab::Text,
            DockTab::BrandKit,
            DockTab::Uploads,
            DockTab::MagicStudio,
            DockTab::ChartsAndTables,
            DockTab::Draw,
            DockTab::Layers,
            DockTab::Production,
            DockTab::Book,
            DockTab::Puzzles,
            DockTab::AppInfo,
        ]
    }

    fn icon_and_label(&self) -> (&'static str, &'static str) {
        match self {
            DockTab::Templates => ("📐", "Design"),
            DockTab::Elements => ("❖", "Elements"),
            DockTab::Text => ("𝐓", "Text"),
            DockTab::BrandKit => ("👑", "Brand"),
            DockTab::Uploads => ("🖼", "Uploads"),
            DockTab::MagicStudio => ("✨", "Tools"),
            DockTab::ChartsAndTables => ("📊", "Charts"),
            DockTab::Draw => ("✏", "Draw"),
            DockTab::Layers => ("☰", "Layers"),
            DockTab::Production => ("▤", "Production"),
            DockTab::Book => ("▥", "Book"),
            DockTab::Puzzles => ("▦", "Puzzles"),
            DockTab::AppInfo => ("ⓘ", "About"),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SaveOutcome {
    Saved,
    Cancelled,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReplacementIntent {
    OpenSaved(PathBuf),
    NewOrTemplate,
    RecoverUnsaved {
        recovery_path: PathBuf,
        origin_path: Option<PathBuf>,
    },
}

#[derive(Clone, Debug)]
pub struct PendingDocumentReplacement {
    pub project: Project,
    pub path: Option<PathBuf>,
    pub banner: String,
    pub intent: ReplacementIntent,
}

struct CanvasStudioApp {
    screen: WorkspaceScreen,
    project: Project,
    current_page_idx: usize,
    selected_id: Option<u64>,
    active_dock: DockTab,
    dock_open: bool,
    canvas_state: CanvasInteractionState,
    history: HistoryManager,
    stock_assets: Vec<SampleStockAsset>,
    status_banner: String,
    presentation_mode: bool,
    show_export_modal: bool,
    export_transparent: bool,
    export_dpi: u32,
    pdf_selection_mode: usize,
    pdf_range_start: usize,
    pdf_range_end: usize,
    custom_w_input: f32,
    custom_h_input: f32,
    custom_size_unit: u8, // 0 = px, 1 = mm, 2 = in
    magic_write_topic: String,
    magic_write_tone: MagicWriteTone,
    palette_shuffle_cursor: usize,
    clipboard_elements: Vec<CanvasElement>,
    library_entries: Vec<template_library::TemplateEntry>,
    library_warnings: Vec<String>,
    library_filter: String,
    library_name: String,
    library_tags: String,
    library_previews: std::collections::HashMap<String, egui::TextureHandle>,
    production_key: String,
    shared_style_name: String,
    batch_editable: bool,
    production_job:
        Option<std::sync::mpsc::Receiver<Result<crate::production_ui::ProductionOutcome, String>>>,
    completed_artwork: Option<Project>,
    puzzle_draft: crate::puzzle_ui::PuzzleDraft,
    master_edit_session: Option<crate::book_ui::MasterEditSession>,
    book_page_offset: usize,
    book_preview_cache: std::collections::HashMap<(String, u64, u64), egui::TextureHandle>,
    output_preset_name: String,
    new_guide_mm: f32,
    inspector_unit: u8,
    inspector_aspect_lock: bool,
    show_notes_drawer: bool,
    current_project_path: Option<PathBuf>,
    dirty: bool,
    document_revision: u64,
    saved_revision: Option<u64>,
    revision_clock: u64,
    continuous_edit_active: bool,
    mutation_recorded_this_frame: bool,
    last_autosave_time: f64,
    pending_replace_project: Option<PendingDocumentReplacement>,
    pending_close: bool,
    allow_close_without_prompt: bool,
    recovery_available: bool,
    active_recovery_path: Option<PathBuf>,
    recovery_dir_override: Option<PathBuf>,
    last_save_error: Option<String>,
    last_viewport_size: Vec2,
    pending_canvas_fit: bool,
    auto_fit_canvas: bool,
    frame_snapshot_cache: crate::model::SnapshotCache,
    queued_save: Option<bool>,
    queued_history: i8,
}

impl CanvasStudioApp {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::new_headless_with_recovery_dir(None)
    }

    fn new_headless_with_recovery_dir(recovery_dir_override: Option<PathBuf>) -> Self {
        let project = Project::default();
        let stock_assets = generate_builtin_stock_assets();

        let mut app = Self {
            screen: WorkspaceScreen::StudioEditor,
            project,
            current_page_idx: 0,
            selected_id: None,
            active_dock: DockTab::Templates,
            dock_open: true,
            canvas_state: CanvasInteractionState::new(),
            history: HistoryManager::default(),
            stock_assets,
            status_banner: "Ready • Create a document or choose a studio template".to_string(),
            presentation_mode: false,
            show_export_modal: false,
            export_transparent: false,
            export_dpi: 300,
            pdf_selection_mode: 1,
            pdf_range_start: 1,
            pdf_range_end: 1,
            custom_w_input: 1200.0,
            custom_h_input: 900.0,
            custom_size_unit: 0,
            magic_write_topic: "Your next creative project".to_string(),
            magic_write_tone: MagicWriteTone::BoldLaunch,
            palette_shuffle_cursor: 0,
            clipboard_elements: Vec::new(),
            library_entries: Vec::new(),
            library_warnings: Vec::new(),
            library_filter: String::new(),
            library_name: String::new(),
            library_tags: String::new(),
            library_previews: Default::default(),
            production_key: String::new(),
            shared_style_name: String::new(),
            batch_editable: false,
            production_job: None,
            completed_artwork: None,
            puzzle_draft: Default::default(),
            master_edit_session: None,
            book_page_offset: 0,
            book_preview_cache: Default::default(),
            output_preset_name: "Print output".into(),
            new_guide_mm: 10.,
            inspector_unit: 0,
            inspector_aspect_lock: true,
            show_notes_drawer: false,
            current_project_path: None,
            dirty: false,
            document_revision: 0,
            saved_revision: Some(0),
            revision_clock: 1,
            continuous_edit_active: false,
            mutation_recorded_this_frame: false,
            last_autosave_time: 0.0,
            pending_replace_project: None,
            pending_close: false,
            allow_close_without_prompt: false,
            recovery_available: false,
            active_recovery_path: None,
            recovery_dir_override,
            last_save_error: None,
            last_viewport_size: Vec2::new(1100.0, 720.0),
            pending_canvas_fit: true,
            auto_fit_canvas: true,
            frame_snapshot_cache: Default::default(),
            queued_save: None,
            queued_history: 0,
        };
        app.recovery_available = app.latest_available_recovery_path().is_some();
        app.refresh_template_library();
        app
    }

    fn template_library_dir(&self) -> PathBuf {
        self.recovery_dir_override
            .as_ref()
            .map(|p| p.join("templates"))
            .unwrap_or_else(|| application_data_dir().join("canvas-studio/templates"))
    }

    fn refresh_template_library(&mut self) {
        match template_library::list(&self.template_library_dir()) {
            Ok((entries, warnings)) => {
                self.library_entries = entries;
                self.library_warnings = warnings;
            }
            Err(e) => self.library_warnings = vec![e],
        }
    }

    fn save_library_template(&mut self) {
        let name = if self.library_name.trim().is_empty() {
            self.project.title.clone()
        } else {
            self.library_name.clone()
        };
        match template_library::save(
            &self.template_library_dir(),
            &name,
            &self.library_tags,
            &self.project,
        ) {
            Ok(entry) => {
                self.status_banner = format!("Saved master '{}' to My Templates", entry.name);
                self.refresh_template_library();
            }
            Err(e) => self.status_banner = format!("Template could not be saved: {e}"),
        }
    }

    fn open_library_copy(&mut self, id: &str) {
        match template_library::open_copy(&self.template_library_dir(), id) {
            Ok(project) => {
                let title = project.title.clone();
                self.request_replace_document(
                    project,
                    None,
                    format!("New working copy of '{title}' — master preserved"),
                );
            }
            Err(e) => self.status_banner = format!("Template could not be opened: {e}"),
        }
    }

    fn render_template_library(&mut self, ui: &mut Ui) {
        ui.strong("My Templates");
        ui.label("Saved masters stay unchanged. Open a fresh working copy.");
        ui.add(
            egui::TextEdit::singleline(&mut self.library_name)
                .hint_text("Template name (defaults to document title)"),
        );
        ui.add(
            egui::TextEdit::singleline(&mut self.library_tags)
                .hint_text("Tags, separated, by commas"),
        );
        if ui.button("Save Current Design as New Master").clicked() {
            self.save_library_template();
        }
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.library_filter)
                    .desired_width(180.)
                    .hint_text("Search name or tag"),
            );
            if ui.small_button("Refresh").clicked() {
                self.refresh_template_library();
            }
        });
        for warning in &self.library_warnings {
            ui.colored_label(Color32::LIGHT_RED, warning);
        }
        let entries: Vec<_> = self
            .library_entries
            .iter()
            .filter(|e| e.matches(&self.library_filter))
            .cloned()
            .collect();
        if entries.is_empty() {
            ui.label("No matching saved templates yet.");
        }
        for entry in entries {
            egui::Frame::group(ui.style()).show(ui, |ui| {
                if !self.library_previews.contains_key(&entry.id) {
                    if let Ok(image) = image::open(
                        self.template_library_dir()
                            .join(&entry.id)
                            .join("preview.png"),
                    ) {
                        let image = image.to_rgba8();
                        let texture = ui.ctx().load_texture(
                            format!("template-{}", entry.id),
                            egui::ColorImage::from_rgba_unmultiplied(
                                [image.width() as usize, image.height() as usize],
                                image.as_raw(),
                            ),
                            egui::TextureOptions::LINEAR,
                        );
                        self.library_previews.insert(entry.id.clone(), texture);
                    }
                }
                if let Some(texture) = self.library_previews.get(&entry.id) {
                    ui.add(egui::Image::new(texture).max_size(Vec2::new(230., 160.)));
                }
                ui.strong(&entry.name);
                ui.label(format!("{} · {} page(s)", entry.format, entry.page_count));
                ui.label(entry.tags.join(", "));
                if ui.button("Open New Copy").clicked() {
                    self.open_library_copy(&entry.id);
                }
            });
        }
    }

    fn recovery_dir(&self) -> PathBuf {
        self.recovery_dir_override
            .clone()
            .unwrap_or_else(|| application_data_dir().join("canvas-studio/recovery"))
    }

    fn recovery_path_for_document(&self, document_id: &str) -> PathBuf {
        let safe_id: String = document_id
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '-' || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        let id_part = if safe_id.is_empty() {
            "default"
        } else {
            &safe_id
        };
        self.recovery_dir()
            .join(format!("canvas_autosave_recovery_{id_part}.canvas.json"))
    }

    fn legacy_recovery_path(&self) -> PathBuf {
        self.recovery_dir_override
            .clone()
            .unwrap_or_else(std::env::temp_dir)
            .join("canvas_autosave_recovery.canvas.json")
    }

    fn latest_available_recovery_path(&self) -> Option<PathBuf> {
        if let Some(active) = &self.active_recovery_path {
            if active.is_file() {
                return Some(active.clone());
            }
        }
        let dir = self.recovery_dir();
        let mut best: Option<(std::time::SystemTime, PathBuf)> = None;
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if !path.is_file() {
                    continue;
                }
                let Some(fname) = path.file_name().and_then(|s| s.to_str()) else {
                    continue;
                };
                if fname.starts_with("canvas_autosave_recovery") && fname.ends_with(".canvas.json")
                {
                    let mtime = entry
                        .metadata()
                        .and_then(|m| m.modified())
                        .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                    if best.as_ref().map(|(t, _)| mtime >= *t).unwrap_or(true) {
                        best = Some((mtime, path));
                    }
                }
            }
        }
        let legacy = self.legacy_recovery_path();
        if legacy.is_file() {
            let mtime = legacy
                .metadata()
                .and_then(|m| m.modified())
                .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
            if best.as_ref().map(|(t, _)| mtime >= *t).unwrap_or(true) {
                best = Some((mtime, legacy));
            }
        }
        best.map(|(_, p)| p)
    }

    /// Removes only recovery files belonging to `owned_doc_id`, never deleting another document's recovery snapshot.
    fn remove_owned_recovery_files(&mut self, owned_doc_id: &str) {
        let doc_rec = self.recovery_path_for_document(owned_doc_id);
        if doc_rec.exists() {
            let _ = std::fs::remove_file(&doc_rec);
        }

        if let Some(active_rec) = self.active_recovery_path.clone() {
            let owns_active = active_rec == doc_rec || self.project.document_id == owned_doc_id;
            if owns_active {
                let _ = std::fs::remove_file(&active_rec);
                self.active_recovery_path = None;
            }
        }

        let legacy_rec = self.legacy_recovery_path();
        if legacy_rec.exists() {
            let owns_legacy = load_project_json(&legacy_rec)
                .map(|p| p.document_id == owned_doc_id)
                .unwrap_or(false);
            if owns_legacy {
                let _ = std::fs::remove_file(&legacy_rec);
            }
        }

        self.recovery_available = self.latest_available_recovery_path().is_some();
    }

    fn advance_revision(&mut self) {
        self.allow_close_without_prompt = false;
        self.document_revision = self.revision_clock;
        self.revision_clock += 1;
    }

    fn sync_dirty_from_revision(&mut self) {
        self.dirty = self.saved_revision != Some(self.document_revision);
    }

    fn apply_replacement(&mut self, mut pending: PendingDocumentReplacement) {
        if pending.project.pages.is_empty() {
            pending.project.pages.push(CanvasPage::new(1, "Page 1"));
        }
        self.master_edit_session = None;
        self.pending_canvas_fit = true;
        self.auto_fit_canvas = true;
        self.history.clear();
        self.canvas_state.reset_for_document();
        self.selected_id = None;
        self.current_page_idx = 0;
        self.pdf_range_start = 1;
        self.pdf_range_end = pending.project.pages.len().max(1);
        self.continuous_edit_active = false;
        self.allow_close_without_prompt = false;
        self.mutation_recorded_this_frame = true;
        self.last_save_error = None;

        match pending.intent {
            ReplacementIntent::OpenSaved(saved_path) => {
                pending.project.recovery_origin_path = Some(saved_path.clone());
                self.project = pending.project;
                self.current_project_path = Some(saved_path);
                self.active_recovery_path = None;
                self.document_revision = 0;
                self.saved_revision = Some(0);
                self.dirty = false;
            }
            ReplacementIntent::NewOrTemplate => {
                self.project = pending.project;
                self.current_project_path = pending.path;
                self.active_recovery_path = None;
                self.document_revision = 0;
                self.saved_revision = Some(0);
                self.dirty = false;
            }
            ReplacementIntent::RecoverUnsaved {
                recovery_path,
                origin_path,
            } => {
                let effective_origin =
                    origin_path.or_else(|| pending.project.recovery_origin_path.clone());
                pending.project.recovery_origin_path = effective_origin.clone();
                self.project = pending.project;
                self.current_project_path = effective_origin;
                self.active_recovery_path = Some(recovery_path);
                self.document_revision = 1;
                self.saved_revision = None;
                self.dirty = true;
            }
        }

        self.revision_clock = self.document_revision + 1;
        self.pending_replace_project = None;
        self.screen = WorkspaceScreen::StudioEditor;
        self.status_banner = pending.banner;
        self.recovery_available = self.latest_available_recovery_path().is_some();
    }

    /// Centralizes document replacement (`Open`, `New`, `Template`, `Layout Import`) so old undo
    /// history, selection, inline edit state, and page index are always reset cleanly.
    #[allow(dead_code)] // Convenience entrypoint retained for model tests and native integrations.
    fn replace_document(&mut self, new_project: Project, path: Option<PathBuf>) {
        let intent = match &path {
            Some(p) => ReplacementIntent::OpenSaved(p.clone()),
            None => ReplacementIntent::NewOrTemplate,
        };
        let banner = self.status_banner.clone();
        self.apply_replacement(PendingDocumentReplacement {
            project: new_project,
            path,
            banner,
            intent,
        });
    }

    fn request_replace_document_with_intent(
        &mut self,
        new_project: Project,
        path: Option<PathBuf>,
        banner: String,
        intent: ReplacementIntent,
    ) {
        let pending = PendingDocumentReplacement {
            project: new_project,
            path,
            banner,
            intent,
        };
        // Commit replacement only after this frame's text/widget edits have been observed.
        self.pending_replace_project = Some(pending);
    }

    /// Prompts for confirmation if the current project has unsaved changes before replacing it.
    fn request_replace_document(
        &mut self,
        new_project: Project,
        path: Option<PathBuf>,
        banner: String,
    ) {
        let intent = match &path {
            Some(p) => ReplacementIntent::OpenSaved(p.clone()),
            None => ReplacementIntent::NewOrTemplate,
        };
        self.request_replace_document_with_intent(new_project, path, banner, intent);
    }

    /// Requests recovery of an unsaved recovery snapshot from disk, preserving the snapshot on disk
    /// and opening the recovered document with `dirty = true`.
    fn request_recover_document_from_path(&mut self, recovery_path: &Path) -> Result<(), String> {
        let proj = load_project_json(recovery_path)?;
        let origin_path = proj.recovery_origin_path.clone();
        let banner = format!("Recovered unsaved project '{}' from autosave", proj.title);
        self.request_replace_document_with_intent(
            proj,
            origin_path.clone(),
            banner,
            ReplacementIntent::RecoverUnsaved {
                recovery_path: recovery_path.to_path_buf(),
                origin_path,
            },
        );
        Ok(())
    }

    /// Reconciles `current_page_idx`, `selected_id`, `selected_ids`, and `inline_edit_id`
    /// after any `undo`, `redo`, or page mutation so stale indices/IDs never occur.
    fn reconcile_selection_and_page(&mut self) {
        if self.project.pages.is_empty() {
            self.project.pages.push(CanvasPage::new(1, "Page 1"));
        }
        if self.current_page_idx >= self.project.pages.len() {
            self.current_page_idx = self.project.pages.len() - 1;
        }
        let page = &self.project.pages[self.current_page_idx];
        if let Some(sel) = self.selected_id {
            if !page.elements.iter().any(|e| e.id == sel) {
                self.selected_id = None;
            }
        }
        if let Some(edit_id) = self.canvas_state.inline_edit_id {
            if !page.elements.iter().any(|e| e.id == edit_id) {
                self.canvas_state.inline_edit_id = None;
            }
        }
        self.canvas_state
            .selected_ids
            .retain(|id| page.elements.iter().any(|e| e.id == *id));
        if let Some(sel) = self.selected_id {
            if !self.canvas_state.selected_ids.contains(&sel) {
                self.canvas_state.selected_ids.insert(0, sel);
            }
        }
    }

    fn record_mutation(&mut self) {
        let _ = self
            .history
            .record_with_revision(&self.project, self.document_revision);
        self.advance_revision();
        self.sync_dirty_from_revision();
        self.mutation_recorded_this_frame = true;
    }

    fn perform_undo(&mut self) -> bool {
        if self.master_edit_session.is_some() {
            self.status_banner = "Save or cancel the active master edit before undo/redo".into();
            return false;
        }
        if self
            .history
            .undo_with_revision(&mut self.project, &mut self.document_revision)
        {
            self.reconcile_selection_and_page();
            self.sync_dirty_from_revision();
            self.mutation_recorded_this_frame = true;
            self.continuous_edit_active = false;
            self.status_banner = "Undo applied".to_string();
            true
        } else {
            false
        }
    }

    fn perform_redo(&mut self) -> bool {
        if self.master_edit_session.is_some() {
            self.status_banner = "Save or cancel the active master edit before undo/redo".into();
            return false;
        }
        if self
            .history
            .redo_with_revision(&mut self.project, &mut self.document_revision)
        {
            self.reconcile_selection_and_page();
            self.sync_dirty_from_revision();
            self.mutation_recorded_this_frame = true;
            self.continuous_edit_active = false;
            self.status_banner = "Redo applied".to_string();
            true
        } else {
            false
        }
    }

    fn set_custom_size_unit(&mut self, new_unit: u8) {
        if new_unit != self.custom_size_unit {
            let (nw, nh) = self.project.convert_dimension_units(
                self.custom_w_input,
                self.custom_h_input,
                self.custom_size_unit,
                new_unit,
            );
            self.custom_w_input = nw;
            self.custom_h_input = nh;
            self.custom_size_unit = new_unit;
        }
    }

    fn apply_custom_size(&mut self) -> Result<(), String> {
        let mut candidate = self.project.clone();
        match candidate.apply_custom_dimensions(
            self.custom_w_input,
            self.custom_h_input,
            self.custom_size_unit,
            true,
        ) {
            Ok(()) => {
                self.record_mutation();
                self.project = candidate;
                self.status_banner = format!(
                    "Applied custom size {:.1}×{:.1} px ({:.2}×{:.2} mm)",
                    self.project.width,
                    self.project.height,
                    self.project.effective_width_mm(),
                    self.project.effective_height_mm()
                );
                Ok(())
            }
            Err(e) => {
                self.status_banner = format!("Invalid custom size: {e}");
                Err(e)
            }
        }
    }

    fn fitted_canvas_zoom(&self) -> f32 {
        let ruler_pad = if self.canvas_state.show_rulers {
            24.0
        } else {
            0.0
        };
        let avail_w = (self.last_viewport_size.x - ruler_pad - 64.0).max(160.0);
        let avail_h = (self.last_viewport_size.y - ruler_pad - 64.0).max(160.0);
        let zx = avail_w / self.project.width.max(1.0);
        let zy = avail_h / self.project.height.max(1.0);
        zx.min(zy).clamp(0.15, 2.5)
    }

    fn fit_canvas_to_viewport(&mut self) {
        self.pending_canvas_fit = false;
        self.auto_fit_canvas = true;
        self.canvas_state.zoom = self.fitted_canvas_zoom();
        self.canvas_state.pan = Vec2::ZERO;
    }

    fn request_close_document(&mut self) -> bool {
        if self.dirty && !self.allow_close_without_prompt {
            self.pending_close = true;
            false
        } else {
            true
        }
    }

    fn confirm_unsaved_save_and_continue(&mut self) -> SaveOutcome {
        let outcome = self.save_current_or_prompt();
        if outcome == SaveOutcome::Saved {
            if let Some(pending) = self.pending_replace_project.take() {
                self.apply_replacement(pending);
            }
            if self.pending_close {
                self.pending_close = false;
                self.allow_close_without_prompt = true;
            }
        }
        outcome
    }

    fn confirm_unsaved_discard(&mut self) {
        let discarded_id = self.project.document_id.clone();
        self.remove_owned_recovery_files(&discarded_id);
        self.dirty = false;
        self.saved_revision = Some(self.document_revision);
        if let Some(pending) = self.pending_replace_project.take() {
            self.apply_replacement(pending);
        }
        if self.pending_close {
            self.pending_close = false;
            self.allow_close_without_prompt = true;
        }
    }

    fn confirm_unsaved_cancel(&mut self) {
        self.pending_replace_project = None;
        self.pending_close = false;
    }

    fn group_selected_elements(&mut self) {
        if self.canvas_state.selected_ids.len() < 2 {
            return;
        }
        self.record_mutation();
        let gid = self.project.alloc_id();
        let selected_set = self.canvas_state.selected_ids.clone();
        if let Some(page) = self.current_page_mut() {
            for el in &mut page.elements {
                if selected_set.contains(&el.id) {
                    el.group_id = Some(gid);
                }
            }
        }
        self.status_banner = format!("Grouped {} elements", selected_set.len());
    }

    fn ungroup_selected_elements(&mut self) {
        let mut target_groups = Vec::new();
        if let Some(page) = self.current_page() {
            for el in &page.elements {
                if (self.selected_id == Some(el.id)
                    || self.canvas_state.selected_ids.contains(&el.id))
                    && el.group_id.is_some()
                {
                    if let Some(g) = el.group_id {
                        if !target_groups.contains(&g) {
                            target_groups.push(g);
                        }
                    }
                }
            }
        }
        if target_groups.is_empty() {
            return;
        }
        self.record_mutation();
        if let Some(page) = self.current_page_mut() {
            for el in &mut page.elements {
                if let Some(g) = el.group_id {
                    if target_groups.contains(&g) {
                        el.group_id = None;
                    }
                }
            }
        }
        self.status_banner = "Ungrouped selected elements".to_string();
    }

    fn current_page_mut(&mut self) -> Option<&mut CanvasPage> {
        self.project.pages.get_mut(self.current_page_idx)
    }

    fn current_page(&self) -> Option<&CanvasPage> {
        self.project.pages.get(self.current_page_idx)
    }

    fn handle_keyboard_shortcuts(&mut self, ctx: &Context) {
        if !self.presentation_mode {
            let action = ctx.input_mut(|i| {
                let pressed = |i: &mut egui::InputState, key, shift| {
                    let ctrl = egui::Modifiers {
                        ctrl: true,
                        shift,
                        ..Default::default()
                    };
                    let command = egui::Modifiers {
                        command: true,
                        shift,
                        ..Default::default()
                    };
                    i.consume_key(ctrl, key) || i.consume_key(command, key)
                };
                if pressed(i, Key::S, false) {
                    Some(0)
                } else if pressed(i, Key::Z, true) || pressed(i, Key::Y, false) {
                    Some(2)
                } else if pressed(i, Key::Z, false) {
                    Some(1)
                } else {
                    None
                }
            });
            if let Some(action) = action {
                match action {
                    0 => self.queued_save = Some(false),
                    1 => self.queued_history = -1,
                    _ => self.queued_history = 1,
                }
                return;
            }
        }
        if ctx.wants_keyboard_input() && !self.presentation_mode {
            return;
        }

        if self.presentation_mode {
            if ctx.input(|i| i.key_pressed(Key::Escape)) {
                self.presentation_mode = false;
            } else if ctx.input(|i| i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::Space)) {
                if self.current_page_idx + 1 < self.project.pages.len() {
                    self.current_page_idx += 1;
                    self.canvas_state.anim_start_time = ctx.input(|i| i.time);
                }
            } else if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) && self.current_page_idx > 0 {
                self.current_page_idx -= 1;
                self.canvas_state.anim_start_time = ctx.input(|i| i.time);
            }
            return;
        }

        let ctrl = ctx.input(|i| i.modifiers.ctrl || i.modifiers.command);
        let shift = ctx.input(|i| i.modifiers.shift);

        if ctrl && ctx.input(|i| i.key_pressed(Key::Z)) {
            if shift {
                let _ = self.perform_redo();
            } else {
                let _ = self.perform_undo();
            }
        } else if ctrl && ctx.input(|i| i.key_pressed(Key::Y)) {
            let _ = self.perform_redo();
        } else if ctrl && ctx.input(|i| i.key_pressed(Key::G)) {
            if shift {
                self.ungroup_selected_elements();
            } else {
                self.group_selected_elements();
            }
        } else if ctrl && ctx.input(|i| i.key_pressed(Key::S)) {
            let _ = self.save_current_or_prompt();
        } else if ctrl && ctx.input(|i| i.key_pressed(Key::D)) {
            self.duplicate_selected();
        } else if ctrl && ctx.input(|i| i.key_pressed(Key::C)) {
            self.clipboard_elements = self
                .current_page()
                .map(|p| {
                    document_ops::selected_elements(
                        p,
                        &self.canvas_state.selected_ids,
                        self.selected_id,
                    )
                })
                .unwrap_or_default();
            self.status_banner = format!("Copied {} element(s)", self.clipboard_elements.len());
        } else if ctrl && ctx.input(|i| i.key_pressed(Key::V)) {
            self.insert_copies(self.clipboard_elements.clone(), 28.0);
        } else if ctx.input(|i| i.key_pressed(Key::Delete) || i.key_pressed(Key::Backspace)) {
            self.delete_selected();
        } else if ctx.input(|i| i.key_pressed(Key::F5)) {
            self.presentation_mode = true;
            self.canvas_state.anim_start_time = ctx.input(|i| i.time);
        }

        // Arrow nudging
        let step = if shift { 10.0 } else { 2.0 };
        let mut nudge = Vec2::ZERO;
        if ctx.input(|i| i.key_pressed(Key::ArrowLeft)) {
            nudge.x -= step;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowRight)) {
            nudge.x += step;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowUp)) {
            nudge.y -= step;
        }
        if ctx.input(|i| i.key_pressed(Key::ArrowDown)) {
            nudge.y += step;
        }
        if nudge != Vec2::ZERO {
            self.nudge_selected(nudge);
        }
    }

    fn active_selection_ids(&self) -> Vec<u64> {
        self.current_page()
            .map(|p| {
                document_ops::selection_ids(p, &self.canvas_state.selected_ids, self.selected_id)
            })
            .unwrap_or_default()
    }

    fn selection_is_locked(&self, ids: &[u64]) -> bool {
        self.current_page()
            .is_some_and(|p| p.elements.iter().any(|e| ids.contains(&e.id) && e.locked))
    }

    fn select_layer(&mut self, id: u64) {
        self.selected_id = Some(id);
        self.canvas_state.selected_ids = self
            .current_page()
            .map(|p| document_ops::selection_ids(p, &[id], Some(id)))
            .unwrap_or_default();
    }

    fn nudge_selected(&mut self, delta: Vec2) {
        let ids = self.active_selection_ids();
        if ids.is_empty() || self.selection_is_locked(&ids) {
            return;
        }
        self.record_mutation();
        if let Some(page) = self.current_page_mut() {
            for e in &mut page.elements {
                if ids.contains(&e.id) {
                    e.x += delta.x;
                    e.y += delta.y;
                }
            }
        }
    }

    fn save_current_or_prompt(&mut self) -> SaveOutcome {
        let target_path = if let Some(existing) = self.current_project_path.clone() {
            Some(existing)
        } else {
            rfd::FileDialog::new()
                .set_file_name("design.canvas.json")
                .save_file()
        };
        if let Some(path) = target_path {
            self.write_project_to_path(&path)
        } else {
            SaveOutcome::Cancelled
        }
    }

    fn save_as_prompt(&mut self) -> SaveOutcome {
        let default_name = self
            .current_project_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("design.canvas.json");
        if let Some(path) = rfd::FileDialog::new()
            .set_file_name(default_name)
            .save_file()
        {
            self.write_project_to_path(&path)
        } else {
            SaveOutcome::Cancelled
        }
    }

    fn write_project_to_path(&mut self, path: &PathBuf) -> SaveOutcome {
        if self.master_edit_session.is_some() {
            self.finish_master_edit(true);
        }
        match save_project_json(&self.project, path) {
            Ok(_) => {
                self.mutation_recorded_this_frame = true;
                self.continuous_edit_active = false;
                self.current_project_path = Some(path.clone());
                self.project.recovery_origin_path = Some(path.clone());
                self.saved_revision = Some(self.document_revision);
                self.dirty = false;
                self.last_save_error = None;
                let owned_id = self.project.document_id.clone();
                self.remove_owned_recovery_files(&owned_id);
                self.status_banner = format!("Saved project to {}", path.display());
                SaveOutcome::Saved
            }
            Err(e) => {
                self.last_save_error = Some(e.clone());
                self.status_banner = format!("Save error: {e}");
                SaveOutcome::Failed
            }
        }
    }

    fn maybe_run_autosave(&mut self, now: f64) {
        if self.dirty && (now - self.last_autosave_time) >= 30.0 {
            self.last_autosave_time = now;
            if let Err(e) = self.write_recovery_snapshot() {
                self.status_banner = format!("Autosave failed: {e}");
            }
        }
    }

    fn write_recovery_snapshot(&mut self) -> Result<PathBuf, String> {
        self.project.recovery_origin_path = self.current_project_path.clone();
        let rec_path = self.recovery_path_for_document(&self.project.document_id);
        save_project_json(&self.project, &rec_path)?;
        self.active_recovery_path = Some(rec_path.clone());
        self.recovery_available = true;
        Ok(rec_path)
    }

    fn insert_copies(&mut self, elements: Vec<CanvasElement>, offset: f32) {
        if elements.is_empty() {
            return;
        }
        self.record_mutation();
        let copies = document_ops::fresh_copies(&mut self.project, &elements, offset, offset);
        let ids: Vec<_> = copies.iter().map(|e| e.id).collect();
        if let Some(page) = self.current_page_mut() {
            page.elements.extend(copies);
        }
        self.selected_id = ids.last().copied();
        self.canvas_state.selected_ids = ids;
        self.status_banner = format!("Inserted {} copied element(s)", elements.len());
    }

    fn duplicate_selected(&mut self) {
        let elements = self
            .current_page()
            .map(|p| {
                document_ops::selected_elements(
                    p,
                    &self.canvas_state.selected_ids,
                    self.selected_id,
                )
            })
            .unwrap_or_default();
        self.insert_copies(elements, 30.0);
    }

    fn insert_template_pages(&mut self, source: &Project) {
        self.record_mutation();
        let first = self.project.pages.len();
        let pages = document_ops::fitted_pages(&mut self.project, source);
        let count = pages.len();
        self.project.pages.extend(pages);
        self.current_page_idx = first.min(self.project.pages.len().saturating_sub(1));
        self.selected_id = None;
        self.canvas_state.reset_for_document();
        self.status_banner =
            format!("Inserted {count} page(s), fitted proportionally to this document");
    }

    fn delete_selected(&mut self) {
        let ids = self.active_selection_ids();
        if ids.is_empty() {
            return;
        }
        if self.selection_is_locked(&ids) {
            self.status_banner = "Unlock the selection before deleting it".into();
            return;
        }
        self.record_mutation();
        if let Some(page) = self.current_page_mut() {
            page.elements.retain(|e| !ids.contains(&e.id));
        }
        self.selected_id = None;
        self.canvas_state.selected_ids.clear();
        self.status_banner = "Deleted selected element(s)".into();
    }

    fn move_selected_layer(&mut self, forward: bool) {
        let ids = self
            .current_page()
            .map(|p| {
                document_ops::selection_ids(p, &self.canvas_state.selected_ids, self.selected_id)
            })
            .unwrap_or_default();
        if ids.is_empty() {
            return;
        }
        if self.selection_is_locked(&ids) {
            return;
        }
        self.record_mutation();
        if let Some(page) = self.current_page_mut() {
            if forward {
                for i in (0..page.elements.len().saturating_sub(1)).rev() {
                    if ids.contains(&page.elements[i].id) && !ids.contains(&page.elements[i + 1].id)
                    {
                        page.elements.swap(i, i + 1);
                    }
                }
            } else {
                for i in 1..page.elements.len() {
                    if ids.contains(&page.elements[i].id) && !ids.contains(&page.elements[i - 1].id)
                    {
                        page.elements.swap(i, i - 1);
                    }
                }
            }
        }
    }

    fn toggle_selected_lock(&mut self) {
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
        if selected.is_empty() {
            return;
        }
        self.record_mutation();
        let lock = selected.iter().any(|e| !e.locked);
        if let Some(page) = self.current_page_mut() {
            for e in &mut page.elements {
                if selected.iter().any(|s| s.id == e.id) {
                    e.locked = lock;
                }
            }
        }
    }

    fn import_image_from_disk(&mut self) {
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
            .pick_file()
        {
            self.load_image_path(&path);
        }
    }

    fn replace_selected_image_from_disk(&mut self) {
        let Some(sel_id) = self.selected_id else {
            return;
        };
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Images", &["png", "jpg", "jpeg", "webp", "bmp", "gif"])
            .pick_file()
        {
            let _ = self.replace_image_element_from_path(sel_id, &path);
        }
    }

    fn replace_image_element_from_path(
        &mut self,
        element_id: u64,
        path: &Path,
    ) -> Result<(), String> {
        let pre_snapshot = crate::model::dehydrate_project_assets(&self.project);
        let pre_rev = self.document_revision;
        let Some(el) = self
            .current_page_mut()
            .and_then(|p| p.elements.iter_mut().find(|e| e.id == element_id))
        else {
            return Err(format!("Element {element_id} not found on current page"));
        };
        match replace_element_image_from_path(el, path) {
            Ok(()) => {
                let _ = self.history.record_with_revision(&pre_snapshot, pre_rev);
                self.advance_revision();
                self.sync_dirty_from_revision();
                self.mutation_recorded_this_frame = true;
                self.status_banner = format!("Replaced image with '{}'", path.display());
                Ok(())
            }
            Err(e) => {
                self.status_banner = format!("Replace image error: {e}");
                Err(e)
            }
        }
    }

    fn load_image_path(&mut self, path: &PathBuf) {
        match image::open(path) {
            Ok(dyn_img) => {
                let rgba_img = dyn_img.to_rgba8();
                let (w, h) = rgba_img.dimensions();
                let raw = rgba_img.into_raw();
                self.record_mutation();
                let new_id = self.project.alloc_id();
                let max_dim = 520.0;
                let scale = (max_dim / (w.max(h) as f32)).min(1.0);
                let disp_w = (w as f32 * scale).max(64.0);
                let disp_h = (h as f32 * scale).max(64.0);
                let cx = (self.project.width - disp_w) * 0.5;
                let cy = (self.project.height - disp_h) * 0.5;
                let fname = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("Uploaded Image")
                    .to_string();

                if let Some(page) = self.current_page_mut() {
                    page.elements.push(CanvasElement {
                        id: new_id,
                        name: fname.clone(),
                        x: cx,
                        y: cy,
                        width: disp_w,
                        height: disp_h,
                        rotation: 0.0,
                        opacity: 1.0,
                        locked: false,
                        visible: true,
                        flip_h: false,
                        flip_v: false,
                        shadow: DropShadow::default(),
                        animation: ElementAnimation::Fade,
                        group_id: None,
                        text_format: Default::default(),
                        style_id: None,
                        binding: None,
                        kind: ElementKind::Image {
                            source_name: fname.clone(),
                            width_px: w,
                            height_px: h,
                            raw_rgba: raw.into(),
                            mask: ImageMask::None,
                            adjustments: ImageAdjustments::default(),
                            border_color: rgb(255, 255, 255),
                            border_width: 0.0,
                            revision: 1,
                        },
                    });
                    self.selected_id = Some(new_id);
                    self.canvas_state.selected_ids = vec![new_id];
                    self.status_banner = format!("Imported '{}' ({}x{})", fname, w, h);
                }
            }
            Err(e) => {
                self.status_banner = format!("Failed to load image: {e}");
            }
        }
    }
}

impl eframe::App for CanvasStudioApp {
    fn update(&mut self, ctx: &Context, _frame: &mut eframe::Frame) {
        self.update_ui(ctx);
    }
}

impl CanvasStudioApp {
    fn update_ui(&mut self, ctx: &Context) {
        self.poll_production_job();
        apply_canvas_studio_theme(ctx);
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
        self.maybe_run_autosave(ctx.input(|i| i.time));

        self.mutation_recorded_this_frame = false;
        let pre_frame_dehydrated = self.frame_snapshot_cache.snapshot(&self.project);
        let pre_frame_revision = self.document_revision;

        let decision_was_pending = self.pending_close || self.pending_replace_project.is_some();
        if ctx.input(|i| i.viewport().close_requested()) && !self.allow_close_without_prompt {
            let _ = self.request_close_document();
            self.pending_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
        }

        if decision_was_pending && self.dirty {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("Finish the save decision to continue editing.");
            });
            self.render_unsaved_confirmation_modal(ctx);
            return;
        }
        self.handle_keyboard_shortcuts(ctx);

        if self.presentation_mode {
            self.finish_pending_if_clean(ctx);
            self.render_presentation_overlay(ctx);
            return;
        }

        self.render_top_navigation_bar(ctx);

        match self.screen {
            WorkspaceScreen::HomeHub => {
                egui::CentralPanel::default().show(ctx, |ui| {
                    self.render_home_hub(ui);
                });
            }
            WorkspaceScreen::StudioEditor => {
                self.render_contextual_action_bar(ctx);
                self.render_bottom_pages_bar(ctx);
                self.render_left_dock_panels(ctx);

                egui::CentralPanel::default()
                    .frame(egui::Frame::none().fill(Color32::from_rgb(24, 25, 28)))
                    .show(ctx, |ui| {
                        let available = ui.available_size();
                        if available != self.last_viewport_size && self.auto_fit_canvas {
                            self.pending_canvas_fit = true;
                        }
                        self.last_viewport_size = available;
                        if self.pending_canvas_fit {
                            self.fit_canvas_to_viewport();
                        }
                        let zoom_before = self.canvas_state.zoom;
                        let action = render_interactive_canvas(
                            ctx,
                            ui,
                            &mut self.project,
                            self.current_page_idx,
                            &mut self.selected_id,
                            &mut self.canvas_state,
                            &mut self.history,
                            false,
                        );
                        if self.canvas_state.zoom != zoom_before {
                            self.auto_fit_canvas = false;
                        }
                        match action {
                            CanvasAction::DuplicateSelected => self.duplicate_selected(),
                            CanvasAction::DeleteSelected => self.delete_selected(),
                            CanvasAction::BringForward => self.move_selected_layer(true),
                            CanvasAction::SendBackward => self.move_selected_layer(false),
                            CanvasAction::ToggleLock => self.toggle_selected_lock(),
                            CanvasAction::None => {}
                        }
                    });
            }
        }

        if self.show_export_modal {
            self.render_export_dialog(ctx);
        }

        // Catch any direct UI widget or canvas interaction mutations that occurred during this frame,
        // coalescing continuous drag/slider/inline-typing gestures into a single undo entry while
        // marking dirty immediately on the first change.
        let pointer_down = ctx.input(|i| i.pointer.any_down());
        let post_frame_dehydrated = self.frame_snapshot_cache.snapshot(&self.project);
        if !self.mutation_recorded_this_frame && post_frame_dehydrated != pre_frame_dehydrated {
            if !self.continuous_edit_active {
                let _ = self
                    .history
                    .record_with_revision(&pre_frame_dehydrated, pre_frame_revision);
            }
            self.advance_revision();
            self.sync_dirty_from_revision();
            if pointer_down || self.canvas_state.inline_edit_id.is_some() {
                self.continuous_edit_active = true;
            }
        }
        if !pointer_down && self.canvas_state.inline_edit_id.is_none() {
            self.continuous_edit_active = false;
        }

        match std::mem::take(&mut self.queued_history) {
            -1 => {
                self.perform_undo();
            }
            1 => {
                self.perform_redo();
            }
            _ => {}
        }
        if let Some(save_as) = self.queued_save.take() {
            if save_as {
                let _ = self.save_as_prompt();
            } else {
                let _ = self.save_current_or_prompt();
            }
        }
        self.finish_pending_if_clean(ctx);
        if self.pending_close || self.pending_replace_project.is_some() {
            ctx.request_repaint();
        }
        self.maybe_run_autosave(ctx.input(|i| i.time));
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }

    fn finish_pending_if_clean(&mut self, ctx: &Context) {
        if self.dirty {
            return;
        }
        if let Some(pending) = self.pending_replace_project.take() {
            self.apply_replacement(pending);
            ctx.request_repaint();
        }
        if self.pending_close && !self.dirty {
            self.pending_close = false;
            self.allow_close_without_prompt = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}

impl CanvasStudioApp {
    fn render_top_navigation_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("canvas_top_header")
            .exact_height(78.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(15, 17, 23))
                    .inner_margin(Margin::symmetric(14.0, 8.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 41, 52))),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let home_btn = egui::Button::new(
                        RichText::new("✦ Canvas Studio")
                            .strong()
                            .size(16.0)
                            .color(Color32::from_rgb(0, 245, 212)),
                    )
                    .fill(Color32::from_rgb(28, 32, 46))
                    .rounding(8.0);
                    if ui
                        .add(home_btn)
                        .on_hover_text("Toggle between Visual Suite Home & Studio Editor")
                        .clicked()
                    {
                        self.screen = match self.screen {
                            WorkspaceScreen::HomeHub => WorkspaceScreen::StudioEditor,
                            WorkspaceScreen::StudioEditor => WorkspaceScreen::HomeHub,
                        };
                    }

                    if ui
                        .selectable_label(self.screen == WorkspaceScreen::HomeHub, "🏠 Home Hub")
                        .clicked()
                    {
                        self.screen = WorkspaceScreen::HomeHub;
                    }
                    if ui
                        .selectable_label(
                            self.screen == WorkspaceScreen::StudioEditor,
                            "🎨 Studio Editor",
                        )
                        .clicked()
                    {
                        self.screen = WorkspaceScreen::StudioEditor;
                    }

                    ui.separator();

                    // Undo / Redo with revision-accurate dirty reconciliation
                    if ui
                        .add_enabled(self.history.can_undo(), egui::Button::new("↩ Undo"))
                        .on_hover_text("Undo (Ctrl+Z)")
                        .clicked()
                    {
                        self.queued_history=-1;
                    }
                    if ui
                        .add_enabled(self.history.can_redo(), egui::Button::new("↪ Redo"))
                        .on_hover_text("Redo (Ctrl+Y)")
                        .clicked()
                    {
                        self.queued_history=1;
                    }

                    ui.separator();

                });
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    // Magic Switch / Resize Dropdown
                    egui::ComboBox::from_id_salt("magic_switch_top")
                        .width(170.0)
                        .selected_text(self.project.format.label())
                        .show_ui(ui, |ui| {
                            ui.label(
                                RichText::new("Print & Digital Format Presets")
                                    .strong()
                                    .color(Color32::from_rgb(0, 245, 212)),
                            );
                            for fmt in DesignFormat::all_presets() {
                                let (fw, fh) = fmt.dimensions();
                                let (mm_w, mm_h) = fmt.physical_dimensions_mm();
                                if ui
                                    .selectable_label(
                                        self.project.format == fmt,
                                        format!(
                                            "{} ({:.0}×{:.0} px • {:.1}×{:.1} mm)",
                                            fmt.label(),
                                            fw,
                                            fh,
                                            mm_w,
                                            mm_h
                                        ),
                                    )
                                    .clicked()
                                {
                                    self.record_mutation();
                                    magic_resize_project(&mut self.project, fmt);
                                    self.status_banner = format!(
                                        "Resized project to {} ({:.0}×{:.0} px, {:.1}×{:.1} mm)",
                                        fmt.label(),
                                        fw,
                                        fh,
                                        mm_w,
                                        mm_h
                                    );
                                }
                            }
                        });

                    ui.add(
                        egui::TextEdit::singleline(&mut self.project.title)
                            .desired_width(120.0)
                            .hint_text("Project Title"),
                    );
                    if self.dirty {
                        ui.colored_label(Color32::from_rgb(251, 191, 36), "● Unsaved");
                    }

                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        let exp_btn = egui::Button::new(
                            RichText::new("⬇ Print PDF / Export")
                                .strong()
                                .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(125, 42, 232))
                        .rounding(8.0);
                        if ui.add(exp_btn).clicked() {
                            self.pdf_range_end = self.project.pages.len().max(1);
                            self.show_export_modal = true;
                        }

                        let pres_btn = egui::Button::new(
                            RichText::new("▶ Present")
                                .strong()
                                .color(Color32::from_rgb(15, 23, 42)),
                        )
                        .fill(Color32::from_rgb(0, 245, 212))
                        .rounding(8.0);
                        if ui
                            .add(pres_btn)
                            .on_hover_text("Start Full-Screen Presentation Mode (F5)")
                            .clicked()
                        {
                            self.presentation_mode = true;
                            self.canvas_state.anim_start_time = ctx.input(|i| i.time);
                        }

                        if ui
                            .button("Save As...")
                            .on_hover_text("Save project to a new file path")
                            .clicked()
                        {
                            self.queued_save=Some(true);
                        }
                        if ui
                            .button("💾 Save")
                            .on_hover_text("Save project (Ctrl+S). Keep the JSON file and its assets folder together when moving a project.")
                            .clicked()
                        {
                            self.queued_save=Some(false);
                        }
                        if self.recovery_available
                            && ui
                                .button("♻ Recover Autosave")
                                .on_hover_text("Restore most recent autosave recovery file")
                                .clicked()
                        {
                            if let Some(rec_path) = self.latest_available_recovery_path() {
                                if let Err(e) = self.request_recover_document_from_path(&rec_path) {
                                    self.status_banner = format!("Recovery error: {e}");
                                }
                            }
                        }
                        if ui.button("📂 Open").clicked() {
                            if let Some(path) = rfd::FileDialog::new()
                                .add_filter("Canvas Project", &["json","canvas"])
                                .pick_file()
                            {
                                match load_project_json(&path) {
                                    Ok(proj) => {
                                        let msg = format!("Loaded project from {}", path.display());
                                        self.request_replace_document(
                                            proj,
                                            Some(path.clone()),
                                            msg,
                                        );
                                    }
                                    Err(e) => self.status_banner = format!("Open error: {e}"),
                                }
                            }
                        }
                    });
                });
            });
    }

    fn render_unsaved_confirmation_modal(&mut self, ctx: &Context) {
        egui::Window::new("Unsaved Changes")
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(360.0);
                ui.label(
                    RichText::new("You have unsaved changes in the current project.")
                        .strong()
                        .color(Color32::WHITE),
                );
                ui.add_space(6.0);
                ui.label("Would you like to save your changes before continuing?");
                if let Some(err) = &self.last_save_error {
                    ui.add_space(6.0);
                    ui.colored_label(Color32::from_rgb(248, 113, 113), format!("⚠ {err}"));
                }
                ui.add_space(14.0);
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("💾 Save & Continue")
                                    .strong()
                                    .color(Color32::from_rgb(15, 23, 42)),
                            )
                            .fill(Color32::from_rgb(0, 245, 212)),
                        )
                        .clicked()
                    {
                        let outcome = self.confirm_unsaved_save_and_continue();
                        if outcome == SaveOutcome::Saved && self.allow_close_without_prompt {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }

                    if ui
                        .add(
                            egui::Button::new(
                                RichText::new("Discard Changes")
                                    .strong()
                                    .color(Color32::WHITE),
                            )
                            .fill(Color32::from_rgb(220, 38, 38)),
                        )
                        .clicked()
                    {
                        let was_closing = self.pending_close;
                        self.confirm_unsaved_discard();
                        if was_closing && self.allow_close_without_prompt {
                            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                    }

                    if ui.button("Cancel").clicked() {
                        self.confirm_unsaved_cancel();
                    }
                });
            });
    }

    fn render_home_hub(&mut self, ui: &mut Ui) {
        ScrollArea::vertical().show(ui, |ui| {
            ui.add_space(16.0);
            egui::Frame::none()
                .fill(Color32::from_rgb(26, 20, 56))
                .rounding(18.0)
                .inner_margin(Margin::symmetric(32.0, 28.0))
                .stroke(Stroke::new(1.5_f32, Color32::from_rgb(0, 196, 204)))
                .show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        ui.label(
                            RichText::new("What will you design today?")
                                .size(36.0)
                                .strong()
                                .color(Color32::WHITE),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            RichText::new(
                                "Create A4, US Letter and 5×7 designs locally. Check print preflight before exporting.",
                            )
                            .size(16.0)
                            .color(Color32::from_rgb(186, 230, 253)),
                        );
                        ui.add_space(18.0);

                        ui.horizontal_wrapped(|ui| {
                            for fmt in DesignFormat::all_presets() {
                                let (w, h) = fmt.dimensions();
                                let (mm_w, mm_h) = fmt.physical_dimensions_mm();
                                let btn = egui::Button::new(
                                    RichText::new(format!(
                                        "{}\n{:.0}×{:.0} px ({:.0}×{:.0} mm)",
                                        fmt.label(),
                                        w,
                                        h,
                                        mm_w,
                                        mm_h
                                    ))
                                    .size(12.5)
                                    .color(Color32::WHITE),
                                )
                                .fill(Color32::from_rgb(38, 43, 68))
                                .rounding(10.0)
                                .min_size(Vec2::new(165.0, 58.0));

                                if ui.add(btn).clicked() {
                                    let new_proj = Project {
                                        schema_version: 2,
                                        document_id: crate::model::generate_document_id(),
                                        recovery_origin_path: None,
                                        title: format!("Untitled {}", fmt.label()),
                                        format: fmt,
                                        width: w,
                                        height: h,
                                        width_mm: mm_w,
                                        height_mm: mm_h,
                                        units_per_inch: 96.0,
                                        bleed_mm: 0.0,
                                        crop_marks: false,
                                        pages: vec![CanvasPage::new(1, "Page 1")],
                                        brand_kit: Default::default(),
                                        imported_fonts: Vec::new(),
                                        print_settings: Default::default(),
                                        production: Default::default(),
                                        next_id: 100,
                                    };
                                    let msg = format!("Created new {} project", fmt.label());
                                    self.request_replace_document(new_proj, None, msg);
                                }
                            }
                        });
                    });
                });



            ui.add_space(24.0);
            ui.label(
                RichText::new("✨ Templates for Absolutely Anything")
                    .size(22.0)
                    .strong()
                    .color(Color32::WHITE),
            );
            ui.add_space(10.0);

            ui.horizontal_wrapped(|ui| {
                for tpl in available_templates() {
                    let (tw, th) = tpl.format.dimensions();
                    egui::Frame::none()
                        .fill(Color32::from_rgb(24, 27, 36))
                        .rounding(12.0)
                        .inner_margin(Margin::same(16.0))
                        .stroke(Stroke::new(1.0_f32, to_egui_color(tpl.accent)))
                        .show(ui, |ui| {
                            ui.set_width(240.0);
                            ui.label(
                                RichText::new(tpl.category)
                                    .size(11.0)
                                    .strong()
                                    .color(to_egui_color(tpl.accent)),
                            );
                            ui.label(
                                RichText::new(tpl.name)
                                    .size(17.0)
                                    .strong()
                                    .color(Color32::WHITE),
                            );
                            ui.label(
                                RichText::new(tpl.subtitle)
                                    .size(12.0)
                                    .color(Color32::from_rgb(148, 163, 184)),
                            );
                            ui.label(
                                RichText::new(format!("{:.0} x {:.0} px", tw, th))
                                    .size(11.0)
                                    .color(Color32::GRAY),
                            );
                            ui.add_space(8.0);
                            if ui
                                .add(
                                    egui::Button::new(
                                        RichText::new("Open in Studio ->").strong(),
                                    )
                                    .fill(to_egui_color(tpl.accent)),
                                )
                                .clicked()
                            {
                                let proj = build_template_project(tpl.id);
                                let msg = format!("Loaded template '{}'", tpl.name);
                                self.request_replace_document(proj, None, msg);
                            }
                        });
                }
            });
        });
    }

    fn render_contextual_action_bar(&mut self, ctx: &Context) {
        TopBottomPanel::top("canvas_contextual_toolbar")
            .exact_height(46.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(22, 24, 31))
                    .inner_margin(Margin::symmetric(12.0, 6.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 41, 52))),
            )
            .show(ctx, |ui| {
                ScrollArea::horizontal().show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        if self.canvas_state.drawing_mode {
                            ui.label(
                                RichText::new("✏ Freehand Draw Active")
                                    .strong()
                                    .color(Color32::from_rgb(0, 245, 212)),
                            );
                            if ui.button("Exit Draw Mode").clicked() {
                                self.canvas_state.drawing_mode = false;
                            }
                            return;
                        }

                        if self.canvas_state.selected_ids.len() >= 2 {
                            if ui
                                .button("🔗 Group (Ctrl+G)")
                                .on_hover_text("Group selected elements together")
                                .clicked()
                            {
                                self.group_selected_elements();
                            }
                        }
                        let has_grouped_selection = self
                            .current_page()
                            .map(|p| {
                                p.elements.iter().any(|e| {
                                    (self.selected_id == Some(e.id)
                                        || self.canvas_state.selected_ids.contains(&e.id))
                                        && e.group_id.is_some()
                                })
                            })
                            .unwrap_or(false);
                        if has_grouped_selection
                            && ui
                                .button("🔓 Ungroup (Ctrl+Shift+G)")
                                .on_hover_text("Ungroup selected group")
                                .clicked()
                        {
                            self.ungroup_selected_elements();
                        }

                        let Some(sel_id) = self.selected_id else {
                            ui.label(RichText::new("Page Background:").strong());
                            if let Some(page) = self.project.pages.get_mut(self.current_page_idx) {
                                let mut next_bg: Option<PageBackground> = None;
                                match &mut page.background {
                                    PageBackground::Solid(c) => {
                                        color_Edit_button(ui, c);
                                        if ui.button("Switch to Gradient").clicked() {
                                            next_bg = Some(PageBackground::LinearGradient {
                                                start: *c,
                                                end: rgb(125, 42, 232),
                                                angle_deg: 135.0,
                                            });
                                        }
                                        if ui.button("Switch to Grid").clicked() {
                                            next_bg = Some(PageBackground::GridPattern {
                                                bg: rgb(15, 23, 42),
                                                line: rgba(56, 189, 248, 45),
                                                spacing: 40.0,
                                            });
                                        }
                                    }
                                    PageBackground::LinearGradient { start, end, .. } => {
                                        ui.label("Start:");
                                        color_Edit_button(ui, start);
                                        ui.label("End:");
                                        color_Edit_button(ui, end);
                                        if ui.button("Switch to Radial").clicked() {
                                            next_bg = Some(PageBackground::RadialGradient {
                                                center: *start,
                                                outer: *end,
                                            });
                                        }
                                        if ui.button("Solid").clicked() {
                                            next_bg = Some(PageBackground::Solid(*start));
                                        }
                                    }
                                    PageBackground::RadialGradient { center, outer } => {
                                        ui.label("Center:");
                                        color_Edit_button(ui, center);
                                        ui.label("Outer:");
                                        color_Edit_button(ui, outer);
                                        if ui.button("Solid").clicked() {
                                            next_bg = Some(PageBackground::Solid(*center));
                                        }
                                    }
                                    PageBackground::GridPattern { bg, line, spacing } => {
                                        ui.label("BG:");
                                        color_Edit_button(ui, bg);
                                        ui.label("Grid:");
                                        color_Edit_button(ui, line);
                                        ui.add(
                                            egui::Slider::new(spacing, 16.0..=120.0).text("Step"),
                                        );
                                        if ui.button("Solid").clicked() {
                                            next_bg = Some(PageBackground::Solid(*bg));
                                        }
                                    }
                                }
                                if let Some(nb) = next_bg {
                                    page.background = nb;
                                }

                                ui.separator();
                                ui.label("Transition:");
                                egui::ComboBox::from_id_salt("page_trans_combo")
                                    .selected_text(page.transition.label())
                                    .show_ui(ui, |ui| {
                                        for tr in PageTransition::all() {
                                            ui.selectable_value(
                                                &mut page.transition,
                                                *tr,
                                                tr.label(),
                                            );
                                        }
                                    });
                            }
                            return;
                        };

                        let mut trigger_replace_image = false;
                        let imported_fonts=self.project.imported_fonts.clone();
                        let pw = self.project.width;
                        let ph = self.project.height;
                        if let Some(page) = self.project.pages.get_mut(self.current_page_idx) {
                            if let Some(el) = page.elements.iter_mut().find(|e| e.id == sel_id) {
                                if el.locked { ui.disable(); }
                                match &mut el.kind {
                                    ElementKind::Shape {
                                        shape_type,
                                        fill,
                                        stroke_color,
                                        stroke_width,
                                        corner_radius,
                                    } => {
                                        egui::ComboBox::from_id_salt("shape_type_bar")
                                            .selected_text(shape_type.label())
                                            .show_ui(ui, |ui| {
                                                for st in ShapeType::all() {
                                                    ui.selectable_value(
                                                        shape_type,
                                                        *st,
                                                        st.label(),
                                                    );
                                                }
                                            });

                                        match fill {
                                            FillStyle::Solid(c) => {
                                                ui.label("Fill:");
                                                color_Edit_button(ui, c);
                                                if ui.small_button("Gradient").clicked() {
                                                    *fill = FillStyle::LinearGradient {
                                                        start: *c,
                                                        end: rgb(125, 42, 232),
                                                        vertical: false,
                                                    };
                                                }
                                            }
                                            FillStyle::LinearGradient {
                                                start,
                                                end,
                                                vertical,
                                            } => {
                                                ui.label("Grad:");
                                                color_Edit_button(ui, start);
                                                color_Edit_button(ui, end);
                                                ui.checkbox(vertical, "Vert");
                                                if ui.small_button("Solid").clicked() {
                                                    *fill = FillStyle::Solid(*start);
                                                }
                                            }
                                        }

                                        ui.separator();
                                        ui.label("Border:");
                                        color_Edit_button(ui, stroke_color);
                                        ui.add(
                                            egui::DragValue::new(stroke_width)
                                                .range(0.0..=30.0)
                                                .speed(0.5)
                                                .prefix("W: "),
                                        );
                                        if *shape_type == ShapeType::RoundedRect {
                                            ui.add(
                                                egui::DragValue::new(corner_radius)
                                                    .range(0.0..=120.0)
                                                    .prefix("R: "),
                                            );
                                        }
                                    }
                                    ElementKind::Text {
                                        content,
                                        font_family,
                                        font_size,
                                        color,
                                        bold,
                                        italic,
                                        underline,
                                        uppercase,
                                        align,
                                        effect,
                                        list_style,
                                        ..
                                    } => {
                                        egui::ComboBox::from_id_salt("font_fam_bar")
                                            .selected_text(imported_fonts.iter().find(|f|FontFamily::Imported(f.id)==*font_family).map(|f|f.name.clone()).unwrap_or_else(||font_family.label()))
                                            .show_ui(ui, |ui| {
                                                for ff in FontFamily::all() {
                                                    ui.selectable_value(
                                                        font_family,
                                                        *ff,
                                                        ff.label(),
                                                    );
                                                }
                                                for font in &imported_fonts {ui.selectable_value(font_family,FontFamily::Imported(font.id),&font.name);}

                                            });

                                        if ui.small_button("−").clicked() {
                                            *font_size = (*font_size - 2.0).max(6.0);
                                        }
                                        ui.add(
                                            egui::DragValue::new(font_size)
                                                .range(6.0..=260.0)
                                                .speed(1.0),
                                        );
                                        if ui.small_button("+").clicked() {
                                            *font_size = (*font_size + 2.0).min(260.0);
                                        }

                                        color_Edit_button(ui, color);
                                        ui.add_enabled_ui(!matches!(font_family,FontFamily::Imported(_)),|ui| {
                                            ui.toggle_value(bold, RichText::new("B").strong());
                                            ui.toggle_value(italic, RichText::new("I").italics());
                                        }).response.on_hover_text("For imported fonts, select the actual bold or italic face from the font menu");
                                        ui.toggle_value(underline, RichText::new("U").underline());
                                        ui.toggle_value(uppercase, "AA");

                                        if ui
                                            .selectable_label(*align == TextAlign::Left, "⫷")
                                            .on_hover_text("Align Left")
                                            .clicked()
                                        {
                                            *align = TextAlign::Left;
                                        }
                                        if ui
                                            .selectable_label(*align == TextAlign::Center, "⫸")
                                            .on_hover_text("Align Center")
                                            .clicked()
                                        {
                                            *align = TextAlign::Center;
                                        }
                                        if ui
                                            .selectable_label(*align == TextAlign::Right, "⫹")
                                            .on_hover_text("Align Right")
                                            .clicked()
                                        {
                                            *align = TextAlign::Right;
                                        }

                                        if ui
                                            .selectable_label(
                                                *list_style == ListStyle::Bullet,
                                                "• List",
                                            )
                                            .clicked()
                                        {
                                            *list_style = if *list_style == ListStyle::Bullet {
                                                ListStyle::None
                                            } else {
                                                ListStyle::Bullet
                                            };
                                        }

                                        egui::ComboBox::from_id_salt("txt_effect_bar")
                                            .selected_text(format!("Effect: {}", effect.label()))
                                            .show_ui(ui, |ui| {
                                                if ui
                                                    .selectable_label(
                                                        matches!(effect, TextEffect::None),
                                                        "None",
                                                    )
                                                    .clicked()
                                                {
                                                    *effect = TextEffect::None;
                                                }
                                                if ui
                                                    .selectable_label(
                                                        matches!(
                                                            effect,
                                                            TextEffect::DropShadow { .. }
                                                        ),
                                                        "Drop Shadow",
                                                    )
                                                    .clicked()
                                                {
                                                    *effect = TextEffect::DropShadow {
                                                        offset: 4.0,
                                                        color: rgba(0, 0, 0, 160),
                                                    };
                                                }
                                                if ui
                                                    .selectable_label(
                                                        matches!(
                                                            effect,
                                                            TextEffect::NeonGlow { .. }
                                                        ),
                                                        "Neon Glow",
                                                    )
                                                    .clicked()
                                                {
                                                    *effect = TextEffect::NeonGlow {
                                                        glow_color: rgb(0, 245, 212),
                                                        intensity: 1.1,
                                                    };
                                                }
                                                if ui
                                                    .selectable_label(
                                                        matches!(effect, TextEffect::Splice { .. }),
                                                        "Splice Offset",
                                                    )
                                                    .clicked()
                                                {
                                                    *effect = TextEffect::Splice {
                                                        offset: 5.0,
                                                        fill_color: rgb(0, 196, 204),
                                                    };
                                                }
                                                if ui
                                                    .selectable_label(
                                                        matches!(effect, TextEffect::Echo { .. }),
                                                        "Echo Trail",
                                                    )
                                                    .clicked()
                                                {
                                                    *effect = TextEffect::Echo {
                                                        color: rgba(125, 42, 232, 160),
                                                    };
                                                }
                                                if ui
                                                    .selectable_label(
                                                        matches!(
                                                            effect,
                                                            TextEffect::HollowOutline { .. }
                                                        ),
                                                        "Hollow Outline",
                                                    )
                                                    .clicked()
                                                {
                                                    *effect = TextEffect::HollowOutline {
                                                        thickness: 2.0,
                                                        color: rgb(0, 245, 212),
                                                    };
                                                }
                                                if ui
                                                    .selectable_label(
                                                        matches!(
                                                            effect,
                                                            TextEffect::BackgroundPill { .. }
                                                        ),
                                                        "Background Pill",
                                                    )
                                                    .clicked()
                                                {
                                                    *effect = TextEffect::BackgroundPill {
                                                        bg_color: rgb(125, 42, 232),
                                                        radius: 14.0,
                                                        padding: 12.0,
                                                    };
                                                }
                                            });

                                        let before=content.clone();
                                        if ui.add(egui::TextEdit::singleline(&mut *content).desired_width(170.).hint_text("Edit text...")).changed() {
                                            el.text_format.reconcile_edit(&before,content);
                                        }
                                    }
                                    ElementKind::Image {
                                        width_px,
                                        height_px,
                                        raw_rgba,
                                        mask,
                                        adjustments,
                                        revision,
                                        ..
                                    } => {
                                        if ui
                                            .button("🔄 Replace Image")
                                            .on_hover_text(
                                                "Replace image while keeping frame geometry & mask",
                                            )
                                            .clicked()
                                        {
                                            trigger_replace_image = true;
                                        }

                                        let prev_fit = adjustments.fit;
                                        egui::ComboBox::from_id_salt("img_fit_bar")
                                            .selected_text(format!("Fit: {}", adjustments.fit.label()))
                                            .show_ui(ui, |ui| {
                                                for f in ImageFit::all() {
                                                    ui.selectable_value(
                                                        &mut adjustments.fit,
                                                        *f,
                                                        f.label(),
                                                    );
                                                }
                                            });
                                        if adjustments.fit != prev_fit {
                                            *revision += 1;
                                        }

                                        let prev_mask = *mask;
                                        egui::ComboBox::from_id_salt("img_mask_bar")
                                            .selected_text(mask.label())
                                            .show_ui(ui, |ui| {
                                                for m in ImageMask::all() {
                                                    ui.selectable_value(mask, *m, m.label());
                                                }
                                            });
                                        if *mask != prev_mask {
                                            *revision += 1;
                                        }

                                        ui.label("Crop:");
                                        if ui
                                            .add(
                                                egui::DragValue::new(&mut adjustments.crop.left)
                                                    .range(0.0..=0.45)
                                                    .speed(0.01)
                                                    .prefix("L:"),
                                            )
                                            .changed()
                                            || ui
                                                .add(
                                                    egui::DragValue::new(&mut adjustments.crop.top)
                                                        .range(0.0..=0.45)
                                                        .speed(0.01)
                                                        .prefix("T:"),
                                                )
                                                .changed()
                                            || ui
                                                .add(
                                                    egui::DragValue::new(
                                                        &mut adjustments.crop.right,
                                                    )
                                                    .range(0.0..=0.45)
                                                    .speed(0.01)
                                                    .prefix("R:"),
                                                )
                                                .changed()
                                            || ui
                                                .add(
                                                    egui::DragValue::new(
                                                        &mut adjustments.crop.bottom,
                                                    )
                                                    .range(0.0..=0.45)
                                                    .speed(0.01)
                                                    .prefix("B:"),
                                                )
                                                .changed()
                                            || ui
                                                .add(
                                                    egui::DragValue::new(
                                                        &mut adjustments.focal_point.0,
                                                    )
                                                    .range(0.0..=1.0)
                                                    .speed(0.01)
                                                    .prefix("FX:"),
                                                )
                                                .on_hover_text("Horizontal focal point (0..1)")
                                                .changed()
                                            || ui
                                                .add(
                                                    egui::DragValue::new(
                                                        &mut adjustments.focal_point.1,
                                                    )
                                                    .range(0.0..=1.0)
                                                    .speed(0.01)
                                                    .prefix("FY:"),
                                                )
                                                .on_hover_text("Vertical focal point (0..1)")
                                                .changed()
                                        {
                                            *revision += 1;
                                        }

                                        if ui
                                            .toggle_value(
                                                &mut adjustments.bg_removed,
                                                "✂ Edge-Color BG Cutout",
                                            )
                                            .on_hover_text(
                                                "Heuristic corner-color background removal for simple studio backdrops",
                                            )
                                            .changed()
                                        {
                                            *revision += 1;
                                        }
                                        if adjustments.bg_removed
                                            && ui
                                                .add(
                                                    egui::Slider::new(
                                                        &mut adjustments.bg_tolerance,
                                                        0.05..=0.65,
                                                    )
                                                    .text("Tol"),
                                                )
                                                .changed()
                                        {
                                            *revision += 1;
                                        }
                                        if ui
                                            .button("🔍 2× Bilinear + Sharpen")
                                            .on_hover_text(
                                                "2× bilinear resampling and sharpening; does not restore missing detail",
                                            )
                                            .clicked()
                                        {
                                            let (nw, nh, nbuf) =
                                                upscale_image_2x(raw_rgba, *width_px, *height_px);
                                            *width_px = nw;
                                            *height_px = nh;
                                            *raw_rgba = nbuf.into();
                                            *revision += 1;
                                        }
                                        if ui.button("🧼 Blend Centre Patch").clicked() {
                                            magic_erase_patch(
                                                raw_rgba, *width_px, *height_px, 0.35, 0.35, 0.3,
                                                0.3,
                                            );
                                            *revision += 1;
                                        }
                                        if ui
                                            .toggle_value(&mut adjustments.grayscale, "B&W")
                                            .changed()
                                            || ui
                                                .toggle_value(&mut adjustments.sepia, "Sepia")
                                                .changed()
                                            || ui
                                                .toggle_value(
                                                    &mut adjustments.posterize,
                                                    "Posterize",
                                                )
                                                .changed()
                                        {
                                            *revision += 1;
                                        }
                                    }
                                    ElementKind::Chart {
                                        chart_type,
                                        title,
                                        show_values,
                                        show_grid,
                                        ..
                                    } => {
                                        egui::ComboBox::from_id_salt("chart_type_bar")
                                            .selected_text(chart_type.label())
                                            .show_ui(ui, |ui| {
                                                for ct in ChartType::all() {
                                                    ui.selectable_value(
                                                        chart_type,
                                                        *ct,
                                                        ct.label(),
                                                    );
                                                }
                                            });
                                        ui.add(
                                            egui::TextEdit::singleline(title).desired_width(150.0),
                                        );
                                        ui.checkbox(show_values, "Values");
                                        ui.checkbox(show_grid, "Grid");
                                    }
                                    _ => {}
                                }

                                ui.separator();

                                ui.add(
                                    egui::DragValue::new(&mut el.opacity)
                                        .range(0.05..=1.0)
                                        .speed(0.02)
                                        .prefix("Opacity: "),
                                );
                                ui.checkbox(&mut el.shadow.enabled, "Shadow (preview only)");

                                egui::ComboBox::from_id_salt("el_anim_bar")
                                    .selected_text(format!("🎞 {}", el.animation.label()))
                                    .show_ui(ui, |ui| {
                                        for a in ElementAnimation::all() {
                                            ui.selectable_value(&mut el.animation, *a, a.label());
                                        }
                                    });

                                if ui.small_button("↔ Center H").clicked() {
                                    el.x = (pw - el.width) * 0.5;
                                }
                                if ui.small_button("↕ Center V").clicked() {
                                    el.y = (ph - el.height) * 0.5;
                                }
                            }
                        }

                        if trigger_replace_image {
                            self.replace_selected_image_from_disk();
                        }
                    });
                });
            });
    }

    fn render_left_dock_panels(&mut self, ctx: &Context) {
        egui::SidePanel::left("canvas_left_rail")
            .exact_width(68.0)
            .resizable(false)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(15, 17, 23))
                    .inner_margin(Margin::symmetric(6.0, 10.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(38, 41, 52))),
            )
            .show(ctx, |ui| {
                ScrollArea::vertical()
                    .id_salt("tool-rail-scroll")
                    .show(ui, |ui| {
                        ui.vertical_centered(|ui| {
                            for tab in DockTab::all() {
                                let (icon, label) = tab.icon_and_label();
                                let active = self.dock_open && self.active_dock == *tab;
                                let bg = if active {
                                    Color32::from_rgb(45, 52, 78)
                                } else {
                                    Color32::TRANSPARENT
                                };
                                let fg = if active {
                                    Color32::from_rgb(0, 245, 212)
                                } else {
                                    Color32::from_rgb(175, 184, 201)
                                };

                                let btn = egui::Button::new(
                                    RichText::new(format!(
                                        "{icon}\n{}",
                                        if label == "Elements" { "Shapes" } else { label }
                                    ))
                                    .size(11.5)
                                    .color(fg),
                                )
                                .fill(bg)
                                .rounding(8.0)
                                .min_size(Vec2::new(56.0, 48.0));

                                if ui.add(btn).clicked() {
                                    if self.active_dock == *tab && self.dock_open {
                                        self.dock_open = false;
                                    } else {
                                        self.active_dock = *tab;
                                        self.dock_open = true;
                                        self.canvas_state.drawing_mode = *tab == DockTab::Draw;
                                    }
                                }
                                ui.add_space(4.0);
                            }
                        });
                    });
            });

        if !self.dock_open {
            return;
        }

        egui::SidePanel::left("canvas_side_drawer")
            .default_width(310.0)
            .width_range(270.0..=380.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(24, 26, 34))
                    .inner_margin(Margin::same(14.0))
                    .stroke(Stroke::new(1.0_f32, Color32::from_rgb(42, 46, 58))),
            )
            .show(ctx, |ui| {
                ScrollArea::vertical().show(ui, |ui| match self.active_dock {
                    DockTab::Templates => self.render_dock_templates(ui),
                    DockTab::Elements => self.render_dock_elements(ui),
                    DockTab::Text => self.render_dock_text(ui),
                    DockTab::BrandKit => self.render_dock_brand_kit(ui),
                    DockTab::Uploads => self.render_dock_uploads(ui),
                    DockTab::MagicStudio => self.render_dock_magic_studio(ui),
                    DockTab::ChartsAndTables => self.render_dock_charts_tables(ui),
                    DockTab::Draw => self.render_dock_draw(ui),
                    DockTab::Layers => self.render_dock_layers(ui),
                    DockTab::Production => self.render_production(ui),
                    DockTab::Book => self.render_book(ui),
                    DockTab::Puzzles => self.render_puzzles(ui),
                    DockTab::AppInfo => self.render_dock_app_info(ui),
                });
            });
    }

    fn render_print_guides(&mut self, ui: &mut Ui) {
        ui.collapsing("Print guides & safe area", |ui| {
            ui.checkbox(
                &mut self.project.print_settings.show_guides,
                "Show guides on canvas",
            );
            ui.horizontal(|ui| {
                ui.label("Safe margin mm");
                ui.add(
                    egui::DragValue::new(&mut self.project.print_settings.safe_margin_mm)
                        .range(0.0..=50.0)
                        .speed(0.5),
                );
            });
            ui.horizontal(|ui| {
                ui.add(
                    egui::DragValue::new(&mut self.new_guide_mm)
                        .range(0.0..=1270.0)
                        .prefix("mm "),
                );
                if ui.button("+ Vertical").clicked() {
                    self.project
                        .print_settings
                        .guides_x_mm
                        .push(self.new_guide_mm);
                }
                if ui.button("+ Horizontal").clicked() {
                    self.project
                        .print_settings
                        .guides_y_mm
                        .push(self.new_guide_mm);
                }
            });
            for (label, guides) in [
                ("Vertical", &mut self.project.print_settings.guides_x_mm),
                ("Horizontal", &mut self.project.print_settings.guides_y_mm),
            ] {
                let mut remove = None;
                for (index, guide) in guides.iter_mut().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(label);
                        ui.add(
                            egui::DragValue::new(guide)
                                .range(0.0..=1270.0)
                                .suffix(" mm"),
                        );
                        if ui.small_button("Remove").clicked() {
                            remove = Some(index);
                        }
                    });
                }
                if let Some(index) = remove {
                    guides.remove(index);
                }
            }
            ui.label("Guides are editing aids and do not appear in exported artwork.");
        });
    }

    fn render_output_presets(&mut self, ui: &mut Ui) {
        ui.collapsing("Saved output presets", |ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.output_preset_name).hint_text("Preset name"),
            );
            if ui.button("Save / Replace Current Settings").clicked() {
                let preset = crate::model::OutputPreset {
                    name: self.output_preset_name.trim().into(),
                    dpi: self.export_dpi,
                    bleed_mm: self.project.bleed_mm,
                    crop_marks: self.project.crop_marks,
                    transparent: self.export_transparent,
                };
                if !preset.name.is_empty() {
                    self.project
                        .print_settings
                        .presets
                        .retain(|p| p.name != preset.name);
                    self.project.print_settings.presets.push(preset);
                }
            }
            for preset in self.project.print_settings.presets.clone() {
                if ui.button(&preset.name).clicked() {
                    self.export_dpi = preset.dpi.clamp(36, 600);
                    self.project.bleed_mm = preset.bleed_mm.clamp(0., 10.);
                    self.project.crop_marks = preset.crop_marks;
                    self.export_transparent = preset.transparent;
                }
            }
        });
    }

    fn render_dock_templates(&mut self, ui: &mut Ui) {
        ui.heading("📐 Design Templates & Print Collections");
        ui.label(
            RichText::new("Start with a built-in template or import your own layout JSON.")
                .size(12.0)
                .color(Color32::GRAY),
        );
        ui.add_space(8.0);

        if ui.button("📂 Import Layout JSON...").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Layout JSON", &["json"])
                .pick_file()
            {
                match import_layout_json(&path) {
                    Ok(proj) => {
                        let title = proj.title.clone();
                        let msg = format!("Imported layout '{}'", title);
                        self.request_replace_document(proj, None, msg);
                    }
                    Err(e) => self.status_banner = format!("Layout JSON import error: {e}"),
                }
            }
        }

        ui.separator();
        self.render_template_library(ui);
        self.render_print_guides(ui);
        ui.separator();
        ui.strong("Built-In Studio Templates");
        ui.add_space(6.0);

        for tpl in available_templates() {
            egui::Frame::none()
                .fill(Color32::from_rgb(33, 37, 49))
                .rounding(10.0)
                .inner_margin(Margin::same(10.0))
                .stroke(Stroke::new(1.0_f32, to_egui_color(tpl.accent)))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.colored_label(to_egui_color(tpl.accent), "●");
                        ui.strong(tpl.name);
                    });
                    ui.label(
                        RichText::new(tpl.subtitle)
                            .size(11.5)
                            .color(Color32::LIGHT_GRAY),
                    );
                    ui.horizontal(|ui| {
                        if ui.small_button("Replace Project").clicked() {
                            let proj = build_template_project(tpl.id);
                            let msg = format!("Loaded template '{}'", tpl.name);
                            self.request_replace_document(proj, None, msg);
                        }
                        if ui.small_button("+ Insert All Pages (Fit)")
                            .on_hover_text("Fit every template page proportionally into this document; preserves its page background")
                            .clicked() {
                            self.insert_template_pages(&build_template_project(tpl.id));
                        }
                    });
                });
            ui.add_space(8.0);
        }

        ui.separator();
        ui.strong("Custom Canvas Size (px / mm / in)");
        ui.horizontal(|ui| {
            ui.add(
                egui::DragValue::new(&mut self.custom_w_input)
                    .range(0.01..=10000.0)
                    .max_decimals(4)
                    .prefix("W: "),
            );
            ui.add(
                egui::DragValue::new(&mut self.custom_h_input)
                    .range(0.01..=10000.0)
                    .max_decimals(4)
                    .prefix("H: "),
            );
            let mut unit = self.custom_size_unit;
            egui::ComboBox::from_id_salt("custom_size_unit_cb")
                .width(52.0)
                .selected_text(["px", "mm", "in"][unit as usize])
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut unit, 0, "px");
                    ui.selectable_value(&mut unit, 1, "mm");
                    ui.selectable_value(&mut unit, 2, "in");
                });
            self.set_custom_size_unit(unit);
            if ui.button("Apply").clicked() {
                let _ = self.apply_custom_size();
            }
        });
    }

    fn render_dock_elements(&mut self, ui: &mut Ui) {
        ui.heading("❖ Vector Shapes & Graphics");
        ui.label(
            RichText::new("Click any shape to insert it onto the current page.")
                .size(12.0)
                .color(Color32::GRAY),
        );
        ui.add_space(10.0);

        let palette = [
            rgb(0, 196, 204),
            rgb(125, 42, 232),
            rgb(255, 92, 138),
            rgb(255, 202, 58),
            rgb(56, 189, 248),
            rgb(16, 185, 129),
        ];

        ui.horizontal_wrapped(|ui| {
            for (idx, st) in ShapeType::all().iter().enumerate() {
                let col = palette[idx % palette.len()];
                let btn =
                    egui::Button::new(RichText::new(st.label()).size(12.0).color(Color32::WHITE))
                        .fill(Color32::from_rgb(36, 40, 54))
                        .stroke(Stroke::new(1.0_f32, to_egui_color(col)))
                        .rounding(8.0)
                        .min_size(Vec2::new(128.0, 38.0));

                if ui.add(btn).clicked() {
                    self.record_mutation();
                    let id = self.project.alloc_id();
                    let w = if *st == ShapeType::LineHorizontal {
                        420.0
                    } else if *st == ShapeType::Pill {
                        280.0
                    } else {
                        220.0
                    };
                    let h = if *st == ShapeType::LineHorizontal {
                        16.0
                    } else if *st == ShapeType::Pill {
                        84.0
                    } else {
                        220.0
                    };
                    let cx = (self.project.width - w) * 0.5;
                    let cy = (self.project.height - h) * 0.5;
                    if let Some(page) = self.current_page_mut() {
                        page.elements.push(CanvasElement {
                            id,
                            name: st.label().to_string(),
                            x: cx,
                            y: cy,
                            width: w,
                            height: h,
                            rotation: 0.0,
                            opacity: 1.0,
                            locked: false,
                            visible: true,
                            flip_h: false,
                            flip_v: false,
                            shadow: DropShadow {
                                enabled: false,
                                offset_x: 0.0,
                                offset_y: 6.0,
                                blur: 14.0,
                                color: rgba(0, 0, 0, 70),
                            },
                            animation: ElementAnimation::Pop,
                            group_id: None,
                            text_format: Default::default(),
                            style_id: None,
                            binding: None,
                            kind: ElementKind::Shape {
                                shape_type: *st,
                                fill: FillStyle::Solid(col),
                                stroke_color: rgb(255, 255, 255),
                                stroke_width: 0.0,
                                corner_radius: 24.0,
                            },
                        });
                        self.selected_id = Some(id);
                    }
                }
            }
        });
    }

    fn render_dock_text(&mut self, ui: &mut Ui) {
        ui.heading("𝐓 Typography & Presets");
        ui.add_space(8.0);
        self.render_selected_typography(ui);
        if ui.button("Import TrueType Font (.ttf)").clicked() {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Static TrueType face", &["ttf"])
                .pick_file()
            {
                self.import_font_from_path(&path);
            }
        }
        ui.label("Imported faces travel with the document. Choose each actual weight or italic face from the font menu.");

        if ui
            .add_sized(
                [260.0, 44.0],
                egui::Button::new(RichText::new("Add a heading").size(20.0).strong())
                    .fill(Color32::from_rgb(42, 47, 64)),
            )
            .clicked()
        {
            self.insert_text_preset(
                "Heading",
                "Add a Heading",
                FontFamily::PosterHeavy,
                68.0,
                rgb(28, 34, 44),
                true,
                TextEffect::None,
            );
        }
        ui.add_space(6.0);
        if ui
            .add_sized(
                [260.0, 38.0],
                egui::Button::new(RichText::new("Add a subheading").size(16.0))
                    .fill(Color32::from_rgb(36, 40, 54)),
            )
            .clicked()
        {
            self.insert_text_preset(
                "Subheading",
                "Add a subheading",
                FontFamily::Sans,
                38.0,
                rgb(50, 58, 68),
                false,
                TextEffect::None,
            );
        }
        ui.add_space(6.0);
        if ui
            .add_sized(
                [260.0, 34.0],
                egui::Button::new(RichText::new("Add a little bit of body text").size(13.0))
                    .fill(Color32::from_rgb(32, 35, 48)),
            )
            .clicked()
        {
            self.insert_text_preset(
                "Body Text",
                "Add a little bit of body text for paragraphs and captions.",
                FontFamily::Sans,
                24.0,
                rgb(65, 72, 82),
                false,
                TextEffect::None,
            );
        }

        ui.add_space(16.0);
        ui.strong("✨ Decorative Text Effects");
        ui.label("These effects are for preview. Set the effect to None before production export.");
        ui.add_space(8.0);

        let presets = [
            (
                "Neon Cyber Glow",
                "CYBER PULSE",
                FontFamily::PosterHeavy,
                76.0,
                rgb(255, 255, 255),
                TextEffect::NeonGlow {
                    glow_color: rgb(0, 245, 212),
                    intensity: 1.2,
                },
            ),
            (
                "Retro Splice Offset",
                "RETRO WAVE",
                FontFamily::PosterHeavy,
                72.0,
                rgb(255, 255, 255),
                TextEffect::Splice {
                    offset: 6.0,
                    fill_color: rgb(255, 92, 138),
                },
            ),
            (
                "Promo Pill Badge",
                "LIMITED TIME OFFER",
                FontFamily::MonoCode,
                28.0,
                rgb(15, 23, 42),
                TextEffect::BackgroundPill {
                    bg_color: rgb(254, 228, 64),
                    radius: 16.0,
                    padding: 14.0,
                },
            ),
            (
                "Echo Motion Title",
                "VELOCITY",
                FontFamily::PosterHeavy,
                80.0,
                rgb(0, 245, 212),
                TextEffect::Echo {
                    color: rgba(125, 42, 232, 180),
                },
            ),
            (
                "Hollow Brutalist",
                "OUTLINE STUDIO",
                FontFamily::PosterHeavy,
                68.0,
                rgb(15, 23, 42),
                TextEffect::HollowOutline {
                    thickness: 2.5,
                    color: rgb(0, 245, 212),
                },
            ),
        ];

        for (label, sample, fam, sz, col, eff) in presets {
            if ui
                .add_sized(
                    [260.0, 40.0],
                    egui::Button::new(RichText::new(format!("{label} — \"{sample}\"")).size(12.5))
                        .fill(Color32::from_rgb(36, 40, 54)),
                )
                .clicked()
            {
                self.insert_text_preset(label, sample, fam, sz, col, true, eff);
            }
            ui.add_space(6.0);
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn insert_text_preset(
        &mut self,
        name: &str,
        content: &str,
        font_family: FontFamily,
        font_size: f32,
        color: ColorRgba,
        bold: bool,
        effect: TextEffect,
    ) {
        self.record_mutation();
        let id = self.project.alloc_id();
        let w = (self.project.width * 0.55).max(260.0);
        let h = (font_size * 1.8).max(60.0);
        let cx = (self.project.width - w) * 0.5;
        let cy = (self.project.height - h) * 0.5;
        if let Some(page) = self.current_page_mut() {
            page.elements.push(CanvasElement {
                id,
                name: name.to_string(),
                x: cx,
                y: cy,
                width: w,
                height: h,
                rotation: 0.0,
                opacity: 1.0,
                locked: false,
                visible: true,
                flip_h: false,
                flip_v: false,
                shadow: DropShadow::default(),
                animation: ElementAnimation::Fade,
                group_id: None,
                text_format: Default::default(),
                style_id: None,
                binding: None,
                kind: ElementKind::Text {
                    content: content.to_string(),
                    font_family,
                    font_size,
                    color,
                    bold,
                    italic: false,
                    underline: false,
                    uppercase: false,
                    align: TextAlign::Center,
                    letter_spacing: 0.5,
                    line_height: 1.2,
                    effect,
                    list_style: ListStyle::None,
                },
            });
            self.selected_id = Some(id);
        }
    }

    fn render_dock_brand_kit(&mut self, ui: &mut Ui) {
        ui.heading("👑 Brand Kit");
        ui.label(
            RichText::new(
                "Stay on-brand with one-click color palette shuffling & brand typography.",
            )
            .size(12.0)
            .color(Color32::GRAY),
        );
        ui.add_space(8.0);

        ui.add(
            egui::TextEdit::singleline(&mut self.project.brand_kit.brand_name)
                .hint_text("Brand Name"),
        );
        ui.add_space(10.0);

        let palettes = self.project.brand_kit.palettes.clone();
        for (p_idx, pal) in palettes.iter().enumerate() {
            egui::Frame::none()
                .fill(Color32::from_rgb(33, 37, 49))
                .rounding(10.0)
                .inner_margin(Margin::same(10.0))
                .show(ui, |ui| {
                    ui.strong(&pal.name);
                    ui.horizontal(|ui| {
                        for col in &pal.colors {
                            let (rect, resp) =
                                ui.allocate_exact_size(Vec2::new(32.0, 26.0), egui::Sense::click());
                            ui.painter().rect_filled(rect, 6.0, to_egui_color(*col));
                            if resp
                                .on_hover_text("Click to apply color to selected element")
                                .clicked()
                            {
                                self.apply_color_to_selection(*col);
                            }
                        }
                    });
                    if ui
                        .small_button("🔀 Shuffle Palette onto Current Page")
                        .clicked()
                    {
                        self.record_mutation();
                        self.project.brand_kit.active_palette_idx = p_idx;
                        self.palette_shuffle_cursor += 1;
                        let rot = self.palette_shuffle_cursor;
                        if let Some(page) = self.current_page_mut() {
                            if let Some(bg_col) = pal.colors.get(rot % pal.colors.len()) {
                                page.background = PageBackground::Solid(*bg_col);
                            }
                            for (i, el) in page.elements.iter_mut().enumerate() {
                                let c = pal.colors[(i + rot + 1) % pal.colors.len()];
                                match &mut el.kind {
                                    ElementKind::Shape { fill, .. } => fill.set_primary_color(c),
                                    ElementKind::Text { color, .. } => *color = c,
                                    _ => {}
                                }
                            }
                        }
                    }
                });
            ui.add_space(8.0);
        }
    }

    fn apply_color_to_selection(&mut self, col: ColorRgba) {
        let Some(sel_id) = self.selected_id else {
            return;
        };
        self.record_mutation();
        if let Some(el) = self
            .current_page_mut()
            .and_then(|p| p.elements.iter_mut().find(|e| e.id == sel_id))
        {
            match &mut el.kind {
                ElementKind::Shape { fill, .. } => fill.set_primary_color(col),
                ElementKind::Text { color, .. } => *color = col,
                ElementKind::FreehandPath { color, .. } => *color = col,
                _ => {}
            }
        }
    }

    fn render_dock_uploads(&mut self, ui: &mut Ui) {
        ui.heading("🖼 Uploads & Studio Stock");
        ui.add_space(8.0);

        if ui
            .add_sized(
                [260.0, 42.0],
                egui::Button::new(
                    RichText::new("📤 Upload Image from Computer")
                        .strong()
                        .color(Color32::WHITE),
                )
                .fill(Color32::from_rgb(125, 42, 232)),
            )
            .clicked()
        {
            self.import_image_from_disk();
        }

        ui.add_space(14.0);
        ui.strong("Built-In Studio Assets (Ready for Edge-Colour Cutout)");
        ui.add_space(6.0);

        let assets_len = self.stock_assets.len();
        for i in 0..assets_len {
            let name = self.stock_assets[i].name;
            let sub = self.stock_assets[i].subtitle;
            egui::Frame::none()
                .fill(Color32::from_rgb(33, 37, 49))
                .rounding(8.0)
                .inner_margin(Margin::same(10.0))
                .show(ui, |ui| {
                    ui.strong(name);
                    ui.label(RichText::new(sub).size(11.0).color(Color32::LIGHT_GRAY));
                    if ui.small_button("+ Add to Canvas").clicked() {
                        self.record_mutation();
                        let id = self.project.alloc_id();
                        let el = make_stock_image_element(
                            id,
                            &self.stock_assets[i],
                            self.project.width * 0.3,
                            self.project.height * 0.25,
                        );
                        if let Some(page) = self.current_page_mut() {
                            page.elements.push(el);
                            self.selected_id = Some(id);
                        }
                    }
                });
            ui.add_space(6.0);
        }
    }

    fn render_dock_magic_studio(&mut self, ui: &mut Ui) {
        ui.heading("✨ Local Studio Tools (Heuristic & Template Suite)");
        ui.label(
            RichText::new(
                "Local deterministic image filters, edge-color cutout, palette extraction, and copy templates.",
            )
            .size(12.0)
            .color(Color32::from_rgb(0, 245, 212)),
        );
        ui.add_space(10.0);

        // 1. Local Copy Templates
        egui::Frame::none()
            .fill(Color32::from_rgb(33, 37, 49))
            .rounding(10.0)
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.strong("✍ Local Copy Templates");
                ui.add(
                    egui::TextEdit::singleline(&mut self.magic_write_topic)
                        .hint_text("Enter topic or product..."),
                );
                egui::ComboBox::from_id_salt("mw_tone")
                    .selected_text(self.magic_write_tone.label())
                    .show_ui(ui, |ui| {
                        for t in MagicWriteTone::all() {
                            ui.selectable_value(&mut self.magic_write_tone, *t, t.label());
                        }
                    });
                if ui.button("✨ Insert Template Headline & Copy").clicked() {
                    let (head, body) =
                        magic_write_generate(&self.magic_write_topic, self.magic_write_tone);
                    self.insert_text_preset(
                        "Template Headline",
                        &head,
                        FontFamily::PosterHeavy,
                        56.0,
                        rgb(0, 245, 212),
                        true,
                        TextEffect::None,
                    );
                    self.insert_text_preset(
                        "Template Body",
                        &body,
                        FontFamily::Sans,
                        26.0,
                        rgb(241, 245, 249),
                        false,
                        TextEffect::None,
                    );
                    self.status_banner =
                        "Inserted local copy template headline & body!".to_string();
                }
            });

        ui.add_space(10.0);

        // 2. Selected Image Heuristic Tools
        egui::Frame::none()
            .fill(Color32::from_rgb(33, 37, 49))
            .rounding(10.0)
            .inner_margin(Margin::same(10.0))
            .show(ui, |ui| {
                ui.strong("🖼 Photo Adjustments & Palette Tools");
                let mut extract_req: Option<(Vec<u8>, u32, u32, f32, f32, f32, f32)> = None;

                if let Some(sel_id) = self.selected_id {
                    if let Some(el) = self
                        .project
                        .pages
                        .get_mut(self.current_page_idx)
                        .and_then(|p| p.elements.iter_mut().find(|e| e.id == sel_id))
                    {
                        let ex = el.x;
                        let ey = el.y;
                        let ew = el.width;
                        let eh = el.height;
                        if let ElementKind::Image {
                            width_px,
                            height_px,
                            raw_rgba,
                            adjustments,
                            revision,
                            ..
                        } = &mut el.kind
                        {
                            if ui
                                .checkbox(
                                    &mut adjustments.bg_removed,
                                    "Edge-Color Background Cutout (Simple Backdrops)",
                                )
                                .changed()
                            {
                                *revision += 1;
                            }
                            if ui
                                .add(
                                    egui::Slider::new(&mut adjustments.brightness, -0.8..=0.8)
                                        .text("Brightness"),
                                )
                                .changed()
                                || ui
                                    .add(
                                        egui::Slider::new(&mut adjustments.contrast, -0.8..=0.8)
                                            .text("Contrast"),
                                    )
                                    .changed()
                                || ui
                                    .add(
                                        egui::Slider::new(&mut adjustments.saturation, -1.0..=1.0)
                                            .text("Saturation"),
                                    )
                                    .changed()
                                || ui
                                    .add(
                                        egui::Slider::new(&mut adjustments.warmth, -1.0..=1.0)
                                            .text("Warmth"),
                                    )
                                    .changed()
                                || ui
                                    .add(
                                        egui::Slider::new(&mut adjustments.blur, 0.0..=6.0)
                                            .text("Blur"),
                                    )
                                    .changed()
                                || ui
                                    .add(
                                        egui::Slider::new(&mut adjustments.sharpen, 0.0..=2.0)
                                            .text("Sharpen"),
                                    )
                                    .changed()
                            {
                                *revision += 1;
                            }

                            ui.horizontal(|ui| {
                                if ui.small_button("Cyber Duotone").clicked() {
                                    adjustments.duotone =
                                        Some((rgb(18, 12, 52), rgb(0, 245, 212)));
                                    *revision += 1;
                                }
                                if ui.small_button("Sunset Duotone").clicked() {
                                    adjustments.duotone =
                                        Some((rgb(74, 20, 140), rgb(255, 202, 58)));
                                    *revision += 1;
                                }
                                if ui.small_button("Reset Filters").clicked() {
                                    *adjustments = ImageAdjustments::default();
                                    *revision += 1;
                                }
                            });

                            ui.add_space(6.0);
                            if ui.button("🎨 Create Sampled-Colour Composition").clicked() {
                                extract_req = Some((
                                    raw_rgba.to_vec(),
                                    *width_px,
                                    *height_px,
                                    ex,
                                    ey,
                                    ew,
                                    eh,
                                ));
                            }
                        } else {
                            ui.label("Select an Image element on the canvas to unlock photo tools.");
                        }
                    }
                } else {
                    ui.label("Select an Image element (or insert one from Uploads) to use Edge-Color BG Cutout, Palette Composition, & Filters.");
                }

                if let Some((raw, w, h, ex, ey, ew, eh)) = extract_req {
                    self.record_mutation();
                    let mut next_id = self.project.next_id;
                    let new_layers =
                        extract_magic_layers_from_image(&raw, w, h, ex, ey, ew, eh, &mut next_id);
                    self.project.next_id = next_id;
                    if let Some(page) = self.current_page_mut() {
                        page.elements.extend(new_layers);
                    }
                    self.status_banner =
                        "Created palette swatches & composition cards from image!".to_string();
                }
            });
    }

    fn render_dock_charts_tables(&mut self, ui: &mut Ui) {
        ui.heading("📊 Charts & Data Tables");
        ui.add_space(8.0);

        for ct in ChartType::all() {
            if ui
                .add_sized(
                    [260.0, 36.0],
                    egui::Button::new(format!("+ Insert {}", ct.label()))
                        .fill(Color32::from_rgb(36, 40, 54)),
                )
                .clicked()
            {
                self.record_mutation();
                let id = self.project.alloc_id();
                let pw = self.project.width;
                let ph = self.project.height;
                if let Some(page) = self.current_page_mut() {
                    page.elements.push(CanvasElement {
                        id,
                        name: ct.label().to_string(),
                        x: pw * 0.2,
                        y: ph * 0.2,
                        width: 640.0,
                        height: 440.0,
                        rotation: 0.0,
                        opacity: 1.0,
                        locked: false,
                        visible: true,
                        flip_h: false,
                        flip_v: false,
                        shadow: DropShadow::default(),
                        animation: ElementAnimation::Rise,
                        group_id: None,
                        text_format: Default::default(),
                        style_id: None,
                        binding: None,
                        kind: ElementKind::Chart {
                            chart_type: *ct,
                            title: "Quarterly Growth Metrics".to_string(),
                            labels: vec![
                                "Q1".to_string(),
                                "Q2".to_string(),
                                "Q3".to_string(),
                                "Q4".to_string(),
                            ],
                            values: vec![38.0, 64.0, 89.0, 125.0],
                            palette: vec![
                                rgb(0, 196, 204),
                                rgb(125, 42, 232),
                                rgb(255, 92, 138),
                                rgb(254, 228, 64),
                            ],
                            show_values: true,
                            show_grid: true,
                            text_color: rgb(255, 255, 255),
                        },
                    });
                    self.selected_id = Some(id);
                }
            }
            ui.add_space(4.0);
        }

        ui.separator();
        if ui
            .add_sized(
                [260.0, 38.0],
                egui::Button::new("+ Insert Structured Data Table")
                    .fill(Color32::from_rgb(45, 52, 78)),
            )
            .clicked()
        {
            self.record_mutation();
            let id = self.project.alloc_id();
            let pw = self.project.width;
            let ph = self.project.height;
            if let Some(page) = self.current_page_mut() {
                page.elements.push(CanvasElement {
                    id,
                    name: "Data Table".to_string(),
                    x: pw * 0.2,
                    y: ph * 0.25,
                    width: 660.0,
                    height: 320.0,
                    rotation: 0.0,
                    opacity: 1.0,
                    locked: false,
                    visible: true,
                    flip_h: false,
                    flip_v: false,
                    shadow: DropShadow::default(),
                    animation: ElementAnimation::Fade,
                    group_id: None,
                    text_format: Default::default(),
                    style_id: None,
                    binding: None,
                    kind: ElementKind::Table {
                        rows: 3,
                        cols: 3,
                        cells: vec![
                            vec![
                                "Metric".to_string(),
                                "Target".to_string(),
                                "Actual".to_string(),
                            ],
                            vec![
                                "Conversion".to_string(),
                                "4.2%".to_string(),
                                "6.8%".to_string(),
                            ],
                            vec![
                                "Retention".to_string(),
                                "85%".to_string(),
                                "94%".to_string(),
                            ],
                        ],
                        header_bg: rgb(0, 196, 204),
                        row_bg: rgb(30, 41, 59),
                        alt_row_bg: rgb(20, 28, 44),
                        border_color: rgb(71, 85, 105),
                        text_color: rgb(255, 255, 255),
                        font_size: 20.0,
                    },
                });
                self.selected_id = Some(id);
            }
        }

        // Live Editor for Selected Chart or Table
        if let Some(sel_id) = self.selected_id {
            if let Some(el) = self
                .project
                .pages
                .get_mut(self.current_page_idx)
                .and_then(|p| p.elements.iter_mut().find(|e| e.id == sel_id))
            {
                match &mut el.kind {
                    ElementKind::Chart { labels, values, .. } => {
                        ui.separator();
                        ui.strong("Edit Selected Chart Data");
                        for i in 0..values.len().min(labels.len()) {
                            ui.horizontal(|ui| {
                                ui.add(
                                    egui::TextEdit::singleline(&mut labels[i]).desired_width(80.0),
                                );
                                ui.add(
                                    egui::DragValue::new(&mut values[i])
                                        .range(0.0..=10000.0)
                                        .speed(1.0),
                                );
                            });
                        }
                        ui.horizontal(|ui| {
                            if ui.small_button("+ Add Bar").clicked() {
                                labels.push(format!("Item {}", labels.len() + 1));
                                values.push(50.0);
                            }
                            if values.len() > 2 && ui.small_button("− Remove Last").clicked() {
                                labels.pop();
                                values.pop();
                            }
                        });
                    }
                    ElementKind::Table {
                        rows, cols, cells, ..
                    } => {
                        ui.separator();
                        ui.strong("Edit Selected Table Cells");
                        for r in 0..*rows {
                            ui.horizontal(|ui| {
                                for c in 0..*cols {
                                    if let Some(cell) =
                                        cells.get_mut(r).and_then(|row| row.get_mut(c))
                                    {
                                        ui.add(
                                            egui::TextEdit::singleline(cell).desired_width(72.0),
                                        );
                                    }
                                }
                            });
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    fn render_dock_draw(&mut self, ui: &mut Ui) {
        ui.heading("✏ Freehand Draw Studio");
        ui.checkbox(
            &mut self.canvas_state.drawing_mode,
            "Enable Canvas Drawing Mode",
        );
        ui.add_space(8.0);

        for b in [
            BrushType::Pen,
            BrushType::Marker,
            BrushType::Highlighter,
            BrushType::NeonGlow,
        ] {
            ui.radio_value(&mut self.canvas_state.draw_brush, b, b.label());
        }

        ui.add_space(8.0);
        ui.add(egui::Slider::new(&mut self.canvas_state.draw_width, 1.0..=36.0).text("Brush Size"));
        ui.horizontal(|ui| {
            ui.label("Brush Color:");
            color_Edit_button(ui, &mut self.canvas_state.draw_color);
        });
    }

    fn import_font_from_path(&mut self, path: &Path) {
        let mut candidate = self.project.clone();
        match imported_fonts::import(&mut candidate, path) {
            Ok(id) => {
                self.record_mutation();
                self.project = candidate;
                let selected = self.selected_id;
                if let Some(element) = self
                    .current_page_mut()
                    .and_then(|p| p.elements.iter_mut().find(|e| Some(e.id) == selected))
                {
                    if let ElementKind::Text {
                        font_family,
                        bold,
                        italic,
                        ..
                    } = &mut element.kind
                    {
                        *font_family = FontFamily::Imported(id);
                        *bold = false;
                        *italic = false;
                    }
                }
                self.status_banner="Imported and embedded font face; choose its actual weight/style in the font menu".into();
            }
            Err(e) => self.status_banner = format!("Font import failed: {e}"),
        }
    }

    fn render_selected_typography(&mut self, ui: &mut Ui) {
        let font_choices: Vec<_> = FontFamily::all()
            .iter()
            .map(|f| (*f, f.label()))
            .chain(
                self.project
                    .imported_fonts
                    .iter()
                    .map(|f| (FontFamily::Imported(f.id), f.name.clone())),
            )
            .collect();
        let selected_range = self.canvas_state.text_selection;
        let units = self.project.effective_units_per_inch();
        let Some(id) = self.selected_id else {
            return;
        };
        let Some(element) = self
            .project
            .pages
            .get_mut(self.current_page_idx)
            .and_then(|p| p.elements.iter_mut().find(|e| e.id == id))
        else {
            return;
        };
        if let ElementKind::Text {
            content,
            font_family,
            font_size,
            color,
            bold,
            italic,
            uppercase,
            align,
            letter_spacing,
            line_height,
            list_style,
            ..
        } = &mut element.kind
        {
            ui.strong("Selected Text · Print Typography");
            ui.add_enabled_ui(!element.locked, |ui| {
                let mut points = *font_size * 72. / units;
                ui.horizontal(|ui| {
                    ui.label("Size (pt)");
                    if ui
                        .add(
                            egui::DragValue::new(&mut points)
                                .range(3.0..=200.0)
                                .speed(0.25),
                        )
                        .changed()
                    {
                        *font_size = points * units / 72.;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Line height");
                    ui.add(
                        egui::DragValue::new(line_height)
                            .range(0.8..=3.0)
                            .speed(0.02),
                    );
                });
                let mut tracking = *letter_spacing * 72. / units;
                ui.horizontal(|ui| {
                    ui.label("Tracking (pt)");
                    if ui
                        .add(
                            egui::DragValue::new(&mut tracking)
                                .range(-5.0..=30.0)
                                .speed(0.1),
                        )
                        .changed()
                    {
                        *letter_spacing = tracking * units / 72.;
                    }
                });
                ui.horizontal(|ui| {
                    ui.label("Text colour");
                    color_Edit_button(ui, color);
                });
                ui.separator();
                ui.strong("Paragraph spacing & frame insets (pt)");
                egui::Grid::new("paragraph-values")
                    .num_columns(2)
                    .show(ui, |ui| {
                        for (label, value) in [
                            ("Before paragraph", &mut element.text_format.space_before),
                            ("After paragraph", &mut element.text_format.space_after),
                            (
                                "First line indent",
                                &mut element.text_format.first_line_indent,
                            ),
                            ("Left inset", &mut element.text_format.inset_left),
                            ("Right inset", &mut element.text_format.inset_right),
                            ("Top inset", &mut element.text_format.inset_top),
                            ("Bottom inset", &mut element.text_format.inset_bottom),
                        ] {
                            let mut points = *value * 72. / units;
                            ui.label(label);
                            if ui
                                .add(
                                    egui::DragValue::new(&mut points)
                                        .range(0.0..=500.0)
                                        .speed(0.25),
                                )
                                .changed()
                            {
                                *value = points * units / 72.;
                            }
                            ui.end_row();
                        }
                    });
                if let Some((id, start, end)) =
                    selected_range.filter(|(id, start, end)| *id == element.id && start < end)
                {
                    let _ = id;
                    ui.label(format!("Selected characters {}–{}", start + 1, end));
                    ui.horizontal(|ui| {
                        for (label, b, i, u) in [
                            ("Bold", Some(true), None, None),
                            ("Italic", None, Some(true), None),
                            ("Underline", None, None, Some(true)),
                            ("Regular", Some(false), Some(false), Some(false)),
                        ] {
                            if ui.button(label).clicked() {
                                element.text_format.apply_run(
                                    crate::model::TextRun {
                                        start,
                                        end,
                                        bold: b,
                                        italic: i,
                                        underline: u,
                                        font_family: None,
                                    },
                                    content.chars().count(),
                                );
                            }
                        }
                    });
                    egui::ComboBox::from_id_salt("character-font")
                        .selected_text("Apply font to selection")
                        .show_ui(ui, |ui| {
                            for (family, name) in &font_choices {
                                if ui.button(name).clicked() {
                                    element.text_format.apply_run(
                                        crate::model::TextRun {
                                            start,
                                            end,
                                            font_family: Some(*family),
                                            ..Default::default()
                                        },
                                        content.chars().count(),
                                    );
                                }
                            }
                        });
                } else {
                    ui.label(
                        "Double-click the canvas text and select characters to apply emphasis.",
                    );
                }
            });
            if ui
                .add_enabled(!element.locked, egui::Button::new("Clear character styles"))
                .clicked()
            {
                element.text_format.runs.clear();
            }
            let layout = crate::paragraph_layout::layout_formatted(
                content,
                *font_family,
                *font_size,
                *bold,
                *italic,
                *uppercase,
                *align,
                *letter_spacing,
                *line_height,
                *list_style,
                element.width,
                element.height,
                &element.text_format,
            );
            if layout.is_overflowing() {
                ui.colored_label(
                    Color32::LIGHT_RED,
                    "Text exceeds its box. Enlarge the frame or reduce the type size.",
                );
            }
            ui.separator();
        }
    }

    fn render_position_inspector(&mut self, ui: &mut Ui) {
        ui.strong("Position & Size");
        ui.horizontal(|ui| {
            egui::ComboBox::from_id_salt("position-units")
                .selected_text(["mm", "in", "layout px"][self.inspector_unit as usize])
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut self.inspector_unit, 0, "mm");
                    ui.selectable_value(&mut self.inspector_unit, 1, "in");
                    ui.selectable_value(&mut self.inspector_unit, 2, "layout px");
                });
            ui.checkbox(&mut self.inspector_aspect_lock, "Aspect lock");
        });
        let Some(id) = self.selected_id else {
            ui.label("Select an element to set its position.");
            return;
        };
        let selected = document_ops::selection_ids(
            &self.project.pages[self.current_page_idx],
            &self.canvas_state.selected_ids,
            Some(id),
        );
        if selected.len() > 1 {
            ui.label("Use the group handles to resize this selection proportionally. Select one layer for numeric geometry.");
            return;
        }
        let units = self.project.effective_units_per_inch();
        let factor = match self.inspector_unit {
            0 => 25.4 / units,
            1 => 1. / units,
            _ => 1.,
        };
        let aspect_lock = self.inspector_aspect_lock;
        if let Some(element) = self
            .project
            .pages
            .get_mut(self.current_page_idx)
            .and_then(|p| p.elements.iter_mut().find(|e| e.id == id))
        {
            ui.add_enabled_ui(!element.locked, |ui| {
                ui.add(egui::TextEdit::singleline(&mut element.name).hint_text("Layer name"));
                let mut values = [
                    element.x * factor,
                    element.y * factor,
                    element.width * factor,
                    element.height * factor,
                ];
                let mut changed = [false; 4];
                egui::Grid::new("position-values")
                    .num_columns(2)
                    .show(ui, |ui| {
                        for (i, label) in ["X", "Y", "Width", "Height"].iter().enumerate() {
                            ui.label(*label);
                            let mut widget = egui::DragValue::new(&mut values[i])
                                .speed(if self.inspector_unit == 1 { 0.01 } else { 0.2 })
                                .max_decimals(3);
                            if i >= 2 {
                                widget = widget.range((factor.max(0.001))..=100_000.0);
                            }
                            changed[i] = ui.add(widget).changed();
                            ui.end_row();
                        }
                    });
                if changed.iter().any(|c| *c) {
                    if let Err(error) =
                        document_ops::set_geometry(element, values, changed, factor, aspect_lock)
                    {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                }
                ui.horizontal(|ui| {
                    ui.label("Rotation °");
                    ui.add(egui::DragValue::new(&mut element.rotation).speed(0.5));
                });
                ui.horizontal(|ui| {
                    ui.checkbox(&mut element.flip_h, "Flip H");
                    ui.checkbox(&mut element.flip_v, "Flip V");
                });
            });
        }
        ui.separator();
    }

    fn render_dock_layers(&mut self, ui: &mut Ui) {
        ui.heading("☰ Layers & Position");
        ui.add_space(6.0);
        self.render_position_inspector(ui);

        let mut selection = None;
        let mut action = None;

        if let Some(page) = self.project.pages.get_mut(self.current_page_idx) {
            let len = page.elements.len();
            for (rev_i, el) in page.elements.iter_mut().enumerate().rev() {
                let is_sel = self.selected_id == Some(el.id);
                egui::Frame::none()
                    .fill(if is_sel {
                        Color32::from_rgb(48, 56, 84)
                    } else {
                        Color32::from_rgb(32, 35, 46)
                    })
                    .rounding(6.0)
                    .inner_margin(Margin::symmetric(8.0, 5.0))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let vis_icon = if el.visible { "👁" } else { "🚫" };
                            if ui.small_button(vis_icon).clicked() {
                                el.visible = !el.visible;
                            }
                            let lock_icon = if el.locked { "🔒" } else { "🔓" };
                            if ui.small_button(lock_icon).clicked() {
                                el.locked = !el.locked;
                            }
                            if ui.selectable_label(is_sel, &el.name).clicked() {
                                selection = Some(el.id);
                            }
                            ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                                if ui.small_button("🗑").clicked() {
                                    action = Some((el.id, 0));
                                }
                                if rev_i > 0 && ui.small_button("⬇").clicked() {
                                    action = Some((el.id, 1));
                                }
                                if rev_i + 1 < len && ui.small_button("⬆").clicked() {
                                    action = Some((el.id, 2));
                                }
                            });
                        });
                    });
                ui.add_space(4.0);
            }
        }

        if let Some(id) = selection {
            self.select_layer(id);
        }
        if let Some((id, command)) = action {
            self.select_layer(id);
            match command {
                0 => self.delete_selected(),
                1 => self.move_selected_layer(false),
                _ => self.move_selected_layer(true),
            }
            self.reconcile_selection_and_page();
        }
    }

    fn render_dock_app_info(&mut self, ui: &mut Ui) {
        ui.heading("Canvas Studio");
        ui.label(format!("Version {}", env!("CARGO_PKG_VERSION")));
        ui.label("A local visual design and print editor.");
        ui.separator();
        ui.strong("Working with your designs");
        ui.label("Create a document or choose a built-in studio template.");
        ui.label("Import your images and layout JSON through the file picker.");
        ui.label("Save the project JSON together with its companion assets folder.");
        ui.label("Use export preflight to check supported content before printing.");
        ui.separator();
        ui.strong("Local by design");
        ui.label("Fonts are embedded in the application. No account or subscription is required.");
        ui.label("Creative tools run locally; no cloud model service is connected.");
        ui.label(
            "PDF, PNG, SVG and HTML exports are available. Editable PPTX export is not included.",
        );
        ui.label("Some decorative effects and charts are blocked by print preflight.");
    }

    fn render_bottom_pages_bar(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("canvas_bottom_bar")
            .exact_height(78.0)
            .frame(
                egui::Frame::none()
                    .fill(Color32::from_rgb(18, 20, 27))
                    .inner_margin(Margin::symmetric(12.0, 6.0)),
            )
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let count = self.project.pages.len();
                    if ui
                        .add_enabled(self.current_page_idx > 0, egui::Button::new("Previous"))
                        .clicked()
                    {
                        self.current_page_idx -= 1;
                        self.selected_id = None;
                        self.canvas_state.reset_for_document();
                    }
                    egui::ComboBox::from_id_salt("current-book-page")
                        .width(220.0)
                        .selected_text(format!(
                            "{} / {} · {}",
                            self.current_page_idx + 1,
                            count,
                            self.project.pages[self.current_page_idx].title
                        ))
                        .show_ui(ui, |ui| {
                            for index in 0..count {
                                if ui
                                    .selectable_value(
                                        &mut self.current_page_idx,
                                        index,
                                        format!(
                                            "{} · {}",
                                            index + 1,
                                            self.project.pages[index].title
                                        ),
                                    )
                                    .changed()
                                {
                                    self.selected_id = None;
                                    self.canvas_state.reset_for_document();
                                }
                            }
                        });
                    if ui
                        .add_enabled(self.current_page_idx + 1 < count, egui::Button::new("Next"))
                        .clicked()
                    {
                        self.current_page_idx += 1;
                        self.selected_id = None;
                        self.canvas_state.reset_for_document();
                    }
                    ui.add_enabled_ui(self.master_edit_session.is_none(), |ui| {
                        if ui.button("Add Page").clicked() {
                            self.record_mutation();
                            let page = CanvasPage::new(
                                self.project.alloc_id(),
                                format!("Page {}", count + 1),
                            );
                            self.project.pages.push(page);
                            self.current_page_idx = count;
                            self.selected_id = None;
                            self.canvas_state.reset_for_document();
                        }
                        if ui.button("Duplicate Page").clicked() {
                            self.duplicate_book_page(self.current_page_idx);
                        }
                        if ui
                            .add_enabled(count > 1, egui::Button::new("Delete Page"))
                            .clicked()
                        {
                            self.record_mutation();
                            self.project.pages.remove(self.current_page_idx);
                            self.reconcile_selection_and_page();
                        }
                    });
                    ui.toggle_value(&mut self.show_notes_drawer, "Notes");
                });
                ui.horizontal(|ui| {
                    let status_width = (ui.available_width() - 510.).max(80.);
                    ui.add_sized(
                        [status_width, 20.],
                        egui::Label::new(
                            RichText::new(&self.status_banner)
                                .size(11.5)
                                .color(Color32::from_rgb(148, 163, 184)),
                        )
                        .truncate(),
                    );
                    ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Fit").clicked() {
                            self.fit_canvas_to_viewport();
                        }
                        let response = ui.add(
                            egui::Slider::new(&mut self.canvas_state.zoom, 0.15..=2.5)
                                .text("Zoom")
                                .fixed_decimals(2),
                        );
                        if response.changed() && (response.dragged() || response.has_focus()) {
                            self.auto_fit_canvas = false;
                        }
                        ui.checkbox(&mut self.canvas_state.snap_enabled, "Snap");
                        ui.checkbox(&mut self.canvas_state.show_grid, "Grid");
                        ui.checkbox(&mut self.canvas_state.show_rulers, "Rulers");
                    });
                });
            });

        if self.show_notes_drawer {
            TopBottomPanel::bottom("canvas_speaker_notes_drawer")
                .exact_height(110.0)
                .frame(
                    egui::Frame::none()
                        .fill(Color32::from_rgb(22, 25, 34))
                        .inner_margin(Margin::symmetric(16.0, 8.0))
                        .stroke(Stroke::new(1.0_f32, Color32::from_rgb(45, 52, 70))),
                )
                .show(ctx, |ui| {
                    if let Some(page) = self.project.pages.get_mut(self.current_page_idx) {
                        ui.horizontal(|ui| {
                            ui.strong(format!("📝 Speaker Notes & Slide Title ({}):", page.title));
                            ui.add(
                                egui::TextEdit::singleline(&mut page.title)
                                    .desired_width(180.0)
                                    .hint_text("Page Title"),
                            );
                        });
                        ui.add_space(4.0);
                        ui.add(
                            egui::TextEdit::multiline(&mut page.notes)
                                .desired_width(f32::INFINITY)
                                .desired_rows(3)
                                .hint_text("Add speaker notes, script talking points, or layout reminders for this page..."),
                        );
                    }
                });
        }
    }

    fn render_presentation_overlay(&mut self, ctx: &Context) {
        TopBottomPanel::bottom("presentation_controls")
            .exact_height(48.0)
            .frame(egui::Frame::none().fill(Color32::from_rgb(12, 14, 20)))
            .show(ctx, |ui| {
                ui.horizontal_centered(|ui| {
                    ui.strong(format!(
                        "▶ Presenting: {} — Slide {} / {}",
                        self.project.title,
                        self.current_page_idx + 1,
                        self.project.pages.len()
                    ));
                    ui.separator();
                    if ui.button("⬅ Prev Slide").clicked() && self.current_page_idx > 0 {
                        self.current_page_idx -= 1;
                        self.canvas_state.anim_start_time = ctx.input(|i| i.time);
                    }
                    if ui.button("Next Slide ➡").clicked()
                        && self.current_page_idx + 1 < self.project.pages.len()
                    {
                        self.current_page_idx += 1;
                        self.canvas_state.anim_start_time = ctx.input(|i| i.time);
                    }
                    if ui.button("🔄 Replay Animations").clicked() {
                        self.canvas_state.anim_start_time = ctx.input(|i| i.time);
                    }
                    if ui.button("✖ Exit Presentation (Esc)").clicked() {
                        self.presentation_mode = false;
                    }
                });
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(Color32::from_rgb(8, 10, 15)))
            .show(ctx, |ui| {
                render_interactive_canvas(
                    ctx,
                    ui,
                    &mut self.project,
                    self.current_page_idx,
                    &mut self.selected_id,
                    &mut self.canvas_state,
                    &mut self.history,
                    true,
                );
            });
    }

    fn render_export_dialog(&mut self, ctx: &Context) {
        let mut open = self.show_export_modal;
        egui::Window::new("⬇ Print PDF & High-DPI Export")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .anchor(egui::Align2::CENTER_CENTER, Vec2::ZERO)
            .show(ctx, |ui| {
                ui.set_min_width(430.0);
                ScrollArea::vertical().max_height((ctx.screen_rect().height()-120.).max(200.)).show(ui,|ui| {
                let (w_mm, h_mm) = (
                    self.project.effective_width_mm(),
                    self.project.effective_height_mm(),
                );
                let (w_pt, h_pt) = self.project.pdf_points_dimensions();
                let (dpi_w, dpi_h) = self.project.pixel_dimensions_for_dpi(self.export_dpi);

                ui.label(
                    RichText::new(format!(
                        "Physical Trim Size: {:.1} × {:.1} mm ({:.2} × {:.2} pt)",
                        w_mm, h_mm, w_pt, h_pt
                    ))
                    .strong()
                    .color(Color32::from_rgb(0, 245, 212)),
                );
                ui.label(format!(
                    "Raster Output at {} DPI: {} × {} px",
                    self.export_dpi, dpi_w, dpi_h
                ));

                ui.horizontal(|ui| {
                    ui.label("Print DPI:");
                    ui.radio_value(&mut self.export_dpi, 96, "96 DPI (Screen)");
                    ui.radio_value(&mut self.export_dpi, 150, "150 DPI (Proof)");
                    ui.radio_value(&mut self.export_dpi, 300, "300 DPI (Print)");
                });

                ui.horizontal(|ui| {
                    ui.label("Bleed (mm):");
                    ui.add(
                        egui::DragValue::new(&mut self.project.bleed_mm)
                            .range(0.0..=10.0)
                            .speed(0.5),
                    );
                    ui.checkbox(&mut self.project.crop_marks, "Include Crop Marks in PDF");
                });

                ui.checkbox(
                    &mut self.export_transparent,
                    "Transparent Background (PNG / SVG)",
                );

                ui.separator();

                if ui.button("Save Editable Portable Copy (.canvas)").clicked() {
                    if let Some(path)=rfd::FileDialog::new().set_file_name("design.canvas").add_filter("Portable Canvas Studio project",&["canvas"]).save_file() {
                        self.status_banner=match project_package::save(&self.project,&path){Ok(())=>format!("Saved portable copy to {}",path.display()),Err(e)=>format!("Portable save failed: {e}")};
                    }
                }

                self.render_output_presets(ui);
                // Export Preflight Check
                let preflight = run_export_preflight(&self.project, self.export_dpi);
                if preflight.issues.is_empty() {
                    ui.colored_label(
                        Color32::from_rgb(16, 185, 129),
                        "✓ Print Preflight Passed (No overflow, missing fonts, or low-DPI warnings)",
                    );
                } else {
                    egui::Frame::none()
                        .fill(Color32::from_rgb(55, 28, 32))
                        .rounding(6.0)
                        .inner_margin(Margin::same(8.0))
                        .show(ui, |ui| {
                            ui.colored_label(
                                Color32::from_rgb(248, 113, 113),
                                format!(
                                    "⚠ Preflight Warnings ({} issue(s)):",
                                    preflight.issues.len()
                                ),
                            );
                            ScrollArea::vertical().max_height(80.0).show(ui, |ui| {
                                for issue in &preflight.issues {
                                    ui.label(
                                        RichText::new(format!(
                                            "• Page {} ({}): {}",
                                            issue.page_index + 1,
                                            issue.element_name,
                                            issue.message
                                        ))
                                        .size(11.0),
                                    );
                                }
                            });
                        });
                }

                ui.separator();
                ui.strong("PDF Page Selection:");
                ui.horizontal(|ui| {
                    ui.radio_value(&mut self.pdf_selection_mode, 0, "Current Page");
                    ui.radio_value(
                        &mut self.pdf_selection_mode,
                        1,
                        format!("All Pages ({})", self.project.pages.len()),
                    );
                    ui.radio_value(&mut self.pdf_selection_mode, 2, "Range:");
                    if self.pdf_selection_mode == 2 {
                        let max_p = self.project.pages.len().max(1);
                        ui.add(
                            egui::DragValue::new(&mut self.pdf_range_start).range(1..=max_p),
                        );
                        ui.label("to");
                        ui.add(
                            egui::DragValue::new(&mut self.pdf_range_end).range(1..=max_p),
                        );
                    }
                });

                ui.add_space(6.0);
                if ui
                    .add_sized(
                        [410.0, 40.0],
                        egui::Button::new(
                            RichText::new(
                                "🖨 Export Print-Ready Vector PDF (.pdf with Embedded Fonts)",
                            )
                            .strong()
                            .color(Color32::WHITE),
                        )
                        .fill(Color32::from_rgb(225, 29, 72)),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("print_production.pdf")
                        .save_file()
                    {
                        let page_selection = match self.pdf_selection_mode {
                            0 => PageSelection::CurrentPage(self.current_page_idx),
                            2 => PageSelection::Range(
                                self.pdf_range_start.saturating_sub(1),
                                self.pdf_range_end.saturating_sub(1),
                            ),
                            _ => PageSelection::AllPages,
                        };
                        let options = PdfExportOptions {
                            page_selection,
                            units_per_inch: self.project.effective_units_per_inch(),
                            bleed_mm: self.project.bleed_mm,
                            include_crop_marks: self.project.crop_marks,
                            target_image_dpi: self.export_dpi,
                        };
                        match export_project_to_pdf(&self.project, &options, &path) {
                            Ok(_report) => {
                                self.status_banner =
                                    format!("Exported Vector PDF to {}", path.display())
                            }
                            Err(e) => self.status_banner = format!("PDF export error: {e}"),
                        }
                    }
                }
                ui.add_space(6.0);

                if ui
                    .add_sized(
                        [410.0, 36.0],
                        egui::Button::new(
                            RichText::new(format!(
                                "🖼 Export {} DPI PNG ({}×{} px) (.png)",
                                self.export_dpi, dpi_w, dpi_h
                            ))
                            .strong(),
                        )
                        .fill(Color32::from_rgb(0, 196, 204)),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("design.png")
                        .save_file()
                    {
                        match render_project_page_at_dpi(
                            &self.project,
                            self.current_page_idx,
                            self.export_dpi,
                            self.export_transparent,
                        ) { Ok(img) => {
                            match img.save_with_format(&path, image::ImageFormat::Png) {
                                Ok(_) => {
                                    self.status_banner = format!(
                                        "Exported {} DPI PNG to {}",
                                        self.export_dpi,
                                        path.display()
                                    )
                                }
                                Err(e) => self.status_banner = format!("PNG export error: {e}"),
                            }
                        }, Err(e) => self.status_banner=format!("PNG export error: {e}") }
                    }
                }
                ui.add_space(6.0);

                if ui
                    .add_sized(
                        [410.0, 36.0],
                        egui::Button::new(
                            RichText::new(format!(
                                "📷 Export {} DPI JPEG ({}×{} px) (.jpg)",
                                self.export_dpi, dpi_w, dpi_h
                            ))
                            .strong(),
                        )
                        .fill(Color32::from_rgb(16, 185, 129)),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("design.jpg")
                        .save_file()
                    {
                        match render_project_page_at_dpi(
                            &self.project,
                            self.current_page_idx,
                            self.export_dpi,
                            false,
                        ) { Ok(img) => {
                            let rgb_img = image::DynamicImage::ImageRgba8(img).into_rgb8();
                            match rgb_img.save_with_format(&path, image::ImageFormat::Jpeg) {
                                Ok(_) => {
                                    self.status_banner = format!(
                                        "Exported {} DPI JPEG to {}",
                                        self.export_dpi,
                                        path.display()
                                    )
                                }
                                Err(e) => self.status_banner = format!("JPEG export error: {e}"),
                            }
                        }, Err(e) => self.status_banner=format!("JPEG export error: {e}") }
                    }
                }
                ui.add_space(6.0);

                if ui
                    .add_sized(
                        [410.0, 36.0],
                        egui::Button::new(
                            RichText::new("📐 Export Scalable Vector SVG (.svg)").strong(),
                        )
                        .fill(Color32::from_rgb(125, 42, 232)),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("design.svg")
                        .save_file()
                    {
                        if self.project.pages.get(self.current_page_idx).is_some() {
                            let result = crate::exporter::export_project_page_to_svg(&self.project,self.current_page_idx,self.export_transparent)
                                .and_then(|svg|std::fs::write(&path,svg).map_err(|e|e.to_string()));
                            self.status_banner=match result {
                                Ok(())=>format!("Exported Vector SVG to {}",path.display()),
                                Err(e)=>format!("SVG export error: {e}"),
                            };
                        }
                    }
                }
                ui.add_space(6.0);

                if ui
                    .add_sized(
                        [410.0, 36.0],
                        egui::Button::new(
                            RichText::new("🌐 Export Interactive HTML5 Slide Deck (.html)").strong(),
                        )
                        .fill(Color32::from_rgb(37, 99, 235)),
                    )
                    .clicked()
                {
                    if let Some(path) = rfd::FileDialog::new()
                        .set_file_name("presentation_deck.html")
                        .save_file()
                    {
                        self.status_banner=match export_project_to_html_deck(&self.project)
                            .and_then(|html|std::fs::write(&path,html).map_err(|e|e.to_string())) {
                            Ok(())=>format!("Exported HTML Deck to {}",path.display()),
                            Err(e)=>format!("HTML export error: {e}"),
                        };
                    }
                }
                });
            });
        self.show_export_modal = open;
    }
}

#[allow(non_snake_case)]
fn color_Edit_button(ui: &mut Ui, color: &mut ColorRgba) {
    let mut srgba = Color32::from_rgba_unmultiplied(color[0], color[1], color[2], color[3]);
    if ui.color_edit_button_srgba(&mut srgba).changed() {
        *color = [srgba.r(), srgba.g(), srgba.b(), srgba.a()];
    }
}

fn apply_canvas_studio_theme(ctx: &Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.window_fill = Color32::from_rgb(20, 22, 30);
    visuals.panel_fill = Color32::from_rgb(20, 22, 30);
    visuals.widgets.noninteractive.bg_fill = Color32::from_rgb(30, 33, 44);
    visuals.widgets.inactive.bg_fill = Color32::from_rgb(36, 40, 54);
    visuals.widgets.hovered.bg_fill = Color32::from_rgb(52, 58, 82);
    visuals.widgets.active.bg_fill = Color32::from_rgb(0, 196, 204);
    visuals.selection.bg_fill = Color32::from_rgb(125, 42, 232);
    visuals.window_rounding = Rounding::same(12.0);
    ctx.set_visuals(visuals);
}

fn build_window_icon() -> egui::IconData {
    let size = 64u32;
    let mut rgba = vec![0u8; (size * size * 4) as usize];
    let center = (size as f32) * 0.5;
    for y in 0..size {
        for x in 0..size {
            let fx = x as f32 + 0.5;
            let fy = y as f32 + 0.5;
            let qx = (fx - center).abs() - 20.0;
            let qy = (fy - center).abs() - 20.0;
            let dist = (qx.max(0.0).hypot(qy.max(0.0)) + qx.min(qy).min(0.0)) - 8.0;
            if dist <= 0.5 {
                let t = ((fx + fy) / (2.0 * size as f32)).clamp(0.0, 1.0);
                let mut r = (0.0 * (1.0 - t) + 125.0 * t) as u8;
                let mut g = (196.0 * (1.0 - t) + 42.0 * t) as u8;
                let mut b = (204.0 * (1.0 - t) + 232.0 * t) as u8;

                let dx = (fx - center).abs();
                let dy = (fy - center).abs();
                let star = (dx * 0.7 + dy * 0.22).min(dx * 0.22 + dy * 0.7);
                if star < 4.2 && (dx + dy) < 19.0 {
                    r = 255;
                    g = 255;
                    b = 255;
                }
                let idx = ((y * size + x) * 4) as usize;
                rgba[idx] = r;
                rgba[idx + 1] = g;
                rgba[idx + 2] = b;
                rgba[idx + 3] = 255;
            }
        }
    }
    egui::IconData {
        rgba,
        width: size,
        height: size,
    }
}

fn application_data_dir() -> PathBuf {
    #[cfg(target_os = "windows")]
    if let Some(path) = std::env::var_os("LOCALAPPDATA").or_else(|| std::env::var_os("APPDATA")) {
        return PathBuf::from(path);
    }
    #[cfg(target_os = "macos")]
    if let Some(home) = std::env::var_os("HOME") {
        return PathBuf::from(home).join("Library/Application Support");
    }
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(std::env::temp_dir)
}

fn main() -> eframe::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version") {
        println!("canvas-studio {}", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if args.iter().any(|a| a == "--self-check") {
        let fonts = crate::model::font_registry::FontRegistry::global();
        if !fonts.is_healthy() {
            eprintln!(
                "Embedded font validation failed: {:?}",
                fonts.load_warnings()
            );
            std::process::exit(1);
        }
        println!("Canvas Studio {}: all 10 embedded fonts are available; no external asset folder required.", env!("CARGO_PKG_VERSION"));
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--export-layout") {
        let (Some(input), Some(output)) = (args.get(i + 1), args.get(i + 2)) else {
            eprintln!("Usage: canvas-studio --export-layout INPUT.json OUTPUT.pdf");
            std::process::exit(2);
        };
        let result = import_layout_json(Path::new(input)).and_then(|project| {
            export_project_to_pdf(&project, &PdfExportOptions::default(), Path::new(output))
        });
        if let Err(error) = result {
            eprintln!("Layout export failed: {error}");
            std::process::exit(1);
        }
        println!("Exported layout to {output}");
        return Ok(());
    }
    if let Some(i) = args.iter().position(|a| a == "--verify-project") {
        let Some(path) = args.get(i + 1) else {
            eprintln!("Usage: canvas-studio --verify-project PATH");
            std::process::exit(2);
        };
        match load_project_json(Path::new(path)) {
            Ok(p) => println!("Verified {} ({} pages)", p.title, p.pages.len()),
            Err(e) => {
                eprintln!("Project validation failed: {e}");
                std::process::exit(1);
            }
        }
        return Ok(());
    }
    if args.iter().any(|a| a == "--serve-browser") {
        let port = args
            .iter()
            .position(|a| a == "--port")
            .and_then(|i| args.get(i + 1))
            .map(|s| s.parse::<u16>())
            .transpose()
            .unwrap_or_else(|_| {
                eprintln!("Port must be 0–65535");
                std::process::exit(2);
            })
            .unwrap_or(8787);
        let initial = args
            .iter()
            .position(|a| a == "--open")
            .and_then(|i| args.get(i + 1))
            .map(PathBuf::from);
        if let Err(error) = browser_editor::run(port, initial) {
            eprintln!("{error}");
            std::process::exit(1);
        }
        return Ok(());
    }
    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title(format!(
                "Canvas Studio {} — Local Design & Print Editor",
                env!("CARGO_PKG_VERSION")
            ))
            .with_inner_size([1480.0, 920.0])
            .with_min_inner_size([1024.0, 680.0])
            .with_icon(build_window_icon()),
        ..Default::default()
    };

    eframe::run_native(
        "Canvas Studio",
        native_options,
        Box::new(move |cc| {
            let mut app = CanvasStudioApp::new(cc);
            if let Some(index) = args.iter().position(|a| a == "--open") {
                if let Some(path) = args.get(index + 1) {
                    match load_project_json(Path::new(path)) {
                        Ok(project) => {
                            app.replace_document(project, Some(PathBuf::from(path)));
                            app.screen = WorkspaceScreen::StudioEditor;
                        }
                        Err(error) => app.status_banner = format!("Open failed: {error}"),
                    }
                }
            }
            Ok(Box::new(app))
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::exporter::{migrate_v1_project_to_v2, render_page_to_rgba_image};
    use crate::magic_studio::apply_image_pipeline;
    use crate::model::compute_smart_snap;
    use crate::model::font_registry::{FontFaceId, FontRegistry};
    use crate::model::pdf_export::{export_page_to_pdf, export_project_to_pdf_bytes};
    use crate::model::text_layout::layout_text_block;
    use crate::model::ImageCrop;

    #[test]
    fn test_all_seven_templates_and_magic_resize() {
        let templates = available_templates();
        assert_eq!(templates.len(), 7);
        for tpl in &templates {
            let proj = build_template_project(tpl.id);
            let (ew, eh) = tpl.format.dimensions();
            assert_eq!(proj.width, ew);
            assert_eq!(proj.height, eh);
            assert!(!proj.pages.is_empty());
            assert!(!proj.pages[0].elements.is_empty());
        }

        let mut proj = build_template_project("pitch_deck");
        assert_eq!(proj.width, 1920.0);
        assert_eq!(proj.height, 1080.0);
        assert_eq!(proj.pages.len(), 2);

        magic_resize_project(&mut proj, DesignFormat::InstagramPost);
        assert_eq!(proj.width, 1080.0);
        assert_eq!(proj.height, 1080.0);

        magic_resize_project(&mut proj, DesignFormat::Custom(1600, 900));
        assert_eq!(proj.width, 1600.0);
        assert_eq!(proj.height, 900.0);
    }

    #[test]
    fn test_typography_fonts_glyphs_and_wrapping() {
        let reg = FontRegistry::global();
        assert!(
            reg.load_warnings().is_empty(),
            "Bundled Noto TTF fonts must load with zero warnings: {:?}",
            reg.load_warnings()
        );

        // Distinct font metrics between Sans, Serif, and Mono
        let w_sans = reg.advance_width(FontFaceId::NotoSansRegular, 'M', 48.0);
        let w_serif = reg.advance_width(FontFaceId::NotoSerifRegular, 'M', 48.0);
        let w_mono = reg.advance_width(FontFaceId::NotoSansMonoRegular, 'i', 48.0);
        let w_sans_i = reg.advance_width(FontFaceId::NotoSansRegular, 'i', 48.0);
        assert!((w_sans - w_serif).abs() > 0.05 || (w_mono - w_sans_i).abs() > 2.0);

        // Mixed-case, ampersand, accented Latin, curly quotes, em dash, bullet, multiplication sign
        let rich_copy = "Chloé & José — “Editorial” • 5×7";
        let layout = layout_text_block(
            rich_copy,
            FontFamily::DisplaySerif,
            18.0,
            false,
            false,
            false,
            TextAlign::Left,
            0.0,
            1.25,
            ListStyle::None,
            420.0,
            80.0,
        );
        assert_eq!(layout.lines.len(), 1);
        assert_eq!(layout.lines[0].text, rich_copy);
        assert!(!layout.is_overflowing());

        // Word wrapping inside width-constrained box
        let paragraph = "This is a multi-word editorial newspaper column that must wrap cleanly across multiple lines inside a narrow column.";
        let wrapped = layout_text_block(
            paragraph,
            FontFamily::Sans,
            16.0,
            false,
            false,
            false,
            TextAlign::Left,
            0.0,
            1.3,
            ListStyle::None,
            180.0,
            40.0, // Intentionally short height to trigger overflow detection
        );
        assert!(wrapped.lines.len() >= 3);
        assert!(wrapped.overflows_height);
        assert!(wrapped.is_overflowing());
    }

    #[test]
    fn test_scene_consistency_rotation_transparent_stroke_and_image_mask() {
        let mut page = CanvasPage::new(1, "Consistency Test");
        page.background = PageBackground::Solid(rgb(255, 255, 255));

        // 1. Transparent fill with visible stroke must emit fill="none" in SVG and leave interior white in PNG
        page.elements.push(CanvasElement {
            id: 10,
            name: "Outline Box".to_string(),
            x: 40.0,
            y: 40.0,
            width: 120.0,
            height: 80.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Shape {
                shape_type: ShapeType::Rectangle,
                fill: FillStyle::Solid(rgba(0, 0, 0, 0)),
                stroke_color: rgb(220, 38, 38),
                stroke_width: 4.0,
                corner_radius: 0.0,
            },
        });

        let svg = export_page_to_svg(&page, 400.0, 300.0, false).unwrap();
        assert!(
            svg.contains("fill=\"none\""),
            "Transparent shape fill must emit fill=\"none\" in SVG"
        );
        let png = render_page_to_rgba_image(&page, 400.0, 300.0, 400, 300, false);
        // Center of outline box (100, 80) must remain white (255, 255, 255)
        let center_px = png.get_pixel(100, 80).0;
        assert!(center_px[0] > 240 && center_px[1] > 240 && center_px[2] > 240);
        // Top border of outline box (100, 41) must have red stroke
        let border_px = png.get_pixel(100, 41).0;
        assert!(border_px[0] > 180 && border_px[1] < 100);

        // 2. Rotated bar at 45 degrees must differ from 0 degrees in PNG
        let mut page_rot0 = CanvasPage::new(1, "Rot0");
        let mut bar = CanvasElement {
            id: 20,
            name: "Bar".to_string(),
            x: 50.0,
            y: 90.0,
            width: 100.0,
            height: 20.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Shape {
                shape_type: ShapeType::Rectangle,
                fill: FillStyle::Solid(rgb(20, 20, 20)),
                stroke_color: rgba(0, 0, 0, 0),
                stroke_width: 0.0,
                corner_radius: 0.0,
            },
        };
        page_rot0.elements.push(bar.clone());
        let mut page_rot45 = CanvasPage::new(1, "Rot45");
        bar.rotation = 45.0;
        page_rot45.elements.push(bar);

        let img0 = render_page_to_rgba_image(&page_rot0, 200.0, 200.0, 200, 200, false);
        let img45 = render_page_to_rgba_image(&page_rot45, 200.0, 200.0, 200, 200, false);
        // Point (125, 125) is along the 45-degree diagonal from center (100, 100), so dark in img45 and white in img0
        assert!(img0.get_pixel(125, 125).0[0] > 240);
        assert!(img45.get_pixel(125, 125).0[0] < 80);

        // 3. Circle-masked image must leave corners transparent/background in PNG
        let mut page_mask = CanvasPage::new(1, "Mask");
        let solid_blue = vec![0u8, 80u8, 220u8, 255u8].repeat(64 * 64);
        page_mask.elements.push(CanvasElement {
            id: 30,
            name: "Circle Photo".to_string(),
            x: 20.0,
            y: 20.0,
            width: 100.0,
            height: 100.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Image {
                source_name: "blue.png".to_string(),
                width_px: 64,
                height_px: 64,
                raw_rgba: solid_blue.into(),
                mask: ImageMask::Circle,
                adjustments: ImageAdjustments {
                    fit: ImageFit::Cover,
                    crop: ImageCrop {
                        left: 0.05,
                        top: 0.05,
                        right: 0.05,
                        bottom: 0.05,
                    },
                    ..ImageAdjustments::default()
                },
                border_color: rgba(0, 0, 0, 0),
                border_width: 0.0,
                revision: 1,
            },
        });
        let img_mask = render_page_to_rgba_image(&page_mask, 140.0, 140.0, 140, 140, true);
        // Corner of element bounding box (22, 22) is outside the circle -> alpha == 0
        assert_eq!(img_mask.get_pixel(22, 22).0[3], 0);
        // Center of circle (70, 70) is inside -> opaque blue
        assert_eq!(img_mask.get_pixel(70, 70).0[3], 255);
        assert!(img_mask.get_pixel(70, 70).0[2] > 180);
    }

    #[test]
    fn test_print_formats_calibration_square_and_pdf_export() {
        // Check physical dimensions & 300 DPI pixel counts for A4, US Letter, and 5x7 in
        assert_eq!(
            DesignFormat::DocumentA4.pixel_dimensions_at_dpi(300),
            (2480, 3508)
        );
        assert_eq!(
            DesignFormat::DocumentLetter.pixel_dimensions_at_dpi(300),
            (2550, 3300)
        );
        assert_eq!(
            DesignFormat::Stationery5x7.pixel_dimensions_at_dpi(300),
            (1500, 2100)
        );

        // 1-inch calibration square (96 x 96 logical units at 96 units/inch) -> 72 x 72 pt in PDF
        let mut proj = Project {
            schema_version: 2,
            document_id: crate::model::generate_document_id(),
            recovery_origin_path: None,
            title: "Calibration Test".to_string(),
            format: DesignFormat::DocumentLetter,
            width: 816.0,
            height: 1056.0,
            width_mm: 215.9,
            height_mm: 279.4,
            units_per_inch: 96.0,
            bleed_mm: 0.0,
            crop_marks: false,
            pages: vec![CanvasPage::new(1, "Page 1")],
            brand_kit: Default::default(),
            imported_fonts: Vec::new(),
            print_settings: Default::default(),
            production: Default::default(),
            next_id: 50,
        };
        let (pt_w, pt_h) = proj.pdf_points_dimensions();
        assert!((pt_w - 612.0).abs() < 0.05);
        assert!((pt_h - 792.0).abs() < 0.05);

        proj.pages[0].elements.push(CanvasElement {
            id: 1,
            name: "1-Inch Square".to_string(),
            x: 96.0,
            y: 96.0,
            width: 96.0,
            height: 96.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Shape {
                shape_type: ShapeType::Rectangle,
                fill: FillStyle::Solid(rgb(0, 0, 0)),
                stroke_color: rgba(0, 0, 0, 0),
                stroke_width: 0.0,
                corner_radius: 0.0,
            },
        });
        proj.pages[0].elements.push(CanvasElement {
            id: 2,
            name: "Unicode Headline".to_string(),
            x: 96.0,
            y: 220.0,
            width: 500.0,
            height: 60.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Text {
                content: "THE WEDDING TIMES • Chloé & José".to_string(),
                font_family: FontFamily::DisplaySerif,
                font_size: 24.0,
                color: rgb(20, 20, 20),
                bold: true,
                italic: false,
                underline: false,
                uppercase: false,
                align: TextAlign::Left,
                letter_spacing: 0.0,
                line_height: 1.2,
                effect: TextEffect::None,
                list_style: ListStyle::None,
            },
        });

        let pdf_bytes = export_project_to_pdf_bytes(&proj, &PdfExportOptions::default())
            .expect("PDF export should succeed");
        let pdf_str = String::from_utf8_lossy(&pdf_bytes);
        assert!(pdf_str.starts_with("%PDF-1.4"));
        assert!(pdf_str.contains("/MediaBox [0 0 612.000 792.000]"));
        assert!(pdf_str.contains("/FontFile2"));
        assert!(pdf_str.contains("/ToUnicode"));
        // Verify 1-inch square (at x=96, y=96 on 792pt-tall Letter page: y_top=792-72=720, y_bottom=792-144=648) measures 72.000 x 72.000 pt in PDF stream
        assert!(
            pdf_str.contains(
                "72.000 720.000 m 144.000 720.000 l 144.000 648.000 l 72.000 648.000 l h f"
            ),
            "1-inch calibration square (96x96 units) must emit a 72x72 pt rectangle in PDF"
        );

        let tmp_pdf = std::env::temp_dir().join("canvas_calibration_test.pdf");
        export_page_to_pdf(&proj, 0, &tmp_pdf).expect("export_page_to_pdf should succeed");
        assert!(tmp_pdf.exists());
        let _ = std::fs::remove_file(&tmp_pdf);
    }

    #[test]
    fn test_asset_deduplication_v1_migration_and_dehydrated_undo_memory() {
        let mut proj = Project::default();
        let big_rgba = vec![120u8; 400 * 400 * 4]; // 640 KB raw RGBA
        proj.pages[0].elements.push(CanvasElement {
            id: 10,
            name: "Photo".to_string(),
            x: 10.0,
            y: 10.0,
            width: 200.0,
            height: 200.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Image {
                source_name: "photo.png".to_string(),
                width_px: 400,
                height_px: 400,
                raw_rgba: big_rgba.into(),
                mask: ImageMask::None,
                adjustments: ImageAdjustments::default(),
                border_color: rgb(255, 255, 255),
                border_width: 0.0,
                revision: 1,
            },
        });
        proj.pages[0].elements.push(CanvasElement {
            id: 11,
            name: "Headline".to_string(),
            x: 10.0,
            y: 220.0,
            width: 300.0,
            height: 50.0,
            rotation: 0.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Text {
                content: "Initial".to_string(),
                font_family: FontFamily::Sans,
                font_size: 20.0,
                color: rgb(0, 0, 0),
                bold: false,
                italic: false,
                underline: false,
                uppercase: false,
                align: TextAlign::Left,
                letter_spacing: 0.0,
                line_height: 1.2,
                effect: TextEffect::None,
                list_style: ListStyle::None,
            },
        });

        // 1. Save project JSON: must store asset_ref instead of raw_rgba array (< 15 KB)
        let tmp_dir = std::env::temp_dir().join("canvas_v2_pkg_test");
        let _ = std::fs::create_dir_all(&tmp_dir);
        let save_path = tmp_dir.join("test.canvas.json");
        save_project_json(&proj, &save_path).expect("save v2 project");
        let json_bytes = std::fs::metadata(&save_path).unwrap().len();
        assert!(
            json_bytes < 15_000,
            "Saved .canvas.json should be compact (< 15 KB), got {} bytes",
            json_bytes
        );

        // 2. Dehydrated Undo Stack: 25 text edits must not duplicate the 640 KB image buffer
        let mut history = HistoryManager::default();
        for i in 0..25 {
            history.record(&proj);
            if let ElementKind::Text { content, .. } = &mut proj.pages[0].elements[1].kind {
                *content = format!("Edited headline step {i}");
            }
        }
        assert!(
            history.approximate_heap_bytes() < 500_000,
            "Dehydrated undo stack after 25 text edits must use < 500 KB for snapshots, got {} bytes",
            history.approximate_heap_bytes()
        );
        assert!(history.undo(&mut proj));
        // Image raw_rgba must still be hydrated after undo
        if let ElementKind::Image { raw_rgba, .. } = &proj.pages[0].elements[0].kind {
            assert_eq!(raw_rgba.len(), 400 * 400 * 4);
        } else {
            panic!("Expected Image element");
        }

        // 3. Migrate v1 raw-array JSON to v2
        let v2_path = tmp_dir.join("migrated_v2.canvas.json");
        migrate_v1_project_to_v2(&save_path, &v2_path).expect("migrate v1/v2");
        assert!(v2_path.exists());
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn test_state_reconciliation_and_grouping() {
        let mut app = CanvasStudioApp::new_headless_with_recovery_dir(Some(
            std::env::temp_dir().join(format!("canvas-state-test-{}", std::process::id())),
        ));
        app.current_page_idx = 1;
        app.selected_id = Some(21);
        app.record_mutation();
        app.canvas_state.inline_edit_id = Some(21);

        // When dirty, request_replace_document queues pending_replace_project instead of immediately discarding
        let social = build_template_project("social_launch");
        app.request_replace_document(social.clone(), None, "Loaded".to_string());
        assert!(app.pending_replace_project.is_some());

        // Replacing document must clear history, reset page index to 0, clear selection & inline edit
        app.replace_document(social, None);
        assert!(app.pending_replace_project.is_none());
        assert!(!app.history.can_undo());
        assert_eq!(app.current_page_idx, 0);
        assert_eq!(app.selected_id, None);
        assert_eq!(app.canvas_state.inline_edit_id, None);
        assert!(!app.dirty);

        // Multi-selection grouping & ungrouping
        let ids: Vec<u64> = app.project.pages[0]
            .elements
            .iter()
            .take(2)
            .map(|e| e.id)
            .collect();
        app.canvas_state.selected_ids = ids.clone();
        app.selected_id = ids.first().copied();
        app.group_selected_elements();
        let g0 = app.project.pages[0].elements[0].group_id;
        let g1 = app.project.pages[0].elements[1].group_id;
        assert!(g0.is_some() && g0 == g1);

        app.ungroup_selected_elements();
        assert!(app.project.pages[0].elements[0].group_id.is_none());
        assert!(app.project.pages[0].elements[1].group_id.is_none());
    }

    #[test]
    fn test_svg_png_html_and_json_export_pipeline() {
        let proj = build_template_project("social_launch");
        let page = &proj.pages[0];

        let svg_opaque = export_page_to_svg(page, proj.width, proj.height, false).unwrap();
        assert!(svg_opaque.contains("<svg"));
        assert!(svg_opaque.contains("FUTURE"));
        assert!(svg_opaque.contains("url(#page_bg_grad)"));

        let svg_trans = export_page_to_svg(page, proj.width, proj.height, true).unwrap();
        assert!(svg_trans.contains("<svg"));
        assert!(!svg_trans.contains("url(#page_bg_grad)"));

        let rgba_1x = render_page_to_rgba_image(page, proj.width, proj.height, 270, 270, false);
        assert_eq!(rgba_1x.dimensions(), (270, 270));

        let rgba_2x = render_page_to_rgba_image(page, proj.width, proj.height, 540, 540, true);
        assert_eq!(rgba_2x.dimensions(), (540, 540));
        assert_eq!(rgba_2x.get_pixel(0, 0).0[3], 0);

        let deck_proj = build_template_project("pitch_deck");
        let html = export_project_to_html_deck(&deck_proj).unwrap();
        assert!(html.contains("<!DOCTYPE html>"));
        assert!(html.contains("class=\"slide active\""));

        let tmp_path = std::env::temp_dir().join("canvas_audit_test.canvas.json");
        save_project_json(&deck_proj, &tmp_path).expect("save_project_json should succeed");
        let loaded = load_project_json(&tmp_path).expect("load_project_json should succeed");
        assert_eq!(loaded.title, deck_proj.title);
        assert_eq!(loaded.pages.len(), deck_proj.pages.len());
        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_magic_studio_image_and_copy_tools() {
        let assets = generate_builtin_stock_assets();
        assert_eq!(assets.len(), 4);
        let studio = &assets[0];

        let mut adj = ImageAdjustments::default();
        adj.bg_removed = true;
        let processed = apply_image_pipeline(&studio.rgba, studio.width, studio.height, &adj);
        assert_eq!(processed[3], 0);

        let (nw, nh, up) = upscale_image_2x(&studio.rgba, studio.width, studio.height);
        assert_eq!(nw, studio.width * 2);
        assert_eq!(nh, studio.height * 2);
        assert_eq!(up.len(), (nw * nh * 4) as usize);

        let mut erase_buf = studio.rgba.clone();
        magic_erase_patch(
            &mut erase_buf,
            studio.width,
            studio.height,
            0.35,
            0.35,
            0.3,
            0.3,
        );
        assert_eq!(erase_buf.len(), studio.rgba.len());

        let mut next_id = 500;
        let extracted = extract_magic_layers_from_image(
            &studio.rgba,
            studio.width,
            studio.height,
            100.0,
            100.0,
            320.0,
            320.0,
            &mut next_id,
        );
        assert!(extracted.len() >= 3);
        assert!(next_id > 500);

        for tone in MagicWriteTone::all() {
            let (headline, body) = magic_write_generate("Canvas Studio", *tone);
            assert!(!headline.is_empty());
            assert!(!body.is_empty());
            assert!(!body.contains("3.4x"));
            assert!(
                headline.to_uppercase().contains("CANVAS STUDIO") || body.contains("Canvas Studio")
            );
        }
    }

    #[test]
    fn test_history_manager_smart_snap_and_hit_testing() {
        let mut proj = Project::default();
        let mut history = HistoryManager::default();
        assert!(!history.can_undo());

        history.record(&proj);
        proj.title = "Updated Title".to_string();
        assert!(history.can_undo());

        assert!(history.undo(&mut proj));
        assert_eq!(proj.title, "Untitled Design");
        assert!(history.can_redo());

        assert!(history.redo(&mut proj));
        assert_eq!(proj.title, "Updated Title");

        let (sx, sy, guides) =
            compute_smart_snap(999, 856.0, 443.0, 200.0, 200.0, 1920.0, 1080.0, &[], 8.0);
        assert_eq!(sx, 860.0);
        assert_eq!(sy, 440.0);
        assert_eq!(guides.len(), 2);

        let el = CanvasElement {
            id: 1,
            name: "Rotated Box".to_string(),
            x: 100.0,
            y: 100.0,
            width: 200.0,
            height: 100.0,
            rotation: 45.0,
            opacity: 1.0,
            locked: false,
            visible: true,
            flip_h: false,
            flip_v: false,
            shadow: DropShadow::default(),
            animation: ElementAnimation::None,
            group_id: None,
            text_format: Default::default(),
            style_id: None,
            binding: None,
            kind: ElementKind::Shape {
                shape_type: ShapeType::Rectangle,
                fill: FillStyle::Solid(rgb(0, 196, 204)),
                stroke_color: rgb(255, 255, 255),
                stroke_width: 0.0,
                corner_radius: 0.0,
            },
        };
        assert!(el.contains_point(200.0, 150.0));
        assert!(!el.contains_point(10.0, 10.0));
    }
}

#[cfg(test)]
mod regression_tests;

#[cfg(test)]
mod standalone_tests;
