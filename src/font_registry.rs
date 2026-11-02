use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};

use ab_glyph::{Font, FontArc, GlyphId};

use crate::model::FontFamily;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum FontFaceId {
    NotoSansRegular,
    NotoSansBold,
    NotoSansItalic,
    NotoSansBoldItalic,
    NotoSerifRegular,
    NotoSerifBold,
    NotoSerifItalic,
    NotoSerifBoldItalic,
    NotoSansMonoRegular,
    NotoSansMonoBold,
}

impl FontFaceId {
    pub fn all() -> &'static [FontFaceId] {
        &[
            FontFaceId::NotoSansRegular,
            FontFaceId::NotoSansBold,
            FontFaceId::NotoSansItalic,
            FontFaceId::NotoSansBoldItalic,
            FontFaceId::NotoSerifRegular,
            FontFaceId::NotoSerifBold,
            FontFaceId::NotoSerifItalic,
            FontFaceId::NotoSerifBoldItalic,
            FontFaceId::NotoSansMonoRegular,
            FontFaceId::NotoSansMonoBold,
        ]
    }

    fn embedded_bytes(&self) -> &'static [u8] {
        match self {
            Self::NotoSansRegular => include_bytes!("../assets/fonts/NotoSans-Regular.ttf"),
            Self::NotoSansBold => include_bytes!("../assets/fonts/NotoSans-Bold.ttf"),
            Self::NotoSansItalic => include_bytes!("../assets/fonts/NotoSans-Italic.ttf"),
            Self::NotoSansBoldItalic => include_bytes!("../assets/fonts/NotoSans-BoldItalic.ttf"),
            Self::NotoSerifRegular => include_bytes!("../assets/fonts/NotoSerif-Regular.ttf"),
            Self::NotoSerifBold => include_bytes!("../assets/fonts/NotoSerif-Bold.ttf"),
            Self::NotoSerifItalic => include_bytes!("../assets/fonts/NotoSerif-Italic.ttf"),
            Self::NotoSerifBoldItalic => include_bytes!("../assets/fonts/NotoSerif-BoldItalic.ttf"),
            Self::NotoSansMonoRegular => include_bytes!("../assets/fonts/NotoSansMono-Regular.ttf"),
            Self::NotoSansMonoBold => include_bytes!("../assets/fonts/NotoSansMono-Bold.ttf"),
        }
    }

    pub fn filename(&self) -> &'static str {
        match self {
            FontFaceId::NotoSansRegular => "NotoSans-Regular.ttf",
            FontFaceId::NotoSansBold => "NotoSans-Bold.ttf",
            FontFaceId::NotoSansItalic => "NotoSans-Italic.ttf",
            FontFaceId::NotoSansBoldItalic => "NotoSans-BoldItalic.ttf",
            FontFaceId::NotoSerifRegular => "NotoSerif-Regular.ttf",
            FontFaceId::NotoSerifBold => "NotoSerif-Bold.ttf",
            FontFaceId::NotoSerifItalic => "NotoSerif-Italic.ttf",
            FontFaceId::NotoSerifBoldItalic => "NotoSerif-BoldItalic.ttf",
            FontFaceId::NotoSansMonoRegular => "NotoSansMono-Regular.ttf",
            FontFaceId::NotoSansMonoBold => "NotoSansMono-Bold.ttf",
        }
    }

    pub fn postscript_name(&self) -> &'static str {
        match self {
            FontFaceId::NotoSansRegular => "NotoSans-Regular",
            FontFaceId::NotoSansBold => "NotoSans-Bold",
            FontFaceId::NotoSansItalic => "NotoSans-Italic",
            FontFaceId::NotoSansBoldItalic => "NotoSans-BoldItalic",
            FontFaceId::NotoSerifRegular => "NotoSerif-Regular",
            FontFaceId::NotoSerifBold => "NotoSerif-Bold",
            FontFaceId::NotoSerifItalic => "NotoSerif-Italic",
            FontFaceId::NotoSerifBoldItalic => "NotoSerif-BoldItalic",
            FontFaceId::NotoSansMonoRegular => "NotoSansMono-Regular",
            FontFaceId::NotoSansMonoBold => "NotoSansMono-Bold",
        }
    }

    pub fn egui_family_name(&self) -> &'static str {
        self.postscript_name()
    }

    pub fn svg_family_stack(&self) -> &'static str {
        match self {
            FontFaceId::NotoSansRegular
            | FontFaceId::NotoSansBold
            | FontFaceId::NotoSansItalic
            | FontFaceId::NotoSansBoldItalic => "'Noto Sans', 'DejaVu Sans', sans-serif",
            FontFaceId::NotoSerifRegular
            | FontFaceId::NotoSerifBold
            | FontFaceId::NotoSerifItalic
            | FontFaceId::NotoSerifBoldItalic => "'Noto Serif', 'DejaVu Serif', Georgia, serif",
            FontFaceId::NotoSansMonoRegular | FontFaceId::NotoSansMonoBold => {
                "'Noto Sans Mono', 'DejaVu Sans Mono', monospace"
            }
        }
    }

    pub fn is_bold(&self) -> bool {
        matches!(
            self,
            FontFaceId::NotoSansBold
                | FontFaceId::NotoSansBoldItalic
                | FontFaceId::NotoSerifBold
                | FontFaceId::NotoSerifBoldItalic
                | FontFaceId::NotoSansMonoBold
        )
    }

    pub fn is_italic(&self) -> bool {
        matches!(
            self,
            FontFaceId::NotoSansItalic
                | FontFaceId::NotoSansBoldItalic
                | FontFaceId::NotoSerifItalic
                | FontFaceId::NotoSerifBoldItalic
        )
    }

    pub fn is_serif(&self) -> bool {
        matches!(
            self,
            FontFaceId::NotoSerifRegular
                | FontFaceId::NotoSerifBold
                | FontFaceId::NotoSerifItalic
                | FontFaceId::NotoSerifBoldItalic
        )
    }

    pub fn is_monospace(&self) -> bool {
        matches!(
            self,
            FontFaceId::NotoSansMonoRegular | FontFaceId::NotoSansMonoBold
        )
    }

    pub fn resolve(family: FontFamily, bold: bool, italic: bool) -> Self {
        match family {
            FontFamily::Sans => match (bold, italic) {
                (false, false) => FontFaceId::NotoSansRegular,
                (true, false) => FontFaceId::NotoSansBold,
                (false, true) => FontFaceId::NotoSansItalic,
                (true, true) => FontFaceId::NotoSansBoldItalic,
            },
            FontFamily::PosterHeavy => {
                if italic {
                    FontFaceId::NotoSansBoldItalic
                } else {
                    FontFaceId::NotoSansBold
                }
            }
            FontFamily::DisplaySerif => match (bold, italic) {
                (false, false) => FontFaceId::NotoSerifRegular,
                (true, false) => FontFaceId::NotoSerifBold,
                (false, true) => FontFaceId::NotoSerifItalic,
                (true, true) => FontFaceId::NotoSerifBoldItalic,
            },
            FontFamily::EditorialItalic => {
                if bold {
                    FontFaceId::NotoSerifBoldItalic
                } else {
                    FontFaceId::NotoSerifItalic
                }
            }
            FontFamily::MonoCode => {
                if bold {
                    FontFaceId::NotoSansMonoBold
                } else {
                    FontFaceId::NotoSansMonoRegular
                }
            }
        }
    }
}

#[derive(Clone)]
pub struct LoadedFontFace {
    pub id: FontFaceId,
    pub bytes: Arc<Vec<u8>>,
    pub font: FontArc,
    pub source_path: PathBuf,
    pub used_fallback_dir: bool,
}

pub struct FontRegistry {
    faces: HashMap<FontFaceId, LoadedFontFace>,
    pub load_warnings: Vec<String>,
}

static GLOBAL_REGISTRY: OnceLock<FontRegistry> = OnceLock::new();

impl FontRegistry {
    pub fn global() -> &'static FontRegistry {
        GLOBAL_REGISTRY.get_or_init(FontRegistry::load_default)
    }

    pub fn load_default() -> Self {
        let mut faces = HashMap::new();
        let mut load_warnings = Vec::new();
        for &id in FontFaceId::all() {
            let bytes = id.embedded_bytes().to_vec();
            match FontArc::try_from_vec(bytes.clone()) {
                Ok(font) => {
                    faces.insert(
                        id,
                        LoadedFontFace {
                            id,
                            bytes: Arc::new(bytes),
                            font,
                            source_path: PathBuf::from(format!("embedded/{}", id.filename())),
                            used_fallback_dir: false,
                        },
                    );
                }
                Err(_) => load_warnings.push(format!("Invalid embedded font '{}'", id.filename())),
            }
        }
        Self {
            faces,
            load_warnings,
        }
    }

    pub fn get(&self, id: FontFaceId) -> Option<&LoadedFontFace> {
        self.faces
            .get(&id)
            .or_else(|| self.faces.get(&FontFaceId::NotoSansRegular))
    }

    pub fn resolve_face(
        &self,
        family: FontFamily,
        bold: bool,
        italic: bool,
    ) -> Option<&LoadedFontFace> {
        let id = FontFaceId::resolve(family, bold, italic);
        self.get(id)
    }

    /// Selects the primary face if it contains a valid glyph for `ch`, or falls back through
    /// Noto Sans Regular / Noto Serif Regular / Noto Sans Mono Regular if the primary face lacks the glyph.
    pub fn resolve_glyph_for_char(
        &self,
        primary_id: FontFaceId,
        ch: char,
    ) -> Option<(FontFaceId, &LoadedFontFace, GlyphId, bool)> {
        if let Some(primary) = self.faces.get(&primary_id) {
            let gid = primary.font.glyph_id(ch);
            if gid.0 != 0 {
                return Some((primary_id, primary, gid, false));
            }
        }
        for &fallback_id in &[
            FontFaceId::NotoSansRegular,
            FontFaceId::NotoSerifRegular,
            FontFaceId::NotoSansMonoRegular,
        ] {
            if let Some(fb) = self.faces.get(&fallback_id) {
                let gid = fb.font.glyph_id(ch);
                if gid.0 != 0 {
                    return Some((fallback_id, fb, gid, true));
                }
            }
        }
        self.faces.get(&primary_id).map(|primary| {
            let gid = primary.font.glyph_id(ch);
            (primary_id, primary, gid, true)
        })
    }

    pub fn is_healthy(&self) -> bool {
        self.load_warnings.is_empty() && self.faces.len() == FontFaceId::all().len()
    }

    pub fn load_warnings(&self) -> &[String] {
        &self.load_warnings
    }

    pub fn advance_width(&self, id: FontFaceId, ch: char, px_size: f32) -> f32 {
        use ab_glyph::{Font as _, ScaleFont as _};
        if let Some((_, face, gid, _)) = self.resolve_glyph_for_char(id, ch) {
            let scaled = face.font.as_scaled(px_size);
            scaled.h_advance(gid)
        } else {
            px_size * 0.5
        }
    }

    /// Returns the glyph advance width in PDF 1/1000 EM units (`hmtx` advance / `units_per_em` * 1000).
    pub fn pdf_glyph_width_1000(&self, id: FontFaceId, ch: char) -> i32 {
        use ab_glyph::Font as _;
        if let Some((_, face, gid, _)) = self.resolve_glyph_for_char(id, ch) {
            let upem = face.font.units_per_em().unwrap_or(1000.0).max(1.0);
            let h_unscaled = face.font.h_advance_unscaled(gid);
            ((h_unscaled * 1000.0 / upem).round() as i32).clamp(100, 2000)
        } else {
            500
        }
    }

    pub fn verify_font_directory(dir: &Path) -> Vec<String> {
        let mut missing = Vec::new();
        for &face_id in FontFaceId::all() {
            let p = dir.join(face_id.filename());
            if !p.exists() {
                missing.push(format!("Missing font file: {}", p.display()));
            }
        }
        missing
    }
}

/// Registers all 10 loaded Noto Sans, Noto Serif, and Noto Sans Mono faces into `egui::Context`
/// so the live canvas preview renders the exact same font files as PNG, SVG, and PDF exports.
pub fn register_egui_fonts(ctx: &egui::Context) {
    let registered_id = egui::Id::new("canvas-fonts-registered");
    if ctx.data_mut(|d| d.get_temp::<bool>(registered_id).unwrap_or(false)) {
        return;
    }
    let reg = FontRegistry::global();
    let mut defs = egui::FontDefinitions::default();

    for &face_id in FontFaceId::all() {
        if let Some(loaded) = reg.get(face_id) {
            let key = face_id.egui_family_name().to_string();
            defs.font_data.insert(
                key.clone(),
                egui::FontData::from_owned((*loaded.bytes).clone()).into(),
            );
            defs.families
                .entry(egui::FontFamily::Name(key.clone().into()))
                .or_default()
                .insert(0, key.clone());
        }
    }

    // Set NotoSans-Regular as default Proportional and NotoSansMono-Regular as default Monospace
    if defs
        .font_data
        .contains_key(FontFaceId::NotoSansRegular.egui_family_name())
    {
        defs.families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(
                0,
                FontFaceId::NotoSansRegular.egui_family_name().to_string(),
            );
    }
    if defs
        .font_data
        .contains_key(FontFaceId::NotoSansMonoRegular.egui_family_name())
    {
        defs.families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(
                0,
                FontFaceId::NotoSansMonoRegular
                    .egui_family_name()
                    .to_string(),
            );
    }

    ctx.set_fonts(defs);
    ctx.data_mut(|d| d.insert_temp(registered_id, true));
}
