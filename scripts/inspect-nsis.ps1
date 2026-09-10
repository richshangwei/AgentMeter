param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,
    [string]$ExpectedVersion = '0.1.0'
)

$ErrorActionPreference = 'Stop'
$resolvedInstaller = [IO.Path]::GetFullPath($InstallerPath)
if (-not (Test-Path -LiteralPath $resolvedInstaller -PathType Leaf)) {
    throw "Installer does not exist: $resolvedInstaller"
}
$sevenZip = Get-Command 7z.exe -ErrorAction Stop
$temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
$inspectionDirectory = Join-Path $temporaryRoot ("agentmeter-nsis-inspect-" + [guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $inspectionDirectory | Out-Null

try {
    $listing = & $sevenZip.Source 'l' '-slt' $resolvedInstaller 2>&1
    if ($LASTEXITCODE -ne 0) {
        throw "7-Zip could not inspect the installer"
    }
    $listingText = $listing -join "`n"
    if ($listingText -notmatch '(?m)^Type = Nsis$') {
        throw "Artifact is not an NSIS archive"
    }
    if ($listingText -notmatch '(?m)^Path = agentmeter-desktop-p0\.exe$') {
        throw "Installer does not contain the expected AgentMeter executable"
    }

    & $sevenZip.Source 'e' '-y' ("-o$inspectionDirectory") $resolvedInstaller 'agentmeter-desktop-p0.exe' | Out-Null
    if ($LASTEXITCODE -ne 0) {
        throw "7-Zip could not extract the embedded AgentMeter executable"
    }
    $embeddedPath = Join-Path $inspectionDirectory 'agentmeter-desktop-p0.exe'
    $embedded = Get-Item -LiteralPath $embeddedPath
    if ($embedded.VersionInfo.ProductVersion -ne $ExpectedVersion) {
        throw "Embedded product version '$($embedded.VersionInfo.ProductVersion)' does not match '$ExpectedVersion'"
    }
    $embeddedAscii = [Text.Encoding]::ASCII.GetString([IO.File]::ReadAllBytes($embeddedPath))
    $startupDiagnosticMarkers = @(
        'desktop_startup_failed',
        'startup_registration_failed',
        'startup-error.log',
        'agentmeter.desktop-startup-diagnostic/v1'
    )
    foreach ($marker in $startupDiagnosticMarkers) {
        if (-not $embeddedAscii.Contains($marker)) {
            throw "Embedded executable is missing startup diagnostic marker: $marker"
        }
    }
    $copilotBillingMarkers = @(
        'refresh_copilot',
        'save_copilot_source',
        'clear_copilot_source',
        'copilot_authoritative_billing_usage',
        'not_exposed_by_billing_usage_endpoint',
        '2026-03-10'
    )
    foreach ($marker in $copilotBillingMarkers) {
        if (-not $embeddedAscii.Contains($marker)) {
            throw "Embedded executable is missing Copilot Billing marker: $marker"
        }
    }

    $installer = Get-Item -LiteralPath $resolvedInstaller
    [pscustomobject]@{
        schema_version = 'agentmeter.nsis-inspection/v1'
        installer_path = $installer.FullName
        installer_bytes = $installer.Length
        installer_sha256 = (Get-FileHash -LiteralPath $installer.FullName -Algorithm SHA256).Hash
        installer_signature = (Get-AuthenticodeSignature -LiteralPath $installer.FullName).Status.ToString()
        archive_type = 'nsis'
        download_component_present = $listingText -match '(?m)^Path = \$PLUGINSDIR\\NSISdl\.dll$'
        startup_diagnostic_markers_present = $true
        copilot_billing_markers_present = $true
        embedded_executable = $embedded.Name
        embedded_bytes = $embedded.Length
        embedded_sha256 = (Get-FileHash -LiteralPath $embedded.FullName -Algorithm SHA256).Hash
        embedded_signature = (Get-AuthenticodeSignature -LiteralPath $embedded.FullName).Status.ToString()
        product_version = $embedded.VersionInfo.ProductVersion
        file_version = $embedded.VersionInfo.FileVersion
    } | ConvertTo-Json -Depth 3
} finally {
    $resolvedInspection = [IO.Path]::GetFullPath($inspectionDirectory)
    if (-not $resolvedInspection.StartsWith($temporaryRoot, [StringComparison]::OrdinalIgnoreCase)) {
        throw "Refusing to remove an inspection directory outside the system temporary root"
    }
    if (Test-Path -LiteralPath $resolvedInspection) {
        Remove-Item -LiteralPath $resolvedInspection -Recurse -Force
    }
}
