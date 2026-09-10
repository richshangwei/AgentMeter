param(
    [Parameter(Mandatory = $true)]
    [string]$CollectorPath,
    [Parameter(Mandatory = $true)]
    [string]$GitHubCliPath,
    [Parameter(Mandatory = $true)]
    [string]$OutputPath,
    [string]$PersonalAccount = '',
    [string]$BusinessAccount = '',
    [string]$EnterpriseAccount = '',
    [Parameter(Mandatory = $true)]
    [ValidateRange(2000, 9999)]
    [int]$Year,
    [Parameter(Mandatory = $true)]
    [ValidateRange(1, 12)]
    [int]$Month,
    [ValidateRange(1, 60000)]
    [int]$TimeoutMs = 10000,
    [switch]$AllowProviderRequests
)

$ErrorActionPreference = 'Stop'
function Test-FullyQualifiedWindowsPath([string]$Value) {
    return $Value -match '^(?:[A-Za-z]:[\\/]|\\\\)'
}
function Get-Sha256([string]$Path) {
    $algorithm = [Security.Cryptography.SHA256]::Create()
    $stream = [IO.File]::OpenRead($Path)
    try {
        return ([BitConverter]::ToString($algorithm.ComputeHash($stream))).Replace('-', '')
    } finally {
        $stream.Dispose()
        $algorithm.Dispose()
    }
}
if (-not $AllowProviderRequests) {
    throw 'Refusing to contact GitHub without -AllowProviderRequests'
}

$collector = [IO.Path]::GetFullPath($CollectorPath)
$gh = [IO.Path]::GetFullPath($GitHubCliPath)
$output = [IO.Path]::GetFullPath($OutputPath)
if (-not (Test-FullyQualifiedWindowsPath $CollectorPath) -or -not (Test-Path -LiteralPath $collector -PathType Leaf)) {
    throw 'CollectorPath must identify an existing absolute file'
}
if (-not (Test-FullyQualifiedWindowsPath $GitHubCliPath) -or -not (Test-Path -LiteralPath $gh -PathType Leaf)) {
    throw 'GitHubCliPath must identify an existing absolute file'
}
if (-not (Test-FullyQualifiedWindowsPath $OutputPath)) {
    throw 'OutputPath must be absolute'
}
if (Test-Path -LiteralPath $output) {
    throw 'Refusing to overwrite an existing evidence file'
}
$outputParent = Split-Path -Parent $output
if (-not (Test-Path -LiteralPath $outputParent -PathType Container)) {
    throw 'OutputPath parent directory must already exist'
}

$contexts = @(
    [pscustomobject]@{ context = 'personal'; account = $PersonalAccount.Trim() },
    [pscustomobject]@{ context = 'business'; account = $BusinessAccount.Trim() },
    [pscustomobject]@{ context = 'enterprise'; account = $EnterpriseAccount.Trim() }
)
if (@($contexts | Where-Object { $_.account }).Count -eq 0) {
    throw 'Provide at least one PersonalAccount, BusinessAccount, or EnterpriseAccount'
}

$results = [Collections.Generic.List[object]]::new()
$untested = [Collections.Generic.List[object]]::new()
$requestFailures = 0
foreach ($entry in $contexts) {
    if (-not $entry.account) {
        $untested.Add([pscustomobject]@{
            context = $entry.context
            reason = 'account_slug_not_provided'
        })
        continue
    }
    foreach ($meter in @('ai-credits', 'premium-requests')) {
        $arguments = @(
            'collect',
            '--gh-bin', $gh,
            '--context', $entry.context,
            '--account', $entry.account,
            '--meter', $meter,
            '--year', $Year.ToString([Globalization.CultureInfo]::InvariantCulture),
            '--month', $Month.ToString([Globalization.CultureInfo]::InvariantCulture),
            '--timeout-ms', $TimeoutMs.ToString([Globalization.CultureInfo]::InvariantCulture)
        )
        $errorPath = Join-Path ([IO.Path]::GetTempPath()) ("agentmeter-copilot-runner-" + [guid]::NewGuid().ToString('N') + '.stderr')
        try {
            $rawLines = & $collector @arguments 2> $errorPath
            $collectorExit = $LASTEXITCODE
            $raw = $rawLines -join "`n"
            try {
                $report = $raw | ConvertFrom-Json
                if ($report.schema_version -ne 'copilot-p0/v1') {
                    throw 'unexpected schema'
                }
            } catch {
                $report = [pscustomobject]@{
                    schema_version = 'copilot-p0/v1'
                    outcome = 'failure'
                    observation = $null
                    failure_code = 'collector_contract_failed'
                    message = 'Collector did not emit its documented JSON contract'
                }
                $collectorExit = 1
            }
        } finally {
            if (Test-Path -LiteralPath $errorPath) {
                Remove-Item -LiteralPath $errorPath -Force
            }
        }
        if ($collectorExit -ne 0) {
            $requestFailures++
        }
        $results.Add([pscustomobject]@{
            context = $entry.context
            account = $entry.account
            meter = $meter
            collector_exit = $collectorExit
            report = $report
        })
    }
}

$outcome = if ($requestFailures -gt 0) {
    'failure'
} elseif ($untested.Count -gt 0) {
    'completed_with_gaps'
} else {
    'success'
}
$evidence = [ordered]@{
    schema_version = 'agentmeter.copilot-live-evidence/v1'
    outcome = $outcome
    tested_at = [DateTimeOffset]::Now.ToString('o')
    api_version = '2026-03-10'
    requested_period = [ordered]@{ year = $Year; month = $Month }
    collector = [ordered]@{
        file_name = [IO.Path]::GetFileName($collector)
        sha256 = Get-Sha256 $collector
    }
    github_cli = [ordered]@{
        file_name = [IO.Path]::GetFileName($gh)
        sha256 = Get-Sha256 $gh
    }
    results = @($results)
    untested_contexts = @($untested)
    limitations = @(
        'Only explicitly supplied account contexts were contacted.',
        'Billing usage does not establish included allowance, remaining quota, or reset.',
        'Provider stderr and credential material are not retained.'
    )
    credential_material_recorded = $false
}
$json = $evidence | ConvertTo-Json -Depth 20
$stream = [IO.File]::Open($output, [IO.FileMode]::CreateNew, [IO.FileAccess]::Write, [IO.FileShare]::None)
try {
    $writer = [IO.StreamWriter]::new($stream, [Text.UTF8Encoding]::new($false))
    try {
        $writer.Write($json)
        $writer.Flush()
    } finally {
        $writer.Dispose()
    }
} finally {
    $stream.Dispose()
}
$json
if ($requestFailures -gt 0) {
    exit 1
}
