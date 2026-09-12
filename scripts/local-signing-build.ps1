param([string]$Version = '0.2.5')
$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Security
$keyDirectory = Join-Path ([Environment]::GetFolderPath([Environment+SpecialFolder]::UserProfile)) '.agentmeter-signing'
$encryptedKey = Join-Path $keyDirectory 'updater.key.dpapi'
$publicKey = Join-Path $keyDirectory 'updater.key.pub'
if (!(Test-Path -LiteralPath $keyDirectory)) { New-Item -ItemType Directory -Path $keyDirectory | Out-Null }
if (!(Test-Path -LiteralPath $encryptedKey)) {
    if (Test-Path -LiteralPath $publicKey) { throw 'Existing public key without protected private key; refusing replacement.' }
    # Capture keys in memory; no plaintext private-key file is created.
    $start = New-Object Diagnostics.ProcessStartInfo
    $start.FileName = (Get-Command cargo).Source
    $start.Arguments = 'tauri signer generate --ci'
    $start.UseShellExecute = $false
    $start.CreateNoWindow = $true
    $start.RedirectStandardOutput = $true
    $start.RedirectStandardError = $true
    $process = [Diagnostics.Process]::Start($start)
    $stdout = $process.StandardOutput.ReadToEndAsync()
    $stderr = $process.StandardError.ReadToEndAsync()
    $process.WaitForExit()
    if ($process.ExitCode -ne 0) { throw 'Key generation failed; output withheld.' }
    $captured = $stdout.Result
    $match = [regex]::Match($captured, 'Private: \(Keep it secret!\)\s+([A-Za-z0-9+/=]+)\s+Public:\s+([A-Za-z0-9+/=]+)')
    if (!$match.Success) { throw 'Unexpected signer output; output withheld.' }
    $keyBytes = [Text.Encoding]::UTF8.GetBytes($match.Groups[1].Value)
    $protected = [Security.Cryptography.ProtectedData]::Protect($keyBytes, $null, [Security.Cryptography.DataProtectionScope]::CurrentUser)
    $roundtrip = [Security.Cryptography.ProtectedData]::Unprotect($protected, $null, [Security.Cryptography.DataProtectionScope]::CurrentUser)
    if ([Convert]::ToBase64String($keyBytes) -ne [Convert]::ToBase64String($roundtrip)) { throw 'Key protection verification failed.' }
    $stream = [IO.File]::Open($encryptedKey, [IO.FileMode]::CreateNew)
    try { $stream.Write($protected, 0, $protected.Length) } finally { $stream.Dispose() }
    [IO.File]::WriteAllText($publicKey, $match.Groups[2].Value, (New-Object Text.UTF8Encoding($false)))
    [Array]::Clear($keyBytes, 0, $keyBytes.Length)
    [Array]::Clear($roundtrip, 0, $roundtrip.Length)
    $captured = $null
    $match = $null
    $stdout = $null
    $stderr = $null
    $process.Dispose()
}
try {
    $clearBytes = [Security.Cryptography.ProtectedData]::Unprotect([IO.File]::ReadAllBytes($encryptedKey), $null, [Security.Cryptography.DataProtectionScope]::CurrentUser)
    $env:TAURI_SIGNING_PRIVATE_KEY = [Text.Encoding]::UTF8.GetString($clearBytes).Trim()
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = ''
    $env:AGENTMETER_UPDATE_PUBLIC_KEY = [IO.File]::ReadAllText($publicKey).Trim()
    $env:AGENTMETER_UPDATE_ENDPOINT = 'https://github.com/richshangwei/AgentMeter/releases/latest/download/latest.json'
    & (Join-Path $PSScriptRoot 'build-signed-update.ps1') -Version $Version
} finally {
    $env:TAURI_SIGNING_PRIVATE_KEY = $null
    $env:TAURI_SIGNING_PRIVATE_KEY_PASSWORD = $null
    if ($clearBytes) { [Array]::Clear($clearBytes, 0, $clearBytes.Length) }
}
