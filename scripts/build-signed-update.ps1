param(
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)$')]
    [string]$Version
)

$ErrorActionPreference = 'Stop'
$releaseRepo = Split-Path -Parent $PSScriptRoot
$desktopPath = Join-Path $releaseRepo 'desktop-p0'
$config = Get-Content -LiteralPath (Join-Path $desktopPath 'tauri.conf.json') -Raw | ConvertFrom-Json
if ($config.version -ne $Version) { throw 'Requested version must match desktop-p0/tauri.conf.json. Update release metadata first.' }
$cargoManifest = Get-Content -LiteralPath (Join-Path $desktopPath 'Cargo.toml') -Raw
if ($cargoManifest -notmatch ('(?m)^version\s*=\s*"' + [regex]::Escape($Version) + '"\s*$')) {
    throw 'Requested version must match desktop-p0/Cargo.toml.'
}
foreach ($variable in @('TAURI_SIGNING_PRIVATE_KEY', 'AGENTMETER_UPDATE_PUBLIC_KEY', 'AGENTMETER_UPDATE_ENDPOINT')) {
    if ([string]::IsNullOrWhiteSpace([Environment]::GetEnvironmentVariable($variable))) { throw "Required build environment variable is missing: $variable" }
}
$expectedEndpoint = 'https://github.com/richshangwei/AgentMeter/releases/latest/download/latest.json'
if ($env:AGENTMETER_UPDATE_ENDPOINT -ne $expectedEndpoint) { throw 'Update endpoint must be the pinned AgentMeter latest.json endpoint.' }
# Private signing material is inherited through the environment, never placed in arguments or files.
# TAURI_SIGNING_PRIVATE_KEY_PASSWORD is optional only for an unencrypted existing key.
Get-Command cargo -ErrorAction Stop | Out-Null
Get-Command node -ErrorAction Stop | Out-Null
Push-Location $desktopPath
try {
    $draftDirectory = Join-Path $desktopPath ('target/update-drafts/' + $Version + '-' + [guid]::NewGuid().ToString('N'))
    New-Item -ItemType Directory -Path $draftDirectory | Out-Null
    # Tauri CLI also needs the public key to validate its signing configuration.
    $publicConfigPath = Join-Path $draftDirectory 'public-updater-config.json'
    $publicConfig = @{ plugins = @{ updater = @{ pubkey = $env:AGENTMETER_UPDATE_PUBLIC_KEY } } } | ConvertTo-Json -Depth 4
    [IO.File]::WriteAllText($publicConfigPath, $publicConfig, (New-Object Text.UTF8Encoding($false)))
    & cargo tauri build --ci --config tauri.updater.conf.json --config $publicConfigPath --target x86_64-pc-windows-msvc --bundles nsis -- --offline --locked
    if ($LASTEXITCODE -ne 0) { throw 'Signed Tauri build failed; no manifest was created.' }
    $builtInstaller = Join-Path $desktopPath "target/x86_64-pc-windows-msvc/release/bundle/nsis/AgentMeter P0_${Version}_x64-setup.exe"
    & (Join-Path $PSScriptRoot 'inspect-nsis.ps1') -InstallerPath $builtInstaller -ExpectedVersion $Version
    # GitHub rewrites spaces in release asset names. Publish an explicit safe name
    # so the manifest, backend allowlist and public asset URL remain identical.
    $installer = Join-Path $draftDirectory "AgentMeter-P0_${Version}_x64-setup.exe"
    Copy-Item -LiteralPath $builtInstaller -Destination $installer
    Copy-Item -LiteralPath ($builtInstaller + '.sig') -Destination ($installer + '.sig')
    & node (Join-Path $PSScriptRoot 'create-update-manifest.mjs') $Version $installer (Join-Path $draftDirectory 'latest.json')
    if ($LASTEXITCODE -ne 0) { throw 'Update manifest validation failed.' }
    Write-Output "Local release draft ready: $draftDirectory"
    Write-Output 'Nothing has been uploaded, published or installed.'
} finally {
    Pop-Location
}
