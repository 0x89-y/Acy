@echo off
setlocal
rem Always run from this script's folder (the project root)
cd /d "%~dp0"

echo ===========================================
echo   Stage Acy release  (installer + latest.json)
echo ===========================================
echo.
echo Takes the NSIS installer that build.bat just produced, renames it to the
echo stable URL the updater points at, and writes latest.json with the matching
echo signature already pasted in. Run build.bat first.
echo.

rem ---- Version comes from tauri.conf.json, the single source of truth --------
set "VER="
for /f "usebackq delims=" %%v in (`powershell -NoProfile -Command "(Get-Content -Raw 'src-tauri\tauri.conf.json' | ConvertFrom-Json).version"`) do set "VER=%%v"
if not defined VER (
    echo ERROR: could not read "version" from src-tauri\tauri.conf.json.
    echo.
    pause
    exit /b 1
)
echo Version: %VER%
echo.

set "BUNDLE=src-tauri\target\release\bundle\nsis"
set "SETUP=%BUNDLE%\Acy_%VER%_x64-setup.exe"
set "SIG=%SETUP%.sig"

if not exist "%SETUP%" (
    echo ERROR: installer not found at "%SETUP%".
    echo Run build.bat first, and check the version matches.
    echo.
    pause
    exit /b 1
)
if not exist "%SIG%" (
    echo ERROR: signature not found at "%SIG%".
    echo createUpdaterArtifacts must be on and the build must have been signed.
    echo.
    pause
    exit /b 1
)

rem ---- Release notes: one line, shown in the updater's latest.json -----------
set "NOTES="
set /p "NOTES=Release notes (one line, blank for a default): "
if not defined NOTES set "NOTES=Acy %VER%. See Settings for the full changelog."

if not exist "release" mkdir "release"
copy /y "%SETUP%" "release\Acy-windows-x64-setup.exe" >nul

rem ---- Write latest.json with the signature inlined --------------------------
rem The signature file is a base64 blob; read it raw so no newline sneaks in.
rem Write with WriteAllText + UTF8Encoding($false), NOT `Set-Content -Encoding utf8`:
rem Windows PowerShell 5.1 emits a UTF-8 BOM, and the Tauri updater parses the
rem manifest with serde_json, which rejects a leading BOM. The client surfaces
rem that as the useless "error decoding response body".
powershell -NoProfile -Command ^
  "$sig = (Get-Content -Raw '%SIG%').Trim();" ^
  "$manifest = [ordered]@{" ^
  "  version   = '%VER%';" ^
  "  notes     = '%NOTES%';" ^
  "  pub_date  = (Get-Date).ToUniversalTime().ToString('yyyy-MM-ddTHH:mm:ssZ');" ^
  "  platforms = [ordered]@{ 'windows-x86_64' = [ordered]@{ signature = $sig; url = 'https://0x89-y.xyz/acy/Acy-windows-x64-setup.exe' } }" ^
  "};" ^
  "$json = $manifest | ConvertTo-Json -Depth 5;" ^
  "$out = Join-Path (Get-Location).Path 'release\latest.json';" ^
  "[System.IO.File]::WriteAllText($out, $json, (New-Object System.Text.UTF8Encoding $false))"

if not "%ERRORLEVEL%"=="0" (
    echo.
    echo FAILED to write release\latest.json
    echo.
    pause
    exit /b 1
)

echo.
echo STAGED OK  (v%VER%).
echo.
echo   release\Acy-windows-x64-setup.exe
echo   release\latest.json
echo.
echo   Next: upload BOTH files to https://0x89-y.xyz/acy/ .
echo   Existing installs pick up the update on their next check (on startup, then
echo   periodically), or immediately via Settings ^> Software updates ^> Check for updates.
echo.

start "" "release"

pause
