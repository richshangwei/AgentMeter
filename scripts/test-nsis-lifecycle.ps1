param(
    [Parameter(Mandatory = $true)]
    [string]$InstallerPath,
    [Parameter(Mandatory = $true)]
    [ValidatePattern('^[A-Fa-f0-9]{64}$')]
    [string]$ExpectedSha256,
    [string]$ExpectedVersion = '0.1.0',
    [string]$UpgradeInstallerPath,
    [ValidatePattern('^[A-Fa-f0-9]{64}$')]
    [string]$UpgradeExpectedSha256,
    [string]$UpgradeExpectedVersion,
    [Parameter(Mandatory = $true)]
    [ValidateSet('preserve', 'purge')]
    [string]$DataPolicy,
    [Parameter(Mandatory = $true)]
    [string]$EvidencePath,
    [ValidateRange(30, 600)]
    [int]$IdleSampleSeconds = 300,
    [switch]$AllowSystemMutation
)

$ErrorActionPreference = 'Stop'
if ($env:OS -ne 'Windows_NT') { throw 'This lifecycle probe requires Windows' }
if (-not $AllowSystemMutation) {
    throw 'Refusing system changes without -AllowSystemMutation'
}

$upgradeArgumentCount = @(
    $UpgradeInstallerPath,
    $UpgradeExpectedSha256,
    $UpgradeExpectedVersion
).Where({ -not [string]::IsNullOrWhiteSpace($_) }).Count
if ($upgradeArgumentCount -ne 0 -and $upgradeArgumentCount -ne 3) {
    throw 'UpgradeInstallerPath, UpgradeExpectedSha256 and UpgradeExpectedVersion must be supplied together'
}
$upgradeRequested = $upgradeArgumentCount -eq 3
try { $baseVersion = [version]$ExpectedVersion } catch { throw "Invalid ExpectedVersion: $ExpectedVersion" }

$resolvedInstaller = [IO.Path]::GetFullPath($InstallerPath)
if (-not (Test-Path -LiteralPath $resolvedInstaller -PathType Leaf)) {
    throw "Installer does not exist: $resolvedInstaller"
}
$actualHash = (Get-FileHash -LiteralPath $resolvedInstaller -Algorithm SHA256).Hash
if ($actualHash -ne $ExpectedSha256.ToUpperInvariant()) {
    throw "Installer SHA-256 mismatch; expected $ExpectedSha256 but observed $actualHash"
}
$resolvedUpgradeInstaller = $null
$actualUpgradeHash = $null
if ($upgradeRequested) {
    $resolvedUpgradeInstaller = [IO.Path]::GetFullPath($UpgradeInstallerPath)
    if (-not (Test-Path -LiteralPath $resolvedUpgradeInstaller -PathType Leaf)) {
        throw "Upgrade installer does not exist: $resolvedUpgradeInstaller"
    }
    $actualUpgradeHash = (Get-FileHash -LiteralPath $resolvedUpgradeInstaller -Algorithm SHA256).Hash
    if ($actualUpgradeHash -ne $UpgradeExpectedSha256.ToUpperInvariant()) {
        throw "Upgrade installer SHA-256 mismatch; expected $UpgradeExpectedSha256 but observed $actualUpgradeHash"
    }
    try { $upgradeVersion = [version]$UpgradeExpectedVersion } catch { throw "Invalid UpgradeExpectedVersion: $UpgradeExpectedVersion" }
    if ($upgradeVersion -le $baseVersion) {
        throw "UpgradeExpectedVersion must be greater than ExpectedVersion ($ExpectedVersion)"
    }
    if ($actualUpgradeHash -eq $actualHash) {
        throw 'Upgrade installer must not be identical to the base installer'
    }
}
$resolvedEvidence = [IO.Path]::GetFullPath($EvidencePath)
$evidenceParent = Split-Path -Parent $resolvedEvidence
if (-not (Test-Path -LiteralPath $evidenceParent -PathType Container)) {
    throw "Evidence parent directory does not exist: $evidenceParent"
}
if (-not $env:APPDATA -or -not $env:LOCALAPPDATA) {
    throw 'APPDATA and LOCALAPPDATA must be available for this current-user lifecycle probe'
}

$bundleId = 'com.agentmeter.p0'
$productName = 'AgentMeter P0'
$startupName = 'AgentMeter P0'
$legacyStartupName = 'AgentMeterP0'
$runKey = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Run'
$uninstallKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\$productName"
$roamingData = [IO.Path]::GetFullPath((Join-Path $env:APPDATA $bundleId))
$localData = [IO.Path]::GetFullPath((Join-Path $env:LOCALAPPDATA $bundleId))
$probeRoot = [IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetTempPath()) ("agentmeter-install-probe-" + [guid]::NewGuid().ToString('N'))))
$installedExecutable = Join-Path $probeRoot 'agentmeter-desktop-p0.exe'
$uninstaller = Join-Path $probeRoot 'uninstall.exe'
$roamingMarker = Join-Path $roamingData 'lifecycle-probe.marker'
$localMarker = Join-Path $localData 'lifecycle-probe.marker'

function Get-RunValue([string]$Name) {
    try {
        $properties = Get-ItemProperty -LiteralPath $runKey -Name $Name -ErrorAction Stop
        $property = $properties.PSObject.Properties[$Name]
        if ($property) { return $property.Value }
        return $null
    }
    catch [System.Management.Automation.ItemNotFoundException] { $null }
    catch [System.Management.Automation.PSArgumentException] { $null }
}

function Invoke-BoundedProcess([string]$FilePath, [string[]]$ArgumentList, [int]$TimeoutSeconds) {
    $process = Start-Process -FilePath $FilePath -ArgumentList $ArgumentList -PassThru -WindowStyle Hidden
    if (-not $process.WaitForExit($TimeoutSeconds * 1000)) {
        Stop-Process -Id $process.Id -Force -ErrorAction SilentlyContinue
        throw "Process exceeded ${TimeoutSeconds}s: $FilePath"
    }
    if ($process.ExitCode -ne 0) {
        throw "Process exited with code $($process.ExitCode): $FilePath"
    }
}

function Wait-AppReady(
    [string]$FilePath,
    [System.Diagnostics.Process]$Primary,
    [System.Diagnostics.Stopwatch]$LaunchTimer,
    [int]$TimeoutSeconds
) {
    $deadline = [DateTimeOffset]::UtcNow.AddSeconds($TimeoutSeconds)
    while ([DateTimeOffset]::UtcNow -lt $deadline) {
        if ($Primary.HasExited) { throw 'Installed application exited before readiness verification' }
        $probe = Start-Process -FilePath $FilePath -ArgumentList @('--probe-ready') -PassThru -WindowStyle Hidden
        if (-not $probe.WaitForExit(5000)) {
            Stop-Process -Id $probe.Id -Force -ErrorAction SilentlyContinue
        } elseif ($probe.ExitCode -eq 0) {
            $LaunchTimer.Stop()
            return $LaunchTimer.Elapsed.TotalMilliseconds
        }
        Start-Sleep -Milliseconds 50
    }
    throw "Application readiness probe exceeded ${TimeoutSeconds}s"
}

function Get-WebView2Version {
    $client = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    foreach ($path in @(
        "HKLM:\SOFTWARE\WOW6432Node\Microsoft\EdgeUpdate\Clients\$client",
        "HKLM:\SOFTWARE\Microsoft\EdgeUpdate\Clients\$client",
        "HKCU:\Software\Microsoft\EdgeUpdate\Clients\$client"
    )) {
        $version = (Get-ItemProperty -LiteralPath $path -Name pv -ErrorAction SilentlyContinue).pv
        if ($version) { return $version }
    }
    return $null
}

if (Get-Process -Name 'agentmeter-desktop-p0' -ErrorAction SilentlyContinue) {
    throw 'Refusing to run while an AgentMeter process already exists'
}
if ((Test-Path -LiteralPath $uninstallKey) -or (Get-RunValue $startupName) -or (Get-RunValue $legacyStartupName)) {
    throw 'Refusing to run over an existing AgentMeter installation or startup registration'
}
if ((Test-Path -LiteralPath $roamingData) -or (Test-Path -LiteralPath $localData)) {
    throw 'Refusing to run where AgentMeter user data already exists'
}

$startedAt = [DateTimeOffset]::UtcNow
$webViewBefore = Get-WebView2Version
$installerSizeBytes = (Get-Item -LiteralPath $resolvedInstaller).Length
$upgradeInstallerSizeBytes = if ($upgradeRequested) { (Get-Item -LiteralPath $resolvedUpgradeInstaller).Length } else { $null }
$cleanupNeeded = $false
$roamingPreserved = $false
$localPreserved = $false
$primary = $null
$restarted = $null
$upgraded = $null
try {
    $installTimer = [Diagnostics.Stopwatch]::StartNew()
    Invoke-BoundedProcess $resolvedInstaller @('/S', "/D=$probeRoot") 120
    $installTimer.Stop()
    $cleanupNeeded = $true
    if (-not (Test-Path -LiteralPath $installedExecutable -PathType Leaf)) {
        throw 'Installed AgentMeter executable is missing'
    }
    if (-not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        throw 'Generated uninstaller is missing'
    }
    if (-not (Test-Path -LiteralPath $uninstallKey)) {
        throw 'Current-user uninstall registration is missing'
    }
    $installedVersion = (Get-Item -LiteralPath $installedExecutable).VersionInfo.ProductVersion
    if ($installedVersion -ne $ExpectedVersion) {
        throw "Installed product version '$installedVersion' does not match '$ExpectedVersion'"
    }

    $installedSizeBytes = (Get-ChildItem -LiteralPath $probeRoot -File -Recurse | Measure-Object -Property Length -Sum).Sum
    $firstLaunchTimer = [Diagnostics.Stopwatch]::StartNew()
    $primary = Start-Process -FilePath $installedExecutable -ArgumentList @('--hidden') -PassThru -WindowStyle Hidden
    $firstLaunchReadyMs = Wait-AppReady $installedExecutable $primary $firstLaunchTimer 15
    $primary.Refresh()
    $idleCpuStartMs = $primary.TotalProcessorTime.TotalMilliseconds
    Start-Sleep -Seconds $IdleSampleSeconds
    if ($primary.HasExited) { throw 'Installed application exited during idle resource sampling' }
    $primary.Refresh()
    $idleWorkingSetBytes = $primary.WorkingSet64
    $idlePrivateMemoryBytes = $primary.PrivateMemorySize64
    $idleProcessorTimeMs = $primary.TotalProcessorTime.TotalMilliseconds - $idleCpuStartMs
    $processorCount = [Math]::Max(1, [Environment]::ProcessorCount)
    $idleCpuPercent = ($idleProcessorTimeMs / ($IdleSampleSeconds * 1000 * $processorCount)) * 100
    Invoke-BoundedProcess $installedExecutable @('--request-exit') 15
    if (-not $primary.WaitForExit(15000)) {
        Stop-Process -Id $primary.Id -Force -ErrorAction SilentlyContinue
        throw 'Installed application did not complete explicit exit'
    }

    Invoke-BoundedProcess $installedExecutable @('--startup-enable') 15
    $startupCommand = Get-RunValue $startupName
    if (-not $startupCommand -or $startupCommand -notmatch '--hidden') {
        throw 'Startup registration was not created with hidden launch'
    }
    if (Get-RunValue $legacyStartupName) { throw 'Legacy startup registration remains' }

    New-Item -ItemType Directory -Path $roamingData -Force | Out-Null
    New-Item -ItemType Directory -Path $localData -Force | Out-Null
    Set-Content -LiteralPath $roamingMarker -Value 'AgentMeter lifecycle probe' -Encoding UTF8
    Set-Content -LiteralPath $localMarker -Value 'AgentMeter lifecycle probe' -Encoding UTF8

    $reinstallTimer = [Diagnostics.Stopwatch]::StartNew()
    Invoke-BoundedProcess $resolvedInstaller @('/S', "/D=$probeRoot") 120
    $reinstallTimer.Stop()
    if (-not (Test-Path -LiteralPath $installedExecutable -PathType Leaf) -or
        -not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        throw 'Same-version reinstall did not preserve the installed application'
    }
    if (-not (Test-Path -LiteralPath $roamingMarker -PathType Leaf) -or
        -not (Test-Path -LiteralPath $localMarker -PathType Leaf)) {
        throw 'Same-version reinstall did not preserve application data'
    }
    if (-not (Get-RunValue $startupName) -or (Get-RunValue $legacyStartupName)) {
        throw 'Same-version reinstall did not preserve the current startup registration'
    }
    $reinstalledVersion = (Get-Item -LiteralPath $installedExecutable).VersionInfo.ProductVersion
    if ($reinstalledVersion -ne $ExpectedVersion) {
        throw "Same-version reinstall changed product version to '$reinstalledVersion'"
    }

    $restartTimer = [Diagnostics.Stopwatch]::StartNew()
    $restarted = Start-Process -FilePath $installedExecutable -ArgumentList @('--hidden') -PassThru -WindowStyle Hidden
    $restartReadyMs = Wait-AppReady $installedExecutable $restarted $restartTimer 15
    Invoke-BoundedProcess $installedExecutable @('--request-exit') 15
    if (-not $restarted.WaitForExit(15000)) {
        Stop-Process -Id $restarted.Id -Force -ErrorAction SilentlyContinue
        throw 'Reinstalled application did not complete explicit exit'
    }

    $upgradeElapsedMs = $null
    $upgradeReadyMs = $null
    if ($upgradeRequested) {
        $upgradeTimer = [Diagnostics.Stopwatch]::StartNew()
        Invoke-BoundedProcess $resolvedUpgradeInstaller @('/S', "/D=$probeRoot") 120
        $upgradeTimer.Stop()
        $upgradeElapsedMs = $upgradeTimer.Elapsed.TotalMilliseconds
        if (-not (Test-Path -LiteralPath $installedExecutable -PathType Leaf) -or
            -not (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
            throw 'Upgrade did not preserve the installed application'
        }
        $observedUpgradeVersion = (Get-Item -LiteralPath $installedExecutable).VersionInfo.ProductVersion
        if ($observedUpgradeVersion -ne $UpgradeExpectedVersion) {
            throw "Upgraded product version '$observedUpgradeVersion' does not match '$UpgradeExpectedVersion'"
        }
        if (-not (Test-Path -LiteralPath $roamingMarker -PathType Leaf) -or
            -not (Test-Path -LiteralPath $localMarker -PathType Leaf)) {
            throw 'Upgrade did not preserve application data'
        }
        if (-not (Get-RunValue $startupName) -or (Get-RunValue $legacyStartupName)) {
            throw 'Upgrade did not preserve the current startup registration'
        }
        $upgradeLaunchTimer = [Diagnostics.Stopwatch]::StartNew()
        $upgraded = Start-Process -FilePath $installedExecutable -ArgumentList @('--hidden') -PassThru -WindowStyle Hidden
        $upgradeReadyMs = Wait-AppReady $installedExecutable $upgraded $upgradeLaunchTimer 15
        Invoke-BoundedProcess $installedExecutable @('--request-exit') 15
        if (-not $upgraded.WaitForExit(15000)) {
            Stop-Process -Id $upgraded.Id -Force -ErrorAction SilentlyContinue
            throw 'Upgraded application did not complete explicit exit'
        }
    }

    Invoke-BoundedProcess $installedExecutable @('--startup-disable') 15
    if ((Get-RunValue $startupName) -or (Get-RunValue $legacyStartupName)) {
        throw 'Startup registration was not removed'
    }

    $uninstallArguments = @('/S')
    if ($DataPolicy -eq 'purge') { $uninstallArguments += '/PURGE' }
    Invoke-BoundedProcess $uninstaller $uninstallArguments 120
    $cleanupNeeded = $false
    Start-Sleep -Milliseconds 500
    if (Test-Path -LiteralPath $probeRoot) { throw 'Install directory remains after uninstall' }
    if (Test-Path -LiteralPath $uninstallKey) { throw 'Uninstall registration remains' }
    if ((Get-RunValue $startupName) -or (Get-RunValue $legacyStartupName)) {
        throw 'Startup registration remains after uninstall'
    }

    $roamingPreserved = Test-Path -LiteralPath $roamingMarker
    $localPreserved = Test-Path -LiteralPath $localMarker
    if ($DataPolicy -eq 'preserve' -and (-not $roamingPreserved -or -not $localPreserved)) {
        throw 'Default uninstall did not preserve both data markers'
    }
    if ($DataPolicy -eq 'purge' -and ($roamingPreserved -or $localPreserved)) {
        throw 'Explicit purge did not remove both data markers'
    }

    $evidence = [ordered]@{
        schema_version = 'agentmeter.windows-install-lifecycle/v1'
        status = 'pass'
        os = [Environment]::OSVersion.VersionString
        architecture = $env:PROCESSOR_ARCHITECTURE
        installer_path = $resolvedInstaller
        installer_sha256 = $actualHash
        expected_version = $ExpectedVersion
        upgrade_requested = $upgradeRequested
        upgrade_installer_path = $resolvedUpgradeInstaller
        upgrade_installer_sha256 = $actualUpgradeHash
        upgrade_expected_version = if ($upgradeRequested) { $UpgradeExpectedVersion } else { $null }
        data_policy = $DataPolicy
        install_scope = 'current_user'
        installer_size_bytes = $installerSizeBytes
        upgrade_installer_size_bytes = $upgradeInstallerSizeBytes
        installed_size_bytes = $installedSizeBytes
        initial_install_elapsed_ms = $installTimer.Elapsed.TotalMilliseconds
        first_launch_ready_ms = $firstLaunchReadyMs
        restart_ready_ms = $restartReadyMs
        idle_sample_seconds = $IdleSampleSeconds
        idle_working_set_bytes = $idleWorkingSetBytes
        idle_private_memory_bytes = $idlePrivateMemoryBytes
        idle_processor_time_ms = $idleProcessorTimeMs
        idle_cpu_percent = $idleCpuPercent
        same_version_reinstall_elapsed_ms = $reinstallTimer.Elapsed.TotalMilliseconds
        upgrade_elapsed_ms = $upgradeElapsedMs
        upgrade_ready_ms = $upgradeReadyMs
        webview2_before = $webViewBefore
        webview2_after = Get-WebView2Version
        started_at_utc = $startedAt.ToString('O')
        finished_at_utc = [DateTimeOffset]::UtcNow.ToString('O')
        installed_launch_and_explicit_exit = $true
        same_version_reinstall_and_restart = $true
        supported_upgrade = if ($upgradeRequested) { $true } else { $null }
        startup_enable_disable = $true
        install_directory_removed = $true
        uninstall_registration_removed = $true
        roaming_data_marker_preserved = $roamingPreserved
        local_data_marker_preserved = $localPreserved
        data_preserved = ($roamingPreserved -and $localPreserved)
        data_purged = (-not $roamingPreserved -and -not $localPreserved)
    }
    $evidence | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $resolvedEvidence -Encoding UTF8
    $evidence | ConvertTo-Json -Depth 4
} catch {
    $failureEvidence = [ordered]@{
        schema_version = 'agentmeter.windows-install-lifecycle/v1'
        status = 'fail'
        os = [Environment]::OSVersion.VersionString
        architecture = $env:PROCESSOR_ARCHITECTURE
        installer_path = $resolvedInstaller
        installer_sha256 = $actualHash
        expected_version = $ExpectedVersion
        upgrade_requested = $upgradeRequested
        upgrade_installer_path = $resolvedUpgradeInstaller
        upgrade_installer_sha256 = $actualUpgradeHash
        upgrade_expected_version = if ($upgradeRequested) { $UpgradeExpectedVersion } else { $null }
        data_policy = $DataPolicy
        install_scope = 'current_user'
        idle_sample_seconds = $IdleSampleSeconds
        webview2_before = $webViewBefore
        started_at_utc = $startedAt.ToString('O')
        finished_at_utc = [DateTimeOffset]::UtcNow.ToString('O')
        failure = $_.Exception.Message
    }
    $failureEvidence | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath $resolvedEvidence -Encoding UTF8
    throw
} finally {
    foreach ($ownedProcess in @($primary, $restarted, $upgraded)) {
        if ($ownedProcess -and -not $ownedProcess.HasExited) {
            try { Invoke-BoundedProcess $installedExecutable @('--request-exit') 15 } catch {}
            if (-not $ownedProcess.WaitForExit(5000)) {
                Stop-Process -Id $ownedProcess.Id -Force -ErrorAction SilentlyContinue
            }
        }
    }
    if ($cleanupNeeded -and (Test-Path -LiteralPath $uninstaller -PathType Leaf)) {
        try { Invoke-BoundedProcess $uninstaller @('/S', '/PURGE') 120 } catch {}
    }
    foreach ($marker in @($roamingMarker, $localMarker)) {
        if (Test-Path -LiteralPath $marker -PathType Leaf) { Remove-Item -LiteralPath $marker -Force }
    }
    foreach ($directory in @($roamingData, $localData)) {
        if ((Test-Path -LiteralPath $directory -PathType Container) -and -not (Get-ChildItem -LiteralPath $directory -Force)) {
            Remove-Item -LiteralPath $directory -Force
        }
    }
    $systemTemp = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())
    $probeLeaf = Split-Path -Leaf $probeRoot
    if (
        (Test-Path -LiteralPath $probeRoot -PathType Container) -and
        $probeRoot.StartsWith($systemTemp, [StringComparison]::OrdinalIgnoreCase) -and
        $probeLeaf.StartsWith('agentmeter-install-probe-', [StringComparison]::Ordinal)
    ) {
        Remove-Item -LiteralPath $probeRoot -Recurse -Force
    }
}
