<#
.SYNOPSIS
    Unattended MWL work loop. Starts one fresh `claude` session per iteration, so the per-session context
    cost is constant no matter how many sessions run. See AGENT_COORDINATOR_PROMPT.md for the design.

.EXAMPLE
    powershell -ExecutionPolicy Bypass -File .claude\loop.ps1 -MaxSessions 300

.EXAMPLE
    # Same, but echo every tool call's full input and full result -- no truncation anywhere.
    powershell -ExecutionPolicy Bypass -File .claude\loop.ps1 -MaxSessions 300 -FullOutput
#>
[CmdletBinding()]
param(
    [int]    $MaxSessions    = 1,
    [string] $Model          = 'opus',
    [string] $PermissionMode = 'bypassPermissions',
    [int]    $MaxStalls      = 10,      # consecutive no-commit sessions before giving up
    [int]    $MaxRetries     = 3,      # consecutive CLI failures (rate limit, crash) before giving up
    [int]    $DelaySeconds   = 0,      # pause between sessions
    [int]    $MaxResultLines = 60,     # lines of a tool result echoed to the console; 0 = no cap
    [int]    $MaxInputLines  = 40,     # lines of a single tool-call argument echoed;   0 = no cap
    [int]    $MaxLineChars   = 500,    # per-line truncation;                           0 = no cap
    [switch] $FullOutput               # echo every line of everything, no caps at all
)

if ($FullOutput) { $MaxResultLines = 0; $MaxInputLines = 0; $MaxLineChars = 0 }

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

# Renders the NDJSON from `claude --output-format stream-json` the way Claude Code's own transcript reads:
# assistant text, thinking, every tool call with its full input, and the result each call came back with.
# Truncation is per line and per block only, and a shortened block always says how much it hid, so nothing
# is ever silently dropped; -FullOutput removes the caps. The raw NDJSON is in the session log regardless.
$script:ToolNames = @{}

# One text blob -> console lines, indented under $prefix. $maxLines of 0 means print all of it.
function Write-Wrapped([string]$text, [string]$prefix, [System.ConsoleColor]$color, [int]$maxLines) {
    if ($null -eq $text) { return }
    $text = ($text -replace "`t", '    ').TrimEnd()
    if (-not $text) { return }
    $lines  = @($text -split "`r?`n")
    $hidden = 0
    if ($maxLines -gt 0 -and $lines.Count -gt $maxLines) {
        $hidden = $lines.Count - $maxLines
        $lines  = $lines[0..($maxLines - 1)]
    }
    foreach ($l in $lines) {
        $t = $l
        if ($MaxLineChars -gt 0 -and $t.Length -gt $MaxLineChars) {
            $t = $t.Substring(0, $MaxLineChars) + ('  [+{0} chars]' -f ($t.Length - $MaxLineChars))
        }
        Write-Host ($prefix + $t) -ForegroundColor $color
    }
    if ($hidden -gt 0) {
        Write-Host ($prefix + ('... {0} more line(s) -- full text in the session log' -f $hidden)) -ForegroundColor DarkGray
    }
}

# A message `content` field is either a plain string or an array of blocks; flatten either to text.
function Get-ContentText($content) {
    if ($null -eq $content) { return '' }
    if ($content -is [string]) { return $content }
    $parts = @()
    foreach ($c in @($content)) {
        if ($c -is [string])                                  { $parts += $c;        continue }
        if ($c.PSObject.Properties['text'] -and $c.text)      { $parts += [string]$c.text; continue }
        if ($c.PSObject.Properties['type'] -and $c.type -eq 'image') { $parts += '[image]'; continue }
        $parts += ($c | ConvertTo-Json -Depth 8 -Compress)
    }
    return ($parts -join "`n")
}

# Every argument of a tool call, not just the first one that looked interesting.
function Show-ToolInput($inputObj) {
    if ($null -eq $inputObj) { return }
    foreach ($p in $inputObj.PSObject.Properties) {
        $v = $p.Value
        if ($null -eq $v) { continue }
        if ($v -is [string]) { $s = $v }
        elseif ($v -is [ValueType]) { $s = [string]$v }
        else { $s = ($v | ConvertTo-Json -Depth 8) }
        if (-not $s -or -not $s.Trim()) { continue }
        if ($s -match "`n") {
            Write-Host ('       {0}:' -f $p.Name) -ForegroundColor DarkCyan
            Write-Wrapped $s '       | ' 'DarkCyan' $MaxInputLines
        }
        else {
            Write-Wrapped ('{0}: {1}' -f $p.Name, $s) '       ' 'DarkCyan' 1
        }
    }
}

function Show-Event([string]$json) {
    if (-not $json -or -not $json.Trim()) { return }
    try { $e = $json | ConvertFrom-Json } catch { Write-Host "   $json" -ForegroundColor DarkGray; return }

    switch ($e.type) {
        'system' {
            if ($e.subtype -eq 'init') {
                Write-Host "   [init] model=$($e.model) cwd=$($e.cwd) session=$($e.session_id)" -ForegroundColor DarkGray
                if ($e.PSObject.Properties['tools']) {
                    Write-Wrapped ('tools: ' + (@($e.tools) -join ', ')) '   [init] ' 'DarkGray' 2
                }
            }
            else {
                Write-Wrapped ($e | ConvertTo-Json -Depth 8 -Compress) "   [$($e.subtype)] " 'DarkGray' 4
            }
        }
        'assistant' {
            foreach ($b in $e.message.content) {
                switch ($b.type) {
                    'text'     { Write-Wrapped $b.text     '   '   'Gray'         0 }
                    'thinking' { Write-Wrapped $b.thinking '   . ' 'DarkMagenta'  $MaxResultLines }
                    'tool_use' {
                        if ($b.PSObject.Properties['id']) { $script:ToolNames[[string]$b.id] = [string]$b.name }
                        Write-Host "   > $($b.name)" -ForegroundColor Cyan
                        Show-ToolInput $b.input
                    }
                }
            }
        }
        'user' {
            foreach ($b in $e.message.content) {
                if ($b.type -eq 'tool_result') {
                    $name = 'result'
                    if ($b.PSObject.Properties['tool_use_id'] -and $script:ToolNames.ContainsKey([string]$b.tool_use_id)) {
                        $name = $script:ToolNames[[string]$b.tool_use_id]
                    }
                    $text = Get-ContentText $b.content
                    if ($b.is_error) {
                        Write-Host "     ! $name failed" -ForegroundColor Red
                        Write-Wrapped $text '     | ' 'Red' $MaxResultLines
                    }
                    else {
                        Write-Host "     < $name" -ForegroundColor DarkGreen
                        Write-Wrapped $text '     | ' 'DarkGray' $MaxResultLines
                    }
                }
                elseif ($b.type -eq 'text') {
                    Write-Wrapped $b.text '   + ' 'White' $MaxResultLines
                }
            }
        }
        'result' {
            $bits = @("$($e.num_turns) turns")
            if ($e.PSObject.Properties['duration_ms']) {
                $bits += (([double]$e.duration_ms / 1000).ToString('F1', [cultureinfo]::InvariantCulture) + 's')
            }
            if ($e.PSObject.Properties['usage'] -and $e.usage) {
                $bits += ('in {0} / out {1} tok' -f $e.usage.input_tokens, $e.usage.output_tokens)
            }
            if ($e.PSObject.Properties['total_cost_usd']) {
                $bits += '$' + ([double]$e.total_cost_usd).ToString('F2', [cultureinfo]::InvariantCulture)
            }
            Write-Host ("   [$($e.subtype)] " + ($bits -join '  ')) -ForegroundColor Yellow
            if ($e.PSObject.Properties['result'] -and $e.result) {
                Write-Wrapped ([string]$e.result) '   ' 'Yellow' $MaxResultLines
            }
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

    # The handoff file's 80-line cap (SESSION_PROMPT.md) is what keeps every future session's read cost
    # constant. It has drifted over twice already, so report it -- never stop on it: a long handoff is a
    # tidiness problem, and halting a 300-session run over one would cost far more than it saves.
    $handoff = Join-Path $repo 'NEXT_SESSION_PROMPT.md'
    if (Test-Path $handoff) {
        $n = (Get-Content $handoff | Measure-Object -Line).Lines
        if ($n -gt 80) {
            Write-Ledger ('       ! NEXT_SESSION_PROMPT.md is {0} lines, over its 80-line cap' -f $n)
        }
    }

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
