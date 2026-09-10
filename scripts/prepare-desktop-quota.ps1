$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$target = Join-Path $repo 'desktop-p0/resources/quota-helper'
$node = (Get-Command node.exe -ErrorAction Stop).Source
$agy = Join-Path $repo '.scratch/quota-smoke-runtime/agy.exe'
if (-not (Test-Path -LiteralPath $agy -PathType Leaf)) { throw 'Run node scripts/prepare-quota-smoke.mjs first.' }
$expected = 'DF10BECFBC71EF23786C2CE9A70C5DBD8008D707019D878A69C4F0E5B58D95AA9B1AC6AD0EF5763205D7EAF1A85050D5429E601B662A9FC2D9022CDFC35E98B9'
if ((Get-FileHash -LiteralPath $agy -Algorithm SHA512).Hash -ne $expected) { throw 'Antigravity checksum mismatch.' }
$support = Join-Path $PSScriptRoot 'quota-smoke-support'
if (-not (Test-Path (Join-Path $support 'node_modules/node-pty'))) { throw 'Run npm ci --ignore-scripts in scripts/quota-smoke-support first.' }
New-Item -ItemType Directory -Path $target -Force | Out-Null
Copy-Item -LiteralPath $node -Destination (Join-Path $target 'node.exe') -Force
Copy-Item -LiteralPath $agy -Destination (Join-Path $target 'agy.exe') -Force
foreach ($name in @('quota-smoke.mjs','quota-desktop.mjs')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $target $name) -Force
}
$supportTarget = Join-Path $target 'quota-smoke-support'
New-Item -ItemType Directory -Path $supportTarget -Force | Out-Null
foreach ($name in @('package.json','package-lock.json','node_modules')) {
    Copy-Item -LiteralPath (Join-Path $support $name) -Destination $supportTarget -Recurse -Force
}
Write-Host 'Desktop quota runtime staged. No global install or PATH changes.'
