// Cliente de la API del VPS: fuente de verdad para staff, whitelist e instancias.
use crate::config::Settings;
use crate::whitelist::StaffEntry;
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};
use serde::{Deserialize, Serialize};

pub const API_BASE: &str = "https://t-eje4myb6.go.altare.global";

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ApiSession {
    pub token: String,
    pub uuid: String,
    pub name: String,
    pub role: String,
    #[serde(default)]
    pub access: Vec<String>,
}

#[derive(Deserialize)]
struct MeResponse {
    uuid: String,
    name: String,
    role: String,
    #[serde(default)]
    access: Vec<String>,
}

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(20))
        .build()
        .unwrap_or_default()
}

fn authed(token: &str) -> HeaderMap {
    let mut h = HeaderMap::new();
    if let Ok(v) = HeaderValue::from_str(&format!("Bearer {}", token)) {
        h.insert(AUTHORIZATION, v);
    }
    h
}

/// Intercambia el access token de Minecraft por una sesion JWT del VPS.
pub async fn login_with_microsoft(mc_access_token: &str) -> Result<ApiSession, String> {
    let res = client()
        .post(format!("{}/auth/microsoft", API_BASE))
        .json(&serde_json::json!({ "access_token": mc_access_token }))
        .send()
        .await
        .map_err(|e| format!("No se pudo conectar con el VPS: {}", e))?;
    if !res.status().is_success() {
        return Err(format!("El VPS rechazo el inicio de sesion ({})", res.status()));
    }
    res.json::<ApiSession>()
        .await
        .map_err(|e| format!("Respuesta del VPS invalida: {}", e))
}

/// Revalida la sesion guardada y devuelve rol/accesos actualizados.
pub async fn me(token: &str) -> Result<ApiSession, String> {
    let res = client()
        .get(format!("{}/me", API_BASE))
        .headers(authed(token))
        .send()
        .await
        .map_err(|e| format!("VPS no accesible: {}", e))?;
    if !res.status().is_success() {
        return Err("Sesion del VPS caducada".into());
    }
    let me = res.json::<MeResponse>().await.map_err(|e| e.to_string())?;
    Ok(ApiSession {
        token: token.to_string(),
        uuid: me.uuid,
        name: me.name,
        role: me.role,
        access: me.access,
    })
}

pub fn load_token() -> Option<String> {
    let s = Settings::load();
    let t = s.api_token.trim().to_string();
    if t.is_empty() { None } else { Some(t) }
}

pub fn save_session(session: &ApiSession) {
    let mut s = Settings::load();
    s.api_token = session.token.clone();
    s.api_role = session.role.clone();
    let _ = s.save();
}

pub fn clear_session() {
    let mut s = Settings::load();
    s.api_token = String::new();
    s.api_role = String::new();
    let _ = s.save();
}

pub fn cached_role() -> String {
    Settings::load().api_role
}

pub async fn whitelist_get(token: &str) -> Result<serde_json::Value, String> {
    let res = client()
        .get(format!("{}/whitelist", API_BASE))
        .headers(authed(token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("No se pudo leer la whitelist ({})", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn whitelist_add(token: &str, instance: &str, name: &str) -> Result<(), String> {
    let res = client()
        .post(format!("{}/whitelist", API_BASE))
        .headers(authed(token))
        .json(&serde_json::json!({ "instance": instance, "name": name }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("No autorizado para agregar whitelist ({})", res.status()));
    }
    Ok(())
}

pub async fn whitelist_remove(token: &str, instance: &str, name: &str) -> Result<(), String> {
    let res = client()
        .delete(format!("{}/whitelist", API_BASE))
        .headers(authed(token))
        .json(&serde_json::json!({ "instance": instance, "name": name }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("No autorizado para borrar whitelist ({})", res.status()));
    }
    Ok(())
}

pub async fn staff_list(token: &str) -> Result<Vec<StaffEntry>, String> {
    let res = client()
        .get(format!("{}/staff", API_BASE))
        .headers(authed(token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err("Solo el owner puede ver el staff".into());
    }
    let v: serde_json::Value = res.json().await.map_err(|e| e.to_string())?;
    let staff = v["staff"]
        .as_array()
        .map(|a| {
            a.iter()
                .map(|s| StaffEntry {
                    name: s["name"].as_str().unwrap_or("").to_string(),
                    uuid: s["uuid"].as_str().unwrap_or("").to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Ok(staff)
}

pub async fn staff_add(token: &str, name: &str, uuid: &str) -> Result<(), String> {
    let res = client()
        .post(format!("{}/staff", API_BASE))
        .headers(authed(token))
        .json(&serde_json::json!({ "name": name, "uuid": uuid }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err("Solo el owner puede agregar staff".into());
    }
    Ok(())
}

pub async fn staff_remove(token: &str, uuid: &str) -> Result<(), String> {
    let res = client()
        .delete(format!("{}/staff", API_BASE))
        .headers(authed(token))
        .json(&serde_json::json!({ "uuid": uuid }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err("Solo el owner puede borrar staff".into());
    }
    Ok(())
}

pub async fn instances_list(token: &str) -> Result<serde_json::Value, String> {
    let res = client()
        .get(format!("{}/instances", API_BASE))
        .headers(authed(token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("No se pudieron cargar las instancias ({})", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn instance_upsert(
    token: &str,
    name: &str,
    slug: &str,
    description: &str,
    mc_version: &str,
    loader: &str,
    loader_version: &str,
) -> Result<(), String> {
    let res = client()
        .post(format!("{}/instances", API_BASE))
        .headers(authed(token))
        .json(&serde_json::json!({
            "name": name,
            "slug": slug,
            "description": description,
            "mc_version": mc_version,
            "loader": loader,
            "loader_version": loader_version,
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Solo el owner puede crear o actualizar instancias ({})", res.status()));
    }
    Ok(())
}

pub async fn instance_delete(token: &str, slug: &str) -> Result<(), String> {
    let res = client()
        .delete(format!("{}/instances", API_BASE))
        .headers(authed(token))
        .json(&serde_json::json!({ "slug": slug }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("No se pudo eliminar la instancia ({})", res.status()));
    }
    Ok(())
}

pub async fn instance_publish(
    token: &str,
    slug: &str,
    files: Vec<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let res = client()
        .post(format!("{}/instances/publish", API_BASE))
        .headers(authed(token))
        .json(&serde_json::json!({ "slug": slug, "files": files }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("El VPS rechazo la publicacion ({})", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn instance_manifest(token: &str, slug: &str) -> Result<serde_json::Value, String> {
    let res = client()
        .get(format!("{}/instances/manifest/{}", API_BASE, slug))
        .headers(authed(token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("No se pudo obtener el manifiesto ({})", res.status()));
    }
    res.json().await.map_err(|e| e.to_string())
}

pub async fn instance_download_file(token: &str, slug: &str, rel_path: &str) -> Result<Vec<u8>, String> {
    let encoded = rel_path
        .split('/')
        .map(|seg| percent_encoding::utf8_percent_encode(seg, percent_encoding::NON_ALPHANUMERIC).to_string())
        .collect::<Vec<_>>()
        .join("/");
    let res = client()
        .get(format!("{}/instances/file/{}/{}", API_BASE, slug, encoded))
        .headers(authed(token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !res.status().is_success() {
        return Err(format!("Error descargando {} ({})", rel_path, res.status()));
    }
    let bytes = res.bytes().await.map_err(|e| e.to_string())?;
    Ok(bytes.to_vec())
}
