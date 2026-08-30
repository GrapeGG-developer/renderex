# Renderex installer for Windows.
#
# Registers the .rx file extension so that double-clicking a scene file
# opens it in a Renderex window. See docs/INSTALL.ru.md for details.
#
# Usage (no administrator rights required):
#     powershell -ExecutionPolicy Bypass -File install\install.ps1
#     powershell -ExecutionPolicy Bypass -File install\install.ps1 -Rebuild
#     powershell -ExecutionPolicy Bypass -File install\install.ps1 -InstallDir "D:\Tools\Renderex"
#     powershell -ExecutionPolicy Bypass -File install\install.ps1 -Uninstall
param(
    [string]$InstallDir = "",
    [switch]$Machine,
    [switch]$Rebuild,
    [switch]$Uninstall
)

$ErrorActionPreference = "Stop"

$RepoRoot = Split-Path -Parent $PSScriptRoot   # repository root
$ExeName  = "renderex.exe"
$IcoName  = "renderex.ico"
$ProgId   = "Renderex.Scene"

if ($InstallDir -eq "") {
    $InstallDir = Join-Path $env:LOCALAPPDATA "Programs\Renderex"
}

if ($Machine) {
    $Root = "HKLM:\Software\Classes"
    $isAdmin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
    if (-not $isAdmin) {
        Write-Error "-Machine requires an elevated PowerShell (Run as administrator)."
    }
} else {
    $Root = "HKCU:\Software\Classes"
}

if ($Uninstall) {
    Write-Host "Uninstalling Renderex..."
    Remove-Item -Path "$Root\.rx" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "$Root\$ProgId" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "HKCU:\Software\Classes\.rx" -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -Path "HKCU:\Software\Classes\$ProgId" -Recurse -Force -ErrorAction SilentlyContinue
    if (Test-Path $InstallDir) {
        Remove-Item -Path $InstallDir -Recurse -Force
        Write-Host "Removed: $InstallDir"
    }
    try { & "$env:SystemRoot\System32\ie4uinit.exe" -show | Out-Null } catch { }
    Write-Host "Renderex uninstalled."
    exit 0
}

# --- build / copy ---------------------------------------------------------

$ReleaseExe = Join-Path $RepoRoot "target\release\$ExeName"
if ($Rebuild -or -not (Test-Path $ReleaseExe)) {
    Write-Host "Building release binary (cargo build --release)..."
    Push-Location $RepoRoot
    try {
        cargo build --release
        if ($LASTEXITCODE -ne 0) { throw "cargo build --release failed." }
    } finally {
        Pop-Location
    }
}

if (-not (Test-Path $ReleaseExe)) {
    throw "Binary not found: $ReleaseExe. Build it first: cargo build --release"
}
$Ico = Join-Path $RepoRoot "assets\$IcoName"
if (-not (Test-Path $Ico)) {
    throw "Icon not found: $Ico"
}

New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
Copy-Item -Path $ReleaseExe -Destination (Join-Path $InstallDir $ExeName) -Force
Copy-Item -Path $Ico -Destination (Join-Path $InstallDir $IcoName) -Force

$ExePath = Join-Path $InstallDir $ExeName
$IcoPath = Join-Path $InstallDir $IcoName

# --- register file association --------------------------------------------

Write-Host "Registering .rx -> $ProgId under $Root ..."

New-Item -Path "$Root\.rx" -Force | Out-Null
Set-ItemProperty -Path "$Root\.rx" -Name "(default)" -Value $ProgId

New-Item -Path "$Root\$ProgId" -Force | Out-Null
Set-ItemProperty -Path "$Root\$ProgId" -Name "(default)" -Value "Renderex scene"

New-Item -Path "$Root\$ProgId\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "$Root\$ProgId\DefaultIcon" -Name "(default)" -Value $IcoPath

New-Item -Path "$Root\$ProgId\shell" -Force | Out-Null
Set-ItemProperty -Path "$Root\$ProgId\shell" -Name "(default)" -Value "open"

New-Item -Path "$Root\$ProgId\shell\open" -Force | Out-Null
Set-ItemProperty -Path "$Root\$ProgId\shell\open" -Name "(default)" -Value "Open in Renderex"

New-Item -Path "$Root\$ProgId\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path "$Root\$ProgId\shell\open\command" -Name "(default)" -Value "`"$ExePath`" `"%1`""

# Refresh the icon cache so Explorer shows the new icon right away.
try { & "$env:SystemRoot\System32\ie4uinit.exe" -show | Out-Null } catch { }

Write-Host ""
Write-Host "Done!"
Write-Host "  Installed to : $InstallDir"
Write-Host "  Registered   : .rx -> $ProgId ($Root)"
Write-Host "  Now double-click any .rx file to render it in a Renderex window."
Write-Host "  To uninstall : powershell -ExecutionPolicy Bypass -File install\install.ps1 -Uninstall"
