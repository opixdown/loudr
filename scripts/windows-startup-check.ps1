# Starts Loudr on Windows and checks that it stays running, twice:
#
#  1. as a brand-new user (no settings yet: the first-run window opens), and
#  2. as a returning user (settings saved, first run done, dark theme).
#
# The second case is how 0.4.0 crashed on people's PCs ("state() called before
# manage()"): the saved settings changed what happens during startup.
#
# If Loudr stops within the first seconds, its crash report and log are
# printed and the script fails.
param([Parameter(Mandatory = $true)][string]$Exe)

$ErrorActionPreference = "Stop"
$env:LOUDR_NO_CRASH_DIALOG = "1"
$config = Join-Path $env:APPDATA "app.loudr.desktop"
$logs = Join-Path $env:LOCALAPPDATA "app.loudr.desktop\logs"

function Start-Loudr([string]$name) {
  Write-Host "`n===== $name ====="
  if (Test-Path $logs) { Remove-Item $logs -Recurse -Force }
  $app = Start-Process -FilePath $Exe -PassThru
  Start-Sleep -Seconds 25
  $stopped = $app.HasExited
  if (Test-Path $logs) {
    Get-ChildItem $logs | ForEach-Object {
      Write-Host "----- $($_.Name) -----"
      Get-Content $_.FullName
    }
  } else {
    Write-Host "No log folder at $logs"
  }
  Get-WinEvent -FilterHashtable @{ LogName = "Application"; Id = 1000 } -MaxEvents 3 -ErrorAction SilentlyContinue |
    Where-Object { $_.Message -match "loudr" } | ForEach-Object { Write-Host $_.Message }
  if ($stopped) {
    Write-Host "Loudr stopped on its own ($name), exit code $($app.ExitCode)"
    exit 1
  }
  Write-Host "Loudr was still running after 25 seconds ($name)"
  Stop-Process -Id $app.Id -Force
  Start-Sleep -Seconds 2
}

# 1. Brand-new user.
if (Test-Path $config) { Remove-Item $config -Recurse -Force }
Start-Loudr "first run"

# 2. Returning user: first run done, dark theme, other keys than the defaults.
New-Item -ItemType Directory -Force -Path $config | Out-Null
@{
  onboarded   = $true
  theme       = "dark"
  hotkey_code = 0xA4 # Left Alt
  speak_key   = 0x78 # F9
} | ConvertTo-Json | Set-Content -Encoding utf8NoBOM (Join-Path $config "settings.json")
Start-Loudr "returning user"
