// Sistema de whitelist por UUID de Minecraft.
//
// Se guarda en whitelist.json dentro de launcher_data_dir:
// {
//   "owner": "uuid-del-propietario-o-vacio",
//   "staff": [{"name": "...", "uuid": "..."}],
//   "access": [
//     {"name": "...", "uuid": "...", "instances": ["*"]}
//   ]
// }
//
// - El propietario (owner) puede gestionarlo todo.
// - El staff puede ver/sincronizar instancias y agregar/borrar entradas
//   de acceso (whitelist), pero no puede cambiar el propietario.
// - Cualquier otro usuario solo puede ver las instancias que tenga en
//   `access` para su UUID. "*" significa TODAS las instancias.
// - La comparacion de UUIDs es insensible a mayusculas y guiones porque
//   Minecraft a veces devuelve el UUID sin guiones.

use crate::config::{whitelist_file, AccountStore};
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WhitelistEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uuid: String,
    /// Nombres de instancia permitidos o ["*"] para todas.
    #[serde(default)]
    pub instances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StaffEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub uuid: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct WhitelistData {
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub staff: Vec<StaffEntry>,
    #[serde(default)]
    pub access: Vec<WhitelistEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WhitelistView {
    pub owner: String,
    pub staff: Vec<StaffEntry>,
    pub access: Vec<WhitelistEntry>,
    pub current_uuid: String,
    pub current_name: String,
    pub is_owner: bool,
    pub is_staff: bool,
    /// Instancias a las que tiene acceso el usuario actual.
    /// ["*"] significa todas. Si esta vacio: sin acceso.
    pub allowed_instances: Vec<String>,
    /// true si el usuario actual puede administrar (owner o staff).
    pub can_admin: bool,
}

/// Propietario fijo en el código: siempre Soy_Kaz. No se puede cambiar desde UI.
pub const HARDCODED_OWNER: &str = "4b2e6d4e1a454ffdab9c492b3f26e44f";

/// Staff predefinido en código para que tengan acceso garantizado en cualquier equipo.
pub const DEFAULT_STAFF: &[(&str, &str)] = &[
    ("ylstae", "5d5354b8fc0048c48c50cde53b78329c"),
    ("CiervoGatuno", "32ff9c948b38447093ca580de4474586"),
    ("Elarhy", "d88b538e754748f6917e21a181c0c0d1"),
];

pub fn normalize_uuid(uuid: &str) -> String {
    uuid.trim()
        .to_lowercase()
        .replace('-', "")
        .replace(' ', "")
}

pub fn load() -> WhitelistData {
    let mut data = WhitelistData::default();
    let path = whitelist_file();
    if let Ok(text) = fs::read_to_string(&path) {
        if let Ok(parsed) = serde_json::from_str::<WhitelistData>(&text) {
            data = parsed;
        }
    }
    // El propietario es fijo en código, sin importar el archivo.
    data.owner = HARDCODED_OWNER.to_string();
    // Asegurar que el staff predefinido siempre esté presente
    for (name, uuid) in DEFAULT_STAFF {
        let norm = normalize_uuid(uuid);
        if !data.staff.iter().any(|s| normalize_uuid(&s.uuid) == norm) {
            data.staff.push(StaffEntry {
                name: name.to_string(),
                uuid: uuid.to_string(),
            });
        }
    }
    data
}

pub fn save(data: &WhitelistData) -> Result<(), String> {
    let json = serde_json::to_string_pretty(data)
        .map_err(|e| format!("serialize whitelist: {e}"))?;
    fs::write(whitelist_file(), json).map_err(|e| format!("guardar whitelist: {e}"))
}

fn current_account() -> Option<crate::config::Account> {
    let store = AccountStore::load();
    if store.mode == "offline" || store.selected_id.trim().is_empty() {
        return None;
    }
    store.selected().cloned()
}

pub fn is_owner(uuid: &str) -> bool {
    // Propietario hardcodeado: siempre Soy_Kaz, no depende del archivo.
    normalize_uuid(uuid) == normalize_uuid(HARDCODED_OWNER)
}

pub fn is_staff(uuid: &str) -> bool {
    if is_owner(uuid) {
        return true;
    }
    let data = load();
    let norm = normalize_uuid(uuid);
    data.staff
        .iter()
        .any(|s| normalize_uuid(&s.uuid) == norm)
}

pub fn is_admin(uuid: &str) -> bool {
    is_owner(uuid) || is_staff(uuid)
}

/// Instancias permitidas para un UUID. ["*"] = todas, [] = ninguna.
pub fn allowed_for(uuid: &str) -> Vec<String> {
    if is_admin(uuid) {
        return vec!["*".to_string()];
    }
    let data = load();
    let norm = normalize_uuid(uuid);
    for entry in &data.access {
        if normalize_uuid(&entry.uuid) == norm {
            if entry.instances.is_empty() {
                return Vec::new();
            }
            return entry.instances.clone();
        }
    }
    Vec::new()
}

pub fn can_access(uuid: &str, instance_name: &str) -> bool {
    if is_admin(uuid) {
        return true;
    }
    let allowed = allowed_for(uuid);
    if allowed.contains(&"*".to_string()) {
        return true;
    }
    let target = instance_name.trim().to_lowercase();
    allowed.iter().any(|a| {
        let norm_a = a.trim().to_lowercase();
        if norm_a == target {
            return true;
        }
        if (norm_a == "hallogeey" || norm_a == "navigay") && (target == "hallogeey" || target == "navigay") {
            return true;
        }
        false
    })
}

fn check_admin(_action: &str) -> Result<String, String> {
    let account = current_account()
        .ok_or_else(|| "Debes iniciar sesión con Microsoft para esta acción.".to_string())?;
    if !is_admin(&account.id) {
        return Err(
            "Solo el propietario o el staff autorizado puede gestionar la whitelist."
                .to_string(),
        );
    }
    Ok(account.id)
}

pub fn view() -> WhitelistView {
    let data = load();
    let account = current_account();
    let current_uuid = account
        .as_ref()
        .map(|a| a.id.clone())
        .unwrap_or_default();
    let current_name = account
        .as_ref()
        .map(|a| a.name.clone())
        .unwrap_or_default();

    let owner_flag = !current_uuid.is_empty() && is_owner(&current_uuid);
    let staff_flag = !current_uuid.is_empty() && is_staff(&current_uuid);

    let allowed = if current_uuid.is_empty() {
        Vec::new()
    } else {
        allowed_for(&current_uuid)
    };

    WhitelistView {
        owner: data.owner.clone(),
        staff: data.staff.clone(),
        access: data.access.clone(),
        allowed_instances: allowed,
        can_admin: owner_flag || staff_flag,
        is_owner: owner_flag,
        is_staff: staff_flag,
        current_uuid,
        current_name,
    }
}

/// Definir el propietario: fijado en código, siempre Soy_Kaz.
pub fn set_owner(_name: String, _uuid: String) -> Result<WhitelistData, String> {
    Err("El propietario está fijado en el código (Soy_Kaz) y no se puede cambiar.".to_string())
}

pub fn add_access(name: String, uuid: String, instances: Vec<String>) -> Result<WhitelistData, String> {
    check_admin("agregar whitelist")?;
    let norm = normalize_uuid(&uuid);
    if norm.is_empty() || norm.len() < 8 {
        return Err("UUID inválido.".to_string());
    }
    let clean_name = name.trim().to_string();
    let mut data = load();
    let clean_instances: Vec<String> = instances
        .iter()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();
    let clean_instances = if clean_instances.is_empty() {
        vec!["*".to_string()]
    } else {
        clean_instances
    };
    if let Some(existing) = data
        .access
        .iter_mut()
        .find(|e| normalize_uuid(&e.uuid) == norm)
    {
        if !clean_name.is_empty() {
            existing.name = clean_name;
        }
        existing.instances = clean_instances;
    } else {
        data.access.push(WhitelistEntry {
            name: clean_name,
            uuid: uuid.trim().to_string(),
            instances: clean_instances,
        });
    }
    save(&data)?;
    Ok(data)
}

pub fn remove_access(uuid: String) -> Result<WhitelistData, String> {
    check_admin("borrar whitelist")?;
    let norm = normalize_uuid(&uuid);
    let mut data = load();
    data.access.retain(|e| normalize_uuid(&e.uuid) != norm);
    save(&data)?;
    Ok(data)
}

pub fn add_staff(name: String, uuid: String) -> Result<WhitelistData, String> {
    // Solo el propietario gestiona el staff.
    let account = current_account()
        .ok_or_else(|| "Debes iniciar sesión con Microsoft para esta acción.".to_string())?;
    if !is_owner(&account.id) {
        return Err("Solo el propietario puede gestionar el staff.".to_string());
    }
    let norm = normalize_uuid(&uuid);
    if norm.is_empty() || norm.len() < 8 {
        return Err("UUID inválido.".to_string());
    }
    let mut data = load();
    if !data.staff.iter().any(|s| normalize_uuid(&s.uuid) == norm) {
        data.staff.push(StaffEntry {
            name: name.trim().to_string(),
            uuid: uuid.trim().to_string(),
        });
    }
    save(&data)?;
    Ok(data)
}

pub fn remove_staff(uuid: String) -> Result<WhitelistData, String> {
    let account = current_account()
        .ok_or_else(|| "Debes iniciar sesión con Microsoft para esta acción.".to_string())?;
    if !is_owner(&account.id) {
        return Err("Solo el propietario puede gestionar el staff.".to_string());
    }
    let norm = normalize_uuid(&uuid);
    let mut data = load();
    data.staff.retain(|s| normalize_uuid(&s.uuid) != norm);
    save(&data)?;
    Ok(data)
}

/// Descarga la whitelist remota desde GitHub para que todos los usuarios tengan
/// los permisos actualizados en tiempo real.
pub async fn sync_from_remote() {
    let client = crate::downloader::http_client();
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let url = format!("https://raw.githubusercontent.com/kazdev97/Hallogeey/main/whitelist.json?t={now}");
    
    let Ok(resp) = client.get(&url).header("User-Agent", "NexusLauncher/0.1").send().await else {
        return;
    };
    if !resp.status().is_success() {
        return;
    }
    let Ok(text) = resp.text().await else {
        return;
    };
    let Ok(mut remote_data) = serde_json::from_str::<WhitelistData>(&text) else {
        return;
    };
    
    remote_data.owner = HARDCODED_OWNER.to_string();
    for (name, uuid) in DEFAULT_STAFF {
        let norm = normalize_uuid(uuid);
        if !remote_data.staff.iter().any(|s| normalize_uuid(&s.uuid) == norm) {
            remote_data.staff.push(StaffEntry {
                name: name.to_string(),
                uuid: uuid.to_string(),
            });
        }
    }
    let _ = save(&remote_data);
}

/// Sube la whitelist actual a GitHub (kazdev97/Hallogeey/main/whitelist.json) usando el token del owner.
pub async fn sync_to_github() -> Result<String, String> {
    let token = crate::config::Settings::load().github_token.trim().to_string();
    if token.is_empty() {
        return Err("No hay Token de GitHub configurado en Ajustes. Guárdalo para sincronizar la whitelist en la nube.".to_string());
    }
    let data = load();
    let json = serde_json::to_string_pretty(&data)
        .map_err(|e| format!("serialize whitelist: {e}"))?;
    
    let client = crate::downloader::http_client();
    let api_url = "https://api.github.com/repos/kazdev97/Hallogeey/contents/whitelist.json";
    
    // Obtener sha del archivo existente si existe
    let req = client
        .get(api_url)
        .header("User-Agent", "NexusLauncher/0.1")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json");
    
    let mut sha: Option<String> = None;
    if let Ok(resp) = req.send().await {
        if resp.status().is_success() {
            if let Ok(val) = resp.json::<serde_json::Value>().await {
                if let Some(s) = val.get("sha").and_then(|v| v.as_str()) {
                    sha = Some(s.to_string());
                }
            }
        }
    }
    
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD.encode(json.as_bytes());
    let mut body = serde_json::json!({
        "message": "Update whitelist.json from NexusLauncher",
        "content": b64,
        "branch": "main"
    });
    if let Some(s) = sha {
        body["sha"] = serde_json::json!(s);
    }
    
    let put_resp = client
        .put(api_url)
        .header("User-Agent", "NexusLauncher/0.1")
        .header("Authorization", format!("Bearer {token}"))
        .header("Accept", "application/vnd.github+json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Error conectando con GitHub API: {e}"))?;
    
    let status = put_resp.status();
    if !status.is_success() {
        let err_text = put_resp.text().await.unwrap_or_default();
        return Err(format!("GitHub API rechazó la subida (HTTP {status}): {err_text}"));
    }
    
    Ok("Whitelist sincronizada en GitHub (kazdev97/Hallogeey) correctamente.".to_string())
}
