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

# ---------------------------------------------------------------------------------------------------
# The goal's own acceptance test -- see .claude/loop-goal.md, which is authoritative for this list.
# No model judgment sits on the stop path: every item is an exit code plus an exact or ordered-substring
# match on real output. The Windows leg short-circuits on the first failure so a broken iteration is cheap;
# the WSL leg runs only once Windows is fully green, because a JIT is exactly where a calling-convention
# divergence between two targets hides.
# ---------------------------------------------------------------------------------------------------

$script:GoalFail = ''

# <repo> -> /mnt/<drive>/<repo>
$script:WslRepo = '/mnt/' + $repo.Substring(0, 1).ToLower() + ($repo.Substring(2) -replace '\\', '/')
$script:WslTarget = '/tmp/mwl-target-wsl'

function Quote-Arg([string]$a) {
    if ($a -match '[\s"]') { return '"' + ($a -replace '"', '\"') + '"' }
    return $a
}

# Runs a native exe with stdout and stderr captured SEPARATELY -- the acceptance list distinguishes them
# (a backtrace and FATAL go to stderr, program output to stdout), and PS 5.1's `2>&1` on a native command
# both interleaves them and corrupts $? . `Start-Process -PassThru` is no good either: the object it hands
# back does not retain the process handle, so `.ExitCode` reads back as $null. Starting the process
# directly keeps the handle, and reading both streams asynchronously is what stops a full pipe buffer from
# deadlocking against WaitForExit.
function Invoke-Capture([string]$exe, [string[]]$exeArgs, [int]$timeoutSec = 1800) {
    $psi = New-Object System.Diagnostics.ProcessStartInfo
    $psi.FileName               = $exe
    $psi.Arguments              = (($exeArgs | ForEach-Object { Quote-Arg $_ }) -join ' ')
    $psi.WorkingDirectory       = $repo
    $psi.UseShellExecute        = $false
    $psi.CreateNoWindow         = $true
    $psi.RedirectStandardOutput = $true
    $psi.RedirectStandardError  = $true

    $p = $null
    try {
        $p = [System.Diagnostics.Process]::Start($psi)
        $ot = $p.StandardOutput.ReadToEndAsync()
        $et = $p.StandardError.ReadToEndAsync()
        if (-not $p.WaitForExit($timeoutSec * 1000)) {
            try { $p.Kill() } catch { }
            return @{ Code = -1; Out = ''; Err = "timed out after ${timeoutSec}s" }
        }
        $p.WaitForExit()
        $o = $ot.Result
        $r = $et.Result
        if ($null -eq $o) { $o = '' }
        if ($null -eq $r) { $r = '' }
        return @{ Code = [int]$p.ExitCode; Out = $o; Err = $r }
    }
    catch {
        return @{ Code = -1; Out = ''; Err = [string]$_.Exception.Message }
    }
    finally {
        if ($null -ne $p) { $p.Dispose() }
    }
}

function Fail-Goal([string]$why) {
    $script:GoalFail = $why
    return $false
}

# `$want` must appear, and each element after the first must appear AFTER the one before it.
function Test-Ordered([string]$text, [string[]]$want) {
    $at = 0
    foreach ($w in $want) {
        $i = $text.IndexOf($w, $at, [System.StringComparison]::Ordinal)
        if ($i -lt 0) { return $false }
        $at = $i + $w.Length
    }
    return $true
}

$script:GoalExact = @(
    @{ File = 'examples/hello.mwl'; Want = 'Hello, World!' }
    @{ File = 'examples/calls.mwl'; Want = 'quadruple(5) = 20' }
    @{ File = 'examples/throw.mwl'; Want = 'caught: boom' }
    @{ File = 'examples/arith.mwl'; Want = 'sum = 998000' }
)

$script:GoalFiles = @(
    'examples/hello.mwl', 'examples/calls.mwl', 'examples/throw.mwl', 'examples/arith.mwl',
    'examples/trace.mwl', 'examples/uncaught.mwl', 'examples/fatal.mwl'
)

# $Run takes a string[] of arguments to place after `mwl run` and returns an Invoke-Capture hashtable.
function Test-GoalLeg([scriptblock]$Run, [string]$leg) {

    foreach ($c in $script:GoalExact) {
        $r = & $Run @(, $c.File)
        if ($r.Code -ne 0) {
            return (Fail-Goal ("{0} {1}: exit {2} -- {3}" -f $leg, $c.File, $r.Code, ($r.Err.Trim() -split "`r?`n")[0]))
        }
        $got = $r.Out.Trim()
        if ($got -ne $c.Want) {
            return (Fail-Goal ("{0} {1}: stdout was '{2}', wanted '{3}'" -f $leg, $c.File, $got, $c.Want))
        }
    }

    # A caught throw's own trace, read back from MWL through getTraceAsString().
    $r = & $Run @(, 'examples/trace.mwl')
    if ($r.Code -ne 0) { return (Fail-Goal ("{0} trace.mwl: exit {1}" -f $leg, $r.Code)) }
    if (-not (Test-Ordered $r.Out @('#0 Deep::inner()', '#1 Deep::outer()'))) {
        return (Fail-Goal ("{0} trace.mwl: no ordered '#0 Deep::inner()' / '#1 Deep::outer()' in stdout" -f $leg))
    }

    # An uncaught throw: non-zero exit plus an MWL-level backtrace on stderr.
    $r = & $Run @(, 'examples/uncaught.mwl')
    if ($r.Code -eq 0) { return (Fail-Goal ("{0} uncaught.mwl: exited 0, wanted non-zero" -f $leg)) }
    if (-not (Test-Ordered $r.Err @('Uncaught Exception: unhandled', '#0 Boom::inner()', '#1 Boom::outer()'))) {
        return (Fail-Goal ("{0} uncaught.mwl: stderr is not an MWL backtrace naming inner then outer" -f $leg))
    }

    # A contained helper panic: FATAL, non-zero, and the output produced before it survives.
    $r = & $Run @('--fault-inject=helper-panic', 'examples/fatal.mwl')
    if ($r.Code -eq 0) { return (Fail-Goal ("{0} fatal.mwl: exited 0, wanted non-zero" -f $leg)) }
    if ($r.Err -notmatch 'FATAL') { return (Fail-Goal ("{0} fatal.mwl: no FATAL on stderr" -f $leg)) }
    if ($r.Out -notmatch 'start') { return (Fail-Goal ("{0} fatal.mwl: stdout lost the output written before the panic" -f $leg)) }

    # --dump-asm prints generated code instead of running, the way --dump-ir already does.
    $r = & $Run @('--dump-asm', 'examples/arith.mwl')
    if ($r.Code -ne 0) { return (Fail-Goal ("{0} --dump-asm: exit {1}" -f $leg, $r.Code)) }
    if ($r.Out.Length -lt 200) { return (Fail-Goal ("{0} --dump-asm: only {1} bytes of output" -f $leg, $r.Out.Length)) }

    return $true
}

# A named test must both EXIST and pass -- `cargo test` is green on a suite that never ran the guard.
function Test-NamedTests([string[]]$cargoArgs, [string[]]$names, [string]$what) {
    $r = Invoke-Capture 'cargo' $cargoArgs
    if ($r.Code -ne 0) {
        return (Fail-Goal ("{0}: cargo test exit {1}" -f $what, $r.Code))
    }
    $all = $r.Out + "`n" + $r.Err
    foreach ($n in $names) {
        if ($all -notmatch [regex]::Escape($n)) {
            return (Fail-Goal ("{0}: test '{1}' did not run" -f $what, $n))
        }
    }
    return $true
}

function Test-Goal {
    $script:GoalFail = ''

    foreach ($f in $script:GoalFiles) {
        if (-not (Test-Path (Join-Path $repo ($f -replace '/', '\')))) {
            return (Fail-Goal "$f is missing -- the acceptance examples are fixed, see .claude/loop-goal.md")
        }
    }

    $winRun = {
        param($mwlArgs)
        Invoke-Capture 'cargo' (@('run', '--quiet', '-p', 'mwl-cli', '--', 'run') + $mwlArgs)
    }
    if (-not (Test-GoalLeg $winRun 'win')) { return $false }

    if (-not (Test-NamedTests @('test', '--release', '-p', 'mwl-abi-probe') `
                @('a_typed_arithmetic_loop_contains_no_call',
                  'a_typed_arithmetic_loop_stays_in_the_native_cost_class') 'abi-probe')) { return $false }

    if (-not (Test-NamedTests @('test', '-p', 'mwl-codegen') `
                @('a_second_script_runs_after_a_contained_helper_panic') 'mwl-codegen')) { return $false }

    # Windows is green -- now pay for the Linux leg.
    $wslRun = {
        param($mwlArgs)
        $inner = 'cd ' + $script:WslRepo + ' && CARGO_TARGET_DIR=' + $script:WslTarget +
                 ' cargo run --quiet -p mwl-cli -- run ' + ($mwlArgs -join ' ')
        Invoke-Capture 'wsl.exe' @('--', 'bash', '-lc', $inner)
    }
    if (-not (Test-GoalLeg $wslRun 'wsl')) { return $false }

    return $true
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
    if (Test-Goal) { $reason = 'GOAL REACHED: M3 acceptance list is green on Windows and WSL'; break }
    Write-Ledger ('       goal check: ' + $script:GoalFail)

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
