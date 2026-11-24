# Arregla un launcher que se quedo en "instalando y reiniciando".
# Busca el .new.exe ya descargado, cierra el launcher por PID (no por nombre),
# reemplaza el ejecutable y lo vuelve a abrir.
param(
  [string]$TargetExe = "",
  [string]$NewExe = ""
)

$ErrorActionPreference = "Stop"
$names = "nexuslauncher|paulauncher|NexusLauncher|PauLauncher"

function Get-UpdateDirs {
  $dirs = @(
    "$env:USERPROFILE\Documents\NexusLauncher\updates",
    "$env:APPDATA\NexusLauncher\updates",
    "$env:USERPROFILE\Documents\PauLauncher\updates",
    "$env:APPDATA\PauLauncher\updates"
  )
  $dirs | Where-Object { Test-Path -LiteralPath $_ }
}

if (-not $NewExe) {
  $candidate = Get-UpdateDirs | ForEach-Object {
    Get-ChildItem -LiteralPath $_ -Filter "*.new.exe" -File -ErrorAction SilentlyContinue
  } | Sort-Object LastWriteTime -Descending | Select-Object -First 1
  if ($candidate) { $NewExe = $candidate.FullName }
}
if (-not $NewExe -or -not (Test-Path -LiteralPath $NewExe)) {
  throw "No se encontro el .new.exe descargado. Descarga builds/nexuslauncher.exe desde GitHub y usa -NewExe 'C:\ruta\nexuslauncher.exe'"
}

if (-not $TargetExe) {
  $proc = Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
    Where-Object { $_.Name -match $names -and $_.ExecutablePath -and $_.ExecutablePath -notlike "*\updates\*" } |
    Select-Object -First 1
  if ($proc) { $TargetExe = $proc.ExecutablePath }
}
if (-not $TargetExe -or -not (Test-Path -LiteralPath $TargetExe)) {
  throw "No se pudo localizar el exe del launcher. Ejecuta: arreglar-actualizacion.ps1 -TargetExe 'C:\ruta\nexuslauncher.exe'"
}

Write-Host "Cerrando el launcher: $TargetExe"
Get-CimInstance Win32_Process -ErrorAction SilentlyContinue |
  Where-Object { $_.ExecutablePath -eq $TargetExe } |
  ForEach-Object { Stop-Process -Id $_.ProcessId -Force -ErrorAction SilentlyContinue }

Start-Sleep -Seconds 2
$copied = $false
for ($i = 0; $i -lt 20; $i++) {
  try {
    Copy-Item -LiteralPath $NewExe -Destination $TargetExe -Force -ErrorAction Stop
    $copied = $true
    break
  } catch {
    Start-Sleep -Milliseconds 500
  }
}
if (-not $copied) { throw "No se pudo copiar sobre $TargetExe. Cierra el launcher desde el Administrador de tareas o ejecuta este script como administrador." }

Remove-Item -LiteralPath $NewExe -Force -ErrorAction SilentlyContinue
Start-Process -FilePath $TargetExe
Write-Host "Listo: el launcher se abrio con la version nueva (0.1.3+). Las proximas actualizaciones ya funcionaran solas."
