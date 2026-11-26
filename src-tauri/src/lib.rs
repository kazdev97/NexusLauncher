pub mod antilag;
pub mod auth;
mod api;
mod vps_instances;
pub mod config;
pub mod downloader;
pub mod instances;
pub mod installer;
pub mod java;
pub mod jvm;
pub mod launch;
pub mod packager;
pub mod sync;
pub mod updater;
pub mod discord;
pub mod whitelist;

use config::{Account, AccountStore, Settings};
use serde::Serialize;
use tauri::Emitter;

fn require_access(instance_name: &str) -> Result<(), String> {
    let store = AccountStore::load();
    if store.mode == "offline" || store.selected_id.trim().is_empty() {
        return Err("Inicia sesión con Microsoft para esta acción.".to_string());
    }
    let account = store
        .selected()
        .ok_or_else(|| "Inicia sesión con Microsoft para esta acción.".to_string())?;
    if !whitelist::can_access(&account.id, instance_name) {
        return Err(format!(
            "No tienes acceso a la instancia '{instance_name}'. Contacta con el staff."
        ));
    }
    Ok(())
}

fn require_admin() -> Result<(), String> {
    let store = AccountStore::load();
    if store.mode == "offline" || store.selected_id.trim().is_empty() {
        return Err("Inicia sesión con Microsoft para esta acción.".to_string());
    }
    let account = store
        .selected()
        .ok_or_else(|| "Inicia sesión con Microsoft para esta acción.".to_string())?;
    if !whitelist::is_admin(&account.id) {
        return Err("Solo el propietario o el staff autorizado puede hacer esto.".to_string());
    }
    Ok(())
}

/// Solo el propietario (Soy_Kaz) puede publicar actualizaciones o tocar el token.
fn require_owner() -> Result<(), String> {
    let store = AccountStore::load();
    if store.mode == "offline" || store.selected_id.trim().is_empty() {
        return Err("Inicia sesión con Microsoft para esta acción.".to_string());
    }
    let account = store
        .selected()
        .ok_or_else(|| "Inicia sesión con Microsoft para esta acción.".to_string())?;
    if !whitelist::is_owner(&account.id) {
        return Err("Solo el propietario puede publicar actualizaciones.".to_string());
    }
    Ok(())
}
#[derive(Clone, Serialize)]
pub struct ProgressMsg {
    pub percent: u32,
    pub status: String,
}
#[tauri::command]
fn login_get_state() -> Result<AccountStore, String> {
    Ok(config::AccountStore::load())
}

#[tauri::command]
async fn login_start(app: tauri::AppHandle) -> Result<Account, String> {
    auth::start_login_flow(app).await
}

#[tauri::command]
fn login_get_account(account_id: String) -> Result<Account, String> {
    let store = config::AccountStore::load();
    store
        .get(&account_id)
        .cloned()
        .ok_or_else(|| "cuenta no encontrada".to_string())
}

#[tauri::command]
fn login_set_selected(account_id: String) -> Result<(), String> {
    config::set_selected_account(&account_id)
}

#[tauri::command]
fn login_remove_account(account_id: String) -> Result<(), String> {
    config::remove_account(&account_id)
}
#[tauri::command]
fn login_set_offline(username: String) -> Result<(), String> {
    let _ = username;
    Err("El modo No premium está deshabilitado: inicia sesión con Microsoft.".to_string())
}
#[tauri::command]
async fn java_resolve(app: tauri::AppHandle, name: String) -> Result<java::JavaInfo, String> {
    let handle = app.clone();
    let dir = instances::instance_path(&name);
    let meta = instances::load_meta(&dir);
    let required = java::resolve_java_major_online(&meta.game_version, &dir).await;
    let exe = java::ensure_java(&meta.game_version, &dir, &mut |status| {
        let _ = handle.emit("java/progress", status);
    })
    .await?;
    let major = java::validate_java_executable(&exe, required).unwrap_or(required);
    Ok(java::JavaInfo { path: exe.to_string_lossy().to_string(), major, is_valid: true })
}

#[tauri::command]
async fn java_get_required(game_version: String) -> u32 {
    java::required_java_major_heuristic(&game_version)
}

#[tauri::command]
async fn java_find(major_version: u32) -> Result<java::JavaInfo, String> {
    if let Some(exe) = java::find_system_java(major_version) {
        let major = java::validate_java_executable(&exe, major_version).unwrap_or(major_version);
        return Ok(java::JavaInfo { path: exe.to_string_lossy().to_string(), major, is_valid: true });
    }
    if let Some(exe) = java::find_bundled_java(major_version) {
        let major = java::validate_java_executable(&exe, major_version).unwrap_or(major_version);
        return Ok(java::JavaInfo { path: exe.to_string_lossy().to_string(), major, is_valid: true });
    }
    Err(format!("No hay un Java {major_version}+ disponible"))
}

#[tauri::command]
async fn java_install(
    app: tauri::AppHandle,
    major_version: u32,
) -> Result<java::JavaInfo, String> {
    let handle = app.clone();
    let exe = java::install_adoptium(major_version, &mut move |status| {
        let _ = handle.emit("java/progress", status);
    })
    .await?;
    let major = java::validate_java_executable(&exe, major_version).unwrap_or(major_version);
    Ok(java::JavaInfo { path: exe.to_string_lossy().to_string(), major, is_valid: true })
}

#[tauri::command]
async fn java_get_system() -> Result<Vec<java::JavaInfo>, String> {
    java::find_all_system_java()
}

#[tauri::command]
async fn java_get_bundled() -> Vec<java::JavaInfo> {
    java::find_all_bundled_java()
}
#[tauri::command]
async fn update_check() -> Result<updater::UpdateInfo, String> {
    updater::check_update().await
}

#[tauri::command]
async fn update_apply() -> Result<(), String> {
    updater::apply_update().await
}
#[tauri::command]
fn update_log() -> String {
    updater::read_log()
}
#[tauri::command]
fn instances_list() -> Vec<instances::InstanceInfo> {
    instances::scan_instances()
}

#[tauri::command]
async fn system_info() -> jvm::SystemInfo {
    jvm::system_info()
}

#[tauri::command]
fn ram_recommend(name: String) -> Result<jvm::RamRecommend, String> {
    jvm::ram_recommend(&name)
}

#[tauri::command]
fn instances_create(
    name: String,
    game_version: String,
    loader: String,
    loader_version: String,
    manifest_url: String,
) -> Result<instances::InstanceInfo, String> {
    require_admin()?;
    let req = instances::CreateInstanceRequest {
        name,
        game_version,
        loader,
        loader_version,
        manifest_url,
    };
    let dir = instances::create_instance_dir(&req)?;
    let clean = req.sanitized_name();
    let meta = instances::InstanceMeta {
        name: clean.clone(),
        game_version: req.game_version.clone(),
        loader: req.loader.clone(),
        loader_version: req.loader_version.clone(),
        manifest_url: req.manifest_url.clone(),
        ..Default::default()
    };
    instances::save_meta(&dir, &meta)?;
    instances::scan_instances()
        .into_iter()
        .find(|i| i.name == clean)
        .ok_or_else(|| "no se pudo crear la instancia".to_string())
}

#[tauri::command]
fn instances_get_meta(name: String) -> Result<instances::InstanceMeta, String> {
    let dir = instances::instance_path(&name);
    if !dir.exists() {
        return Err("La instancia no existe".into());
    }
    Ok(instances::load_meta(&dir))
}

#[tauri::command]
fn instances_delete(name: String) -> Result<(), String> {
    require_admin()?;
    instances::delete_instance(&name)
}
#[tauri::command]
async fn sync_check(name: String) -> Result<sync::SyncDiff, String> {
    require_access(&name)?;
    let dir = instances::instance_path(&name);
    let meta = instances::load_meta(&dir);
    if !meta.github_repo.trim().is_empty() {
        return sync::git_check_dir(&dir, &meta).await;
    }
    if meta.manifest_url.is_empty() {
        return Err("La instancia no tiene URL de manifest configurada".into());
    }
    sync::check_updates(&dir, &meta.manifest_url).await
}

#[tauri::command]
async fn sync_apply(app: tauri::AppHandle, name: String) -> Result<sync::SyncResult, String> {
    require_access(&name)?;
    let dir = instances::instance_path(&name);
    let meta = instances::load_meta(&dir);
    let handle = app.clone();
    let mut on_status = |status: String| {
        let _ = handle.emit("sync/status", status);
    };
    let mut on_progress = |percent: u32, status: String| {
        let _ = handle.emit("sync/progress", ProgressMsg { percent, status });
    };
    if !meta.github_repo.trim().is_empty() {
        return sync::git_apply_dir(&dir, &mut on_status, &mut on_progress).await;
    }
    if meta.manifest_url.is_empty() {
        return Err("La instancia no tiene URL de manifest configurada".into());
    }
    sync::apply_updates(&dir, &meta.manifest_url, &mut on_status).await
}

#[tauri::command]
fn sync_rollback(name: String, revision: String) -> Result<sync::SyncResult, String> {
    require_access(&name)?;
    let dir = instances::instance_path(&name);
    sync::rollback(&dir, &revision)
}

#[tauri::command]
fn sync_list_backups(name: String) -> Vec<String> {
    sync::list_backups(&name)
}
#[tauri::command]
async fn auto_sync_check() -> Result<Vec<sync::InstanceUpdateState>, String> {
    sync::auto_check_all().await
}

#[tauri::command]
async fn auto_sync_apply(app: tauri::AppHandle) -> Result<sync::AutoSyncSummary, String> {
    require_admin()?;
    let handle = app.clone();
    sync::auto_sync_all(&mut |status| {
        let _ = handle.emit("sync/status", status);
    })
    .await
}
#[tauri::command]
fn pack_export(
    instance_name: String,
    destination_dir: String,
    archive_url: String,
    file_base_url: String,
    extra_folders: Vec<String>,
    ram_min_mb: u32,
    ram_max_mb: u32,
) -> Result<packager::ExportResult, String> {
    require_admin()?;
    let instance_dir = instances::instance_path(&instance_name);
    if !instance_dir.exists() {
        return Err("La instancia no existe".into());
    }
    let dest = if destination_dir.trim().is_empty() {
        dirs::desktop_dir()
            .unwrap_or_else(|| config::launcher_data_dir())
            .join("NexusLauncher-packs")
    } else {
        std::path::PathBuf::from(destination_dir)
    };
    packager::export_instance(&instance_dir, &dest, &archive_url, &file_base_url, &extra_folders, ram_min_mb, ram_max_mb)
}

#[tauri::command]
fn pack_whitepaper() -> String {
    packager::export_whitepaper().to_string()
}
#[tauri::command]
async fn source_fetch(url: Option<String>) -> Result<Vec<sync::SourceInstance>, String> {
    let installed: Vec<String> = instances::scan_instances()
        .into_iter()
        .map(|i| i.name)
        .collect();

    let normalize = |s: &str| -> String {
        s.to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect()
    };

    let mut urls: Vec<String> = config::OFFICIAL_SOURCE_URLS
        .iter()
        .map(|s| s.to_string())
        .collect();

    let configured_url = url.filter(|u| !u.trim().is_empty()).unwrap_or_else(|| {
        config::Settings::load().source_url
    });
    let trimmed_cfg = configured_url.trim();
    if !trimmed_cfg.is_empty()
        && !trimmed_cfg.contains("Nexusmodpacks")
        && !urls.iter().any(|u| u.eq_ignore_ascii_case(trimmed_cfg))
    {
        urls.push(trimmed_cfg.to_string());
    }

    let mut list: Vec<sync::SourceInstance> = Vec::new();
    for u in &urls {
        match sync::fetch_source_index(u).await {
            Ok(items) => {
                for item in items {
                    let key = normalize(&item.name);
                    if !list.iter().any(|existing| normalize(&existing.name) == key) {
                        list.push(item);
                    }
                }
            }
            Err(e) => {
                eprintln!("[source_fetch] Advertencia al consultar {u}: {e}");
            }
        }
    }

    // Asegurar que las 2 instancias oficiales siempre estén disponibles aunque falle GitHub
    let has_navigay = list.iter().any(|s| {
        let n = normalize(&s.name);
        n == "navigay" || n == "hallogeey"
    });
    if !has_navigay {
        list.push(sync::SourceInstance {
            id: "navigay".to_string(),
            name: "NaviGay".to_string(),
            description: "Pack oficial Ataplaygame".to_string(),
            game_version: "1.21.11".to_string(),
            loader: "fabric".to_string(),
            loader_version: "0.19.5".to_string(),
            manifest_url: "https://raw.githubusercontent.com/kazdev97/Hallogeey/main/manifest.json".to_string(),
            revision: "2026110646847165".to_string(),
            installed: false,
            github_repo: "kazdev97/Hallogeey".to_string(),
            github_path: "files".to_string(),
            github_branch: "main".to_string(),
        });
    }

    if !list.iter().any(|s| normalize(&s.name).contains("paucalipsis")) {
        list.push(sync::SourceInstance {
            id: "paucalipsis-2".to_string(),
            name: "Paucalipsis 2".to_string(),
            description: "Modpack de Paucalipsis 2".to_string(),
            game_version: "1.20.1".to_string(),
            loader: "forge".to_string(),
            loader_version: "47.4.10".to_string(),
            manifest_url: String::new(),
            revision: "latest".to_string(),
            installed: false,
            github_repo: "kazdev97/Paucalipsis-2".to_string(),
            github_path: "pack".to_string(),
            github_branch: "main".to_string(),
        });
    }

    // Completar rutas de sincronización en caso de venir vacías en el índice y renombrar a NaviGay
    for s in list.iter_mut() {
        let norm = normalize(&s.name);
        if norm == "hallogeey" || norm == "navigay" {
            s.name = "NaviGay".to_string();
            if s.github_repo.trim().is_empty() { s.github_repo = "kazdev97/Hallogeey".to_string(); }
            if s.github_branch.trim().is_empty() { s.github_branch = "main".to_string(); }
            if s.github_path.trim().is_empty() { s.github_path = "files".to_string(); }
            if s.manifest_url.trim().is_empty() { s.manifest_url = "https://raw.githubusercontent.com/kazdev97/Hallogeey/main/manifest.json".to_string(); }
        } else if norm.contains("paucalipsis") {
            if s.github_repo.trim().is_empty() { s.github_repo = "kazdev97/Paucalipsis-2".to_string(); }
            if s.github_branch.trim().is_empty() { s.github_branch = "main".to_string(); }
            if s.github_path.trim().is_empty() { s.github_path = "pack".to_string(); }
        }
        let key = normalize(&s.name);
        s.installed = installed.iter().any(|n| {
            let nk = normalize(n);
            nk == key || ((key == "navigay" || key == "hallogeey") && (nk == "navigay" || nk == "hallogeey"))
        });
    }

    Ok(list)
}

#[tauri::command]
async fn source_install(
    app: tauri::AppHandle,
    name: String,
    game_version: String,
    loader: String,
    loader_version: String,
    manifest_url: String,
    github_repo: String,
    github_path: String,
    github_branch: String,
) -> Result<(), String> {
    let handle = app.clone();
    require_access(&name)?;
    sync::install_remote_instance(
        &name,
        &game_version,
        &loader,
        &loader_version,
        &manifest_url,
        &github_repo,
        &github_path,
        &github_branch,
        &mut |status| {
            let _ = handle.emit("sync/status", status);
        },
        &mut |percent, status| {
            let _ = handle.emit("sync/progress", ProgressMsg { percent, status });
        },
    )
    .await
}
#[tauri::command]
async fn install_vanilla_versions() -> Result<Vec<installer::VersionOverview>, String> {
    installer::get_vanilla_versions().await
}

#[tauri::command]
async fn install_fabric_loaders() -> Result<Vec<String>, String> {
    installer::get_fabric_loaders().await
}

#[tauri::command]
async fn install_forge_versions(game_version: String) -> Result<Vec<String>, String> {
    installer::get_forge_versions(&game_version).await
}

#[tauri::command]
async fn install_neoforge_versions(game_version: String) -> Result<Vec<String>, String> {
    installer::get_neoforge_versions(&game_version).await
}

#[tauri::command]
async fn install_instance(app: tauri::AppHandle, name: String) -> Result<String, String> {
    require_access(&name)?;
    let dir = instances::instance_path(&name);
    let _ = &app;
    installer::install_instance(&dir, &mut |_, _| {}).await
}
#[tauri::command]
async fn launch_game(
    app: tauri::AppHandle,
    name: String,
    java_path: String,
) -> Result<u32, String> {
    let mut store = config::AccountStore::load();
    let account = if store.mode == "offline" && !store.offline_username.trim().is_empty() {
        return Err("Inicia sesión con Microsoft para jugar: el modo No premium está deshabilitado por la whitelist.".to_string());
    } else {
        let mut selected = store.selected().cloned().ok_or_else(|| {
            "No hay una cuenta seleccionada (inicia sesión con Microsoft)."
                .to_string()
        })?;
        if !selected.refresh_token.is_empty() {
            match auth::refresh_token(crate::auth::CLIENT_ID, &selected.refresh_token).await {
                Ok((fresh, _)) => {
                    selected = fresh.clone();
                    store.upsert(fresh);
                    let _ = store.save();
                }
                Err(e) => {
                    if e.contains("invalid_grant") {
                        selected.needs_relogin = true;
                        store.upsert(selected.clone());
                        let _ = store.save();
                        return Err(
                            "Tu sesión de Microsoft caducó. Vuelve a iniciar sesión con Relogin en la pestaña Cuenta."
                                .to_string(),
                        );
                    }
                    return Err(format!(
                        "No se pudo renovar tu sesión de Microsoft: {e}. Inténtalo de nuevo o usa Relogin en la pestaña Cuenta."
                    ));
                }
            }
        }
        selected
    };
    if !whitelist::can_access(&account.id, &name) {
        return Err(format!("No tienes acceso a la instancia '{name}'. Contacta con el staff."));
    }
    launch::launch_minecraft(app, &name, &java_path, account).await
}

#[tauri::command]
fn launch_kill() -> Result<(), String> {
    launch::kill_game()
}

#[tauri::command]
fn launch_is_running() -> bool {
    launch::is_running()
}
#[tauri::command]
fn settings_get() -> Result<Settings, String> {
    // Se devuelve una copia sin el token: el frontend solo necesita saber si
    // hay token guardado (publisher_get_status).
    let mut s = Settings::load();
    s.github_token = String::new();
    Ok(s)
}

#[tauri::command]
fn settings_save(
    java_path_override: String,
    streamer_mode: bool,
    ram_override_mb: u32,
    source_url: String,
    auto_update: bool,
    _discord_rpc_enabled: Option<bool>,
) -> Result<(), String> {
    let mut s = Settings::load();
    s.java_path_override = java_path_override;
    s.streamer_mode = streamer_mode;
    s.max_ram_mb = ram_override_mb;
    s.source_url = source_url;
    s.auto_update = auto_update;
    s.discord_rpc_enabled = true;
    discord::set_enabled(true);
    s.save()
}

#[tauri::command]
fn publisher_set_token(token: String) -> Result<(), String> {
    require_owner()?;
    let mut s = Settings::load();
    s.github_token = token.trim().to_string();
    s.save()
}

#[tauri::command]
fn publisher_get_status() -> Result<bool, String> {
    require_owner()?;
    Ok(!Settings::load().github_token.trim().is_empty())
}

#[tauri::command]
async fn pick_folder() -> Result<Option<String>, String> {
    require_owner()?;
    let picked = tokio::task::spawn_blocking(|| {
        rfd::FileDialog::new()
            .set_title("Selecciona la carpeta local de la instancia")
            .pick_folder()
            .map(|p| p.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| e.to_string())?;
    Ok(picked)
}

#[tauri::command]
fn open_folder(path: String) -> Result<(), String> {
    require_owner()?;
    let p = std::path::Path::new(&path);
    if !p.exists() {
        return Err("La ruta indicada no existe.".to_string());
    }
    open::that_detached(p).map_err(|e| format!("abrir carpeta: {e}"))
}

#[tauri::command]
fn instance_get_local_dir(name: String) -> Result<String, String> {
    let dir = instances::instance_path(&name);
    Ok(dir.to_string_lossy().to_string())
}

#[tauri::command]
fn instance_list_publish_folders(name: String) -> Result<Vec<String>, String> {
    require_owner()?;
    sync::list_publish_folders(&name)
}

#[tauri::command]
fn instance_set_github_repo(name: String, repo: String, branch: Option<String>, path: Option<String>) -> Result<(), String> {
    require_owner()?;
    let dir = instances::instance_path(&name);
    if !dir.exists() {
        return Err("Instancia no encontrada".to_string());
    }
    let mut meta = instances::load_meta(&dir);
    meta.github_repo = repo.trim().to_string();
    meta.github_branch = branch.unwrap_or_else(|| "main".to_string()).trim().to_string();
    meta.github_path = path.unwrap_or_default().trim().to_string();
    instances::save_meta(&dir, &meta)
}

#[tauri::command]
async fn instance_publish(name: String, folders: Vec<String>, message: Option<String>) -> Result<String, String> {
    require_owner()?;
    sync::publish_instance(&name, folders, message.unwrap_or_default()).await
}

#[tauri::command]
async fn publisher_publish(
    local_path: String,
    repo: String,
    branch: Option<String>,
    subpath: Option<String>,
    folders: Vec<String>,
    message: Option<String>,
) -> Result<String, String> {
    require_owner()?;
    let b = branch.unwrap_or_else(|| "main".to_string());
    let sub = subpath.unwrap_or_default();
    let msg = message.unwrap_or_default();
    sync::publish_from_folder(&local_path, &repo, &b, &sub, folders, msg).await
}

#[tauri::command]
fn discord_set_enabled(enabled: bool) -> Result<(), String> {
    let mut s = Settings::load();
    s.discord_rpc_enabled = enabled;
    discord::set_enabled(enabled);
    s.save()
}

#[tauri::command]
fn discord_set_page(page: String) -> Result<(), String> {
    discord::set_menu_page(&page);
    Ok(())
}
#[tauri::command]
async fn whitelist_get() -> whitelist::WhitelistView {
    whitelist::sync_from_remote().await;
    whitelist::view()
}
#[tauri::command]
fn whitelist_set_owner(name: String, uuid: String) -> Result<whitelist::WhitelistData, String> {
    whitelist::set_owner(name, uuid)
}
#[tauri::command]
async fn whitelist_add_access(
    name: String,
    uuid: String,
    instances: Vec<String>,
) -> Result<whitelist::WhitelistData, String> {
    let data = whitelist::add_access(name, uuid, instances)?;
    let _ = whitelist::sync_to_github().await;
    Ok(data)
}
#[tauri::command]
async fn whitelist_remove_access(uuid: String) -> Result<whitelist::WhitelistData, String> {
    let data = whitelist::remove_access(uuid)?;
    let _ = whitelist::sync_to_github().await;
    Ok(data)
}
#[tauri::command]
async fn whitelist_add_staff(name: String, uuid: String) -> Result<whitelist::WhitelistData, String> {
    let data = whitelist::add_staff(name, uuid)?;
    let _ = whitelist::sync_to_github().await;
    Ok(data)
}
#[tauri::command]
async fn whitelist_remove_staff(uuid: String) -> Result<whitelist::WhitelistData, String> {
    let data = whitelist::remove_staff(uuid)?;
    let _ = whitelist::sync_to_github().await;
    Ok(data)
}
#[tauri::command]
async fn whitelist_sync_github() -> Result<String, String> {
    whitelist::sync_to_github().await
}
#[tauri::command]
fn open_url(url: String) -> Result<(), String> {
    open::that(url).map_err(|e| format!("abrir URL: {e}"))
}
#[tauri::command]
async fn antilag_install(app: tauri::AppHandle) -> Result<antilag::AntilagStatus, String> {
    let handle = app.clone();
    antilag::antilag_install(move |percent, status| {
        let _ = handle.emit(
            "antilag/progress",
            ProgressMsg { percent, status },
        );
    })
    .await
}

#[tauri::command]
fn antilag_status() -> Result<antilag::AntilagStatus, String> {
    Ok(antilag::status())
}

#[tauri::command]
fn antilag_set_enabled(enabled: bool) -> Result<(), String> {
    let mut s = Settings::load();
    s.antilag_enabled = enabled;
    s.save()
}

#[tauri::command]
fn antilag_set_host(host: String) -> Result<(), String> {
    let mut s = Settings::load();
    s.antilag_host = host.trim().to_string();
    s.save()
}

#[derive(Debug, serde::Serialize)]
struct UuidLookup {
    uuid: String,
    name: String,
}

/// Consulta el UUID de un jugador por su nombre (API oficial de Mojang).
/// Permite que la whitelist se gestione por nombre sin teclear UUIDs a mano.
#[tauri::command]
async fn uuid_lookup(name: String) -> Result<UuidLookup, String> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err("Escribe un nombre de usuario de Minecraft.".to_string());
    }
    let url = format!(
        "https://api.minecraftservices.com/minecraft/profile/lookup/name/{name}"
    );
    let resp = reqwest::get(&url)
        .await
        .map_err(|_| format!("No se pudo consultar el UUID de '{name}'. Revisa tu conexión."))?;
    let status = resp.status();
    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|_| format!("Respuesta inválida al consultar '{name}'."))?;
    if !status.is_success() {
        return Err(format!("El usuario '{name}' no existe en Minecraft."));
    }
    let uuid = v
        .get("id")
        .and_then(|s| s.as_str())
        .ok_or_else(|| format!("Sin UUID para '{name}'."))?;
    let real_name = v
        .get("name")
        .and_then(|s| s.as_str())
        .unwrap_or(&name)
        .to_string();
    Ok(UuidLookup {
        uuid: uuid.to_string(),
        name: real_name,
    })
}

#[tauri::command]
fn set_window_mode(window: tauri::WebviewWindow, mode: String) -> Result<(), String> {
    let mut max_w = 1920.0_f64;
    let mut max_h = 1080.0_f64;
    if let Ok(Some(monitor)) = window.current_monitor() {
        let scale = monitor.scale_factor();
        let size = monitor.size();
        max_w = (size.width as f64) / scale;
        max_h = (size.height as f64) / scale;
    }
    if mode == "gate" {
        if window.is_maximized().unwrap_or(false) {
            let _ = window.unmaximize();
        }
        let target_w = 960.0_f64.min((max_w * 0.92).floor());
        let target_h = 510.0_f64.min((max_h * 0.88).floor());
        let _ = window.set_resizable(false);
        let _ = window.set_min_size(Some(tauri::LogicalSize::new(
            480.0_f64.min(target_w),
            400.0_f64.min(target_h),
        )));
        let _ = window.set_size(tauri::LogicalSize::new(target_w, target_h));
        let _ = window.center();
    } else {
        let target_w = 1180.0_f64.min((max_w * 0.94).floor());
        let target_h = 760.0_f64.min((max_h * 0.92).floor());
        let _ = window.set_resizable(true);
        let _ = window.set_min_size(Some(tauri::LogicalSize::new(
            980.0_f64.min(target_w),
            640.0_f64.min(target_h),
        )));
        let _ = window.set_size(tauri::LogicalSize::new(target_w, target_h));
        let _ = window.center();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    discord::init(true);

    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            login_get_state,
            login_start,
            login_get_account,
            login_set_selected,
            login_remove_account,
            login_set_offline,
            java_get_required,
            java_resolve,
            java_find,
            java_install,
            java_get_system,
            java_get_bundled,
            instances_list,
            instances_create,
            instances_get_meta,
            instances_delete,
            system_info,
            ram_recommend,
            source_fetch,
            source_install,
            sync_check,
            sync_apply,
            sync_rollback,
            sync_list_backups,
            auto_sync_check,
            auto_sync_apply,
            update_check,
            update_apply,
            update_log,
            pack_export,
            pack_whitepaper,
            install_vanilla_versions,
            install_fabric_loaders,
            install_forge_versions,
            install_neoforge_versions,
            install_instance,
            launch_game,
            launch_kill,
            launch_is_running,
            settings_get,
            settings_save,
            publisher_set_token,
            publisher_get_status,
            pick_folder,
            open_folder,
            instance_get_local_dir,
            instance_list_publish_folders,
            instance_set_github_repo,
            instance_publish,
            publisher_publish,
            discord_set_enabled,
            discord_set_page,
            antilag_install,
            antilag_status,
            antilag_set_enabled,
            antilag_set_host,
            open_url,
            whitelist_get,
            whitelist_set_owner,
            whitelist_add_access,
            whitelist_remove_access,
            whitelist_add_staff,
            whitelist_remove_staff,
            whitelist_sync_github,
            uuid_lookup,
            set_window_mode,
        ])
        .run(tauri::generate_context!())
        .expect("error al iniciar NexusLauncher");
}