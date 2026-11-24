const DEMO = !window.__TAURI__;

const $ = (id) => document.getElementById(id);
const status = (id, msg) => { $(id).textContent = msg || ""; };
const demoStore = { accounts: [], selected_id: "", mode: "offline", offline_username: "Juanito67" };
const demoWhitelist = { owner: "4b2e6d4e1a454ffdab9c492b3f26e44f", staff: [{ name: "Admin", uuid: "demo-uuid-admin-0000-0000" }], access: [{ name: "Jugador", uuid: "demo-uuid-player-0000-000", instances: ["*"] }], current_uuid: "", current_name: "", is_owner: false, is_staff: false, allowed_instances: ["*"], can_admin: false };
let wl = null;
let currentUuid = "";
let isAdmin = false;
let isOwner = false;
let userAccessInstances = null; // null = todas, [] = ninguna, array = lista
let gateReady = false;
const demoInstances = [
  { name: "Vanilla 1.21.4", version_id: "1.21.4", loader: "vanilla", game_version: "1.21.4", loader_version: "", source: "local", manifest_url: "", manifest_type: "", server_version: "", actualizacion: true, installed: true, instance_dir: "C:\\Documentos\\NexusLauncher\\instancias\\Vanilla 1.21.4", last_launched: "2026-09-14 18:00", session_based: false, remote: false, github_repo: "", github_branch: "", github_path: "" },
  { name: "Server de Kaz", version_id: "1.21.4-fabric-0.16.14", loader: "fabric", game_version: "1.21.4", loader_version: "0.16.14", source: "remote", manifest_url: "https://drive.google.com/uc?export=download&id=abc123", manifest_type: "pau", server_version: "2026-09-13", actualizacion: true, installed: true, instance_dir: "C:\\Documentos\\NexusLauncher\\instancias\\Server de Kaz", last_launched: "2026-09-13 21:30", session_based: false, remote: true, github_repo: "kazdev97/Nexusmodpacks", github_branch: "main", github_path: "pack" },
  { name: "Pack de Test", version_id: "", loader: "fabric", game_version: "1.20.1", loader_version: "", source: "remote", manifest_url: "https://drive.google.com/uc?export=download&id=xyz789", manifest_type: "", server_version: "", actualizacion: true, installed: false, instance_dir: "C:\\Documentos\\NexusLauncher\\instancias\\Pack de Test", last_launched: "", session_based: false, remote: true, github_repo: "", github_branch: "", github_path: "" },
];
const demoJava = [
  { path: "C:\\Documentos\\NexusLauncher\\runtime\\jdk-21\\bin\\java.exe", major: 21, is_valid: true },
  { path: "C:\\Program Files\\Java\\jdk-17.0.10\\bin\\java.exe", major: 17, is_valid: true },
];
const demoVersions = [
  { id: "1.21.4", kind: "release", release_time: "2024-12-03T18:21:00+00:00" },
  { id: "1.20.1", kind: "release", release_time: "2023-06-12T12:11:00+00:00" },
];
const demoSettings = { ram_mb: 4096, max_ram_mb: 0, java_path_override: "", streamer_mode: true, close_launcher_after_launch: false, game_res_width: 0, game_res_height: 0, fullscreen: false, last_username: "", source_url: "https://raw.githubusercontent.com/kazdev97/Hallogeey/main/index.json", auto_update: true };
const demoSource = [
  { id: "navigay", name: "NaviGay", description: "Pack oficial Ataplaygame", game_version: "1.21.11", loader: "fabric", loader_version: "0.19.5", manifest_url: "https://raw.githubusercontent.com/kazdev97/Hallogeey/main/manifest.json", revision: "2026110646847165", installed: false, github_repo: "kazdev97/Hallogeey", github_path: "files", github_branch: "main" },
  { id: "paucalipsis-2", name: "Paucalipsis 2", description: "Modpack de Paucalipsis 2", game_version: "1.20.1", loader: "forge", loader_version: "47.4.10", manifest_url: "", revision: "latest", installed: false, github_repo: "kazdev97/Paucalipsis-2", github_path: "pack", github_branch: "main" },
];
const demoDiff = { revision: "20260914T120000Z", up_to_date: false, missing: ["mods/jei-1.21.4.jar", "config/server.properties"], different: ["mods/usefulmod.jar"], extra: ["mods/lag_fix.jar", "config/old_setting.toml"], change_count: 5, has_archive: true };
const demoSyncResult = { applied_revision: "20260914T120000Z", replaced: ["mods/usefulmod.jar"], downloaded: ["mods/jei-1.21.4.jar", "config/server.properties"], removed: ["mods/lag_fix.jar"], backup_dir: "C:\\Documentos\\NexusLauncher\\instancias\\Server de Kaz\\backups\\20260914T120000Z", errors: [] };

function demoCall(cmd) {
  const map = {
    login_get_state: () => demoStore,
    login_start: () => ({ id: "demo-uuid-player-0000-000", name: "JugadorDemo", needs_relogin: false }),
    login_set_offline: () => { throw new Error("Modo offline deshabilitado por política del servidor."); },
    login_set_selected: () => undefined,
    login_remove_account: () => { demoStore.accounts = []; demoStore.selected_id = ""; return undefined; },
    whitelist_get: () => demoWhitelist,
    whitelist_set_owner: () => demoWhitelist,
    whitelist_add_access: () => demoWhitelist,
    whitelist_remove_access: () => demoWhitelist,
    whitelist_add_staff: () => demoWhitelist,
    whitelist_remove_staff: () => demoWhitelist,
    uuid_lookup: () => ({ uuid: "12345678-1234-1234-1234-1234567890ab", name: "JugadorDemo" }),
    instances_list: () => demoInstances,
    java_get_bundled: () => demoJava.filter((j) => j.path.toLowerCase().includes("runtime")),
    java_get_system: () => demoJava.filter((j) => !j.path.toLowerCase().includes("runtime")),
    install_vanilla_versions: () => demoVersions,
    install_fabric_loaders: () => ["0.16.14", "0.16.10"],
    sync_list_backups: () => ["20260914_120000", "20260910_090000", "20260901_140000"],
    sync_check: () => demoDiff,
    sync_apply: () => demoSyncResult,
    settings_get: () => demoSettings,
    system_info: () => ({ cpu_cores: 16, total_ram_mb: 32768 }),
    ram_recommend: () => ({ total_ram_gb: 32, min_mb: 1536, max_mb: 8192 }),
    source_fetch: () => demoSource,
    source_install: () => undefined,
    instances_create: () => undefined,
    sync_all: () => ({ updated: demoInstances.map(i => i.name), checked: demoInstances.length, errors: [] }),
    auto_sync_check: () => [
      { name: "Server de Kaz", up_to_date: false, change_count: 5, revision: "20260914T120000Z", error: null },
      { name: "Vanilla 1.21.4", up_to_date: true, change_count: 0, revision: "", error: null },
    ],
    auto_sync_apply: () => ({ applied: ["Server de Kaz (revisión 20260914T120000Z)"], already_up_to_date: ["Vanilla 1.21.4"], errors: [] }),
    java_resolve: () => ({ path: "C:\\Documentos\\NexusLauncher\\runtime\\jdk-21\\bin\\java.exe", major: 21, is_valid: true }),
    launch_game: () => 4321,
    launch_is_running: () => false,
    update_check: () => ({ current: "0.1.0", latest: "0.2.0", available: true, notes: "Mejoras de compatibilidad", download_url: "" }),
    update_apply: () => undefined,
    publisher_set_token: () => undefined,
    publisher_get_status: () => true,
    pick_folder: () => "C:\\Users\\Kaz\\AppData\\Roaming\\.minecraft",
    open_folder: () => undefined,
    instance_get_local_dir: (args) => `E:\\LAUNCHER NEXUS\\LAUNCHER NEXUS\\instances\\${args?.name || 'Navigay'}`,
    instance_list_publish_folders: () => ["config", "mods", "resourcepacks", "shaderpacks", "scripts"],
    instance_set_github_repo: () => undefined,
    instance_publish: () => "demo0000000000000000000000000000000",
    publisher_publish: () => "demo0000000000000000000000000000000",
    whitelist_sync_github: () => "Whitelist sincronizada en GitHub (modo demo).",
    update_log: () => "(registro no disponible en modo demo)",
  };
  const fn = map[cmd];
  return Promise.resolve(fn ? fn() : undefined);
}

const demoEvent = {
  listen: (_name, _cb) => Promise.resolve(() => {}),
};

const consoleBuf = [];
const CONSOLE_MAX_LINES = 4000;

function appendLog(line, stream) {
  const pre = $("console");
  const wasAtBottom = pre.scrollHeight - pre.scrollTop - pre.clientHeight < 24;
  consoleBuf.push((stream === "stderr" ? "[err] " : "") + line);
  if (consoleBuf.length > CONSOLE_MAX_LINES) {
    consoleBuf.splice(0, consoleBuf.length - Math.floor(CONSOLE_MAX_LINES / 2));
  }
  pre.textContent = consoleBuf.join("\n") + "\n";
  if (wasAtBottom) pre.scrollTop = pre.scrollHeight;
}

function copyConsole() {
  const text = $("console").textContent;
  const done = () => status("game-state", "Consola copiada al portapapeles.");
  const fallback = () => {
    const ta = document.createElement("textarea");
    ta.value = text;
    ta.style.position = "fixed";
    ta.style.opacity = "0";
    document.body.appendChild(ta);
    ta.select();
    try { document.execCommand("copy"); done(); }
    catch (e) { status("game-state", "No se pudo copiar la consola."); }
    document.body.removeChild(ta);
  };
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(text).then(done).catch(fallback);
  } else {
    fallback();
  }
}

$("btn-copy-console").addEventListener("click", copyConsole);
$("btn-clear-console").addEventListener("click", () => {
  consoleBuf.length = 0;
  $("console").textContent = "";
  status("game-state", "Consola limpiada.");
});

const EVENT = DEMO ? demoEvent : window.__TAURI__.event;

async function call(cmd, args) {
  if (DEMO) return demoCall(cmd, args);
  try {
    return await window.__TAURI__.core.invoke(cmd, args || {});
  } catch (e) {
    throw new Error(typeof e === "string" ? e : (e && e.message) || "error desconocido");
  }
}
document.querySelectorAll(".nav-item").forEach((btn) => {
  btn.addEventListener("click", () => {
    document.querySelectorAll(".nav-item").forEach((b) => b.classList.remove("active"));
    document.querySelectorAll(".page").forEach((p) => p.classList.remove("active"));
    btn.classList.add("active");
    $("page-" + btn.dataset.page).classList.add("active");
    if (btn.dataset.page === "instances") {
      loadWhitelist().then(() => {
        loadInstances();
        loadSource();
      }).catch(() => {
        loadInstances();
        loadSource();
      });
    }
    if (btn.dataset.page === "admin") { loadWhitelistUI(); loadPublishUI(); }
    if (btn.dataset.page === "settings") { loadSettings(); setupRam(); fillRollbackForm(); loadAntilag(); loadTokenStatus(); }
    call("discord_set_page", { page: btn.dataset.page }).catch(() => {});
  });
});
function showGate(msg) {
  const gate = $("login-gate");
  const app = $("app-shell");
  if (gate) gate.classList.remove("hidden");
  if (app) app.classList.add("hidden");
  if (msg) status("login-status", msg);
}
function hideGate() {
  const gate = $("login-gate");
  const app = $("app-shell");
  if (gate) gate.classList.add("hidden");
  if (app) app.classList.remove("hidden");
}
function normUuid(u) {
  return (u || "").toLowerCase().replace(/-/g, "").trim();
}
async function loadWhitelist() {
  try {
    wl = await call("whitelist_get", {});
  } catch (e) {
    wl = null;
  }
  computeAccess();
  updateAccessUI();
  if (isAdmin) loadWhitelistUI();
}
function computeAccess() {
  isAdmin = false;
  isOwner = false;
  userAccessInstances = null;
  if (!wl) return;
  const me = normUuid(currentUuid);
  if (!me) return;
  const ownerUuid = normUuid(wl.owner && (wl.owner.uuid || wl.owner));
  if (ownerUuid && me === ownerUuid) { isAdmin = true; isOwner = true; return; }
  const staff = wl.staff || [];
  for (const s of staff) {
    if (normUuid(s.uuid) === me) { isAdmin = true; return; }
  }
  const acc = wl.access || [];
  for (const a of acc) {
    if (normUuid(a.uuid) === me) {
      const list = a.instances || [];
      if (list.includes("*") || list.includes("Todas")) userAccessInstances = null;
      else userAccessInstances = list.slice();
      return;
    }
  }
  userAccessInstances = [];
}
function canAccessInstance(name) {
  if (isAdmin) return true;
  if (userAccessInstances === null) return true;
  if (!userAccessInstances.length) return false;
  if (userAccessInstances.includes("*")) return true;
  const n = (name || "").toLowerCase().trim();
  return userAccessInstances.some((x) => {
    const nx = (x || "").toLowerCase().trim();
    if (nx === n) return true;
    if ((nx === "hallogeey" || nx === "navigay") && (n === "hallogeey" || n === "navigay")) return true;
    return false;
  });
}
function updateAccessUI() {
  const navAdmin = $("nav-admin");
  if (navAdmin) navAdmin.classList.toggle("hidden", !isAdmin);
  // Publicar y guardar token son solo del propietario.
  const cardPublish = $("card-publish");
  if (cardPublish) cardPublish.classList.toggle("hidden", !isOwner);
  const cardToken = $("card-token");
  if (cardToken) cardToken.classList.toggle("hidden", !isOwner);
  const badge = $("access-badge");
  if (badge) {
    if (!currentUuid) badge.classList.add("hidden");
    else {
      badge.classList.remove("hidden");
      badge.textContent = isAdmin ? "ADMIN" : (userAccessInstances === null ? "ACCESO TOTAL" : (userAccessInstances.length ? "WHITELIST: " + userAccessInstances.length : "SIN ACCESO"));
    }
  }
  const noAccess = $("no-access");
  if (noAccess) {
    const show = !!currentUuid && !isAdmin && userAccessInstances !== null && userAccessInstances.length === 0;
    noAccess.classList.toggle("hidden", !show);
    const uuidEl = $("no-access-uuid");
    if (uuidEl && show) uuidEl.textContent = currentUuid;
  }
  applyLauncherTheme();
}
async function doMicrosoftLogin() {
  status("login-status", "Iniciando sesión... Complete el login en el navegador.");
  status("login-status-2", "Iniciando sesión... Complete el login en el navegador.");
  try {
    const acc = await call("login_start", {});
    status("login-status", "Sesión iniciada como " + (acc.name || acc.id));
    status("login-status-2", "Sesión iniciada como " + (acc.name || acc.id));
    gateReady = true;
    await refreshLogin();
    loadInstances();
  } catch (e) {
    const m = (e.message || e) === "__MINECRAFT_NOT_OWNED__"
      ? "Esta cuenta de Microsoft no tiene Minecraft comprado."
      : "Login cancelado o fallido: " + e.message;
    status("login-status", m);
    status("login-status-2", m);
  }
}
async function refreshLogin() {
  try {
    const store = await call("login_get_state", {});
    const hasOffline = store.mode === "offline" && (store.offline_username || "").trim();
    if (hasOffline) {
      showGate("Modo offline deshabilitado. Se requiere cuenta premium de Microsoft.");
      if ($("login-box")) $("login-box").classList.remove("hidden");
      if ($("login-ok")) $("login-ok").classList.add("hidden");
      $("nav-user").textContent = "Sin sesión";
      const navAvatar = $("nav-user-avatar");
      if (navAvatar) navAvatar.classList.add("hidden");
      currentUuid = "";
      await loadWhitelist().catch(() => {});
      return;
    }
    if (store.accounts && store.accounts.length > 0) {
      const sel = store.accounts.find((a) => a.id === store.selected_id) || store.accounts[0];
      const username = sel.name || sel.id;
      const uuid = sel.id || "";
      currentUuid = uuid;
      $("login-name").textContent = username;
      if ($("login-uuid")) $("login-uuid").textContent = uuid;
      const needsRelogin = !!sel.needs_relogin;
      const statusEl = $("account-status");
      if (statusEl) {
        statusEl.textContent = needsRelogin
          ? "● Sesión cerrada. Vuelve a iniciar sesión."
          : "● Conectado a Microsoft";
        statusEl.classList.remove("account-status-online", "account-status-warning");
        statusEl.classList.add(needsRelogin ? "account-status-warning" : "account-status-online");
      }
      const reloginBtn = $("btn-relogin");
      if (reloginBtn) {
        if (needsRelogin) reloginBtn.classList.remove("hidden");
        else reloginBtn.classList.add("hidden");
      }
      const skinBody = $("login-skin-body");
      if (skinBody) {
        skinBody.src = `https://mc-heads.net/body/${encodeURIComponent(username)}/240`;
        skinBody.onerror = () => {
          if (uuid) skinBody.src = `https://crafatar.com/renders/body/${uuid}?overlay=true`;
        };
      }
      const avatarHead = $("login-avatar-head");
      if (avatarHead) {
        avatarHead.src = `https://mc-heads.net/avatar/${encodeURIComponent(username)}/64`;
        avatarHead.onerror = () => {
          avatarHead.src = `https://minotar.net/helm/${encodeURIComponent(username)}/64`;
        };
      }
      $("nav-user").textContent = username;
      const navAvatar = $("nav-user-avatar");
      if (navAvatar) {
        navAvatar.src = `https://mc-heads.net/avatar/${encodeURIComponent(username)}/64`;
        navAvatar.classList.remove("hidden");
        navAvatar.onerror = () => {
          navAvatar.src = `https://minotar.net/helm/${encodeURIComponent(username)}/64`;
        };
      }
      if ($("login-box")) $("login-box").classList.add("hidden");
      if ($("login-ok")) $("login-ok").classList.remove("hidden");
      if (!needsRelogin) {
        hideGate();
        gateReady = true;
      } else {
        showGate("Sesión cerrada. Vuelve a iniciar sesión.");
      }
      await loadWhitelist();
      return;
    }
    currentUuid = "";
    showGate();
    if ($("login-box")) $("login-box").classList.remove("hidden");
    if ($("login-ok")) $("login-ok").classList.add("hidden");
    $("nav-user").textContent = "Sin sesión";
    const navAvatar = $("nav-user-avatar");
    if (navAvatar) navAvatar.classList.add("hidden");
    await loadWhitelist().catch(() => {});
  } catch (e) {
    status("login-status", "Error al leer cuentas: " + e.message);
  }
}

const navUserContainer = $("nav-user-container");
if (navUserContainer) {
  navUserContainer.addEventListener("click", () => {
    const btn = document.querySelector('.nav-item[data-page="login"]');
    if (btn) btn.click();
  });
}

if ($("btn-login")) $("btn-login").addEventListener("click", doMicrosoftLogin);
if ($("btn-login-2")) $("btn-login-2").addEventListener("click", doMicrosoftLogin);
if ($("btn-relogin")) $("btn-relogin").addEventListener("click", async () => {
  status("login-status", "Reiniciando sesión... Complete el login en el navegador si se abre.");
  status("login-status-2", "Reiniciando sesión... Complete el login en el navegador si se abre.");
  try {
    const acc = await call("login_start", {});
    status("login-status", "Sesión renovada como " + (acc.name || acc.id));
    await refreshLogin();
  } catch (e) {
    const m = (e.message || e) === "__MINECRAFT_NOT_OWNED__"
      ? "Esta cuenta de Microsoft no tiene Minecraft comprado."
      : "Relogin cancelado o fallido: " + e.message;
    status("login-status", m);
    status("login-status-2", m);
  }
});
if ($("btn-logout")) $("btn-logout").addEventListener("click", async () => {
  try {
    const store = await call("login_get_state", {});
    if (store.selected_id) await call("login_remove_account", { accountId: store.selected_id });
    currentUuid = "";
    gateReady = false;
    await refreshLogin();
  } catch (e) { status("login-status", e.message); }
});
const offlineSaveBtn = $("btn-save-offline");
if (offlineSaveBtn) offlineSaveBtn.addEventListener("click", async () => {
  status("offline-status", "Modo offline deshabilitado por política del servidor.");
});
const offlineToggle = $("offline-mode");
if (offlineToggle) offlineToggle.addEventListener("change", () => {
  offlineToggle.checked = false;
  status("offline-status", "Modo offline deshabilitado por política del servidor.");
});
// ─── ADMIN / WHITELIST UI ───
// ─── Temas por instancia (Ataplaygame / DoctoraPaula rosa-morado / Nexus base) ───
const THEME_LABELS = { cyber: "Nexus Base", ataplay: "Ataplaygame", paula: "DoctoraPaula" };
function getThemeOverrides() {
  try { return JSON.parse(localStorage.getItem("nexus_instance_themes") || "{}"); }
  catch (e) { return {}; }
}
function setThemeOverride(name, theme) {
  const map = getThemeOverrides();
  if (!theme || theme === "auto") delete map[name];
  else map[name] = theme;
  try { localStorage.setItem("nexus_instance_themes", JSON.stringify(map)); } catch (e) {}
}
function detectTheme(name) {
  const map = getThemeOverrides();
  if (map[name]) return map[name];
  const n = (name || "").toLowerCase();
  if (n.includes("ataplay") || n.includes("hallogeey") || n.includes("navigay")) return "ataplay";
  if (n.includes("paula") || n.includes("paucalipsis")) return "paula";
  return "cyber";
}
function themeChip(name) {
  const t = detectTheme(name);
  if (t === "cyber") return "";
  return `<span class="theme-chip theme-${t}">${escapeHtml(THEME_LABELS[t])}</span>`;
}
// Tema global del launcher: tiñe TODA la UI según la instancia permitida.
// Usuarios normales: forzado por su acceso. Staff: selector manual + automático.
let activeInstanceTheme = "";
let playingInstance = "";

function getStaffThemePref() {
  return localStorage.getItem("nexus_staff_theme") || "auto";
}
function setStaffThemePref(v) {
  try { localStorage.setItem("nexus_staff_theme", v); } catch (e) {}
}

function userOnlyHasAtaplayAccess() {
  if (isAdmin) return false;
  if (userAccessInstances === null) return false; // null = acceso a todas (*)
  if (!Array.isArray(userAccessInstances) || userAccessInstances.length === 0) return false;
  const allowedThemes = [...new Set(userAccessInstances.map((n) => detectTheme(n)))];
  return allowedThemes.length === 1 && allowedThemes[0] === "ataplay";
}

function decideLauncherTheme() {
  // 1. Si se está jugando esa instancia (NaviGay / Ataplaygame): aplica siempre (staff y usuarios)
  if (playingInstance) {
    const playTheme = detectTheme(playingInstance);
    if (playTheme === "ataplay") return "ataplay";
    if (playTheme && playTheme !== "cyber") return playTheme;
  }

  // 2. Si es staff / admin:
  if (isAdmin) {
    const pref = getStaffThemePref();
    if (pref && pref !== "auto") return pref;
    return "cyber";
  }

  // 3. Si como usuario normal solo tiene acceso a esa instancia:
  if (userOnlyHasAtaplayAccess()) {
    return "ataplay";
  }

  // 4. Si como usuario normal solo tiene acceso a otra instancia temática específica:
  if (Array.isArray(userAccessInstances) && userAccessInstances.length > 0) {
    const allowedThemes = [...new Set(userAccessInstances.map((n) => detectTheme(n)))];
    if (allowedThemes.length === 1 && allowedThemes[0] !== "cyber") return allowedThemes[0];
  }

  return "cyber";
}
function applyLauncherTheme() {
  const t = decideLauncherTheme();
  document.body.dataset.launcherTheme = t;
  const sel = $("launcher-theme-select");
  if (sel) sel.value = isAdmin ? getStaffThemePref() : "auto";
  return t;
}
function setActiveInstance(inst) {
  activeInstanceTheme = inst && inst.name ? inst.name : "";
  return applyLauncherTheme();
}
async function loadThemeUI() {
  const list = $("theme-list");
  if (!list) return;
  const map = getThemeOverrides();
  list.innerHTML = "";
  const source = instances.length ? instances : [];
  if (!source.length) { list.innerHTML = '<p class="muted">Sin instancias. Crea una primero.</p>'; return; }
  source.forEach((inst) => {
    const row = document.createElement("div");
    row.className = "instance-card";
    const t = detectTheme(inst.name);
    row.innerHTML = `<div class="icon">${escapeHtml((inst.name || "?").charAt(0).toUpperCase())}</div><div class="info"><b>${escapeHtml(inst.name)}</b><div class="sub">Tema activo: ${escapeHtml(THEME_LABELS[t])}</div></div><div class="actions"><select data-theme-sel="${escapeHtml(inst.name)}" class="theme-sel">
      <option value="auto" ${!map[inst.name] ? "selected" : ""}>Automático</option>
      <option value="cyber" ${map[inst.name] === "cyber" ? "selected" : ""}>Blanco y negro (base)</option>
      <option value="ataplay" ${map[inst.name] === "ataplay" ? "selected" : ""}>Ataplaygame</option>
      <option value="paula" ${map[inst.name] === "paula" ? "selected" : ""}>DoctoraPaula</option>
    </select></div>`;
    list.appendChild(row);
  });
  list.querySelectorAll("[data-theme-sel]").forEach((sel) => {
    sel.addEventListener("change", () => {
      setThemeOverride(sel.getAttribute("data-theme-sel"), sel.value);
      status("theme-status", `Tema de '${sel.getAttribute("data-theme-sel")}' actualizado.`);
      loadInstances();
      loadThemeUI();
    });
  });
}
async function loadWhitelistUI() {
  if (!wl) await loadWhitelist();
  if (!wl) return;
  // Staff
  const staffList = $("staff-list");
  if (staffList) {
    staffList.innerHTML = "";
    (wl.staff || []).forEach((s) => {
      const row = document.createElement("div");
      row.className = "instance-card";
      row.innerHTML = `<div class="icon">${escapeHtml((s.name || "?").charAt(0).toUpperCase())}</div><div class="info"><b>${escapeHtml(s.name)}</b><div class="sub">${escapeHtml(s.uuid)}</div></div><div class="actions"><button class="btn danger small" data-del>Quitar</button></div>`;
      row.querySelector("[data-del]").addEventListener("click", async () => {
        try { await call("whitelist_remove_staff", { uuid: s.uuid }); await loadWhitelist(); loadWhitelistUI(); status("staff-status", "Staff eliminado."); }
        catch (e) { status("staff-status", e.message); }
      });
      staffList.appendChild(row);
    });
    if (!(wl.staff || []).length) staffList.innerHTML = '<p class="muted">Sin staff.</p>';
  }
  // Access
  const accList = $("access-list");
  if (accList) {
    accList.innerHTML = "";
    (wl.access || []).forEach((a) => {
      const row = document.createElement("div");
      row.className = "instance-card";
      row.innerHTML = `<div class="icon">${escapeHtml((a.name || "?").charAt(0).toUpperCase())}</div><div class="info"><b>${escapeHtml(a.name)}</b><div class="sub">${escapeHtml(a.uuid)} · ${escapeHtml((a.instances || []).join(", ") || "*")}</div></div><div class="actions"><button class="btn danger small" data-del>Quitar</button></div>`;
      row.querySelector("[data-del]").addEventListener("click", async () => {
        try { await call("whitelist_remove_access", { uuid: a.uuid }); await loadWhitelist(); loadWhitelistUI(); status("wl-status", "Acceso eliminado."); }
        catch (e) { status("wl-status", e.message); }
      });
      accList.appendChild(row);
    });
    if (!(wl.access || []).length) accList.innerHTML = '<p class="muted">Nadie en whitelist.</p>';
  }
  loadThemeUI();
}
function bindAdmin() {
  const syncAllBtn = $("btn-sync-all");
  if (syncAllBtn) syncAllBtn.addEventListener("click", async () => {
    if (!isAdmin) { status("sync-all-status", "Solo staff."); return; }
    status("sync-all-status", "Sincronizando todas las instancias...");
    try {
      const res = await call("auto_sync_apply", {});
      status("sync-all-status", res.applied.length ? `Actualizadas: ${res.applied.join(", ")}` : "Todo al día." + ((res.errors || []).length ? ` · Errores: ${res.errors.join("; ")}` : ""));
      loadInstances();
    } catch (e) { status("sync-all-status", e.message); }
  });
  const checkAllBtn = $("btn-check-all");
  if (checkAllBtn) checkAllBtn.addEventListener("click", async () => {
    if (!isAdmin) { status("sync-all-status", "Solo staff."); return; }
    status("sync-all-status", "Comprobando actualizaciones...");
    try {
      const states = await call("auto_sync_check", {}) || [];
      const pending = states.filter((s) => !s.error && !s.up_to_date && s.change_count > 0);
      status("sync-all-status", pending.length ? "Pendientes: " + pending.map((s) => s.name).join(", ") : "Todo al día.");
    } catch (e) { status("sync-all-status", e.message); }
  });
  const sel = $("launcher-theme-select");
  if (sel) sel.addEventListener("change", () => {
    setStaffThemePref(sel.value);
    status("theme-status", sel.value === "auto" ? "Tema automático (según la instancia activa)." : `Tema forzado: ${THEME_LABELS[sel.value]}.`);
    applyLauncherTheme();
  });
  const bindUuidLookup = (nameId, uuidId, statusId) => {
    const nameEl = $(nameId);
    const uuidEl = $(uuidId);
    if (!nameEl || !uuidEl) return;
    let timer = null;
    nameEl.addEventListener("input", () => {
      clearTimeout(timer);
      timer = setTimeout(async () => {
        const name = nameEl.value.trim();
        if (!name || name.length < 3) return;
        status(statusId, `Buscando UUID de '${name}'...`);
        try {
          const res = await call("uuid_lookup", { name });
          if (res && res.uuid) {
            uuidEl.value = res.uuid;
            if (res.name && res.name !== name) nameEl.value = res.name;
            status(statusId, `UUID de ${res.name}: ${res.uuid}`);
          }
        } catch (e) { status(statusId, "No encontrado. Escribe el UUID manualmente."); }
      }, 600);
    });
  };
  bindUuidLookup("wl-name", "wl-uuid", "wl-status");
  bindUuidLookup("staff-name", "staff-uuid", "staff-status");
  const addStaffBtn = $("btn-add-staff");
  if (addStaffBtn) addStaffBtn.addEventListener("click", async () => {
    const name = ($("staff-name").value || "").trim();
    const uuid = ($("staff-uuid").value || "").trim();
    if (!name || !uuid) { status("staff-status", "Nombre y UUID obligatorios."); return; }
    try { await call("whitelist_add_staff", { name, uuid }); await loadWhitelist(); loadWhitelistUI(); status("staff-status", "Staff añadido."); $("staff-name").value = ""; $("staff-uuid").value = ""; }
    catch (e) { status("staff-status", e.message); }
  });
  const addAccBtn = $("btn-add-access");
  if (addAccBtn) addAccBtn.addEventListener("click", async () => {
    const name = ($("wl-name").value || "").trim();
    const uuid = ($("wl-uuid").value || "").trim();
    const insts = ($("wl-instances").value || "*").split(",").map((s) => s.trim()).filter(Boolean);
    if (!name || !uuid) { status("wl-status", "Nombre y UUID obligatorios."); return; }
    try { await call("whitelist_add_access", { name, uuid, instances: insts }); await loadWhitelist(); loadWhitelistUI(); status("wl-status", "Añadido a la whitelist."); $("wl-name").value = ""; $("wl-uuid").value = ""; }
    catch (e) { status("wl-status", e.message); }
  });
  const wlInput = $("wl-instances");
  const wlPickAll = $("btn-wl-pick-all");
  if (wlPickAll && wlInput) wlPickAll.addEventListener("click", () => {
    wlInput.value = "*";
  });
  const wlPickHallo = $("btn-wl-pick-navigay") || $("btn-wl-pick-hallogeey");
  if (wlPickHallo && wlInput) wlPickHallo.addEventListener("click", () => {
    const parts = (wlInput.value || "").split(",").map((s) => s.trim()).filter(Boolean);
    if (!parts.includes("NaviGay")) {
      const filtered = parts.filter((s) => s !== "*");
      filtered.push("NaviGay");
      wlInput.value = filtered.join(", ");
    }
  });
  const wlPickPau = $("btn-wl-pick-paucalipsis");
  if (wlPickPau && wlInput) wlPickPau.addEventListener("click", () => {
    const parts = (wlInput.value || "").split(",").map((s) => s.trim()).filter(Boolean);
    if (!parts.includes("Paucalipsis-2")) {
      const filtered = parts.filter((s) => s !== "*");
      filtered.push("Paucalipsis-2");
      wlInput.value = filtered.join(", ");
    }
  });
  const syncWlGhBtn = $("btn-sync-whitelist-gh");
  if (syncWlGhBtn) syncWlGhBtn.addEventListener("click", async () => {
    syncWlGhBtn.disabled = true;
    syncWlGhBtn.textContent = "Sincronizando con GitHub...";
    status("wl-status", "Subiendo whitelist a GitHub...");
    try {
      const res = await call("whitelist_sync_github", {});
      status("wl-status", res || "Whitelist sincronizada en GitHub.");
    } catch (e) {
      status("wl-status", "Error al sincronizar: " + (e.message || e));
    } finally {
      syncWlGhBtn.disabled = false;
      syncWlGhBtn.textContent = "☁️ Sincronizar con GitHub";
    }
  });
}
bindAdmin();

// ─── Publicador de actualizaciones (owner) ───
// El owner selecciona la carpeta local donde edita la instancia y la repo destino.
// Sube solo las carpetas elegidas (mods, config, etc.) a GitHub vía Git Data API.
// Los usuarios finales reciben ese cambio con sync_check/sync_apply al darle a jugar.
let publishRenderToken = 0;

async function scanLocalPublishFolders(path) {
  const box = $("publish-folders");
  if (!box) return;
  const targetPath = (path || ($("publish-local-path") || {}).value || "").trim();
  box.innerHTML = "";
  if (!targetPath) {
    box.innerHTML = '<p class="muted">Elige o escribe una carpeta local para escanear sus subcarpetas.</p>';
    return;
  }
  const token = ++publishRenderToken;
  try {
    status("publish-status", "Escaneando subcarpetas locales...");
    const folders = await call("instance_list_publish_folders", { name: targetPath }) || [];
    if (token !== publishRenderToken) return;
    if (!folders.length) {
      box.innerHTML = '<p class="muted">No se encontraron subcarpetas publicables en esta ruta.</p>';
      status("publish-status", "");
      return;
    }
    status("publish-status", `${folders.length} subcarpeta(s) encontradas.`);
    folders.forEach((f) => {
      const row = document.createElement("label");
      row.className = "folder-check";
      const isDefaultChecked = ["mods", "config", "resourcepacks", "shaderpacks", "scripts", "kubejs", "defaultconfigs"].includes(f.toLowerCase());
      row.innerHTML = `<input type="checkbox" value="${escapeHtml(f)}" ${isDefaultChecked ? "checked" : ""} /> <span>${escapeHtml(f)}</span>`;
      box.appendChild(row);
    });
  } catch (e) {
    if (token !== publishRenderToken) return;
    box.innerHTML = `<p class="muted">Error: ${escapeHtml(e.message)}</p>`;
    status("publish-status", e.message);
  }
}

async function setPublishLocalPath(path) {
  const input = $("publish-local-path");
  if (!input) return;
  input.value = path;
  await scanLocalPublishFolders(path);
}

async function loadPublishUI() {
  if (!isOwner) return;
  const localInput = $("publish-local-path");
  if (localInput && !localInput.value.trim()) {
    try {
      const inst = (instances || []).find((i) => /hallogeey|navigay/i.test(i.name)) || (instances || [])[0];
      if (inst) {
        const dir = await call("instance_get_local_dir", { name: inst.name });
        if (dir) await setPublishLocalPath(dir);
      }
    } catch (_) {}
  }
}

function bindPublisher() {
  const localInput = $("publish-local-path");
  if (localInput) {
    let timer = null;
    localInput.addEventListener("input", () => {
      clearTimeout(timer);
      timer = setTimeout(() => {
        scanLocalPublishFolders(localInput.value.trim());
      }, 500);
    });
  }

  const browseBtn = $("btn-browse-folder");
  if (browseBtn) browseBtn.addEventListener("click", async () => {
    try {
      const picked = await call("pick_folder", {});
      if (picked) {
        await setPublishLocalPath(picked);
      }
    } catch (e) {
      status("publish-status", "Error al seleccionar carpeta: " + e.message);
    }
  });

  const pathHalloBtn = $("btn-path-navigay") || $("btn-path-hallogeey");
  if (pathHalloBtn) pathHalloBtn.addEventListener("click", async () => {
    try {
      const inst = (instances || []).find((i) => /navigay|hallogeey/i.test(i.name));
      const instName = inst ? inst.name : "NaviGay";
      const dir = await call("instance_get_local_dir", { name: instName });
      if (dir) {
        await setPublishLocalPath(dir);
        const repoSel = $("publish-target-repo");
        if (repoSel) repoSel.value = "kazdev97/Hallogeey";
      }
    } catch (e) {
      status("publish-status", "No se pudo obtener la ruta: " + e.message);
    }
  });

  const pathPauBtn = $("btn-path-paucalipsis");
  if (pathPauBtn) pathPauBtn.addEventListener("click", async () => {
    try {
      const inst = (instances || []).find((i) => /paucalipsis/i.test(i.name));
      const instName = inst ? inst.name : "Paucalipsis-2";
      const dir = await call("instance_get_local_dir", { name: instName });
      if (dir) {
        await setPublishLocalPath(dir);
        const repoSel = $("publish-target-repo");
        if (repoSel) repoSel.value = "kazdev97/Paucalipsis-2";
      }
    } catch (e) {
      status("publish-status", "No se pudo obtener la ruta: " + e.message);
    }
  });

  const openExpBtn = $("btn-path-open-explorer");
  if (openExpBtn) openExpBtn.addEventListener("click", async () => {
    const p = (localInput && localInput.value || "").trim();
    if (!p) { status("publish-status", "Escribe o selecciona primero una carpeta."); return; }
    try {
      await call("open_folder", { path: p });
    } catch (e) {
      status("publish-status", "Error al abrir la carpeta: " + e.message);
    }
  });

  const scanFoldersBtn = $("btn-scan-folders");
  if (scanFoldersBtn) scanFoldersBtn.addEventListener("click", async () => {
    scanFoldersBtn.disabled = true;
    scanFoldersBtn.textContent = "Consultando...";
    try {
      await scanLocalPublishFolders();
    } finally {
      scanFoldersBtn.disabled = false;
      scanFoldersBtn.textContent = "🔄 Consultar carpetas";
    }
  });

  const selectAllBtn = $("btn-folders-select-all");
  if (selectAllBtn) selectAllBtn.addEventListener("click", () => {
    document.querySelectorAll("#publish-folders input[type=checkbox]").forEach((cb) => cb.checked = true);
  });
  const modsConfigBtn = $("btn-folders-mods-config");
  if (modsConfigBtn) modsConfigBtn.addEventListener("click", () => {
    document.querySelectorAll("#publish-folders input[type=checkbox]").forEach((cb) => {
      const val = cb.value.toLowerCase();
      cb.checked = (val === "mods" || val === "config");
    });
  });
  const noneBtn = $("btn-folders-none");
  if (noneBtn) noneBtn.addEventListener("click", () => {
    document.querySelectorAll("#publish-folders input[type=checkbox]").forEach((cb) => cb.checked = false);
  });

  const publishBtn = $("btn-publish");
  if (publishBtn) publishBtn.addEventListener("click", async () => {
    const localPath = (localInput && localInput.value || "").trim();
    if (!localPath) {
      status("publish-status", "Selecciona o escribe la carpeta local de la instancia.");
      return;
    }
    const repoSel = $("publish-target-repo");
    const repo = (repoSel && repoSel.value || "").trim();
    if (!repo) {
      status("publish-status", "Selecciona la repo de destino.");
      return;
    }
    const folders = Array.from(document.querySelectorAll("#publish-folders input[type=checkbox]:checked")).map((i) => i.value);
    if (!folders.length) {
      status("publish-status", "Marca al menos una carpeta para sincronizar.");
      return;
    }
    const message = (($("publish-message") || {}).value || "").trim();

    publishBtn.disabled = true;
    publishBtn.textContent = "Publicando a GitHub…";
    status("publish-status", `Subiendo ${folders.length} carpeta(s) a ${repo}...`);

    try {
      const sha = await call("publisher_publish", {
        localPath,
        repo,
        branch: "main",
        subpath: "",
        folders,
        message,
      });
      status("publish-status", `🚀 ¡Actualización publicada! Commit ${sha.slice(0, 10)}. Los usuarios la recibirán al jugar o sincronizar.`);
      $("publish-message").value = "";
    } catch (e) {
      status("publish-status", "Error publicando actualización: " + e.message);
    } finally {
      publishBtn.disabled = false;
      publishBtn.textContent = "🚀 Lanzar actualización";
    }
  });

  // Token de GitHub en Ajustes
  const tokenBtn = $("btn-save-token");
  if (tokenBtn) tokenBtn.addEventListener("click", async () => {
    const token = (($("gh-token") || {}).value || "").trim();
    if (!token) { status("token-status", "Pega tu token de GitHub (ghp_…)."); return; }
    tokenBtn.disabled = true;
    try {
      await call("publisher_set_token", { token });
      $("gh-token").value = "";
      status("token-status", "Token guardado. Ya puedes publicar actualizaciones.");
    } catch (e) {
      status("token-status", "Error guardando el token: " + e.message);
    } finally {
      tokenBtn.disabled = false;
    }
  });
}
bindPublisher();
async function loadTokenStatus() {
  const st = $("token-status");
  if (!st) return;
  if (!isOwner) { st.textContent = ""; return; }
  try {
    const has = await call("publisher_get_status", {});
    st.textContent = has ? "Token configurado. Publicar está habilitado." : "Sin token. Pega uno para poder publicar.";
  } catch (e) {
    st.textContent = "";
  }
}
let instances = [];
let currentInstance = null;

async function loadInstances() {
  try {
    instances = await call("instances_list", {});
  } catch (e) {
    status("instances-status", e.message);
    return;
  }
  const list = $("instances-list");
  list.innerHTML = "";
  if (instances.length === 0) {
    list.innerHTML = '<p class="muted">Sin instancias instaladas. Instala una de las disponibles arriba.</p>';
    return;
  }
  instances.forEach((inst) => list.appendChild(instanceCard(inst)));
  if (!isAdmin && userAccessInstances !== null && userAccessInstances.length === 0) {
    const p = document.createElement("p");
    p.className = "muted";
    p.textContent = "No tienes acceso a ninguna instancia. Contacta con el staff.";
    list.prepend(p);
  }
  fillRollbackForm();
  loadSource();
  loadPublishUI();
  runAutoSync(false);
}

let settingsState = null;
async function getSettings() {
  if (!settingsState) settingsState = await call("settings_get", {});
  return settingsState;
}

async function loadSource() {
  renderSourceStatus("Consultando instancias disponibles...");
  try {
    const s = await getSettings();
    const url = (s.source_url || "").trim();
    const list = await call("source_fetch", { url: url || undefined });
    renderAvailable(list || []);
    status("instances-status", (list && list.length)
      ? `${list.length} instancias disponibles en la fuente.`
      : "No hay instancias disponibles en la fuente.");
  } catch (e) {
    console.error("Error cargando instancias:", e);
    renderAvailable([]);
    status("instances-status", `Error consultando fuente: ${e.message || e}`);
  }
}

const refreshSourceBtn = $("btn-refresh-source");
if (refreshSourceBtn) {
  refreshSourceBtn.addEventListener("click", async () => {
    refreshSourceBtn.disabled = true;
    refreshSourceBtn.textContent = "Consultando...";
    try {
      await loadWhitelist().catch(() => {});
      await loadSource();
    } finally {
      refreshSourceBtn.disabled = false;
      refreshSourceBtn.textContent = "🔄 Consultar";
    }
  });
}

const refreshInstancesBtn = $("btn-refresh-instances");
if (refreshInstancesBtn) {
  refreshInstancesBtn.addEventListener("click", async () => {
    refreshInstancesBtn.disabled = true;
    refreshInstancesBtn.textContent = "Consultando...";
    try {
      await loadWhitelist().catch(() => {});
      await loadInstances();
      await runAutoSync(false);
    } finally {
      refreshInstancesBtn.disabled = false;
      refreshInstancesBtn.textContent = "🔄 Consultar actualizaciones";
    }
  });
}

function renderSourceStatus(msg) {
  $("available-list").innerHTML = `<p class="muted">${escapeHtml(msg)}</p>`;
}

function renderAvailable(list) {
  const el = $("available-list");
  el.innerHTML = "";
  if (list.length === 0) {
    el.innerHTML = '<p class="muted">No hay instancias publicadas todavía.</p>';
    return;
  }
  list.forEach((item) => {
    const card = document.createElement("div");
    card.className = "instance-card";
    card.dataset.theme = detectTheme(item.name);
    const loader = item.loader || "vanilla";
    const meta = `${escapeHtml(loader)} ${escapeHtml(item.game_version || "?")}`
      + (item.loader_version ? " · " + escapeHtml(item.loader_version) : "")
      + (item.revision ? " · rev " + escapeHtml(item.revision) : "");
    card.innerHTML = `
      <div class="icon">${escapeHtml((item.name || "?").charAt(0).toUpperCase())}</div>
      <div class="info">
        <b>${escapeHtml(item.name)}</b>${themeChip(item.name)}
        ${item.installed ? '<span class="badge install">instalada</span>' : ""}
        <div class="sub">${meta}</div>
        ${item.description ? `<div class="sub">${escapeHtml(item.description)}</div>` : ""}
      </div>
      <div class="actions">
        ${item.installed
          ? '<button class="btn ghost small" disabled>instalado</button>'
          : '<button class="btn mint small" data-install>Instalar</button>'}
      </div>`;
    const btn = card.querySelector("[data-install]");
    if (btn) btn.addEventListener("click", () => installFromSource(item, btn));
    el.appendChild(card);
  });
}

async function installFromSource(item, btn) {
  if (!canAccessInstance(item.name)) { status("instances-status", "No tienes acceso a esta instancia."); return; }
  if (DEMO) return installFromSourceDemo(item);
  status("instances-status", `Instalando '${item.name}' desde el source...`);
  setInstallProgress(1, `Instalando '${item.name}'...`);
  if (btn) { btn.disabled = true; btn.textContent = "Instalando…"; }
  try {
    await call("source_install", {
      name: item.name,
      gameVersion: item.game_version || "",
      loader: item.loader || "vanilla",
      loaderVersion: item.loader_version || "",
      manifestUrl: item.manifest_url || "",
      githubRepo: item.github_repo || "",
      githubPath: item.github_path || "",
      githubBranch: item.github_branch || "",
    });
    setInstallProgress(100, "Instalada.");
    status("instances-status", `'${item.name}' instalada correctamente.`);
    loadInstances();
  } catch (e) {
    hideInstallProgress();
    if (btn) { btn.disabled = false; btn.textContent = "Instalar"; }
    status("instances-status", "Error instalando: " + e.message);
  }
}

async function installFromSourceDemo(item) {
  status("instances-status", `Instalando '${item.name}' desde el source...`);
  const steps = [
    [5, "Obteniendo manifiesto..."],
    [15, "Descargando el jar del juego..."],
    [40, "Descargando assets..."],
    [62, "Descargando librerías..."],
    [85, "Descargando mods..."],
    [99, "Finalizando..."],
  ];
  for (const [pct, txt] of steps) {
    await new Promise((r) => setTimeout(r, 450));
    setInstallProgress(pct, txt);
  }
  await new Promise((r) => setTimeout(r, 250));
  hideInstallProgress();
  status("instances-status", `'${item.name}' instalada correctamente.`);
  loadInstances();
}

async function runAutoSync(autoApply) {
  let states;
  try {
    states = await call("auto_sync_check", {});
  } catch (e) {
    return;
  }
  const pending = (states || []).filter((s) =>
    !s.error && !s.up_to_date && s.change_count > 0);
  const banner = $("updates-banner");
  if (pending.length > 0) {
    banner.innerHTML = `
      <div class="row">
        <b>Actualizaciones disponibles:</b>
        <span class="muted">${pending.map((s) => escapeHtml(s.name)).join(", ")}</span>
        <button class="btn mint small" id="btn-apply-all">Actualizar ahora</button>
      </div>`;
    banner.classList.remove("hidden");
    $("btn-apply-all").addEventListener("click", async () => {
      banner.classList.add("hidden");
      status("instances-status", "Aplicando actualizaciones...");
      try {
        const res = await call("auto_sync_apply", {});
        const msg = [
          res.applied.length ? "Actualizado: " + res.applied.join(", ") : "Todo al día.",
          res.errors.length ? "Errores: " + res.errors.join("; ") : "",
        ].filter(Boolean).join("\n");
        status("instances-status", msg);
        loadInstances();
      } catch (e) { status("instances-status", "Error: " + e.message); }
    });
  } else {
    banner.classList.add("hidden");
  }
  if (autoApply && pending.length > 0) {
    try {
      const s = await getSettings();
      if (s.auto_update) {
        status("instances-status", "Buscando actualizaciones (auto)...");
        const res = await call("auto_sync_apply", {});
        banner.classList.add("hidden");
        status("instances-status",
          (res.applied.length ? "Actualizado: " + res.applied.join(", ") : "Todo al día.")
          + (res.errors.length ? " · Errores: " + res.errors.join("; ") : ""));
        loadInstances();
      }
    } catch (e) { status("instances-status", "No se pudo auto-actualizar: " + e.message); }
  }
}

function instanceCard(inst) {
  const el = document.createElement("div");
  el.className = "instance-card";
  const badge = inst.installed ? (inst.last_launched ? "" : "")
    : '<span class="badge outdated">no instalada</span>';
  const allowed = canAccessInstance(inst.name);
  const dis = allowed ? "" : "disabled";
  const note = allowed ? "" : '<span class="badge outdated">sin acceso</span>';
  el.dataset.theme = detectTheme(inst.name);
  el.innerHTML = `
    <div class="icon">${inst.name.charAt(0).toUpperCase()}</div>
    <div class="info">
      <b>${escapeHtml(inst.name)}</b>${themeChip(inst.name)}${badge}${note}
      <div class="sub">${escapeHtml(inst.loader)} ${escapeHtml(inst.game_version)}
        ${inst.loader_version ? "· " + escapeHtml(inst.loader_version) : ""}
        ${inst.manifest_url ? "· manifest: ajustado" : "· manifest: no configurado"}
        ${inst.last_launched ? "· último: " + escapeHtml(inst.last_launched) : ""}
      </div>
    </div>
    <div class="actions">
      ${!inst.installed ? `<button class="btn mint small" data-act="install" ${dis}>Instalar</button>` : ''}
      <button class="btn lila small" data-act="check" ${dis}>Actualizar</button>
      <button class="btn lila small" data-act="launch" ${dis}>Jugar</button>
      <button class="btn ghost small" data-act="delete" ${dis}>Eliminar</button>
    </div>`;
  const btnInstall = el.querySelector('[data-act="install"]');
  if (btnInstall) btnInstall.addEventListener("click", () => installInstance(inst));
  el.querySelector('[data-act="check"]').addEventListener("click", () => checkUpdates(inst));
  el.querySelector('[data-act="launch"]').addEventListener("click", () => launchInstance(inst));
  el.querySelector('[data-act="delete"]').addEventListener("click", async () => {
    if (!confirm(`¿Eliminar la instancia '${inst.name}' y todos sus archivos?`)) return;
    try {
      await call("instances_delete", { name: inst.name });
      loadInstances();
    } catch (e) { status("instances-status", e.message); }
  });
  return el;
}

function escapeHtml(s) {
  return (s || "").replace(/[&<>"']/g, (c) =>
    ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" }[c]));
}

async function installInstance(inst) {
  if (!canAccessInstance(inst.name)) { status("instances-status", "No tienes acceso a esta instancia."); return; }
  status("instances-status", `Instalando '${inst.name}' (descarga de versiones y librerías)...`);
  try {
    await call("install_instance", { name: inst.name });
    status("instances-status", `'${inst.name}' instalada correctamente.`);
    loadInstances();
  } catch (e) {
    status("instances-status", "Error instalando: " + e.message);
  }
}

async function checkUpdates(inst) {
  currentInstance = inst;
  setActiveInstance(inst);
  status("instances-status", "Comprobando actualizaciones...");
  try {
    const diff = await call("sync_check", { name: inst.name });
    showDiffModal(inst, diff);
  } catch (e) {
    showModal(`Error: ${e.message}`);
  }
}

function showDiffModal(inst, diff) {
  const lines = [];
  lines.push(`<div class="group">Revisión del servidor: ${escapeHtml(diff.revision || "sin fecha")}</div>`);
  if (diff.up_to_date && diff.change_count === 0) {
    lines.push('<div class="group">✔ La instancia está al día.</div>');
  } else {
    if (diff.missing.length) {
      lines.push('<div class="group">Añadir (' + diff.missing.length + '):</div>');
      diff.missing.forEach((f) => lines.push(`<div class="item add">${escapeHtml(f)}</div>`));
    }
    if (diff.different.length) {
      lines.push('<div class="group">Reemplazar (' + diff.different.length + '):</div>');
      diff.different.forEach((f) => lines.push(`<div class="item">${escapeHtml(f)}</div>`));
    }
    if (diff.extra.length) {
      lines.push('<div class="group">Eliminar sobrantes (' + diff.extra.length + '):</div>');
      diff.extra.forEach((f) => lines.push(`<div class="item extra">${escapeHtml(f)}</div>`));
    }
  }
  const modal = showModal(
    `<h2>Actualizaciones disponibles</h2><div class="diff">${lines.join("")}</div>`,
    diff.up_to_date && diff.change_count === 0 ? null : "Aplicar cambios"
  );
  if (diff.up_to_date && diff.change_count === 0) return;
  modal.querySelector(".btn.primary")?.addEventListener("click", async () => {
    modal.remove();
    status("instances-status", "Aplicando... (se hace backup automático)");
    try {
      const res = await call("sync_apply", { name: inst.name });
      const msg = [`Aplicado (revisión ${res.applied_revision || "n/a"}).`,
        `Añadidos: ${res.downloaded.length}, reemplazados: ${res.replaced.length}, eliminados: ${res.removed.length}.`,
        res.backup_dir ? `Backup: ${res.backup_dir}` : ""].join("\n");
      showModal(`<h2>Actualización aplicada</h2><pre class="mono small">${escapeHtml(msg)}</pre>`);
      if (res.errors && res.errors.length) showModal(`<h2>Errores parciales</h2><pre class="mono">${escapeHtml(res.errors.join("\n"))}</pre>`);
      loadInstances();
    } catch (e) { status("instances-status", "Error: " + e.message); }
  });
}

function showModal(html, okLabel, okHandler) {
  const backdrop = document.createElement("div");
  backdrop.className = "modal-backdrop";
  backdrop.innerHTML = `<div class="modal">${html}</div>`;
  const modal = backdrop.firstChild;
  if (okLabel) {
    const btn = document.createElement("button");
    btn.className = "btn primary";
    btn.textContent = okLabel;
    modal.appendChild(btn);
    if (okHandler) btn.addEventListener("click", okHandler);
  }
  backdrop.addEventListener("click", (e) => { if (e.target === backdrop) backdrop.remove(); });
  document.body.appendChild(backdrop);
  return backdrop;
}
function setInstallProgress(pct, text) {
  const card = $("install-progress");
  if (!card) return;
  $("install-fill").style.width = pct + "%";
  $("install-text").textContent = (text || "") + (pct > 0 ? ` (${pct}%)` : "");
  if (pct >= 100) card.classList.add("hidden");
  else card.classList.remove("hidden");
}
function hideInstallProgress() {
  const card = $("install-progress");
  if (card) card.classList.add("hidden");
}
async function loadSettings() {
  try {
    const s = await getSettings();
    $("set-java").value = s.java_path_override || "";
    $("set-streamer").checked = !!s.streamer_mode;
    $("set-auto").checked = s.auto_update !== undefined ? !!s.auto_update : true;
  } catch (e) { status("settings-status", e.message); }
}
async function loadAntilag() {
  try {
    const st = await call("antilag_status", {});
    $("antilag-enable").checked = !!st.enabled;
    $("antilag-host").value = st.host || "";
    const hint = $("antilag-hint");
    hint.style.color = "";
    if (st.installed && st.tasks_registered) {
      hint.textContent = st.tunnel_active ? "Túnel activo ahora mismo." : "";
      $("btn-antilag-install").textContent = "Reinstalar modo antilag";
    } else if (st.installed) {
      hint.textContent = "Faltan las tareas de administrador: pulsa Instalar y acepta el permiso.";
    } else {
      hint.textContent = "No instalado. Descarga un motor de ~15 MB y crea una cuenta WARP free.";
    }
  } catch (e) { status("antilag-status", e.message); }
}

function bindAntilag() {
  const installBtn = $("btn-antilag-install");
  installBtn.addEventListener("click", async () => {
    installBtn.disabled = true;
    installBtn.textContent = "Instalando…";
    status("antilag-status", "Instalando modo antilag…");
    try {
      const st = await call("antilag_install", {});
      status("antilag-status", st.tunnel_active
        ? "Modo antilag instalado."
        : "Modo antilag instalado. Actívalo con el interruptor.");
      await loadAntilag();
    } catch (e) {
      status("antilag-status", e.message || String(e));
    } finally {
      installBtn.disabled = false;
    }
  });
  $("antilag-enable").addEventListener("change", async () => {
    try {
      await call("antilag_set_enabled", { enabled: $("antilag-enable").checked });
      status("antilag-status", $("antilag-enable").checked
        ? "Activado: al jugar, la conexión al servidor saldrá por WARP."
        : "Desactivado.");
    } catch (e) { status("antilag-status", e.message || String(e)); }
  });
  let hostTimer = null;
  $("antilag-host").addEventListener("input", (e) => {
    clearTimeout(hostTimer);
    const value = e.target.value.trim();
    hostTimer = setTimeout(async () => {
      try {
        await call("antilag_set_host", { host: value });
        status("antilag-status", "Dirección del servidor guardada.");
      } catch (err) { status("antilag-status", err.message || String(err)); }
    }, 600);
  });
}

let ramRec = { min_mb: 1024, max_mb: 4096, total_ram_gb: 16 };

function populateRamInstanceSelect() {
  const sel = $("ram-instance");
  sel.innerHTML = "";
  instances.forEach((i) => {
    const o = document.createElement("option");
    o.value = i.name;
    o.textContent = i.name;
    sel.appendChild(o);
  });
  if (sel.childElementCount === 0) {
    const o = document.createElement("option");
    o.value = "";
    o.textContent = "No hay instancias";
    sel.appendChild(o);
  }
}

function updateRamFeedback(valueGb, auto) {
  $("ram-value").textContent = (auto ? "Auto · " : "") + valueGb + " GB";
  const fb = $("ram-feedback");
  const recMaxGb = Math.max(1, Math.round((ramRec.max_mb || 4096) / 1024));
  const recMinGb = Math.max(0, Math.round((ramRec.min_mb || 0) / 1024));
  fb.style.color = "var(--menta)";
  if (auto) {
    fb.textContent = `Auto: mínimo ${recMinGb} GB · máximo ${recMaxGb} GB (para esta instancia).`;
    return;
  }
  if (valueGb <= 1) {
    fb.textContent = "Riesgoso: puede que no sea suficiente para iniciar.";
    fb.style.color = "#E23D28";
  } else if (valueGb < recMaxGb - 1) {
    fb.textContent = `Bajo: esta instancia recomienda ${recMinGb}–${recMaxGb} GB.`;
    fb.style.color = "#F8B339";
  } else if (valueGb <= recMaxGb + 1) {
    fb.textContent = `Óptimo para esta instancia (recomendado: ${recMinGb}–${recMaxGb} GB).`;
  } else if (valueGb >= 10) {
    fb.textContent = "Excesivo: a partir de 10 GB puede causar micro-cortes y ralentizar Windows.";
    fb.style.color = "#F8B339";
  } else if (valueGb <= Math.max(2, Math.round(ramRec.total_ram_gb * 0.75))) {
    fb.textContent = "Aceptable: solo si el modpack es muy pesado.";
    fb.style.color = "var(--lila)";
  } else {
    fb.textContent = "Excesivo: puede causar micro-cortes y ralentizar Windows.";
    fb.style.color = "#F8B339";
  }
}

async function setupRam() {
  populateRamInstanceSelect();
  try {
    const info = await call("system_info", {}).catch(() => null);
    ramRec.total_ram_gb = info ? Math.max(1, Math.round((info.total_ram_mb || 8192) / 1024)) : 16;
    if (ramRec.max_mb > ramRec.total_ram_gb * 1024) ramRec.max_mb = ramRec.total_ram_gb * 1024;
    const name = $("ram-instance").value;
    if (name) {
      try {
        const r = await call("ram_recommend", { name });
        ramRec.min_mb = r.min_mb || ramRec.min_mb;
        ramRec.max_mb = r.max_mb || ramRec.max_mb;
        ramRec.total_ram_gb = r.total_ram_gb || ramRec.total_ram_gb;
      } catch (e) {}
    }
    const s = await getSettings();
    const slider = $("ram-slider");
    slider.max = Math.max(1, ramRec.total_ram_gb);
    const auto = !(s.max_ram_mb > 0);
    $("ram-auto").checked = auto;
    let val = Math.max(1, Math.min(Math.round((ramRec.max_mb || 4096) / 1024), slider.max));
    if (!auto && s.max_ram_mb > 0) {
      val = Math.max(1, Math.min(Math.round(s.max_ram_mb / 1024), slider.max));
    }
    slider.value = val;
    $("ram-info").textContent = info
      ? `Tu equipo: ${info.cpu_cores} núcleos · ${ramRec.total_ram_gb} GB de RAM.`
      : `${ramRec.total_ram_gb} GB de RAM detectados.`;
    updateRamFeedback(val, auto);
  } catch (e) {
    $("ram-info").textContent = "No se pudo calcular la recomendación.";
  }
}

$("ram-instance").addEventListener("change", () => setupRam());
$("ram-slider").addEventListener("input", () => {
  $("ram-auto").checked = false;
  updateRamFeedback(parseInt($("ram-slider").value, 10) || 1, false);
});
$("ram-auto").addEventListener("change", () => {
  if (!$("ram-auto").checked) return;
  const slider = $("ram-slider");
  slider.value = Math.max(1, Math.min(Math.round((ramRec.max_mb || 4096) / 1024), slider.max));
  updateRamFeedback(parseInt(slider.value, 10), true);
});

$("btn-save-settings").addEventListener("click", async () => {
  try {
    const s = await getSettings();
    const auto = $("ram-auto").checked;
    const ramOverrideMb = auto ? 0 : (parseInt($("ram-slider").value, 10) || 1) * 1024;
    const sourceUrl = (s.source_url || "").trim();
    await call("settings_save", {
      javaPathOverride: $("set-java").value.trim(),
      streamerMode: $("set-streamer").checked,
      ramOverrideMb,
      sourceUrl,
      autoUpdate: $("set-auto").checked,
      discordRpcEnabled: true,
    });
    settingsState.max_ram_mb = ramOverrideMb;
    status("settings-status", "Ajustes guardados.");
  } catch (e) { status("settings-status", e.message); }
});

bindAntilag();

function fillRollbackForm() {
  const sel = $("rb-instance");
  sel.innerHTML = "";
  instances.forEach((i) => {
    const o = document.createElement("option");
    o.value = i.name;
    o.textContent = i.name;
    sel.appendChild(o);
  });
  if (instances.length) loadBackups(instances[0].name);
  $("rb-instance").addEventListener("change", () => loadBackups($("rb-instance").value));
}

async function loadBackups(name) {
  try {
    const revs = await call("sync_list_backups", { name });
    const sel = $("rb-revision");
    sel.innerHTML = "";
    revs.forEach((r) => {
      const o = document.createElement("option");
      o.value = r;
      o.textContent = r;
      sel.appendChild(o);
    });
  } catch (e) {}
}

$("btn-rollback").addEventListener("click", async () => {
  const name = $("rb-instance").value;
  const rev = $("rb-revision").value;
  if (!name || !rev) { status("settings-status", "No hay revisiones."); return; }
  if (!confirm(`Restaurar la revisión ${rev} de '${name}'? Se sobrescribirá el estado actual.`)) return;
  try {
    await call("sync_rollback", { name, revision: rev });
    status("settings-status", "Rollback completado.");
    loadInstances();
  } catch (e) { status("settings-status", "Error: " + e.message); }
});
async function launchInstance(inst) {
  if (!currentUuid) { status("game-state", "Debes iniciar sesión con Microsoft para jugar."); showGate("Debes iniciar sesión con Microsoft para jugar."); return; }
  if (!canAccessInstance(inst.name)) { status("game-state", "No tienes acceso a esta instancia."); status("instances-status", `No tienes acceso a '${inst.name}'. Contacta con el staff.`); return; }
  setActiveInstance(inst);
  const t = decideLauncherTheme();
  const settings = await call("settings_get", {}).catch(() => null) || {};
  let javaPath = (settings.java_path_override || "").trim();
  if (!javaPath) {
    status("game-state", "Buscando el Java que necesita " + (inst.game_version || "?") + " (Mojang)...");
    try {
      const j = await call("java_resolve", { name: inst.name });
      javaPath = j.path;
    } catch (e) {
      status("game-state", "No se pudo obtener Java: " + e.message);
      return;
    }
  }
  if (settings.auto_update && inst.remote) {
    try {
      const diff = await call("sync_check", { name: inst.name });
      if (!diff.up_to_date && diff.change_count > 0) {
        status("game-state", "Actualizando antes de jugar...");
        await call("sync_apply", { name: inst.name });
        status("game-state", `Actualizado (revisión ${diff.revision}). Lanzando...`);
      }
    } catch (e) {
      status("game-state", "Aviso: no se pudo auto-actualizar (" + e.message + ")");
    }
  }
  status("game-state", `Lanzando '${inst.name}' · Tema ${THEME_LABELS[t]}...`);
  try {
    const pid = await call("launch_game", { name: inst.name, javaPath });
    playingInstance = inst.name;
    applyLauncherTheme();
    status("game-state", `Juego lanzado (PID ${pid}). Tema ${THEME_LABELS[decideLauncherTheme()]}.`);
    if (DEMO) {
      const lines = [
        "[main/INFO]: Loading Minecraft 1.21.4 with Fabric Loader 0.16.14",
        "[main/INFO]: Launched version " + inst.name,
        "[main/INFO]: 45 mods loaded (12 new, 2 updated)",
        "[main/WARN]: Some legacy options were deprecated",
        "[main/INFO]: Backend library: NexusLauncher/0.1 (Minecraft client 1.21.4)",
        "[Render thread/INFO]: Setting user: elusuario_de_prueba",
        "[Render thread/INFO]: Backend library: connecting to play.kazserver.gg...",
        "[Render thread/INFO]: Connected to play.kazserver.gg:25565",
      ];
      lines.forEach((l, i) => setTimeout(() => appendLog(l, null), i * 350));
    }
  } catch (e) {
    playingInstance = "";
    applyLauncherTheme();
    status("game-state", "Error al lanzar: " + e.message);
    refreshLogin();
  }
}

$("btn-launch").addEventListener("click", async () => {
  if (!currentInstance && instances.length > 0) currentInstance = instances[0];
  if (!currentInstance) { status("game-state", "Crea una instancia primero."); return; }
  launchInstance(currentInstance);
});

$("btn-kill").addEventListener("click", async () => {
  try {
    await call("launch_kill", {});
    playingInstance = "";
    applyLauncherTheme();
    status("game-state", "Juego detenido.");
  }
  catch (e) { status("game-state", e.message); }
});

consoleBuf.push("— consola vacía —");
$("console").textContent = "— consola vacía —\n";
EVENT.listen("launch/line", (data) => {
  appendLog(data.payload.line, data.payload.stream);
});
EVENT.listen("launch/exit", () => {
  playingInstance = "";
  applyLauncherTheme();
  appendLog("— el juego terminó —", null);
  status("game-state", "El juego terminó.");
});
EVENT.listen("launch/error", (data) => {
  playingInstance = "";
  applyLauncherTheme();
  status("game-state", data.payload);
});
EVENT.listen("sync/status", (data) => status("instances-status", data.payload));
EVENT.listen("sync/progress", (data) => setInstallProgress(data.payload.percent || 0, data.payload.status || ""));
EVENT.listen("java/progress", (data) => status("settings-status", data.payload));
const donateBtn = $("donate-btn");
if (donateBtn) {
  donateBtn.addEventListener("click", () => {
    const url = "https://ko-fi.com/nexuscreatorsjr";
    if (DEMO) {
      window.open(url, "_blank");
    } else {
      call("open_url", { url }).catch((e) => {
        try { window.open(url, "_blank"); } catch (_) {}
      });
    }
  });
}
function initTheme() {
  try {
    const saved = localStorage.getItem("nexus_theme");
    const prefersDark = window.matchMedia && window.matchMedia("(prefers-color-scheme: dark)").matches;
    if (saved === "dark" || (!saved && prefersDark)) {
      document.body.classList.add("dark");
    } else {
      document.body.classList.remove("dark");
    }
  } catch (_) {}
}

const themeToggle = $("theme-toggle");
if (themeToggle) {
  themeToggle.addEventListener("click", () => {
    const isDark = document.body.classList.toggle("dark");
    try {
      localStorage.setItem("nexus_theme", isDark ? "dark" : "light");
    } catch (_) {}
  });
}
function checkLauncherUpdate() {
  const chip = $("update-chip");
  if (!chip) return;
  call("update_check", {}).then(async (info) => {
    chip.textContent = "";
    const ver = document.createElement("span");
    ver.textContent = `NexusLauncher v${info.current || "?"}`;
    chip.appendChild(ver);
    if (info.available && info.latest && info.latest !== info.current) {
      const sep = document.createElement("span");
      sep.textContent = " · ";
      const link = document.createElement("a");
      link.href = "#";
      link.className = "up";
      link.textContent = `actualización disponible (v${info.latest})`;
      const apply = async () => {
        if (DEMO) { showModal(`<h2>Actualización v${escapeHtml(info.latest)}</h2><div class="group">Demo: se descargaría, instalaría y reiniciaría automáticamente.</div>`); return; }
        chip.textContent = "descargando actualización…";
        link.classList.remove("clickable");
        try {
          await call("update_apply", {});
          chip.textContent = "instalando y reiniciando…";
          // El instalador corre desconectado: si no logra cerrar/reiniciar, dar
          // una salida visible con el registro en vez de dejar el chip colgado.
          setTimeout(async () => {
            if (chip.textContent !== "instalando y reiniciando…") return;
            chip.textContent = "";
            chip.append(ver);
            const help = document.createElement("a");
            help.href = "#";
            help.className = "up";
            help.textContent = " · no se pudo reiniciar, ver ayuda";
            help.addEventListener("click", async (e) => {
              e.preventDefault();
              e.stopPropagation();
              let logTxt = "(sin registro)";
              try { logTxt = (await call("update_log", {})) || logTxt; } catch (_) {}
              showModal(`<h2>La actualización no terminó</h2><div class="group">El instalador no pudo reiniciar NexusLauncher. Descarga el ejecutable y reemplázalo manualmente.</div><p><a class="btn primary" href="${escapeHtml(info.download_url || "#")}" target="_blank" rel="noopener">Descargar manualmente</a></p><pre class="mono">${escapeHtml(String(logTxt).slice(-2500))}</pre>`);
            });
            chip.appendChild(help);
          }, 20000);
        } catch (err) {
          chip.textContent = "";
          chip.append(ver);
          showModal(`<h2>Error al actualizar</h2><pre class="mono">${escapeHtml(err.message)}</pre>`);
        }
      };
      link.addEventListener("click", async (e) => {
        e.preventDefault();
        e.stopPropagation();
        await apply();
      });
      chip.classList.add("clickable");
      chip.appendChild(sep);
      chip.appendChild(link);
      try {
        const s = await getSettings();
        if (s.auto_update) {
          chip.textContent = "";
          chip.append(ver);
          const note = document.createElement("span");
          note.textContent = " · " + (DEMO ? "actualización detectada" : "descargando actualización…");
          chip.appendChild(note);
          await apply();
        }
      } catch (_) {}
    }
  }).catch((err) => {
    chip.textContent = `NexusLauncher v${DEMO ? "0.1.0" : "?"} · sin conexión`;
    if (chip.title) chip.title = err.message;
  });
}
initTheme();
// Consultar automáticamente instancias disponibles y locales de inmediato al abrir el launcher
loadInstances().catch(() => {});
loadSource().catch(() => {});
runAutoSync(true).catch(() => {});

refreshLogin().then(() => {
  loadWhitelist().then(() => {
    loadInstances().catch(() => {});
    loadSource().catch(() => {});
    runAutoSync(true).catch(() => {});
  }).catch(() => {});
}).catch(() => {});
checkLauncherUpdate();
call("discord_set_page", { page: "login" }).catch(() => {});