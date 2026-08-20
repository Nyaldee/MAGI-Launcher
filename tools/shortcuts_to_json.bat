@echo off
rem Double-clic direct : capture .\shortcuts\ (a cote de ce .bat).
rem Glisser-deposer un DOSSIER sur ce .bat : capture ce dossier a la place.

choice /C YN /N /M "Include shortcuts' working directory (cwd) where set? [Y]es / [N]o : "
if errorlevel 2 (
    set "CWD_ARG=-IncludeCwd 0"
) else (
    set "CWD_ARG=-IncludeCwd 1"
)

if "%~1"=="" (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0shortcuts_to_json.ps1" %CWD_ARG%
) else (
    powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0shortcuts_to_json.ps1" -SourceFolder "%~1" %CWD_ARG%
)
pause
