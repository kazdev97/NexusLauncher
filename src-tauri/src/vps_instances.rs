use crate::api;
use base64::Engine;
use serde::Serialize;
use std::path::{Component, Path, PathBuf};

#[derive(Serialize)]
pub struct VpsFileEntry {
    pub path: String,
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
                out.push(VpsFileEntry {
                    path: rel.to_string_lossy().replace('\\', "/"),
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
pub async fn vps_instance_list() -> Result<serde_json::Value, String> {
    let token = api::load_token().ok_or("No hay sesion del VPS")?;
    api::instances_list(&token).await
}

#[tauri::command]
pub async fn vps_instance_publish(
    folder: String,
    name: String,
    slug: String,
    description: String,
    files: Vec<String>,
) -> Result<serde_json::Value, String> {
    if files.is_empty() {
        return Err("Selecciona al menos un archivo".into());
    }
    let token = api::load_token().ok_or("No hay sesion del VPS")?;
    api::instance_upsert(&token, &name, &slug, &description).await?;

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
