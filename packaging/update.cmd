@echo off
rem Pasang build terbaru (target\release) ke notch yang terpasang. Cukup klik dua kali.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0update.ps1"
if errorlevel 1 pause
