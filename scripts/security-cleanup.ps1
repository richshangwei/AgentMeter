<#
.SYNOPSIS
    One-time public-release hygiene pass for the AgentMeter repository.

.DESCRIPTION
    Removes developer scratch material from Git tracking, pins the commit
    identity to a GitHub noreply address so the maintainer's personal email
    stops appearing in new commits, and re-scans every file that would be
    committed for local paths, usernames and personal usage data.

    History is NOT rewritten. Files already published in earlier commits stay
    reachable through those commits; this pass makes every future commit clean.

    The script stages and commits locally. It never pushes. Review the commit,
    then push manually.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/security-cleanup.ps1
.EXAMPLE
    powershell -ExecutionPolicy Bypass -File scripts/security-cleanup.ps1 -ScanOnly
#>
param(
    [switch]$ScanOnly,
    [string]$NoReplyEmail = 'richshangwei@users.noreply.github.com'
)

$ErrorActionPreference = 'Stop'
$repo = Split-Path -Parent $PSScriptRoot
Push-Location $repo
try {
    Get-Command git -ErrorAction Stop | Out-Null
    if (-not (Test-Path -LiteralPath (Join-Path $repo '.git'))) { throw "Not a Git repository: $repo" }

    # --- 1. Commit identity ------------------------------------------------
    $currentEmail = (git config user.email) 2>$null
    Write-Host "Current commit email: $currentEmail"
    if (-not $ScanOnly) {
        git config user.email $NoReplyEmail
        Write-Host "Commit email for this repository set to: $NoReplyEmail" -ForegroundColor Green
        Write-Host "Also enable GitHub -> Settings -> Emails -> 'Keep my email address private'" -ForegroundColor Yellow
        Write-Host "and 'Block command line pushes that expose my email'." -ForegroundColor Yellow
    }

    # --- 2. Untrack scratch material --------------------------------------
    $untrack = @('.scratch', 'Claude outputs', '.release-worktrees')
    foreach ($path in $untrack) {
        $tracked = git ls-files -- "$path" 2>$null
        if ($tracked) {
            $count = ($tracked | Measure-Object).Count
            Write-Host "Tracked under '$path': $count file(s)"
            if (-not $ScanOnly) {
                git rm -r --cached --quiet -- "$path"
                Write-Host "  untracked (files remain on disk)" -ForegroundColor Green
            }
        } else {
            Write-Host "Tracked under '$path': none"
        }
    }

    # --- 3. Scan what would be committed ----------------------------------
    # Anything matching these is a local-machine or personal-data leak.
    $patterns = @(
        @{ Name = 'Windows user profile path'; Regex = 'C:\\Users\\(?!<|%)[A-Za-z0-9_.-]+' },
        @{ Name = 'Windows user profile path (fwd slash)'; Regex = 'C:/Users/(?!<|%)[A-Za-z0-9_.-]+' },
        @{ Name = 'Local workspace root'; Regex = 'D:[\\/]WorkSpace' },
        @{ Name = 'Codex local capture path'; Regex = 'codex-clipboard-[0-9a-fA-F-]{8}|\\generated_images\\' },
        # Key MATERIAL only. Filenames such as updater.key.dpapi are documentation, not secrets.
        @{ Name = 'Private key block'; Regex = '-----BEGIN [A-Z ]*PRIVATE KEY-----' },
        @{ Name = 'Hardcoded signing key'; Regex = 'TAURI_SIGNING_PRIVATE_KEY\s*=\s*[''"]?[A-Za-z0-9+/=]{24,}' },
        @{ Name = 'Credential-shaped token'; Regex = 'gh[pousr]_[A-Za-z0-9]{20,}|github_pat_[A-Za-z0-9_]{30,}|sk-[A-Za-z0-9_-]{20,}|AIza[0-9A-Za-z_-]{30,}|AKIA[0-9A-Z]{16}|xox[baprs]-[A-Za-z0-9-]{10,}' },
        @{ Name = 'Personal email'; Regex = '[A-Za-z0-9._%+-]+@(gmail|outlook|hotmail|yahoo|qq|icloud)\.' }
    )
    $binary = '\.(png|jpg|jpeg|gif|ico|rgba|exe|dll|pdb|zip|7z|woff2?)$'

    # The scanner necessarily contains the patterns it looks for.
    $selfPath = 'scripts/security-cleanup.ps1'
    $files = git ls-files | Where-Object { $_ -notmatch $binary -and $_ -ne $selfPath }
    Write-Host ""
    Write-Host "Scanning $($files.Count) tracked text file(s)..." -ForegroundColor Cyan

    $findings = @()
    foreach ($file in $files) {
        if (-not (Test-Path -LiteralPath $file)) { continue }
        $content = Get-Content -LiteralPath $file -Raw -ErrorAction SilentlyContinue
        if (-not $content) { continue }
        foreach ($p in $patterns) {
            $m = [regex]::Matches($content, $p.Regex)
            if ($m.Count -gt 0) {
                $findings += [pscustomobject]@{
                    File    = $file
                    Issue   = $p.Name
                    Sample  = ($m[0].Value -replace '\s+', ' ')
                    Count   = $m.Count
                }
            }
        }
    }

    if ($findings.Count -eq 0) {
        Write-Host "Clean: no local paths, usernames, key material or credential-shaped strings." -ForegroundColor Green
    } else {
        Write-Host "$($findings.Count) issue(s) found in tracked files:" -ForegroundColor Red
        $findings | Format-Table -AutoSize File, Issue, Count, Sample
        if (-not $ScanOnly) { throw 'Scan found leaks. Fix them before committing.' }
    }

    # --- 4. Commit ---------------------------------------------------------
    if ($ScanOnly) { Write-Host "`nScan-only mode. Nothing changed." -ForegroundColor Cyan; return }

    git add -A -- .gitignore LICENSE README.md docs tasks design-qa.md scripts
    $staged = git diff --cached --name-only
    if (-not $staged) { Write-Host "`nNothing to commit." -ForegroundColor Yellow; return }

    Write-Host "`nStaged changes:" -ForegroundColor Cyan
    $staged | ForEach-Object { Write-Host "  $_" }

    git commit -m @"
chore(security): scrub local paths and personal usage data from public repo

Untrack developer scratch material (.scratch, Claude outputs) and ignore
release worktrees, so build trees, signed installers and screenshots can no
longer be committed.

Replace Windows user-profile paths and the local workspace root with
placeholders in retained documentation. Redact personal account usage values,
reset times and process IDs from docs/evidence while preserving each record's
schema, provider, status and reason.

Add an MIT LICENSE and document the redaction, signing-key and third-party
bundling boundaries in README.

History is unchanged; earlier commits still contain the original values.
"@

    Write-Host "`nCommitted locally. Nothing was pushed." -ForegroundColor Green
    Write-Host "Review with:  git show --stat HEAD"
    Write-Host "Then push with:  git push origin master"
}
finally { Pop-Location }
