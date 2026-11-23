
use crate::config::instances_dir;
use crate::downloader::{download_file, download_many, sha256_file, http_client, DlJob, MAX_CONCURRENT_DOWNLOADS};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub const MANIFEST_FORMAT: &str = "pau-manifest";

/// Acepta strings, numeros o null y los normaliza a String.
fn de_string_or_num<'de, D>(d: D) -> Result<String, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;
    let v = serde_json::Value::deserialize(d)?;
    Ok(match v {
        serde_json::Value::String(s) => s,
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Null => String::new(),
        other => other.to_string(),
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceInstance {
    #[serde(default)]
    pub id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub loader: String,
    #[serde(default)]
    pub loader_version: String,
    #[serde(default)]
    pub manifest_url: String,
    #[serde(default, deserialize_with = "de_string_or_num")]
    pub revision: String,
    #[serde(default)]
    pub installed: bool,
    #[serde(default)]
    pub github_repo: String,
    #[serde(default)]
    pub github_path: String,
    #[serde(default)]
    pub github_branch: String,
}
pub fn default_protected() -> Vec<String> {
    vec![
        "saves".to_string(),
        "logs".to_string(),
        "versions".to_string(),
        "libraries".to_string(),
        "assets".to_string(),
        "backups".to_string(),
        "screenshots".to_string(),
        "kazu_instance.json".to_string(),
        "imgui.ini".to_string(),
        "options.txt".to_string(),
    ]
}
pub fn default_managed() -> Vec<String> {
    vec!["mods".to_string(), "config".to_string(), "resourcepacks".to_string()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    #[serde(default)]
    pub sha256: String,
    #[serde(default)]
    pub size: u64,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default)]
    pub format: String,
    #[serde(default, deserialize_with = "de_string_or_num")]
    pub revision: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub game_version: String,
    #[serde(default)]
    pub loader: String,
    #[serde(default)]
    pub loader_version: String,
    #[serde(default)]
    pub server_version: String,
    #[serde(default)]
    pub archive_url: String,
    #[serde(default)]
    pub ram_min_mb: u32,
    #[serde(default)]
    pub ram_max_mb: u32,
    #[serde(default)]
    pub files: Vec<ManifestFile>,
    #[serde(default)]
    pub protected: Vec<String>,
    #[serde(default)]
    pub managed: Vec<String>,
    #[serde(default)]
    pub default_resourcepacks: Vec<String>,
    #[serde(default)]
    pub banned: Vec<String>,
    /// Archivos que el juego reescribe en caliente (estado de FancyMenu, etc.).
    /// Se actualizan igual que el resto, pero no cuentan como "pendiente" en
    /// el aviso de actualizaciones, porque su hash siempre cambia al jugar.
    #[serde(default)]
    pub volatile: Vec<String>,
    /// Si es true, el launcher borra TODA la carpeta de instancia antes de
    /// aplicar el pack (borrado completo: saves, mods, config, todo).
    #[serde(default)]
    pub wipe: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncDiff {
    pub revision: String,
    pub up_to_date: bool,
    pub missing: Vec<String>,
    pub different: Vec<String>,
    pub extra: Vec<String>,
    pub change_count: usize,
    pub has_archive: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SyncResult {
    pub applied_revision: String,
    pub replaced: Vec<String>,
    pub downloaded: Vec<String>,
    pub removed: Vec<String>,
    pub backup_dir: String,
    pub errors: Vec<String>,
}
fn resolve_download_url(url: &str) -> String {
    let url = url.trim();
    if url.is_empty() {
        return url.to_string();
    }
    let parsed = url::Url::parse(url).ok();
    let host = parsed.as_ref().and_then(|p| p.host_str()).unwrap_or("").to_lowercase();
    if host.contains("clarodrive.com") {
        resolve_clarodrive(url)
    } else if host.contains("github.com") {
        resolve_github(url)
    } else if host.contains("drive.google.com") || host.contains("docs.google.com") {
        resolve_google_drive(url)
    } else {
        url.to_string()
    }
}

fn resolve_clarodrive(url: &str) -> String {
    let path = url.split('?').next().unwrap_or(url).to_lowercase();
    if path.ends_with(".zip") || path.ends_with(".jar") || path.ends_with(".txt")
        || path.ends_with(".json") || path.ends_with(".rar") || path.ends_with(".7z")
    {
        url.to_string()
    } else if url.contains("/download") {
        let base = url.split("/download").next().unwrap_or(url);
        format!("{}/download", base.trim_end_matches('/'))
    } else {
        format!("{}/download", url.trim_end_matches('/'))
    }
}

fn resolve_github(url: &str) -> String {
    let re = regex::Regex::new(r"https?://github\.com/([^/]+)/([^/]+)/blob/([^/]+)/(.+)")
        .unwrap();
    match re.captures(url) {
        Some(c) => format!(
            "https://raw.githubusercontent.com/{}/{}/{}/{}",
            &c[1], &c[2], &c[3], &c[4]
        ),
        None => url.to_string(),
    }
}

fn resolve_google_drive(url: &str) -> String {
    let parsed = url::Url::parse(url).ok();
    let mut file_id = String::new();
    if let Some(m) = regex::Regex::new(r"/file/d/([a-zA-Z0-9_-]+)")
        .unwrap()
        .captures(url)
    {
        file_id = m[1].to_string();
    }
    if file_id.is_empty() {
        if let Some(parsed) = &parsed {
            for (k, v) in parsed.query_pairs() {
                if k == "id" {
                    file_id = v.to_string();
                    break;
                }
            }
        }
    }
    if !file_id.is_empty() && !url.contains("uc?export=download") {
        format!("https://drive.google.com/uc?export=download&id={file_id}")
    } else {
        url.to_string()
    }
}
pub async fn fetch_manifest(url_or_path: &str) -> Result<Manifest, String> {
    let candidate_path = Path::new(url_or_path);
    let raw = if candidate_path.is_file() {
        fs::read_to_string(candidate_path).map_err(|e| format!("leer manifest: {e}"))?
    } else {
        let client = http_client();
        let resolved = resolve_download_url(url_or_path);
        if resolved.to_lowercase().ends_with(".zip") {
            return Err(format!(
                "El manifest '{}' es un archivo. Instala primero la instancia y luego sincroniza.",
                url_or_path
            ));
        }
        let mut last_err = String::new();
        for attempt in 1..=3 {
            match crate::downloader::get_text(&client, &resolved, Duration::from_secs(30)).await {
                Ok(text) => return parse_manifest(&text),
                Err(e) => {
                    last_err = e;
                    if attempt < 3 {
                        tokio::time::sleep(Duration::from_millis(800 * attempt as u64)).await;
                    }
                }
            }
        }
        return Err(format!("no se pudo descargar la informacion del pack: {last_err}"));
    };
    parse_manifest(&raw)
}

pub fn parse_manifest(raw: &str) -> Result<Manifest, String> {
    let cleaned = raw.trim().trim_start_matches('\u{FEFF}');
    let manifest: Manifest =
        serde_json::from_str(cleaned).map_err(|e| format!("manifest JSON inválido: {e}"))?;
    if !manifest.format.is_empty() && manifest.format != MANIFEST_FORMAT {
        return Err(format!("Formato de manifest no soportado: {}", manifest.format));
    }
    Ok(manifest)
}
fn extract_json_block(text: &str) -> String {
    let text = text.trim();
    for (opener, closer) in [('{', '}'), ('[', ']')] {
        if let Some(start) = text.find(opener) {
            let mut depth = 0i32;
            let mut in_string = false;
            let mut escape = false;
            for (i, ch) in text[start..].char_indices() {
                if in_string {
                    if escape {
                        escape = false;
                    } else if ch == '\\' {
                        escape = true;
                    } else if ch == '"' {
                        in_string = false;
                    }
                    continue;
                }
                if ch == '"' {
                    in_string = true;
                } else if ch == opener {
                    depth += 1;
                } else if ch == closer {
                    depth -= 1;
                    if depth == 0 {
                        return text[start..start + i + ch.len_utf8()].to_string();
                    }
                }
            }
        }
    }
    text.to_string()
}
pub fn parse_source_index(raw: &str) -> Result<Vec<SourceInstance>, String> {
    let value: serde_json::Value = match serde_json::from_str(raw.trim()) {
        Ok(v) => v,
        Err(_) => {
            let block = extract_json_block(raw);
            serde_json::from_str(&block).map_err(|e| format!("índice JSON inválido: {e}"))?
        }
    };
    let items: Vec<serde_json::Value> = match &value {
        serde_json::Value::Array(arr) => arr.clone(),
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Array(arr)) =
                map.get("instances").or_else(|| map.get("modpacks"))
            {
                arr.clone()
            } else {
                vec![value.clone()]
            }
        }
        _ => return Err("El índice debe ser un objeto o una lista".to_string()),
    };
    let mut out = Vec::new();
    for item in items {
        match serde_json::from_value::<SourceInstance>(item) {
            Ok(s) => out.push(s),
            Err(e) => return Err(format!("Entrada del índice inválida: {e}")),
        }
    }
    Ok(out)
}
pub async fn fetch_source_index(url: &str) -> Result<Vec<SourceInstance>, String> {
    let client = http_client();
    let resolved = resolve_download_url(url);
    let mut last_err = String::new();
    for attempt in 1..=3 {
        match crate::downloader::get_text(&client, &resolved, Duration::from_secs(30)).await {
            Ok(text) => return parse_source_index(&text),
            Err(e) => {
                last_err = e;
                if attempt < 3 {
                    tokio::time::sleep(Duration::from_millis(800 * attempt as u64)).await;
                }
            }
        }
    }
    Err(format!("no se pudo descargar la lista de instancias: {last_err}"))
}
#[allow(clippy::too_many_arguments)]
pub async fn install_remote_instance(
    name: &str,
    game_version: &str,
    loader: &str,
    loader_version: &str,
    manifest_url: &str,
    github_repo: &str,
    github_path: &str,
    github_branch: &str,
    on_status: &mut (dyn FnMut(String) + Send),
    on_progress: &mut (dyn FnMut(u32, String) + Send),
) -> Result<(), String> {
    let req = crate::instances::CreateInstanceRequest {
        name: name.to_string(),
        game_version: game_version.to_string(),
        loader: loader.to_string(),
        loader_version: loader_version.to_string(),
        manifest_url: manifest_url.to_string(),
    };
    let clean = req.sanitized_name();
    let dir = crate::instances::instance_path(&clean);
    if dir.exists() {
        let finished = dir
            .join("versions")
            .join(&req.game_version)
            .join(format!("{}.json", req.game_version))
            .exists();
        if finished {
            return Err(format!("Ya tienes instalada una instancia llamada '{clean}'"));
        }
        let _ = fs::remove_dir_all(&dir);
    }
    let dir = crate::instances::create_instance_dir(&req)?;
    let meta = crate::instances::InstanceMeta {
        name: clean.clone(),
        game_version: req.game_version.clone(),
        loader: req.loader.clone(),
        loader_version: req.loader_version.clone(),
        manifest_url: req.manifest_url.clone(),
        github_repo: github_repo.trim().to_string(),
        github_path: github_path.trim().to_string(),
        github_branch: github_branch.trim().to_string(),
        ..Default::default()
    };
    if let Err(e) = crate::instances::save_meta(&dir, &meta) {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    on_status(format!("Instalando '{clean}'..."));
    on_progress(1, format!("Instalando '{clean}'..."));
    let mut pipe = |pct: u32, msg: String| {
        on_status(msg.clone());
        on_progress(pct, msg);
    };
    if let Err(e) = crate::installer::install_instance(&dir, &mut pipe).await {
        let _ = fs::remove_dir_all(&dir);
        return Err(e);
    }
    if !github_repo.trim().is_empty() {
        on_status("Descargando el pack desde la repo (primera vez)...".to_string());
        on_progress(85, "Descargando el pack desde la repo...".to_string());
        if let Err(e) = git_apply_dir(&dir, on_status, on_progress).await {
            let _ = fs::remove_dir_all(&dir);
            return Err(e);
        }
    } else if !manifest_url.trim().is_empty() {
        on_status(format!("Aplicando el paquete del owner..."));
        if let Err(e) = apply_updates(&dir, manifest_url, on_status).await {
            let _ = fs::remove_dir_all(&dir);
            return Err(e);
        }
    }
    Ok(())
}
pub fn cache_manifest(instance_dir: &Path, manifest: &Manifest) {
    if let Ok(json) = serde_json::to_string_pretty(manifest) {
        fs::write(instance_dir.join("manifest_cache.json"), json).ok();
    }
}

pub fn load_manifest_cache(instance_dir: &Path) -> Option<Manifest> {
    let path = instance_dir.join("manifest_cache.json");
    let text = fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}
fn managed_folders(manifest: &Manifest) -> Vec<String> {
    if !manifest.managed.is_empty() {
        manifest.managed.clone()
    } else {
        default_managed()
    }
}

fn protected_paths(manifest: &Manifest) -> HashSet<String> {
    let list = if !manifest.protected.is_empty() {
        manifest.protected.clone()
    } else {
        default_protected()
    };
    list.into_iter().collect()
}

fn is_managed(path: &str, managed: &[String]) -> bool {
    let top = path.split('/').next().unwrap_or("");
    managed.iter().any(|f| f == top)
}

fn is_banned(path: &str, banned: &[String]) -> bool {
    if banned.is_empty() {
        return false;
    }
    banned.iter().any(|b| {
        path == b
            || path.starts_with(&format!("{b}/"))
            || path.strip_suffix(".disabled").map_or(false, |p| p == b || p.starts_with(&format!("{b}/")))
    })
}

fn volatile_paths(manifest: &Manifest) -> Vec<String> {
    manifest.volatile.clone()
}

fn is_volatile(path: &str, volatile: &[String]) -> bool {
    if volatile.is_empty() {
        return false;
    }
    volatile
        .iter()
        .any(|v| path == v || path.starts_with(&format!("{v}/")))
}
fn list_local_managed(instance_dir: &Path, managed: &[String]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for folder in managed {
        let dir = instance_dir.join(folder);
        if !dir.exists() {
            continue;
        }
        let mut stack = vec![dir.clone()];
        while let Some(current) = stack.pop() {
            if let Ok(entries) = fs::read_dir(&current) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        let top = p.strip_prefix(&dir).unwrap_or(&p);
                        if top.iter().count() > 12 {
                            continue;
                        }
                        stack.push(p);
                    } else {
                        if let Ok(rel) = p.strip_prefix(&instance_dir) {
                            let rel_str = rel.to_string_lossy().replace('\\', "/");
                            let Ok(hash) = sha256_file(&p) else { continue };
                            out.push((rel_str, hash));
                        }
                    }
                }
            }
        }
    }
    out
}
pub async fn check_updates(instance_dir: &Path, manifest_url: &str) -> Result<SyncDiff, String> {
    let manifest = fetch_manifest(manifest_url).await?;
    let previous = load_manifest_cache(instance_dir);
    let diff = compute_diff(instance_dir, &manifest, previous.as_ref());
    cache_manifest(instance_dir, &manifest);
    Ok(diff)
}

pub fn compute_diff(instance_dir: &Path, manifest: &Manifest, previous: Option<&Manifest>) -> SyncDiff {
    let local_meta = crate::instances::load_meta(instance_dir);
    let local_revision = local_meta.server_version;
    let managed = managed_folders(manifest);
    let protected = protected_paths(manifest);
    let previous_paths: HashSet<String> = previous
        .map(|p| p.files.iter().map(|f| f.path.clone()).collect())
        .unwrap_or_default();
    let mut remote: std::collections::BTreeMap<String, String> = Default::default();

    let local = list_local_managed(instance_dir, &managed);
    let mut local_map: std::collections::BTreeMap<String, String> = Default::default();
    for (path, hash) in local {
        local_map.insert(path, hash);
    }

    for f in &manifest.files {
        if !is_managed(&f.path, &managed) || protected.contains(&f.path) {
            continue;
        }
        remote.insert(f.path.clone(), f.sha256.clone());
    }

    let volatile = volatile_paths(manifest);
    let mut missing = Vec::new();
    let mut different = Vec::new();
    for (path, hash) in &remote {
        match local_map.get(path) {
            None => missing.push(path.clone()),
            Some(local_hash) => {
                if !hash.is_empty() && local_hash != hash && !is_volatile(path, &volatile) {
                    different.push(path.clone());
                }
            }
        }
    }

    let mut extra = Vec::new();
    for path in local_map.keys() {
        if remote.contains_key(path) {
            continue;
        }
        if previous_paths.contains(path) || is_banned(&path, &manifest.banned) {
            extra.push(path.clone());
        }
    }
    missing.sort();
    different.sort();
    extra.sort();

    let change_count = missing.len() + different.len() + extra.len();
    let up_to_date = {
        let revision_changed = !local_revision.is_empty() && local_revision == manifest.revision
            || local_revision.is_empty() && manifest.revision.is_empty();
        let same = if manifest.revision.is_empty() {
            missing.is_empty() && different.is_empty() && extra.is_empty()
        } else {
            local_revision == manifest.revision
        };
        same && change_count == 0 && revision_changed
    };

    SyncDiff {
        revision: manifest.revision.clone(),
        up_to_date,
        missing,
        different,
        extra,
        change_count,
        has_archive: !manifest.archive_url.is_empty(),
    }
}

async fn download_archive(
    manifest: &Manifest,
    instance_dir: &Path,
    on_status: &mut (dyn FnMut(String) + Send),
) -> Result<PathBuf, String> {
    let client = http_client();
    let dest = instance_dir.join(".nexuslauncher_update.zip");
    let resolved = resolve_download_url(&manifest.archive_url);
    on_status(format!(
        "Descargando actualización ({} archivos)...",
        manifest.files.len()
    ));
    download_file(&client, &resolved, &dest, &mut |read, total| {
        if total > 0 {
            on_status(format!("Descargando actualización... {}%", read * 100 / total));
        }
    })
    .await?;
    Ok(dest)
}

fn extract_zip_stripping(archive: &Path, target_root: &Path, strip_prefix: &str) -> Result<(), String> {
    fs::create_dir_all(target_root).map_err(|e| format!("crear dir: {e}"))?;
    let file = fs::File::open(archive).map_err(|e| format!("abrir zip: {e}"))?;
    let mut zip = zip::ZipArchive::new(file).map_err(|e| format!("leer zip: {e}"))?;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| format!("entry: {e}"))?;
        if entry.is_dir() {
            continue;
        }
        let name = entry.mangled_name().to_string_lossy().replace('\\', "/");
        let rel = if let Some(stripped) = name.strip_prefix(strip_prefix.trim_end_matches('/')) {
            stripped.trim_start_matches('/').to_string()
        } else {
            name.clone()
        };
        if rel.is_empty() || rel.starts_with('.') {
            continue;
        }
        let dest = target_root.join(&rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("crear subdir: {e}"))?;
        }
        let mut out = fs::File::create(&dest).map_err(|e| format!("crear archivo: {e}"))?;
        std::io::copy(&mut entry, &mut out).map_err(|e| format!("escribir: {e}"))?;
    }
    Ok(())
}

pub async fn apply_updates(
    instance_dir: &Path,
    manifest_url: &str,
    on_status: &mut (dyn FnMut(String) + Send),
) -> Result<SyncResult, String> {
    let manifest = fetch_manifest(manifest_url).await?;
    if manifest.wipe {
        on_status("Borrando instancia completa...".to_string());
        if instance_dir.exists() {
            fs::remove_dir_all(instance_dir).map_err(|e| format!("borrar instancia: {e}"))?;
        }
        fs::create_dir_all(instance_dir).map_err(|e| format!("recrear instancia: {e}"))?;
    }
    let previous = load_manifest_cache(instance_dir);
    let diff = compute_diff(instance_dir, &manifest, previous.as_ref());
    cache_manifest(instance_dir, &manifest);
    if diff.up_to_date && diff.change_count == 0 {
        return Ok(SyncResult {
            applied_revision: manifest.revision.clone(),
            replaced: Vec::new(),
            downloaded: Vec::new(),
            removed: Vec::new(),
            backup_dir: String::new(),
            errors: Vec::new(),
        });
    }

    let managed = managed_folders(&manifest);
    let protected = protected_paths(&manifest);
    let revision_slug = if manifest.revision.is_empty() {
        chrono::Local::now().format("%Y%m%d_%H%M%S").to_string()
    } else {
        manifest
            .revision
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' { c } else { '_' })
            .collect::<String>()
    };
    let backup_dir = instance_dir.join("backups").join(&revision_slug);
    fs::create_dir_all(&backup_dir).map_err(|e| format!("crear backup: {e}"))?;
    for folder in &managed {
        let src = instance_dir.join(folder);
        let dst = backup_dir.join(folder);
        if src.exists() {
            copy_dir(&src, &dst).map_err(|e| format!("backup {folder}: {e}"))?;
        }
    }
    let meta_src = instance_dir.join(crate::config::INSTANCE_META_FILE);
    if meta_src.exists() {
        fs::copy(&meta_src, backup_dir.join(crate::config::INSTANCE_META_FILE)).ok();
    }
    let mut downloaded = Vec::new();
    let mut replaced = Vec::new();
    let mut errors = Vec::new();

    if !manifest.archive_url.is_empty() {
        let archive = download_archive(&manifest, instance_dir, on_status).await?;
        on_status("Aplicando cambios...".to_string());
        let extract_root = instance_dir.join(".nexuslauncher_update");
        if extract_root.exists() {
            fs::remove_dir_all(&extract_root).ok();
        }
        fs::create_dir_all(&extract_root).ok();
        if extract_zip_stripping(&archive, &extract_root, "files/").is_err() {
            extract_zip_stripping(&archive, &extract_root, "")?;
        }
        fs::remove_file(&archive).ok();
        for f in &manifest.files {
            if !is_managed(&f.path, &managed) || protected.contains(&f.path) {
                continue;
            }
            let rel = f.path.replace('\\', "/");
            let src = extract_root.join(&rel);
            if !src.exists() {
                errors.push(format!("falta {} en el paquete", rel));
                continue;
            }
            let ok = local_has_changed(instance_dir, &rel, &f.sha256);
            let dest = instance_dir.join(&rel);
            let is_replace = dest.exists();
            if ok || !dest.exists() {
                if let Some(parent) = dest.parent() {
                    fs::create_dir_all(parent).map_err(|e| format!("crear dir: {e}"))?;
                }
                fs::copy(&src, &dest).map_err(|e| format!("copiar {rel}: {e}"))?;
                if is_replace {
                    replaced.push(rel);
                } else {
                    downloaded.push(rel);
                }
            }
        }
        fs::remove_dir_all(&extract_root).ok();
    } else {
        let client = http_client();
        on_status("Descargando archivos individuales...".to_string());
        let mut jobs: Vec<DlJob> = Vec::new();
        let mut was_replace: std::collections::HashMap<String, bool> = Default::default();
        for f in &manifest.files {
            if !is_managed(&f.path, &managed) || protected.contains(&f.path) {
                continue;
            }
            let url = match &f.url {
                Some(u) if !u.is_empty() => resolve_download_url(u),
                _ => {
                    errors.push(format!("{} sin URL", f.path));
                    continue;
                }
            };
            let dest = instance_dir.join(&f.path);
            let changed = local_has_changed(instance_dir, &f.path, &f.sha256);
            if !changed && dest.exists() {
                continue;
            }
            if let Some(parent) = dest.parent() {
                fs::create_dir_all(parent).ok();
            }
            was_replace.insert(f.path.clone(), dest.exists());
            jobs.push(DlJob { key: f.path.clone(), urls: vec![url], dest });
        }
        let total_jobs = jobs.len().max(1);
        download_many(&client, jobs, MAX_CONCURRENT_DOWNLOADS, &mut |done, _, outcome| {
            on_status(format!("Descargando archivo {done}/{total_jobs}..."));
            match outcome.result {
                Ok(()) => {
                    if was_replace.get(&outcome.key).copied().unwrap_or(false) {
                        replaced.push(outcome.key);
                    } else {
                        downloaded.push(outcome.key);
                    }
                }
                Err(e) => errors.push(format!("{}: {e}", outcome.key)),
            }
        })
        .await;
    }
    let mut removed = Vec::new();
    for path in &diff.extra {
        if protected.contains(path) {
            continue;
        }
        let dest = instance_dir.join(path);
        if dest.exists() {
            if fs::remove_file(&dest).is_ok() {
                removed.push(path.clone());
            }
        }
        prune_empty_dirs(instance_dir, &managed);
    }
    let mut meta = crate::instances::load_meta(instance_dir);
    meta.server_version = manifest.revision.clone();
    if let Err(e) = crate::instances::save_meta(instance_dir, &meta) {
        errors.push(e);
    }

    Ok(SyncResult {
        applied_revision: manifest.revision.clone(),
        replaced,
        downloaded,
        removed,
        backup_dir: backup_dir.to_string_lossy().to_string(),
        errors,
    })
}

fn local_has_changed(instance_dir: &Path, rel: &str, expected_hash: &str) -> bool {
    let p = instance_dir.join(rel);
    if !p.exists() {
        return true;
    }
    if expected_hash.is_empty() {
        return false;
    }
    sha256_file(&p).map(|h| h.to_lowercase() != expected_hash.to_lowercase()).unwrap_or(true)
}

fn copy_dir(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|e| format!("crear {e}"))?;
    for entry in fs::read_dir(src).map_err(|e| format!("leer {src:?}: {e}"))?.flatten() {
        let p = entry.path();
        let target = dst.join(entry.file_name());
        if p.is_dir() {
            copy_dir(&p, &target)?;
        } else {
            fs::copy(&p, &target).map_err(|e| format!("copiar {p:?}: {e}"))?;
        }
    }
    Ok(())
}

fn prune_empty_dirs(instance_dir: &Path, managed: &[String]) {
    for folder in managed {
        let dir = instance_dir.join(folder);
        if !dir.exists() {
            continue;
        }
        let mut stack = vec![dir.clone()];
        let mut dirs = Vec::new();
        while let Some(current) = stack.pop() {
            if let Ok(entries) = fs::read_dir(&current) {
                let mut has_child = false;
                for entry in entries.flatten() {
                    has_child = true;
                    if entry.path().is_dir() {
                        stack.push(entry.path());
                    }
                }
                if !has_child {
                    dirs.push(current);
                }
            }
        }
        for d in dirs.into_iter().rev() {
            fs::remove_dir(&d).ok();
        }
    }
}
pub fn rollback(instance_dir: &Path, revision: &str) -> Result<SyncResult, String> {
    let backup_dir = instance_dir.join("backups").join(revision);
    if !backup_dir.exists() {
        return Err(format!("No existe el backup para la revisión '{revision}'"));
    }
    let meta = crate::instances::load_meta(instance_dir);
    let managed_meta = if meta.loader.is_empty() {
        default_managed()
    } else {
        default_managed()
    };
    let mut errors = Vec::new();
    let mut restored = Vec::new();
    for folder in managed_meta {
        let src = backup_dir.join(&folder);
        let dst = instance_dir.join(&folder);
        if src.exists() {
            if let Err(e) = copy_dir(&src, &dst) {
                errors.push(format!("restaurar {folder}: {e}"));
            }
            restored.push(folder);
        }
    }
    Ok(SyncResult {
        applied_revision: revision.to_string(),
        replaced: restored,
        downloaded: Vec::new(),
        removed: Vec::new(),
        backup_dir: backup_dir.to_string_lossy().to_string(),
        errors,
    })
}
pub fn list_backups(instance_name: &str) -> Vec<String> {
    let dir = instances_dir().join(instance_name).join("backups");
    if !dir.exists() {
        return Vec::new();
    }
    let mut list: Vec<String> = fs::read_dir(&dir)
        .map(|e| {
            e.flatten()
                .filter(|e| e.path().is_dir())
                .map(|e| e.file_name().to_string_lossy().to_string())
                .collect()
        })
        .unwrap_or_default();
    list.sort();
    list.reverse();
    list
}

#[derive(Debug, Clone)]
struct GitCtx {
    repo: String,
    branch: String,
    path: String, // "" = raíz del repo
}

pub fn parse_github_url(url: &str) -> Option<(String, String, String)> {
    let clean = url.trim();
    if let Some(rest) = clean.strip_prefix("https://raw.githubusercontent.com/") {
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 3 {
            let repo = format!("{}/{}", parts[0], parts[1]);
            let branch = parts[2].to_string();
            let path_parts = if parts.len() > 3 {
                let last = parts[parts.len() - 1];
                if last.ends_with(".json") || last.ends_with(".txt") {
                    &parts[3..parts.len() - 1]
                } else {
                    &parts[3..]
                }
            } else {
                &[]
            };
            let path = path_parts.join("/");
            return Some((repo, branch, path));
        }
    } else if let Some(rest) = clean.strip_prefix("https://github.com/") {
        let parts: Vec<&str> = rest.split('/').filter(|s| !s.is_empty()).collect();
        if parts.len() >= 2 {
            let repo = format!("{}/{}", parts[0], parts[1]);
            let branch = if parts.len() >= 4 && (parts[2] == "tree" || parts[2] == "blob") {
                parts[3].to_string()
            } else {
                "main".to_string()
            };
            let path_parts = if parts.len() > 4 && (parts[2] == "tree" || parts[2] == "blob") {
                &parts[4..]
            } else {
                &[]
            };
            let path = path_parts.join("/");
            return Some((repo, branch, path));
        }
    }
    None
}

fn git_ctx(meta: &crate::instances::InstanceMeta) -> Option<GitCtx> {
    let mut repo = meta.github_repo.trim().to_string();
    let mut branch = meta.github_branch.trim().to_string();
    let mut path = meta.github_path.trim().trim_matches('/').to_string();

    if repo.is_empty() && !meta.manifest_url.trim().is_empty() {
        if let Some((r, b, p)) = parse_github_url(&meta.manifest_url) {
            repo = r;
            if branch.is_empty() { branch = b; }
            if path.is_empty() { path = p; }
        }
    }

    if repo.is_empty() {
        return None;
    }
    if branch.is_empty() {
        branch = "main".to_string();
    }
    Some(GitCtx {
        repo,
        branch,
        path,
    })
}

fn strip_sync_prefix(path: &str, prefix: &str) -> Option<String> {
    if prefix.is_empty() {
        return Some(path.to_string());
    }
    let trimmed = path.trim_start_matches('/');
    let p = prefix.trim_matches('/');
    if trimmed == p {
        return None; // es la carpeta en sí
    }
    trimmed
        .strip_prefix(&format!("{}/", p))
        .map(|s| s.to_string())
}

fn sync_raw_url(ctx: &GitCtx, rel: &str) -> String {
    let base = format!(
        "https://raw.githubusercontent.com/{}/{}/{}",
        ctx.repo,
        ctx.branch,
        ctx.path.trim_matches('/')
    );
    let base = base.trim_end_matches('/');
    format!("{}/{}", base, rel.replace('\\', "/").replace(' ', "%20"))
}
fn lfs_media_url(ctx: &GitCtx, rel: &str) -> String {
    let base = format!(
        "https://media.githubusercontent.com/media/{}/{}/{}",
        ctx.repo,
        ctx.branch,
        ctx.path.trim_matches('/')
    );
    let base = base.trim_end_matches('/');
    format!("{}/{}", base, rel.replace('\\', "/").replace(' ', "%20"))
}

const LFS_POINTER_MAGIC: &[u8] = b"version https://git-lfs.github.com/spec/v1";
fn is_lfs_pointer(path: &Path) -> bool {
    use std::io::Read;
    let Ok(mut f) = fs::File::open(path) else {
        return false;
    };
    let mut buf = vec![0u8; LFS_POINTER_MAGIC.len()];
    let Ok(n) = f.read(&mut buf) else {
        return false;
    };
    n == LFS_POINTER_MAGIC.len() && &buf == LFS_POINTER_MAGIC
}

async fn git_api_get(url: &str) -> Result<serde_json::Value, String> {
    let client = http_client();
    crate::downloader::get_json::<serde_json::Value>(&client, url, Duration::from_secs(30))
        .await
}
async fn git_head_sha(ctx: &GitCtx) -> Result<String, String> {
    let url = format!("https://api.github.com/repos/{}/branches/{}", ctx.repo, ctx.branch);
    let v = git_api_get(&url).await.map_err(|e| format!("leer rama: {e}"))?;
    v["commit"]["sha"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| "respuesta de rama inválida".to_string())
}
async fn git_tree(ctx: &GitCtx) -> Result<Vec<(String, String, u64)>, String> {
    let url = format!(
        "https://api.github.com/repos/{}/git/trees/{}?recursive=1",
        ctx.repo, ctx.branch
    );
    let v = git_api_get(&url).await.map_err(|e| format!("leer árbol: {e}"))?;
    let mut out = Vec::new();
    for e in v["tree"].as_array().cloned().unwrap_or_default() {
        if e["type"].as_str() != Some("blob") {
            continue;
        }
        let full = e["path"].as_str().unwrap_or("");
        let Some(rel) = strip_sync_prefix(full, &ctx.path) else { continue };
        out.push((
            rel,
            e["sha"].as_str().unwrap_or("").to_string(),
            e["size"].as_u64().unwrap_or(0),
        ));
    }
    Ok(out)
}

#[derive(Debug, Clone)]
struct GitPending {
    head: String,
    message: String,
    missing: Vec<String>,   // hay que descargar (nuevos o cambiados)
    different: Vec<String>, // existentes que cambian (UI)
    extra: Vec<String>,     // eliminados en la repo
}

fn git_rev_display(head: &str, message: &str) -> String {
    let short = head.chars().take(7).collect::<String>();
    let msg = message.trim();
    if msg.is_empty() {
        short
    } else {
        let first = msg.lines().next().unwrap_or("");
        let first = first.chars().take(80).collect::<String>();
        format!("{short} · {first}")
    }
}
async fn git_pending(
    ctx: &GitCtx,
    meta: &crate::instances::InstanceMeta,
    instance_dir: &Path,
) -> Result<GitPending, String> {
    let head = git_head_sha(ctx).await?;
    let base = meta.github_base_sha.trim();
    let mut missing = Vec::new();
    let mut different = Vec::new();
    let mut extra = Vec::new();
    let mut message = String::new();

    if !base.is_empty() && base == head {
        return Ok(GitPending { head, message, missing, different, extra });
    }

    if base.is_empty() {
        for (rel, _, _) in git_tree(ctx).await? {
            missing.push(rel);
        }
        return Ok(GitPending { head, message, missing, different, extra });
    }
    let url = format!(
        "https://api.github.com/repos/{}/compare/{}...{}",
        ctx.repo, base, head
    );
    let compare = git_api_get(&url).await;
    match compare {
        Ok(v) => {
            if let Some(commits) = v["commits"].as_array() {
                if let Some(last) = commits.last() {
                    message = last["commit"]["message"].as_str().unwrap_or("").to_string();
                }
            }
            for f in v["files"].as_array().cloned().unwrap_or_default() {
                let full = f["filename"].as_str().unwrap_or("");
                let Some(rel) = strip_sync_prefix(full, &ctx.path) else { continue };
                let status = f["status"].as_str().unwrap_or("");
                match status {
                    "removed" => extra.push(rel),
                    _ => {
                        let local = instance_dir.join(&rel).exists();
                        if local { different.push(rel) } else { missing.push(rel) }
                    }
                }
            }
        }
        Err(_) => {
            for (rel, _, _) in git_tree(ctx).await? {
                missing.push(rel);
            }
        }
    }
    missing.sort();
    different.sort();
    extra.sort();
    Ok(GitPending { head, message, missing, different, extra })
}

fn is_git_protected(rel: &str, protected: &HashSet<String>) -> bool {
    let top = rel.split('/').next().unwrap_or("");
    protected.contains(rel) || protected.contains(top)
}
pub async fn git_check_dir(
    instance_dir: &Path,
    meta: &crate::instances::InstanceMeta,
) -> Result<SyncDiff, String> {
    let ctx = git_ctx(meta).ok_or_else(|| "la instancia no tiene repo configurada".to_string())?;
    let pend = git_pending(&ctx, meta, instance_dir).await?;
    let change_count = pend.missing.len() + pend.different.len() + pend.extra.len();
    Ok(SyncDiff {
        revision: git_rev_display(&pend.head, &pend.message),
        up_to_date: change_count == 0,
        missing: pend.missing,
        different: pend.different,
        extra: pend.extra,
        change_count,
        has_archive: false,
    })
}
pub async fn git_apply_dir(
    instance_dir: &Path,
    on_status: &mut (dyn FnMut(String) + Send),
    on_progress: &mut (dyn FnMut(u32, String) + Send),
) -> Result<SyncResult, String> {
    let mut meta = crate::instances::load_meta(instance_dir);
    let ctx = git_ctx(&meta).ok_or_else(|| "la instancia no tiene repo configurada".to_string())?;
    let pend = git_pending(&ctx, &meta, instance_dir).await?;
    let change_count = pend.missing.len() + pend.different.len() + pend.extra.len();
    if change_count == 0 && !meta.github_base_sha.is_empty() && pend.head == meta.github_base_sha {
        return Ok(SyncResult {
            applied_revision: git_rev_display(&pend.head, &pend.message),
            replaced: Vec::new(),
            downloaded: Vec::new(),
            removed: Vec::new(),
            backup_dir: String::new(),
            errors: Vec::new(),
        });
    }

    let protected = default_protected().into_iter().collect::<HashSet<_>>();
    let revision_slug = pend
        .head
        .chars()
        .take(12)
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect::<String>();
    let backup_dir = instance_dir.join("backups").join(format!("git-{revision_slug}"));
    fs::create_dir_all(&backup_dir).map_err(|e| format!("crear backup: {e}"))?;
    let mut affected: Vec<String> = Vec::new();
    for rel in pend.missing.iter().chain(pend.different.iter()).chain(pend.extra.iter()) {
        if let Some(top) = rel.split('/').next() {
            if !affected.contains(&top.to_string()) {
                affected.push(top.to_string());
            }
        }
    }
    for folder in &affected {
        let src = instance_dir.join(folder);
        if src.exists() {
            copy_dir(&src, &backup_dir.join(folder)).map_err(|e| format!("backup {folder}: {e}"))?;
        }
    }
    if instance_dir.join(crate::config::INSTANCE_META_FILE).exists() {
        fs::copy(
            instance_dir.join(crate::config::INSTANCE_META_FILE),
            backup_dir.join(crate::config::INSTANCE_META_FILE),
        )
        .ok();
    }

    let mut downloaded = Vec::new();
    let mut replaced = Vec::new();
    let mut removed = Vec::new();
    let mut errors = Vec::new();
    let client = http_client();
    let mut jobs: Vec<DlJob> = Vec::new();
    let mut was_replace: std::collections::HashMap<String, bool> = Default::default();
    for rel in pend.missing.iter().chain(pend.different.iter()) {
        if is_git_protected(rel, &protected) || rel.starts_with('.') {
            continue;
        }
        let dest = instance_dir.join(rel);
        was_replace.insert(rel.clone(), dest.exists());
        jobs.push(DlJob { key: rel.clone(), urls: vec![sync_raw_url(&ctx, rel)], dest });
    }
    let total_dl = jobs.len().max(1);
    let mut done = 0usize;
    let mut lfs_retry: Vec<String> = Vec::new();
    download_many(&client, jobs, MAX_CONCURRENT_DOWNLOADS, &mut |d, _, outcome| {
        done = d;
        on_progress(
            85 + 14 * (done as u32) / (total_dl as u32),
            format!("Descargando pack {done}/{total_dl}"),
        );
        on_status(format!("Descargando {}...", outcome.key));
        let rel = outcome.key.clone();
        match outcome.result {
            Ok(()) => {
                // Los archivos LFS llegan como puntero; se reintentan contra media.githubusercontent.com abajo
                if is_lfs_pointer(&outcome.dest) {
                    lfs_retry.push(rel);
                } else if was_replace.get(&rel).copied().unwrap_or(false) {
                    replaced.push(rel)
                } else {
                    downloaded.push(rel)
                }
            }
            Err(e) => errors.push(format!("{rel}: {e}")),
        }
    })
    .await;
    for rel in lfs_retry {
        let dest = instance_dir.join(&rel);
        let media = lfs_media_url(&ctx, &rel);
        match crate::downloader::download_file(&client, &media, &dest, &mut |_, _| {}).await {
            Ok(()) => {
                if was_replace.get(&rel).copied().unwrap_or(false) {
                    replaced.push(rel)
                } else {
                    downloaded.push(rel)
                }
            }
            Err(e) => errors.push(format!("{rel}: {e}")),
        }
    }
    let tracked: Vec<String> = meta.github_tracked.clone();
    for rel in &pend.extra {
        if is_git_protected(rel, &protected) {
            continue;
        }
        if !tracked.is_empty() && !tracked.iter().any(|t| t == rel) {
            continue;
        }
        let dest = instance_dir.join(rel);
        if dest.exists() && fs::remove_file(&dest).is_ok() {
            removed.push(rel.clone());
        }
    }
    let previous: Vec<String> = tracked;
    let mut new_tracked = previous.clone();
    for rel in pend.missing.iter().chain(pend.different.iter()) {
        if !new_tracked.iter().any(|t| t == rel) {
            new_tracked.push(rel.clone());
        }
    }
    for rel in &pend.extra {
        new_tracked.retain(|t| t != rel);
    }
    if new_tracked.is_empty() && (pend.missing.is_empty() && pend.different.is_empty() && !pend.extra.is_empty()) {
        new_tracked = previous;
    }
    meta.server_version = git_rev_display(&pend.head, &pend.message);
    meta.github_base_sha = pend.head.clone();
    meta.github_tracked = new_tracked;
    if let Err(e) = crate::instances::save_meta(instance_dir, &meta) {
        errors.push(e);
    }
    on_progress(99, "Finalizando...".to_string());

    Ok(SyncResult {
        applied_revision: git_rev_display(&pend.head, &pend.message),
        replaced,
        downloaded,
        removed,
        backup_dir: backup_dir.to_string_lossy().to_string(),
        errors,
    })
}
#[derive(Debug, Clone, Serialize)]
pub struct InstanceUpdateState {
    pub name: String,
    pub up_to_date: bool,
    pub change_count: usize,
    pub revision: String,
    pub error: Option<String>,
}
pub async fn auto_check_all() -> Result<Vec<InstanceUpdateState>, String> {
    let mut out = Vec::new();
    for inst in crate::instances::scan_instances() {
        let name = inst.name;
        let dir = crate::instances::instance_path(&name);
        let meta = crate::instances::load_meta(&dir);
        let diff = if !meta.github_repo.trim().is_empty() {
            git_check_dir(&dir, &meta).await
        } else if !meta.manifest_url.trim().is_empty() {
            check_updates(&dir, &meta.manifest_url).await
        } else {
            continue;
        };
        let state = match diff {
            Ok(diff) => InstanceUpdateState {
                name: name.clone(),
                up_to_date: diff.up_to_date && diff.change_count == 0,
                change_count: diff.change_count,
                revision: diff.revision,
                error: None,
            },
            Err(e) => InstanceUpdateState {
                name: name.clone(),
                up_to_date: true,
                change_count: 0,
                revision: String::new(),
                error: Some(e),
            },
        };
        out.push(state);
    }
    Ok(out)
}

#[derive(Debug, Clone, Serialize)]
pub struct AutoSyncSummary {
    pub applied: Vec<String>,
    pub already_up_to_date: Vec<String>,
    pub errors: Vec<String>,
}
pub async fn auto_sync_all(
    on_status: &mut (dyn FnMut(String) + Send),
) -> Result<AutoSyncSummary, String> {
    let states = auto_check_all().await?;
    let mut applied = Vec::new();
    let mut already = Vec::new();
    let mut errors = Vec::new();
    for st in states {
        if let Some(e) = st.error {
            errors.push(format!("{}: {e}", st.name));
            continue;
        }
        if st.up_to_date && st.change_count == 0 {
            already.push(st.name.clone());
            continue;
        }
        on_status(format!("Actualizando '{}'...", st.name));
        let dir = crate::instances::instance_path(&st.name);
        let meta = crate::instances::load_meta(&dir);
        let res = if !meta.github_repo.trim().is_empty() {
            git_apply_dir(&dir, on_status, &mut |_, _| {}).await
        } else {
            apply_updates(&dir, &meta.manifest_url, on_status).await
        };
        match res {
            Ok(r) => applied.push(format!("{} (revisión {})", st.name, r.applied_revision)),
            Err(e) => errors.push(format!("{}: {e}", st.name)),
        }
    }
    Ok(AutoSyncSummary {
        applied,
        already_up_to_date: already,
        errors,
    })
}

// ---- Publisher (owner): sube carpetas seleccionadas a GitHub vía Git Data API ----
// Los usuarios finales reciben la actualización con sync_check/sync_apply o auto_sync_apply al darle a jugar,
// ya que esos leen la rama github_branch del repo configurado en la instancia.

pub fn list_publish_folders(target: &str) -> Result<Vec<String>, String> {
    let p = Path::new(target);
    let dir = if p.is_dir() {
        p.to_path_buf()
    } else {
        crate::instances::instance_path(target)
    };
    if !dir.exists() {
        return Err(format!("La carpeta '{}' no existe", target));
    }
    let protected: HashSet<String> = default_protected().into_iter().collect();
    let mut folders = Vec::new();
    if let Ok(entries) = fs::read_dir(&dir) {
        for entry in entries.flatten() {
            if !entry.path().is_dir() {
                continue;
            }
            let name = entry.file_name().to_string_lossy().to_string();
            if protected.contains(&name) || name.starts_with('.') {
                continue;
            }
            folders.push(name);
        }
    }
    folders.sort();
    Ok(folders)
}

async fn gh_request(
    client: &reqwest::Client,
    method: reqwest::Method,
    url: &str,
    token: &str,
    body: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let mut req = client.request(method, url).bearer_auth(token);
    if let Some(b) = body {
        req = req.json(&b);
    }
    let resp = req.send().await.map_err(|e| format!("GitHub: {e}"))?;
    let status = resp.status();
    let text = resp.text().await.map_err(|e| format!("GitHub respuesta: {e}"))?;
    if !status.is_success() {
        return Err(format!("GitHub HTTP {} para {url}: {}", status.as_u16(), text.chars().take(300).collect::<String>()));
    }
    if text.trim().is_empty() {
        return Ok(serde_json::json!({}));
    }
    serde_json::from_str(&text).map_err(|e| format!("GitHub JSON inválido: {e}"))
}

fn collect_files(base: &Path, dir: &Path, rel: &str, out: &mut Vec<String>) -> Result<(), String> {
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            if name.starts_with('.') {
                continue;
            }
            let rel_path = if rel.is_empty() { name.clone() } else { format!("{rel}/{name}") };
            if path.is_dir() {
                collect_files(base, &path, &rel_path, out)?;
            } else if path.is_file() {
                let _ = base;
                out.push(rel_path);
            }
        }
    }
    Ok(())
}

/// Publica carpetas locales directamente a un repositorio GitHub mediante Git Data API.
pub async fn publish_from_folder(
    local_path: &str,
    repo: &str,
    branch: &str,
    subpath: &str,
    folders: Vec<String>,
    message: String,
) -> Result<String, String> {
    let token = crate::config::Settings::load().github_token.trim().to_string();
    if token.is_empty() {
        return Err("Guarda primero tu token de GitHub en Ajustes (owner).".to_string());
    }
    let p = Path::new(local_path);
    let dir = if p.is_dir() {
        p.to_path_buf()
    } else {
        crate::instances::instance_path(local_path)
    };
    if !dir.exists() || !dir.is_dir() {
        return Err(format!("La carpeta local '{}' no existe o no es un directorio válido.", local_path));
    }
    let repo_clean = repo.trim()
        .trim_start_matches("https://github.com/")
        .trim_end_matches(".git")
        .trim();
    if repo_clean.is_empty() {
        return Err("El repositorio GitHub no puede estar vacío.".to_string());
    }
    let branch_clean = if branch.trim().is_empty() { "main" } else { branch.trim() };
    if folders.is_empty() {
        return Err("Selecciona al menos una carpeta para sincronizar y publicar.".to_string());
    }
    let protected: HashSet<String> = default_protected().into_iter().collect();
    let mut rels = Vec::new();
    for f in &folders {
        let top = f.split('/').next().unwrap_or("");
        if protected.contains(top) || top.starts_with('.') {
            continue;
        }
        let sub = dir.join(f);
        if sub.is_dir() {
            collect_files(&dir, &sub, f, &mut rels)?;
        } else if sub.is_file() {
            rels.push(f.clone());
        }
    }
    if rels.is_empty() {
        return Err("Las carpetas seleccionadas están vacías o no existen en la ruta indicada.".to_string());
    }
    if rels.len() > 2000 {
        return Err(format!("Demasiados archivos ({}) para publicar de golpe. Reduce la selección.", rels.len()));
    }
    let client = http_client();
    let head_url = format!("https://api.github.com/repos/{}/git/refs/heads/{}", repo_clean, branch_clean);
    let head = gh_request(&client, reqwest::Method::GET, &head_url, &token, None).await?;
    let head_sha = head["object"]["sha"].as_str().ok_or("respuesta de rama inválida")?.to_string();
    let commit_url = format!("https://api.github.com/repos/{}/git/commits/{}", repo_clean, head_sha);
    let commit = gh_request(&client, reqwest::Method::GET, &commit_url, &token, None).await?;
    let base_tree = commit["tree"]["sha"].as_str().ok_or("árbol base inválido")?.to_string();

    let blob_url = format!("https://api.github.com/repos/{}/git/blobs", repo_clean);
    let prefix = subpath.trim_matches('/').to_string();
    let mut tree_entries = Vec::new();
    for rel in &rels {
        let bytes = fs::read(dir.join(rel)).map_err(|e| format!("leer {rel}: {e}"))?;
        let b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes);
        let blob = gh_request(&client, reqwest::Method::POST, &blob_url, &token, Some(serde_json::json!({"content": b64, "encoding": "base64"}))).await?;
        let blob_sha = blob["sha"].as_str().ok_or("blob inválido")?.to_string();
        let repo_path = if prefix.is_empty() { rel.clone() } else { format!("{prefix}/{rel}") };
        tree_entries.push(serde_json::json!({"path": repo_path, "mode": "100644", "type": "blob", "sha": blob_sha}));
    }
    let tree_url = format!("https://api.github.com/repos/{}/git/trees", repo_clean);
    let tree = gh_request(&client, reqwest::Method::POST, &tree_url, &token, Some(serde_json::json!({"base_tree": base_tree, "tree": tree_entries}))).await?;
    let new_tree = tree["sha"].as_str().ok_or("árbol nuevo inválido")?.to_string();
    let msg = if message.trim().is_empty() {
        format!("Actualización: {}", folders.join(", "))
    } else {
        message.trim().to_string()
    };
    let new_commit_url = format!("https://api.github.com/repos/{}/git/commits", repo_clean);
    let new_commit = gh_request(&client, reqwest::Method::POST, &new_commit_url, &token, Some(serde_json::json!({"message": msg, "tree": new_tree, "parents": [head_sha]}))).await?;
    let commit_sha = new_commit["sha"].as_str().ok_or("commit nuevo inválido")?.to_string();
    let _ = gh_request(&client, reqwest::Method::PATCH, &head_url, &token, Some(serde_json::json!({"sha": commit_sha}))).await?;
    Ok(commit_sha)
}

/// Publica al repo GitHub de la instancia las carpetas seleccionadas.
/// Retorna el SHA del nuevo commit.
pub async fn publish_instance(name: &str, folders: Vec<String>, message: String) -> Result<String, String> {
    let dir = crate::instances::instance_path(name);
    let meta = crate::instances::load_meta(&dir);
    let ctx = git_ctx(&meta).ok_or_else(|| "La instancia no tiene github_repo configurada.".to_string())?;
    publish_from_folder(&dir.to_string_lossy(), &ctx.repo, &ctx.branch, &ctx.path, folders, message).await
}