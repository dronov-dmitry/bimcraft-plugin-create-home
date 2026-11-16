# Install or update the House Builder BIMCraft plugin from the newest GitHub
# release of dronov-dmitry/bimcraft-plugin-create-home.
#
# One command (Windows, PowerShell):
#   irm https://raw.githubusercontent.com/dronov-dmitry/bimcraft-plugin-create-home/main/scripts/install.ps1 | iex
#
# Environment, all optional:
#   BIMCRAFT_PLUGINS  the plugins folder to install into
#   PLUGIN_REPO       "owner/name" on GitHub
#   PLUGIN_VERSION    a release tag without "v" (default: the newest one)
#   PLUGIN_ZIP_URL    a direct archive URL, overriding the repository and version
#
# Requirements: PowerShell 5.1+.

$ErrorActionPreference = "Stop"

$PluginId = "house-builder"
$PluginModule = "house_builder.wasm"
$Repo = if ($env:PLUGIN_REPO) { $env:PLUGIN_REPO } else { "dronov-dmitry/bimcraft-plugin-create-home" }
$Version = $env:PLUGIN_VERSION
$KeepBackups = 3

if ($env:BIMCRAFT_PLUGINS) {
    $plugins = $env:BIMCRAFT_PLUGINS
} elseif ($env:APPDATA) {
    $plugins = Join-Path $env:APPDATA "BIMCraft\plugins"
} else {
    throw "APPDATA is not set; point BIMCRAFT_PLUGINS at the plugins folder"
}

if ($env:PLUGIN_ZIP_URL) {
    $url = $env:PLUGIN_ZIP_URL
} elseif ($Version) {
    $url = "https://github.com/$Repo/releases/download/v$Version/$PluginId-$Version.zip"
} else {
    $url = "https://github.com/$Repo/releases/latest/download/$PluginId.zip"
}

$destination = Join-Path $plugins $PluginId
$stage = Join-Path $plugins ".$PluginId.new.$PID"
$archive = Join-Path $plugins ".$PluginId.zip.$PID"

New-Item -ItemType Directory -Force -Path $plugins | Out-Null
if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }

try {
    Write-Host "Downloading $url"
    Invoke-WebRequest -UseBasicParsing -Uri $url -OutFile $archive
    New-Item -ItemType Directory -Force -Path $stage | Out-Null
    Expand-Archive -Path $archive -DestinationPath $stage -Force
    Remove-Item -Force $archive

    $manifest = Join-Path $stage "plugin.json"
    $module = Join-Path $stage $PluginModule
    if (-not (Test-Path $manifest)) { throw "the archive has no plugin.json" }
    if (-not (Test-Path $module)) { throw "the archive has no $PluginModule" }
    $declared = Get-Content -Raw -Path $manifest | ConvertFrom-Json
    if ($declared.id -ne $PluginId) { throw "the archive's plugin.json is not $PluginId" }
    $bytes = [System.IO.File]::ReadAllBytes($module)
    if ($bytes.Length -lt 4 -or $bytes[0] -ne 0 -or $bytes[1] -ne 0x61 -or $bytes[2] -ne 0x73 -or $bytes[3] -ne 0x6d) {
        throw "$PluginModule is not a WebAssembly module"
    }

    if (Test-Path $destination) {
        $stamp = (Get-Date).ToUniversalTime().ToString("yyyyMMdd-HHmmss")
        $backup = Join-Path $plugins ".$PluginId.backup-$stamp-$PID"
        $suffix = 1
        while (Test-Path $backup) {
            $backup = Join-Path $plugins ".$PluginId.backup-$stamp-$PID-$suffix"
            $suffix++
        }
        Move-Item -Path $destination -Destination $backup
        Write-Host "Previous version kept at $backup"
    }
    Move-Item -Path $stage -Destination $destination

    Get-ChildItem -Path $plugins -Directory -Filter ".$PluginId.backup-*" |
        Sort-Object Name -Descending |
        Select-Object -Skip $KeepBackups |
        ForEach-Object { Remove-Item -Recurse -Force $_.FullName }

    Write-Host "Installed $PluginId into $destination"
    Write-Host "Restart BIMCraft, or press Refresh in Manage > Plugins."
} finally {
    if (Test-Path $archive) { Remove-Item -Force $archive }
    if (Test-Path $stage) { Remove-Item -Recurse -Force $stage }
}
