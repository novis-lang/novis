<#
.SYNOPSIS
    Unattended MWL work loop. Starts one fresh `claude` session per iteration, so the per-session context
    cost is constant no matter how many sessions run. See AGENT_COORDINATOR_PROMPT.md for the design.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .claude\loop.ps1 -MaxSessions 300
#>
[CmdletBinding()]
param(
    [int]    $MaxSessions    = 1,
    [string] $Model          = 'opus',
    [string] $PermissionMode = 'bypassPermissions',
    [int]    $MaxStalls      = 3,      # consecutive no-commit sessions before giving up
    [int]    $MaxRetries     = 3,      # consecutive CLI failures (rate limit, crash) before giving up
    [int]    $DelaySeconds   = 0       # pause between sessions
)

$ErrorActionPreference = 'Stop'
$repo   = Split-Path -Parent $PSScriptRoot
$logDir = Join-Path $repo '.claude\loop-logs'
$ledger = Join-Path $repo '.claude\loop-log.md'
$status = Join-Path $repo '.claude\loop-status.txt'
$prompt = Join-Path $repo '.claude\SESSION_PROMPT.md'
$goal   = Join-Path $repo '.claude\loop-goal.md'
$stop   = Join-Path $repo '.claude\loop-stop'

foreach ($f in @($prompt, $goal)) {
    if (-not (Test-Path $f)) { throw "missing $f" }
}
if (-not (Test-Path $logDir)) { New-Item -ItemType Directory -Path $logDir | Out-Null }
if (-not (Test-Path $ledger)) { Set-Content -Path $ledger -Value '# Loop ledger' -Encoding utf8 }

Set-Location $repo
$promptText = Get-Content $prompt -Raw

function Write-Ledger([string]$line) {
    Add-Content -Path $ledger -Value $line -Encoding utf8
    Write-Host $line
}

# Renders one NDJSON line from `claude --output-format stream-json` as a readable console line, so the
# run is watchable live. A line that will not parse is printed raw rather than dropped -- nothing the
# session emits should ever be invisible.
function Show-Event([string]$json) {
    if (-not $json -or -not $json.Trim()) { return }
    try { $e = $json | ConvertFrom-Json } catch { Write-Host "   $json" -ForegroundColor DarkGray; return }

    switch ($e.type) {
        'system' {
            if ($e.subtype -eq 'init') {
                Write-Host "   [init] model=$($e.model)" -ForegroundColor DarkGray
            }
        }
        'assistant' {
            foreach ($b in $e.message.content) {
                if ($b.type -eq 'text' -and $b.text -and $b.text.Trim()) {
                    Write-Host ('   ' + $b.text.Trim()) -ForegroundColor Gray
                }
                elseif ($b.type -eq 'tool_use') {
                    $arg = ''
                    if ($b.input) {
                        foreach ($p in @('command', 'file_path', 'pattern', 'path', 'description')) {
                            if ($b.input.PSObject.Properties[$p]) { $arg = [string]$b.input.$p; break }
                        }
                    }
                    $arg = ($arg -replace '\s+', ' ').Trim()
                    if ($arg.Length -gt 100) { $arg = $arg.Substring(0, 100) + '...' }
                    Write-Host "   > $($b.name) $arg" -ForegroundColor Cyan
                }
            }
        }
        'user' {
            foreach ($b in $e.message.content) {
                if ($b.type -eq 'tool_result' -and $b.is_error) {
                    Write-Host '     ! tool error' -ForegroundColor Red
                }
            }
        }
        'result' {
            $cost = ''
            if ($e.PSObject.Properties['total_cost_usd']) {
                $cost = ' $' + ([double]$e.total_cost_usd).ToString('F2', [cultureinfo]::InvariantCulture)
            }
            Write-Host "   [$($e.subtype)] $($e.num_turns) turns$cost" -ForegroundColor Yellow
        }
    }
}

# The goal's own acceptance test. Exit 0 plus exact stdout means the loop is finished, regardless of what
# the session claimed -- no model judgment sits on the stop path.
function Test-Goal {
    if (-not (Test-Path (Join-Path $repo 'examples\hello.mwl'))) { return $false }
    $out = & cargo run --quiet -p mwl-cli -- run examples/hello.mwl
    if ($LASTEXITCODE -ne 0) { return $false }
    return ((($out -join "`n").Trim()) -eq 'Hello, World!')
}

$stalls = 0
$fails  = 0
$reason = "hit MaxSessions ($MaxSessions)"
Write-Ledger ''
Write-Ledger ('## run started ' + (Get-Date -Format 'yyyy-MM-dd HH:mm') + " (max $MaxSessions)")

for ($i = 1; $i -le $MaxSessions; $i++) {

    if (Test-Path $stop) {
        $reason = 'loop-stop file present'
        break
    }

    $headBefore = (& git rev-parse HEAD).Trim()
    $log        = Join-Path $logDir ('{0:D4}.log' -f $i)
    Remove-Item $status -ErrorAction SilentlyContinue
    Write-Host ('== session {0}/{1}  {2}' -f $i, $MaxSessions, (Get-Date -Format 'HH:mm:ss')) -ForegroundColor Cyan

    & claude -p $promptText --model $Model --permission-mode $PermissionMode `
             --output-format stream-json --verbose |
        ForEach-Object {
            Add-Content -Path $log -Value $_ -Encoding utf8
            Show-Event $_
        }
    $cliExit = $LASTEXITCODE

    if ($cliExit -ne 0) {
        $fails++
        Write-Ledger ('- {0:D4} CLI exit {1} (attempt {2}/{3}) -- see {4}' -f $i, $cliExit, $fails, $MaxRetries, $log)
        if ($fails -ge $MaxRetries) { $reason = "claude CLI failed $fails times in a row"; break }
        Start-Sleep -Seconds ([Math]::Min(300, 30 * [Math]::Pow(2, $fails)))
        continue
    }
    $fails = 0

    $line = ''
    if (Test-Path $status) { $line = (Get-Content $status -Raw).Trim() }
    $headAfter = (& git rev-parse HEAD).Trim()
    $commits   = 0
    if ($headAfter -ne $headBefore) {
        $commits = [int](& git rev-list --count "$headBefore..$headAfter")
    }
    $shown = $line
    if (-not $shown) { $shown = '(no status written)' }
    Write-Ledger ('- {0:D4} {1} commit(s) | {2}' -f $i, $commits, $shown)

    # Deterministic goal check first -- it outranks whatever the session reported.
    if (Test-Goal) { $reason = 'GOAL REACHED: hello world runs'; break }

    if ($line -like 'DONE*')    { $reason = "session reported DONE but the acceptance test does not pass yet: $line"; break }
    if ($line -like 'BLOCKED*') { $reason = "blocked on a user decision: $line"; break }

    if ($commits -eq 0) {
        $stalls++
        if ($stalls -ge $MaxStalls) { $reason = "$stalls sessions in a row produced no commit"; break }
    } else {
        $stalls = 0
    }

    if ($DelaySeconds -gt 0) { Start-Sleep -Seconds $DelaySeconds }
}

Write-Ledger ('## run ended ' + (Get-Date -Format 'yyyy-MM-dd HH:mm') + " -- $reason")
Write-Host ''
Write-Host $reason -ForegroundColor Yellow
