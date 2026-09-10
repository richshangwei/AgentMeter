param(
    [Parameter(Mandatory=$true)][int]$PrimaryId,
    [ValidateSet('reuse','isolated','exit','ready')][string]$Expectation = 'reuse'
)
$ErrorActionPreference = 'Stop'
$executable = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../target/debug/agentmeter-desktop-p0.exe'))
$primary = Get-Process -Id $PrimaryId
if ($primary.Path -ne $executable) { throw 'Primary is not the project preview executable' }
$arguments = if ($Expectation -eq 'exit') {
    '--request-exit'
} elseif ($Expectation -eq 'ready') {
    '--probe-ready'
} else {
    '--hidden'
}
$second = Start-Process -FilePath $executable -ArgumentList $arguments -WindowStyle Hidden -PassThru
try {
    if (-not $second.WaitForExit(8000)) { throw "FAIL secondary remained resident: $($second.Id)" }
    if ($Expectation -eq 'isolated') {
        if ($second.ExitCode -eq 0) { throw 'FAIL isolated launch should report unreachable primary' }
    } elseif ($second.ExitCode -ne 0) { throw "FAIL launch exit code: $($second.ExitCode)" }
    if ($Expectation -eq 'exit') {
        if (-not $primary.WaitForExit(5000)) { throw 'FAIL primary did not exit' }
    } elseif ($primary.HasExited) { throw 'FAIL primary unexpectedly exited' }
    Write-Output "PASS $Expectation; secondary exited with $($second.ExitCode)"
} finally {
    if (-not $second.HasExited) { $second.Kill(); $second.WaitForExit() }
}
