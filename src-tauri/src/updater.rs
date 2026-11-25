use crate::downloader::{download_file, get_json, http_client};
use serde::Serialize;
use std::fs;
use std::time::Duration;
pub const APP_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const UPDATE_METADATA_URL: &str =
    "https://raw.githubusercontent.com/kazdev97/NexusLauncher/main/version.json";

#[derive(Debug, Clone, Serialize)]
pub struct UpdateInfo {
    pub current: String,
    pub latest: String,
    pub available: bool,
    pub notes: String,
    pub download_url: String,
    pub error: Option<String>,
}

#[derive(serde::Deserialize)]
struct RemoteMeta {
    #[serde(default)]
    version: String,
    #[serde(default)]
    notes: String,
    #[serde(default)]
    url: String,
}
/// Compara versiones como "0.1.2", "0.1.2r", "0.1.2-r" o "0.1.4beta".
/// Los guiones se ignoran para que "0.1.2-r" == "0.1.2r" (evita bucle de
/// actualización). El sufijo se ordena por revisión: sin sufijo < "r" < "beta"/"b".
pub fn version_gt(a: &str, b: &str) -> bool {
    fn parse(s: &str) -> (Vec<i64>, String) {
        let norm = s.trim().to_lowercase().replace(['-', '_', ' '], "");
        let nums = norm
            .split(|c: char| !c.is_ascii_digit())
            .filter_map(|p| p.parse().ok())
            .collect();
        let suffix = norm.chars().filter(|c| c.is_ascii_alphabetic()).collect();
        (nums, suffix)
    }
    fn rank(suffix: &str) -> u8 {
        match suffix {
            "" => 0,
            "r" => 1,
            "b" | "beta" => 2,
            _ => 3,
        }
    }
    let (an, asuf) = parse(a);
    let (bn, bsuf) = parse(b);
    let n = an.len().max(bn.len());
    for i in 0..n {
        let x = *an.get(i).unwrap_or(&0);
        let y = *bn.get(i).unwrap_or(&0);
        if x != y {
            return x > y;
        }
    }
    let (ar, br) = (rank(&asuf), rank(&bsuf));
    if ar != br {
        return ar > br;
    }
    asuf > bsuf
}
fn updates_dir() -> std::path::PathBuf {
    crate::config::launcher_data_dir().join("updates")
}
fn log_file() -> std::path::PathBuf {
    updates_dir().join("update.log")
}
fn marker_file() -> std::path::PathBuf {
    updates_dir().join("applied.json")
}
fn normalize_version(s: &str) -> String {
    s.trim().to_lowercase().replace(['-', '_', ' '], "")
}

/// Devuelve las últimas líneas del registro del actualizador (para diagnosticar
/// un reinicio que se queda colgado).
pub fn read_log() -> String {
    let text = fs::read_to_string(log_file()).unwrap_or_default();
    let lines: Vec<&str> = text.lines().rev().take(40).collect();
    lines.into_iter().rev().collect::<Vec<_>>().join("\n")
}

/// Evita el bucle descargar -> reiniciar -> descargar: si ya aplicamos esta
/// versión y el exe actual coincide con el que se descargó, no volver a ofrecerla.
fn applied_marker_matches(version: &str) -> bool {
    let Ok(text) = fs::read_to_string(marker_file()) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&text) else {
        return false;
    };
    let Some(v) = value.get("version").and_then(|x| x.as_str()) else {
        return false;
    };
    if normalize_version(v) != normalize_version(version) {
        return false;
    }
    let Some(expected) = value.get("sha256").and_then(|x| x.as_str()) else {
        return false;
    };
    if let Ok(cur) = std::env::current_exe() {
        if let Ok(actual) = crate::downloader::sha256_file(&cur) {
            return actual.eq_ignore_ascii_case(expected);
        }
    }
    false
}

pub async fn check_update() -> Result<UpdateInfo, String> {
    let client = http_client();
    let meta: RemoteMeta =
        get_json(&client, UPDATE_METADATA_URL, Duration::from_secs(30)).await.map_err(|e| {
            format!("no se pudo consultar versiones ({UPDATE_METADATA_URL}): {e}")
        })?;
    let latest = meta.version.trim().to_string();
    let mut available = !latest.is_empty() && version_gt(&latest, APP_VERSION);
    if available && applied_marker_matches(&latest) {
        available = false;
    }
    Ok(UpdateInfo {
        current: APP_VERSION.to_string(),
        latest,
        available,
        notes: meta.notes,
        download_url: meta.url,
        error: None,
    })
}
pub async fn apply_update() -> Result<(), String> {
    let info = check_update().await?;
    if !info.available || info.download_url.trim().is_empty() {
        return Err("No hay una versión más reciente para instalar.".to_string());
    }

    let dir = updates_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("crear carpeta de updates: {e}"))?;
    let new_exe = dir.join(format!("nexuslauncher-{}.new.exe", info.latest.replace(['/', '\\', ':'], "_")));
    let client = http_client();
    download_file(&client, &info.download_url, &new_exe, &mut |_, _| {})
        .await
        .map_err(|e| format!("descargar la actualización: {e}"))?;
    let cur_exe = std::env::current_exe().map_err(|e| format!("localizar el exe actual: {e}"))?;
    // Guardia anti-bucle: si el binario descargado es idéntico al actual
    // (p. ej. version.json apunta al exe viejo), no reiniciar.
    if let (Ok(a), Ok(b)) = (
        crate::downloader::sha256_file(&new_exe),
        crate::downloader::sha256_file(&cur_exe),
    ) {
        if a.eq_ignore_ascii_case(&b) {
            fs::remove_file(&new_exe).ok();
            return Err("La actualización descargada es idéntica a la versión actual; se omite el reinicio.".to_string());
        }
    }

    let new_hash = crate::downloader::sha256_file(&new_exe).unwrap_or_default();
    fs::write(
        marker_file(),
        serde_json::json!({ "version": info.latest, "sha256": new_hash }).to_string(),
    )
    .ok();

    let pid = std::process::id();
    let cur_str = cur_exe.to_string_lossy().replace('\'', "''");
    let new_str = new_exe.to_string_lossy().replace('\'', "''");
    let log_str = log_file().to_string_lossy().replace('\'', "''");
    let latest = info.latest.replace('\'', "''");
    fs::write(
        log_file(),
        format!("Iniciando actualización a v{latest} (PID {pid})\n"),
    )
    .ok();

    // Se detiene por PID (no por nombre: el usuario puede haber renombrado el
    // exe), se espera a que suelte el archivo, se reintentan las copias y se
    // escribe un registro para poder diagnosticar un reinicio fallido.
    let ps = format!(
        r#"$ErrorActionPreference = 'Stop'
$log = '{log}'
function Log {{
  param($m)
  Add-Content -LiteralPath $log -Value ("[{{0}}] {{1}}" -f (Get-Date -Format 'yyyy-MM-dd HH:mm:ss'), $m)
}}
Log 'Deteniendo el launcher (PID {pid})'
Stop-Process -Id {pid} -Force -ErrorAction SilentlyContinue
for ($i = 0; $i -lt 30; $i++) {{
  if (-not (Get-Process -Id {pid} -ErrorAction SilentlyContinue)) {{ break }}
  Start-Sleep -Milliseconds 500
}}
Log 'Proceso detenido'
$copied = $false
for ($i = 0; $i -lt 20; $i++) {{
  try {{
    Copy-Item -LiteralPath '{new}' -Destination '{cur}' -Force -ErrorAction Stop
    $copied = $true
    break
  }} catch {{
    Start-Sleep -Milliseconds 500
  }}
}}
if (-not $copied) {{ Log 'ERROR: no se pudo copiar el ejecutable'; exit 1 }}
Log 'Ejecutable reemplazado'
Remove-Item -LiteralPath '{new}' -Force -ErrorAction SilentlyContinue
try {{
  Start-Process -FilePath '{cur}'
  Log 'Nueva instancia iniciada'
}} catch {{
  Log ('ERROR al iniciar: ' + $_.Exception.Message)
  exit 1
}}
"#,
        log = log_str,
        pid = pid,
        new = new_str,
        cur = cur_str,
    );
    let script = dir.join("apply_update.ps1");
    fs::write(&script, &ps).map_err(|e| format!("escribir script de actualización: {e}"))?;

    let mut cmd = std::process::Command::new("powershell");
    cmd.arg("-NoProfile")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-WindowStyle")
        .arg("Hidden")
        .arg("-File")
        .arg(&script);
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
    }
    cmd.spawn()
        .map_err(|e| format!("lanzar el instalador de la actualización: {e}"))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::version_gt;

    #[test]
    fn revision_is_newer_than_plain_but_not_repeated() {
        assert!(version_gt("0.1.2r", "0.1.2"));
        assert!(version_gt("0.1.2-r", "0.1.2"));
        assert!(!version_gt("0.1.2-r", "0.1.2r"));
        assert!(!version_gt("0.1.2r", "0.1.2-r"));
        assert!(version_gt("0.1.3", "0.1.2r"));
        assert!(!version_gt("0.1.2", "0.1.2r"));
        assert!(version_gt("0.1.4beta", "0.1.4r"));
        assert!(version_gt("0.1.4-beta", "0.1.4r"));
        assert!(version_gt("0.1.4b", "0.1.4r"));
        assert!(!version_gt("0.1.4r", "0.1.4beta"));
        assert!(!version_gt("0.1.4beta", "0.1.4-beta"));
    }
}