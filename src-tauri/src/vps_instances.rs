use crate::api;
use base64::Engine;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

#[derive(Serialize)]
pub struct VpsFileEntry {
    pub path: String,
    pub size: u64,
}

#[tauri::command]
pub fn vps_publisher_scan(path: String) -> Result<Vec<VpsFileEntry>, String> {
    let root = PathBuf::from(&path);
    if !root.is_dir() {
        return Err("La carpeta no existe".into());
    }
    fn walk(root: &Path, dir: &Path, out: &mut Vec<VpsFileEntry>) -> Result<(), String> {
        for entry in std::fs::read_dir(dir).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let file_type = entry.file_type().map_err(|e| e.to_string())?;
            if file_type.is_symlink() {
                continue;
            }
            let p = entry.path();
            if file_type.is_dir() {
                let name = entry.file_name();
                if name == ".git" || name == ".nexuslauncher" {
                    continue;
                }
                walk(root, &p, out)?;
            } else if file_type.is_file() {
                let rel = p.strip_prefix(root).map_err(|e| e.to_string())?;
                let size = entry.metadata().map(|m| m.len()).unwrap_or(0);
                out.push(VpsFileEntry {
                    path: rel.to_string_lossy().replace('\\', "/"),
                    size,
                });
            }
        }
        Ok(())
    }
    let mut files = Vec::new();
    walk(&root, &root, &mut files)?;
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(files)
}

#[tauri::command]
pub async fn vps_instances_list() -> Result<serde_json::Value, String> {
    let token = api::load_token().ok_or("No hay sesion del VPS")?;
    api::instances_list(&token).await
}

#[tauri::command]
pub async fn vps_instance_publish(
    folder: String,
    name: String,
    slug: String,
    description: String,
    mc_version: String,
    loader: String,
    loader_version: String,
    files: Vec<String>,
) -> Result<serde_json::Value, String> {
    if files.is_empty() {
        return Err("Selecciona al menos un archivo".into());
    }
    let token = api::load_token().ok_or("No hay sesion del VPS")?;
    api::instance_upsert(
        &token,
        &name,
        &slug,
        &description,
        &mc_version,
        &loader,
        &loader_version,
    )
    .await?;

    let root = PathBuf::from(&folder);
    let mut payload = Vec::with_capacity(files.len());
    for rel in files {
        let rel_path = Path::new(&rel);
        if rel_path.is_absolute()
            || rel_path.components().any(|c| matches!(c, Component::ParentDir))
        {
            return Err(format!("Ruta relativa invalida: {}", rel));
        }
        let full = root.join(rel_path);
        let bytes = std::fs::read(&full).map_err(|e| format!("{}: {}", rel, e))?;
        payload.push(serde_json::json!({
            "path": rel.replace('\\', "/"),
            "content_base64": base64::engine::general_purpose::STANDARD.encode(bytes),
        }));
    }
    api::instance_publish(&token, &slug, payload).await
}

#[tauri::command]
pub async fn vps_instance_delete(slug: String) -> Result<(), String> {
    let token = api::load_token().ok_or("No hay sesion del VPS")?;
    api::instance_delete(&token, &slug).await
}

fn sha256_hex(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub async fn vps_check_dir(dir: &Path, slug: &str) -> Result<crate::sync::SyncDiff, String> {
    let token = api::load_token().ok_or("No hay sesion del VPS")?;
    let manifest = api::instance_manifest(&token, slug).await?;
    let version = manifest["version"].as_u64().unwrap_or(1).to_string();
    let files = manifest["files"].as_array().cloned().unwrap_or_default();

    let mut missing = Vec::new();
    let mut different = Vec::new();
    for f in files {
        let rel = f["path"].as_str().unwrap_or_default();
        if rel.is_empty() { continue; }
        let expected_sha = f["sha256"].as_str().unwrap_or_default();
        let target = dir.join(rel);
        if !target.exists() {
            missing.push(rel.to_string());
        } else if !expected_sha.is_empty() {
            if let Ok(bytes) = std::fs::read(&target) {
                if sha256_hex(&bytes) != expected_sha {
                    different.push(rel.to_string());
                }
            } else {
                different.push(rel.to_string());
            }
        }
    }
    let change_count = missing.len() + different.len();
    Ok(crate::sync::SyncDiff {
        revision: format!("v{version}"),
        up_to_date: change_count == 0,
        missing,
        different,
        extra: Vec::new(),
        change_count,
        has_archive: false,
    })
}

pub async fn vps_apply_dir(
    dir: &Path,
    slug: &str,
    on_status: &mut (dyn FnMut(String) + Send),
    on_progress: &mut (dyn FnMut(u32, String) + Send),
) -> Result<crate::sync::SyncResult, String> {
    let token = api::load_token().ok_or("No hay sesion del VPS")?;
    let manifest = api::instance_manifest(&token, slug).await?;
    let version = manifest["version"].as_u64().unwrap_or(1).to_string();
    let files = manifest["files"].as_array().cloned().unwrap_or_default();
    let total = files.len().max(1);

    let mut downloaded = Vec::new();
    let mut replaced = Vec::new();
    let mut errors = Vec::new();

    for (idx, f) in files.iter().enumerate() {
        let rel = f["path"].as_str().unwrap_or_default();
        if rel.is_empty() { continue; }
        let rel_path = Path::new(rel);
        if rel_path.is_absolute()
            || rel_path.components().any(|c| matches!(c, Component::ParentDir))
        {
            continue;
        }
        let expected_sha = f["sha256"].as_str().unwrap_or_default();
        let target = dir.join(rel_path);
        let existed = target.exists();
        let needs_dl = if !existed {
            true
        } else if !expected_sha.is_empty() {
            std::fs::read(&target).map(|b| sha256_hex(&b) != expected_sha).unwrap_or(true)
        } else {
            false
        };
        if !needs_dl { continue; }

        let pct = (((idx + 1) * 100) / total) as u32;
        on_progress(pct, format!("Descargando {rel}…"));
        on_status(format!("Descargando {rel}…"));
        match api::instance_download_file(&token, slug, rel).await {
            Ok(bytes) => {
                if let Some(parent) = target.parent() {
                    let _ = std::fs::create_dir_all(parent);
                }
                if let Err(e) = std::fs::write(&target, &bytes) {
                    errors.push(format!("{rel}: {e}"));
                } else if existed {
                    replaced.push(rel.to_string());
                } else {
                    downloaded.push(rel.to_string());
                }
            }
            Err(e) => errors.push(e),
        }
    }

    let mut meta = crate::instances::load_meta(dir);
    meta.server_version = format!("v{version}");
    let _ = crate::instances::save_meta(dir, &meta);

    on_progress(100, "Sincronización completada".to_string());
    Ok(crate::sync::SyncResult {
        applied_revision: format!("v{version}"),
        replaced,
        downloaded,
        removed: Vec::new(),
        backup_dir: String::new(),
        errors,
    })
}

pub async fn vps_install_instance(
    name: &str,
    game_version: &str,
    loader: &str,
    loader_version: &str,
    slug: &str,
    on_status: &mut (dyn FnMut(String) + Send),
    on_progress: &mut (dyn FnMut(u32, String) + Send),
) -> Result<(), String> {
    let dir = crate::instances::instance_path(name);
    std::fs::create_dir_all(&dir).map_err(|e| format!("crear carpeta de instancia: {e}"))?;
    let mut meta = crate::instances::load_meta(&dir);
    meta.name = name.to_string();
    meta.game_version = game_version.to_string();
    meta.loader = loader.to_string();
    meta.loader_version = loader_version.to_string();
    meta.manifest_url = format!("vps://{slug}");
    crate::instances::save_meta(&dir, &meta)?;

    on_status("Descargando archivos desde el VPS…".to_string());
    on_progress(10, "Descargando archivos desde el VPS…".to_string());
    let res = vps_apply_dir(&dir, slug, on_status, on_progress).await?;
    if !res.errors.is_empty() {
        return Err(res.errors.join("\n"));
    }
    on_status("Instalando Minecraft y loader…".to_string());
    on_progress(85, "Instalando Minecraft y loader…".to_string());
    let _ = crate::installer::install_instance(&dir, &mut |_, _| {}).await?;
    on_progress(100, "Instancia lista para jugar".to_string());
    Ok(())
}
