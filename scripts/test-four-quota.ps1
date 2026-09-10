$ErrorActionPreference = 'Stop'
$node = Get-Command node.exe -ErrorAction SilentlyContinue
if (-not $node) {
    Write-Error 'Node.js is required to run the minimal test.'
    exit 2
}
& $node.Source (Join-Path $PSScriptRoot 'quota-smoke.mjs')
exit $LASTEXITCODE
