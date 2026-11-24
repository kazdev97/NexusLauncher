@echo off
chcp 65001 >nul
echo ============================================
echo  Arreglar actualizacion atascada
echo ============================================
echo.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0arreglar-actualizacion.ps1" %*
echo.
if errorlevel 1 (
  echo Si fallo por permisos, haz clic derecho en este BAT y elige "Ejecutar como administrador".
)
pause
