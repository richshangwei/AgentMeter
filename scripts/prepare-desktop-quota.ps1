$ErrorActionPreference = 'Stop'
$repo = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$target = Join-Path $repo 'desktop-p0/resources/quota-helper'
$node = (Get-Command node.exe -ErrorAction Stop).Source
$agy = Join-Path $repo '.scratch/quota-smoke-runtime/agy.exe'
if (-not (Test-Path -LiteralPath $agy -PathType Leaf)) { throw 'Run node scripts/prepare-quota-smoke.mjs first.' }
$expected = 'DF10BECFBC71EF23786C2CE9A70C5DBD8008D707019D878A69C4F0E5B58D95AA9B1AC6AD0EF5763205D7EAF1A85050D5429E601B662A9FC2D9022CDFC35E98B9'
if ((Get-FileHash -LiteralPath $agy -Algorithm SHA512).Hash -ne $expected) { throw 'Antigravity checksum mismatch.' }
$support = Join-Path $PSScriptRoot 'quota-smoke-support'
$npm = (Get-Command npm.cmd -ErrorAction Stop).Source
& $npm ci --ignore-scripts --prefix $support
if ($LASTEXITCODE -ne 0) { throw "Pinned quota-helper dependency restore failed with exit code $LASTEXITCODE." }
if (-not (Test-Path (Join-Path $support 'node_modules/node-pty'))) { throw 'Pinned node-pty dependency is missing after npm ci.' }
New-Item -ItemType Directory -Path $target -Force | Out-Null
Copy-Item -LiteralPath $node -Destination (Join-Path $target 'node.exe') -Force
Copy-Item -LiteralPath $agy -Destination (Join-Path $target 'agy.exe') -Force
foreach ($name in @('quota-smoke.mjs','quota-desktop.mjs')) {
    Copy-Item -LiteralPath (Join-Path $PSScriptRoot $name) -Destination (Join-Path $target $name) -Force
}
$ptyAgent = Join-Path $support 'node_modules/node-pty/lib/windowsPtyAgent.js'
$ptySource = [IO.File]::ReadAllText($ptyAgent)
$forkWithoutHide = "child_process_1.fork(path.join(__dirname, 'conpty_console_list_agent'), [_this._innerPid.toString()])"
$forkHidden = "child_process_1.fork(path.join(__dirname, 'conpty_console_list_agent'), [_this._innerPid.toString()], { windowsHide: true })"
if (-not $ptySource.Contains($forkWithoutHide) -and -not $ptySource.Contains($forkHidden)) {
    throw 'Pinned node-pty cleanup seam changed; refusing to stage an unverified runtime.'
}
[IO.File]::WriteAllText($ptyAgent, $ptySource.Replace($forkWithoutHide, $forkHidden), [Text.UTF8Encoding]::new($false))
$supportTarget = [IO.Path]::GetFullPath((Join-Path $target 'quota-smoke-support'))
$targetPrefix = [IO.Path]::GetFullPath($target) + [IO.Path]::DirectorySeparatorChar
if (-not $supportTarget.StartsWith($targetPrefix, [StringComparison]::OrdinalIgnoreCase)) {
    throw 'Refusing to replace quota-helper dependencies outside the staging directory.'
}
if (Test-Path -LiteralPath $supportTarget) {
    Remove-Item -LiteralPath $supportTarget -Recurse -Force
}
Copy-Item -LiteralPath $support -Destination $target -Recurse -Force
Write-Host 'Desktop quota runtime staged. No global install or PATH changes.'
