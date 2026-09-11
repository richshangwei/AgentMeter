$ErrorActionPreference = 'Stop'
$projectRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
Push-Location $projectRoot
try {
    foreach ($powerShellRelativePath in @(
        'scripts\inspect-nsis.ps1',
        'scripts\test-nsis-lifecycle.ps1',
        'scripts\test-desktop-process-lifecycle.ps1',
        'scripts\test-copilot-live.ps1',
        'scripts\test-four-quota.ps1',
        'scripts\test-refresh-console.ps1',
        'scripts\prepare-desktop-quota.ps1',
        'scripts\build-signed-update.ps1',
        'scripts\local-signing-build.ps1',
        'desktop-p0\tests\instance-probe.ps1'
    )) {
        Write-Host "Checking: PowerShell syntax ($powerShellRelativePath)"
        $powerShellTokens = $null
        $powerShellErrors = $null
        $powerShellScript = Join-Path $projectRoot $powerShellRelativePath
        [System.Management.Automation.Language.Parser]::ParseFile(
            $powerShellScript,
            [ref]$powerShellTokens,
            [ref]$powerShellErrors
        ) | Out-Null
        if ($powerShellErrors.Count -ne 0) {
            $messages = ($powerShellErrors | ForEach-Object { $_.Message }) -join '; '
            throw "PowerShell syntax failed ($powerShellRelativePath): $messages"
        }
    }
$checks = @(
        @{Name='Quota parsers'; Tool='node'; Args=@('--test','tests/quota_smoke.test.mjs')},
        @{Name='Quota desktop entry syntax'; Tool='node'; Args=@('--check','scripts/quota-desktop.mjs')},
        @{Name='Automatic quota UI'; Tool='node'; Args=@('--test','desktop-p0/tests/auto-quota-ui.test.cjs')},
        @{Name='Desktop layout rules'; Tool='node'; Args=@('--test','tests/desktop_layout.test.cjs')},
        @{Name='Responsive browser verifier syntax'; Tool='node'; Args=@('--check','scripts/verify-desktop-responsive.cjs')},
        @{Name='Packaged updater startup config'; Tool='node'; Args=@('--test','tests/updater_boot_config.test.cjs')},
        @{Name='Application updater controller'; Tool='node'; Args=@('--test','tests/app_updater.test.cjs')},
        @{Name='Signed update manifest'; Tool='node'; Args=@('--test','scripts/create-update-manifest.test.mjs')},
        @{Name='Application updater UI syntax'; Tool='node'; Args=@('--check','desktop-p0/ui/app-updater-ui.js')},
        @{Name='Root formatting'; Tool='cargo'; Args=@('fmt','--all','--','--check')},
        @{Name='Desktop formatting'; Tool='cargo'; Args=@('fmt','--manifest-path','desktop-p0/Cargo.toml','--all','--','--check')},
        @{Name='Root tests'; Tool='cargo'; Args=@('test','--offline','--locked','--quiet')},
        @{Name='Desktop tests'; Tool='cargo'; Args=@('test','--manifest-path','desktop-p0/Cargo.toml','--offline','--locked','--quiet')},
        @{Name='Browser logic tests'; Tool='node'; Args=@('--test','tests/tablet_protocol.test.cjs','tests/tablet_recovery.test.cjs','tests/tablet_view.test.cjs','tests/background_window_contract.test.cjs','tests/updater_contract.test.cjs','desktop-p0/tests/source-watch.test.cjs','desktop-p0/tests/setup.test.cjs')},
        @{Name='Tablet client syntax'; Tool='node'; Args=@('--check','tablet-ui/client.js')},
        @{Name='Desktop client syntax'; Tool='node'; Args=@('--check','desktop-p0/ui/dashboard.js')},
        @{Name='Setup client syntax'; Tool='node'; Args=@('--check','desktop-p0/ui/setup.js')},
        @{Name='Root Clippy'; Tool='cargo'; Args=@('clippy','--offline','--locked','--all-targets','--','-D','warnings')},
        @{Name='Desktop Clippy'; Tool='cargo'; Args=@('clippy','--manifest-path','desktop-p0/Cargo.toml','--offline','--locked','--all-targets','--','-D','warnings')}
    )
    foreach ($check in $checks) {
        Write-Host "Checking: $($check.Name)"
        $checkArgs = $check.Args
        & $check.Tool @checkArgs
        if ($LASTEXITCODE -ne 0) { throw "$($check.Name) failed with exit code $LASTEXITCODE" }
    }
    Write-Host 'Local checks passed. This does not certify authenticated Providers, physical tablets or clean-VM installation.'
} finally {
    Pop-Location
}
