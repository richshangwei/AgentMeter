param(
    [Parameter(Mandatory = $true)]
    [string]$ExecutablePath,
    [ValidateRange(1, 20)]
    [int]$RepeatCount = 5,
    [string]$EvidencePath,
    [switch]$AllowProcessLaunch
)

$ErrorActionPreference = 'Stop'
if (-not $AllowProcessLaunch) {
    throw 'Refusing to launch the desktop process without -AllowProcessLaunch'
}

$resolvedExecutable = [IO.Path]::GetFullPath($ExecutablePath)
if (-not (Test-Path -LiteralPath $resolvedExecutable -PathType Leaf)) {
    throw "Executable does not exist: $resolvedExecutable"
}
if ([IO.Path]::GetFileName($resolvedExecutable) -ne 'agentmeter-desktop-p0.exe') {
    throw 'Executable filename must be agentmeter-desktop-p0.exe'
}

function Get-AgentMeterProcess {
    @(Get-Process -Name 'agentmeter-desktop-p0' -ErrorAction SilentlyContinue | Where-Object {
        try { $_.Path -eq $resolvedExecutable } catch { $false }
    })
}

function Wait-AgentMeterReady {
    for ($attempt = 0; $attempt -lt 20; $attempt++) {
        Start-Sleep -Milliseconds 250
        $probe = Start-Process -FilePath $resolvedExecutable -ArgumentList '--probe-ready' -WindowStyle Hidden -PassThru
        if (-not $probe.WaitForExit(8000)) {
            throw 'Readiness probe remained resident'
        }
        if ($probe.ExitCode -eq 0) {
            return $true
        }
        if ($probe.ExitCode -ne 3) {
            throw "Unexpected readiness exit code: $($probe.ExitCode)"
        }
    }
    $false
}

if ((Get-AgentMeterProcess).Count -ne 0) {
    throw 'Refusing to start: a matching AgentMeter process already exists'
}

$primary = $null
$abnormalPrimary = $null
$restartedPrimary = $null
$forcedCleanup = $false
$result = $null
try {
    $primary = Start-Process -FilePath $resolvedExecutable -ArgumentList '--hidden' -WindowStyle Hidden -PassThru
    $ready = Wait-AgentMeterReady
    if (-not $ready) {
        throw 'Primary did not become ready'
    }

    $launches = @()
    for ($iteration = 1; $iteration -le $RepeatCount; $iteration++) {
        $secondary = Start-Process -FilePath $resolvedExecutable -ArgumentList '--hidden' -WindowStyle Hidden -PassThru
        if (-not $secondary.WaitForExit(8000)) {
            throw "Secondary launch $iteration remained resident"
        }
        $resident = @(Get-AgentMeterProcess)
        $launches += [pscustomobject]@{
            iteration = $iteration
            secondary_exit = $secondary.ExitCode
            resident_count = $resident.Count
            resident_pid = if ($resident.Count -eq 1) { $resident[0].Id } else { $null }
        }
        if ($secondary.ExitCode -ne 0 -or $resident.Count -ne 1 -or $resident[0].Id -ne $primary.Id) {
            throw "Single-instance invariant failed at launch $iteration"
        }
    }

    $exitRequest = Start-Process -FilePath $resolvedExecutable -ArgumentList '--request-exit' -WindowStyle Hidden -PassThru
    if (-not $exitRequest.WaitForExit(8000)) {
        throw 'Full-exit request remained resident'
    }
    if ($exitRequest.ExitCode -ne 0) {
        throw "Full-exit request failed with code $($exitRequest.ExitCode)"
    }
    if (-not $primary.WaitForExit(8000)) {
        throw 'Primary did not complete full exit'
    }
    $residentAfterExit = (Get-AgentMeterProcess).Count
    if ($residentAfterExit -ne 0) {
        throw "Expected zero resident processes after full exit, found $residentAfterExit"
    }

    $abnormalPrimary = Start-Process -FilePath $resolvedExecutable -ArgumentList '--hidden' -WindowStyle Hidden -PassThru
    if (-not (Wait-AgentMeterReady)) {
        throw 'Abnormal-termination primary did not become ready'
    }
    Stop-Process -Id $abnormalPrimary.Id -Force
    if (-not $abnormalPrimary.WaitForExit(8000)) {
        throw 'Exact abnormal-termination PID remained resident'
    }
    $residentAfterAbnormalTermination = (Get-AgentMeterProcess).Count
    if ($residentAfterAbnormalTermination -ne 0) {
        throw "Expected zero residents after abnormal termination, found $residentAfterAbnormalTermination"
    }

    $restartedPrimary = Start-Process -FilePath $resolvedExecutable -ArgumentList '--hidden' -WindowStyle Hidden -PassThru
    if (-not (Wait-AgentMeterReady)) {
        throw 'Post-abnormal-restart primary did not become ready'
    }
    $postAbnormalSecondary = Start-Process -FilePath $resolvedExecutable -ArgumentList '--hidden' -WindowStyle Hidden -PassThru
    if (-not $postAbnormalSecondary.WaitForExit(8000)) {
        throw 'Post-abnormal secondary launch remained resident'
    }
    $postAbnormalResident = @(Get-AgentMeterProcess)
    if ($postAbnormalSecondary.ExitCode -ne 0 -or $postAbnormalResident.Count -ne 1 -or $postAbnormalResident[0].Id -ne $restartedPrimary.Id) {
        throw 'Post-abnormal single-instance invariant failed'
    }
    $postAbnormalExitRequest = Start-Process -FilePath $resolvedExecutable -ArgumentList '--request-exit' -WindowStyle Hidden -PassThru
    if (-not $postAbnormalExitRequest.WaitForExit(8000)) {
        throw 'Post-abnormal full-exit request remained resident'
    }
    if ($postAbnormalExitRequest.ExitCode -ne 0 -or -not $restartedPrimary.WaitForExit(8000)) {
        throw 'Post-abnormal primary did not complete full exit'
    }
    $postAbnormalResidentAfterExit = (Get-AgentMeterProcess).Count
    if ($postAbnormalResidentAfterExit -ne 0) {
        throw "Expected zero post-abnormal residents after full exit, found $postAbnormalResidentAfterExit"
    }

    $file = Get-Item -LiteralPath $resolvedExecutable
    $result = [ordered]@{
        schema_version = 'agentmeter.desktop-process-lifecycle/v1'
        outcome = 'success'
        executable = $resolvedExecutable
        executable_sha256 = (Get-FileHash -LiteralPath $resolvedExecutable -Algorithm SHA256).Hash
        product_version = $file.VersionInfo.ProductVersion
        os_version = [Environment]::OSVersion.VersionString
        primary_pid = $primary.Id
        readiness = 'accepted'
        repeated_launches = $launches
        full_exit_request_code = $exitRequest.ExitCode
        resident_after_exit = $residentAfterExit
        abnormal_termination = [ordered]@{
            terminated_pid = $abnormalPrimary.Id
            resident_after_termination = $residentAfterAbnormalTermination
        }
        post_abnormal_relaunch = [ordered]@{
            primary_pid = $restartedPrimary.Id
            readiness = 'accepted'
            secondary_exit = $postAbnormalSecondary.ExitCode
            resident_count = $postAbnormalResident.Count
            resident_pid = $postAbnormalResident[0].Id
            full_exit_request_code = $postAbnormalExitRequest.ExitCode
            resident_after_exit = $postAbnormalResidentAfterExit
        }
        same_interactive_desktop_only = $true
        cross_desktop_policy = 'reject_without_starting_a_second_instance'
    }
} finally {
    foreach ($ownedPrimary in @($primary, $abnormalPrimary, $restartedPrimary)) {
        if ($null -ne $ownedPrimary -and -not $ownedPrimary.HasExited) {
            $forcedCleanup = $true
            Stop-Process -Id $ownedPrimary.Id -Force -ErrorAction SilentlyContinue
            $ownedPrimary.WaitForExit(3000) | Out-Null
        }
    }
}

if ($forcedCleanup) {
    throw 'The primary required forced cleanup; no success evidence was written'
}

$json = $result | ConvertTo-Json -Depth 6
if ($EvidencePath) {
    $resolvedEvidence = [IO.Path]::GetFullPath($EvidencePath)
    $parent = Split-Path -Parent $resolvedEvidence
    if (-not (Test-Path -LiteralPath $parent -PathType Container)) {
        throw "Evidence directory does not exist: $parent"
    }
    [IO.File]::WriteAllText($resolvedEvidence, $json, [Text.UTF8Encoding]::new($false))
}
$json
