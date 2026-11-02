use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock};

use base64::{engine::general_purpose::STANDARD as BASE64_STD, Engine as _};
use serde::{
    de::{self, SeqAccess, Visitor},
    Deserializer, Serializer,
};

#[derive(Clone, Debug)]
pub struct StoredAsset {
    pub asset_id: String,
    pub extension: String,
    pub width_px: u32,
    pub height_px: u32,
    pub compressed_bytes: Arc<Vec<u8>>,
    pub decoded_rgba: Arc<Vec<u8>>,
    pub original_path: Option<PathBuf>,
}

#[derive(Default)]
pub struct AssetStoreInner {
    pub by_id: HashMap<String, StoredAsset>,
    pub path_to_id: HashMap<PathBuf, String>,
    pub active_package_dir: Option<PathBuf>,
}

pub struct AssetStore {
    inner: Mutex<AssetStoreInner>,
}

static GLOBAL_ASSET_STORE: OnceLock<AssetStore> = OnceLock::new();

thread_local! {
    static THREAD_ACTIVE_PACKAGE_DIR: std::cell::RefCell<Option<PathBuf>> = const { std::cell::RefCell::new(None) };
}

impl AssetStore {
    pub fn global() -> &'static AssetStore {
        GLOBAL_ASSET_STORE.get_or_init(|| AssetStore {
            inner: Mutex::new(AssetStoreInner::default()),
        })
    }

    pub fn set_active_package_dir(&self, dir: Option<PathBuf>) {
        THREAD_ACTIVE_PACKAGE_DIR.with(|cell| {
            *cell.borrow_mut() = dir.clone();
        });
        if let Ok(mut guard) = self.inner.lock() {
            guard.active_package_dir = dir;
        }
    }

    pub fn active_package_dir(&self) -> Option<PathBuf> {
        THREAD_ACTIVE_PACKAGE_DIR.with(|cell| cell.borrow().clone())
    }

    pub fn clear_memory_cache(&self) {
        if let Ok(mut guard) = self.inner.lock() {
            guard.by_id.clear();
            guard.path_to_id.clear();
        }
    }

    /// Computes a deterministic 128-bit content hash formatted as 32 hex chars.
    pub fn compute_hash(bytes: &[u8]) -> String {
        let mut h1: u64 = 0xcbf29ce484222325;
        let mut h2: u64 = 0x84222325cbf29ce4;
        for (i, &b) in bytes.iter().enumerate() {
            h1 ^= b as u64;
            h1 = h1.wrapping_mul(0x100000001b3);
            h2 ^= (b as u64).wrapping_add(i as u64);
            h2 = h2.wrapping_mul(0x100000001b3).rotate_left(5);
        }
        format!("{h1:016x}{h2:016x}")
    }

    /// Loads an image file from disk, preserving its exact original compressed bytes (PNG/JPEG/WebP)
    /// and caching the decoded RGBA buffer once by content hash.
    pub fn import_image_file(&self, path: &Path) -> Result<StoredAsset, String> {
        let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let compressed = std::fs::read(path)
            .map_err(|e| format!("Failed to read image file '{}': {e}", path.display()))?;
        let asset_id = Self::compute_hash(&compressed);

        if let Ok(guard) = self.inner.lock() {
            if let Some(existing) = guard.by_id.get(&asset_id) {
                return Ok(existing.clone());
            }
        }

        let dyn_img = image::load_from_memory(&compressed)
            .map_err(|e| format!("Failed to decode image '{}': {e}", path.display()))?;
        let rgba_img = dyn_img.to_rgba8();
        let (width_px, height_px) = rgba_img.dimensions();
        let decoded_rgba = Arc::new(rgba_img.into_raw());
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .unwrap_or("png")
            .to_ascii_lowercase();

        let stored = StoredAsset {
            asset_id: asset_id.clone(),
            extension: ext,
            width_px,
            height_px,
            compressed_bytes: Arc::new(compressed),
            decoded_rgba,
            original_path: Some(canonical.clone()),
        };

        if let Ok(mut guard) = self.inner.lock() {
            guard.path_to_id.insert(canonical, asset_id.clone());
            guard.by_id.insert(asset_id, stored.clone());
        }
        Ok(stored)
    }

    /// Registers a raw RGBA buffer into the content-addressed store, encoding it as PNG.
    pub fn register_rgba(&self, raw_rgba: &[u8], width_px: u32, height_px: u32) -> StoredAsset {
        let asset_id = Self::compute_hash(raw_rgba);
        let has_exact_dims = width_px > 0
            && height_px > 0
            && (width_px as usize) * (height_px as usize) * 4 == raw_rgba.len();

        if let Ok(guard) = self.inner.lock() {
            if let Some(existing) = guard.by_id.get(&asset_id) {
                if !has_exact_dims
                    || (existing.width_px == width_px && existing.height_px == height_px)
                {
                    return existing.clone();
                }
            }
        }

        let (w, h) = if has_exact_dims {
            (width_px, height_px)
        } else {
            let px_count = (raw_rgba.len() / 4).max(1) as u32;
            (px_count, 1)
        };

        let mut png_bytes: Vec<u8> = Vec::new();
        if let Some(img_buf) = image::RgbaImage::from_raw(w, h, raw_rgba.to_vec()) {
            let mut cursor = std::io::Cursor::new(&mut png_bytes);
            let _ = img_buf.write_to(&mut cursor, image::ImageFormat::Png);
        }

        let stored = StoredAsset {
            asset_id: asset_id.clone(),
            extension: "png".to_string(),
            width_px: w,
            height_px: h,
            compressed_bytes: Arc::new(png_bytes),
            decoded_rgba: Arc::new(raw_rgba.to_vec()),
            original_path: None,
        };

        if let Ok(mut guard) = self.inner.lock() {
            guard.by_id.insert(asset_id, stored.clone());
        }
        stored
    }

    pub fn get(&self, asset_id: &str) -> Option<StoredAsset> {
        self.inner
            .lock()
            .ok()
            .and_then(|g| g.by_id.get(asset_id).cloned())
    }

    pub fn unique_asset_count(&self) -> usize {
        self.inner.lock().map(|g| g.by_id.len()).unwrap_or(0)
    }

    /// Persists an asset into `<package_dir>/assets/<asset_id>.<ext>` and verifies it on disk.
    pub fn write_asset_to_dir(
        &self,
        asset: &StoredAsset,
        package_dir: &Path,
    ) -> Result<PathBuf, String> {
        if asset.compressed_bytes.is_empty() {
            return Err(format!(
                "Cannot write image asset '{}': encoded image buffer is empty",
                asset.asset_id
            ));
        }
        let assets_dir = package_dir.join("assets");
        if assets_dir.exists() && !assets_dir.is_dir() {
            return Err(format!(
                "Cannot create assets directory '{}': a non-directory file exists at that path",
                assets_dir.display()
            ));
        }
        std::fs::create_dir_all(&assets_dir).map_err(|e| {
            format!(
                "Failed to create assets dir '{}': {e}",
                assets_dir.display()
            )
        })?;
        let file_path = assets_dir.join(format!("{}.{}", asset.asset_id, asset.extension));

        let needs_write = std::fs::read(&file_path)
            .ok()
            .and_then(|bytes| image::load_from_memory(&bytes).ok())
            .map(|img| {
                img.width() != asset.width_px
                    || img.height() != asset.height_px
                    || img.to_rgba8().as_raw() != asset.decoded_rgba.as_ref()
            })
            .unwrap_or(true);

        if needs_write {
            let tmp_path = assets_dir.join(format!(
                ".{}.{}.{}.tmp",
                asset.asset_id,
                asset.extension,
                std::process::id()
            ));
            use std::io::Write;
            std::fs::File::create(&tmp_path)
                .and_then(|mut f| {
                    f.write_all(asset.compressed_bytes.as_slice())?;
                    f.sync_all()
                })
                .map_err(|e| {
                    let _ = std::fs::remove_file(&tmp_path);
                    format!(
                        "Failed to write temporary asset '{}': {e}",
                        tmp_path.display()
                    )
                })?;
            std::fs::rename(&tmp_path, &file_path).map_err(|e| {
                let _ = std::fs::remove_file(&tmp_path);
                format!(
                    "Failed to commit asset '{}' to '{}': {e}",
                    tmp_path.display(),
                    file_path.display()
                )
            })?;
        }

        let meta = std::fs::metadata(&file_path).map_err(|e| {
            format!(
                "Failed to verify written asset '{}' on disk: {e}",
                file_path.display()
            )
        })?;
        if !meta.is_file() || meta.len() == 0 {
            return Err(format!(
                "Written asset '{}' is empty or not a regular file",
                file_path.display()
            ));
        }
        Ok(file_path)
    }

    /// Strictly loads and validates an asset by ID from `<package_dir>/assets/<asset_id>.*` on disk.
    pub fn load_asset_from_dir_strict(
        &self,
        asset_id: &str,
        package_dir: &Path,
    ) -> Result<StoredAsset, String> {
        let assets_dir = package_dir.join("assets");
        for ext in ["png", "jpg", "jpeg", "webp"] {
            let candidate = assets_dir.join(format!("{asset_id}.{ext}"));
            if candidate.exists() {
                let bytes = std::fs::read(&candidate).map_err(|e| {
                    format!(
                        "Failed to read required asset '{}' at '{}': {e}",
                        asset_id,
                        candidate.display()
                    )
                })?;
                if bytes.is_empty() {
                    return Err(format!(
                        "Required asset '{}' at '{}' is empty (0 bytes)",
                        asset_id,
                        candidate.display()
                    ));
                }
                let dyn_img = image::load_from_memory(&bytes).map_err(|e| {
                    format!(
                        "Required asset '{}' at '{}' is corrupt or unreadable: {e}",
                        asset_id,
                        candidate.display()
                    )
                })?;
                let rgba = dyn_img.to_rgba8();
                let (w, h) = rgba.dimensions();
                let raw = rgba.into_raw();
                if w == 0 || h == 0 || raw.is_empty() {
                    return Err(format!(
                        "Required asset '{}' at '{}' decoded to empty image ({w}x{h})",
                        asset_id,
                        candidate.display()
                    ));
                }
                if Self::compute_hash(&raw) != asset_id {
                    return Err(format!(
                        "Required image asset '{asset_id}' has changed pixel content: {}",
                        candidate.display()
                    ));
                }
                let stored = StoredAsset {
                    asset_id: asset_id.to_string(),
                    extension: ext.to_string(),
                    width_px: w,
                    height_px: h,
                    compressed_bytes: Arc::new(bytes),
                    decoded_rgba: Arc::new(raw),
                    original_path: Some(candidate),
                };
                if let Ok(mut guard) = self.inner.lock() {
                    guard.by_id.insert(asset_id.to_string(), stored.clone());
                }
                return Ok(stored);
            }
        }
        Err(format!(
            "Required image asset '{asset_id}' is missing in '{}'",
            assets_dir.display()
        ))
    }

    /// Loads an asset by ID from `<package_dir>/assets/<asset_id>.*` if available.
    pub fn load_asset_from_dir(&self, asset_id: &str, package_dir: &Path) -> Option<StoredAsset> {
        self.load_asset_from_dir_strict(asset_id, package_dir).ok()
    }
}

/// Custom serde serializer/deserializer for `ElementKind::Image::raw_rgba` that:
/// 1. Deduplicates raw RGBA pixels into `AssetStore` by content hash and writes compressed PNGs
///    into `<package_dir>/assets/<asset_id>.png` instead of pretty-printing millions of JSON numbers.
/// 2. Transparently reads both v2 `"asset://<asset_id>"` references AND legacy v1 `[u8, ...]` arrays,
///    strictly failing when a required asset cannot be written or loaded.
pub mod serde_rgba_asset {
    use super::*;

    pub fn serialize<S>(raw_rgba: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        if raw_rgba.is_empty() {
            return Err(serde::ser::Error::custom(
                "Cannot serialize image element with empty RGBA buffer",
            ));
        }
        let store = AssetStore::global();
        let asset = store.register_rgba(raw_rgba, 0, 0);

        if let Some(pkg_dir) = store.active_package_dir() {
            let path = pkg_dir
                .join("assets")
                .join(format!("{}.{}", asset.asset_id, asset.extension));
            if !path.is_file() {
                return Err(serde::ser::Error::custom(format!(
                    "Asset '{}' was not staged",
                    asset.asset_id
                )));
            }
            return serializer.serialize_str(&format!("asset://{}", asset.asset_id));
        }

        // If serialized in-memory without an active package directory (e.g. serde_json::to_value),
        // emit the compact deterministic asset URI; for tiny synthetic buffers (<= 64 bytes), append base64.
        if raw_rgba.len() <= 64 {
            let b64 = BASE64_STD.encode(raw_rgba);
            serializer.serialize_str(&format!("asset://{}:raw:{}", asset.asset_id, b64))
        } else {
            serializer.serialize_str(&format!("asset://{}", asset.asset_id))
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RgbaAssetVisitor;

        impl<'de> Visitor<'de> for RgbaAssetVisitor {
            type Value = Vec<u8>;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an asset:// reference string or a legacy v1 byte array")
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                if v == "asset://empty" || v.is_empty() {
                    return Err(de::Error::custom(
                        "Image element references empty asset ('asset://empty')",
                    ));
                }
                if let Some(rest) = v.strip_prefix("asset://") {
                    let parts: Vec<&str> = rest.split(':').collect();
                    let asset_id = parts[0];
                    if asset_id.is_empty() {
                        return Err(de::Error::custom("Image element has empty asset ID"));
                    }
                    let store = AssetStore::global();

                    // When loading from a package directory on disk, strictly validate the on-disk asset file
                    // so missing or corrupt files on disk are never masked by an in-memory cache entry.
                    if let Some(pkg_dir) = store.active_package_dir() {
                        let asset = store
                            .load_asset_from_dir_strict(asset_id, &pkg_dir)
                            .map_err(de::Error::custom)?;
                        return Ok((*asset.decoded_rgba).clone());
                    }

                    if let Some(asset) = store.get(asset_id) {
                        return Ok((*asset.decoded_rgba).clone());
                    }
                    if parts.len() >= 3 && parts[1] == "raw" {
                        let bytes = BASE64_STD.decode(parts[2]).map_err(|e| {
                            de::Error::custom(format!(
                                "Failed to decode inline raw asset '{asset_id}': {e}"
                            ))
                        })?;
                        if bytes.is_empty() {
                            return Err(de::Error::custom(format!(
                                "Inline raw asset '{asset_id}' decoded to 0 bytes"
                            )));
                        }
                        return Ok(bytes);
                    }
                    return Err(de::Error::custom(format!(
                        "Required image asset '{asset_id}' is missing from asset store"
                    )));
                }
                // Fallback base64 string
                let decoded = BASE64_STD.decode(v).map_err(de::Error::custom)?;
                if decoded.is_empty() {
                    return Err(de::Error::custom("Decoded base64 image asset is empty"));
                }
                Ok(decoded)
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut bytes = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(b) = seq.next_element::<u8>()? {
                    bytes.push(b);
                }
                if bytes.is_empty() {
                    return Err(de::Error::custom(
                        "Legacy v1 image byte array is empty (0 bytes)",
                    ));
                }
                let _ = AssetStore::global().register_rgba(&bytes, 0, 0);
                Ok(bytes)
            }
        }

        deserializer.deserialize_any(RgbaAssetVisitor)
    }
}
