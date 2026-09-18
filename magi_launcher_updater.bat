@echo off
color 0A
cd /d "%~dp0"

taskkill /IM "magi_launcher.exe" /F >nul 2>&1
timeout /t 2 /nobreak >nul

echo Backing up your configuration...
if exist "apps.json" copy /y "apps.json" "%TEMP%\magi_apps.json.bak" >nul
if exist "themes.json" copy /y "themes.json" "%TEMP%\magi_themes.json.bak" >nul

echo Downloading latest version...
curl -L -o "%TEMP%\MAGILauncher-update.zip" "https://github.com/Nyaldee/MAGI-Launcher/releases/latest/download/MAGI.Launcher.Windows.zip" || (echo Download failed. & pause & exit /b 1)

echo Installing...
tar -xf "%TEMP%\MAGILauncher-update.zip" -C .. --exclude="MAGI Launcher/magi_launcher_updater.bat" || (echo Extraction failed. & pause & exit /b 1)

del /q "%TEMP%\MAGILauncher-update.zip"

echo Restoring your configuration...
if exist "%TEMP%\magi_apps.json.bak" move /y "%TEMP%\magi_apps.json.bak" "apps.json" >nul
if exist "%TEMP%\magi_themes.json.bak" move /y "%TEMP%\magi_themes.json.bak" "themes.json" >nul

if exist "magi_launcher.exe" (
    start "" "magi_launcher.exe"
) else (
    echo.
    echo Move this file into the "MAGI Launcher" folder.
    pause
)
