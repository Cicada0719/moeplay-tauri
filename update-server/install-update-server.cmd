@echo off
rem MoePlay update server - one click installer (run this on the update server PC)
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0install.ps1"
