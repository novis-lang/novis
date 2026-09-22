<#
.SYNOPSIS
The live service check on Windows: install, start, request, stop, uninstall, against the real
service control manager — and the one scheduled task that runs it elevated without a UAC prompt.

.DESCRIPTION
The SCM only talks to an elevated process, and an agent session is never elevated. Rather than
weaken UAC for the whole machine, this script registers ONE scheduled task, `novis-service-live`,
whose only action is this script with `-Run`, at the highest run level. Registering it costs one UAC
prompt; from then on the task's owner starts it from any shell with no prompt, and it can run
nothing but this file. Everything is derived from the script's own location, so a clone anywhere
registers its own task; a clone moved re-registers.

    powershell -NoProfile -ExecutionPolicy Bypass -File tools\service-live.ps1 -Register    # once, one UAC prompt
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\service-live.ps1              # the check, no prompt
    powershell -NoProfile -ExecutionPolicy Bypass -File tools\service-live.ps1 -Unregister  # remove the task

The check runs the debug binary (`target\debug\nvs.exe`, or the release one where there is no
debug build) as a service named `novis-live` on a loopback port, and writes what it saw to
`.agent-tmp\service-live\run.log`, which the unelevated caller prints. It leaves nothing behind:
the last step is `nvs service uninstall`, run even when an earlier step failed.
#>
[CmdletBinding()]
param(
    [switch]$Register,
    [switch]$Unregister,
    [switch]$Run,
    [switch]$Elevated
)

$ErrorActionPreference = 'Continue'
$Root = Split-Path -Parent $PSScriptRoot
$Self = $PSCommandPath
$TaskName = 'novis-service-live'
$Dir = Join-Path $Root '.agent-tmp\service-live'
$Log = Join-Path $Dir 'run.log'
$Status = Join-Path $Dir 'status'

function Invoke-Elevated([string[]]$Switches) {
    # Re-runs this script elevated, once, and waits for it: the UAC prompt is this call's.
    $arguments = @('-NoProfile', '-ExecutionPolicy', 'Bypass', '-File', "`"$Self`"") + $Switches + @('-Elevated')
    $process = Start-Process -FilePath 'powershell.exe' -Verb RunAs -ArgumentList $arguments -Wait -PassThru
    exit $process.ExitCode
}

if ($Register) {
    if (-not $Elevated) { Invoke-Elevated @('-Register') }
    $action = New-ScheduledTaskAction -Execute 'powershell.exe' `
        -Argument "-NoProfile -ExecutionPolicy Bypass -WindowStyle Hidden -File `"$Self`" -Run"
    # Interactive: the task runs in the registering user's own session, with no password stored,
    # and only while they are logged on. Highest: the elevated half of their token.
    $principal = New-ScheduledTaskPrincipal -UserId "$env:USERDOMAIN\$env:USERNAME" `
        -LogonType Interactive -RunLevel Highest
    $settings = New-ScheduledTaskSettingsSet -ExecutionTimeLimit (New-TimeSpan -Minutes 10) `
        -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -MultipleInstances IgnoreNew
    Register-ScheduledTask -TaskName $TaskName -Action $action -Principal $principal `
        -Settings $settings -Force | Out-Null
    Write-Host "registered task `"$TaskName`" -> $Self -Run"
    exit 0
}

if ($Unregister) {
    if (-not $Elevated) { Invoke-Elevated @('-Unregister') }
    Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
    Write-Host "removed task `"$TaskName`""
    exit 0
}

if (-not $Run) {
    # The unelevated half: start the task, wait for it, print what it wrote.
    $task = Get-ScheduledTask -TaskName $TaskName -ErrorAction SilentlyContinue
    if ($null -eq $task) {
        Write-Host "the task `"$TaskName`" is not registered on this machine; run this script once with -Register"
        exit 2
    }
    New-Item -ItemType Directory -Force $Dir | Out-Null
    Remove-Item -Force -ErrorAction SilentlyContinue $Log, $Status
    Start-ScheduledTask -TaskName $TaskName
    $deadline = (Get-Date).AddMinutes(10)
    do {
        Start-Sleep -Milliseconds 500
        $state = (Get-ScheduledTask -TaskName $TaskName).State
    } while ($state -eq 'Running' -and (Get-Date) -lt $deadline)
    if (Test-Path $Log) { Get-Content $Log | Write-Host } else { Write-Host 'the task wrote no log' }
    if (Test-Path $Status) { exit [int](Get-Content $Status) }
    exit 1
}

# ---- The check itself, elevated, on the task's thread. Nothing below prompts or reads input. ----

New-Item -ItemType Directory -Force $Dir | Out-Null
Set-Content -Path $Status -Value 1 -Encoding ascii
$script:failed = 0

function Say([string]$text) {
    $line = '{0:HH:mm:ss.fff} {1}' -f (Get-Date), $text
    Add-Content -Path $Log -Value $line -Encoding utf8
}

function Step([string]$name, [scriptblock]$body) {
    Say "== $name"
    try {
        $ok = & $body
        if ($ok) { Say "   ok" } else { Say "   FAILED"; $script:failed++ }
    } catch {
        Say "   FAILED: $($_.Exception.Message)"
        $script:failed++
    }
}

function Nvs([string[]]$Argv) {
    # One array, passed to a named parameter: splatting or remaining-arguments binding both
    # rearranged the words on the way to the program.
    Say "   $ nvs $($Argv -join ' ')"
    $out = & $Exe $Argv 2>&1
    foreach ($line in $out) { Say "   | $line" }
    return $LASTEXITCODE
}

$Exe = Join-Path $Root 'target\debug\nvs.exe'
if (-not (Test-Path $Exe)) { $Exe = Join-Path $Root 'target\release\nvs.exe' }
if (-not (Test-Path $Exe)) {
    Say "no nvs.exe under $Root\target; build first"
    exit 1
}
# `$ServiceName` and not `$Name`: PowerShell variable names are case-insensitive, and `Step`'s
# `$name` parameter would shadow it inside every step.
$ServiceName = 'novis-live'
$Port = 18790
$Config = Join-Path $Dir 'nvs.toml'
$ServiceLog = Join-Path $Dir 'service.log'
$Entry = Join-Path $Root 'examples\hello.nvs'
Set-Content -Path $Config -Encoding ascii -Value @(
    '[server]',
    'health_path = "/healthz"',
    'workers = 4',
    # Short, because the PowerShell that made the requests keeps their connections pooled, and
    # an idle keep-alive connection is closed at the drain period's end: with the default the
    # stop step takes the whole 30 s on a service that is doing nothing.
    'drain_timeout = "3s"',
    '[control]',
    "socket = '\\.\pipe\$ServiceName-control'"
)
$Began = Get-Date
Say "binary: $Exe"
Say "user: $env:USERNAME, elevated: $(([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator))"

function Wait-Port([int]$seconds) {
    $end = (Get-Date).AddSeconds($seconds)
    while ((Get-Date) -lt $end) {
        try {
            $client = New-Object System.Net.Sockets.TcpClient
            $client.Connect('127.0.0.1', $Port)
            $client.Close()
            return $true
        } catch { Start-Sleep -Milliseconds 200 }
    }
    return $false
}

function Service-State {
    # `sc query` prints `STATE : 4  RUNNING` in English and `STATUS : 4  RUNNING` in German;
    # the number is the same everywhere, so it is what is read.
    $line = (& sc.exe query $ServiceName 2>&1 | Select-String ':\s+[1-7]\s+[A-Z_]+' | Select-Object -First 1)
    if ($null -eq $line) { return 'ABSENT' }
    switch -Regex ($line.ToString()) {
        ':\s+1\s' { return 'STOPPED' }
        ':\s+2\s' { return 'START_PENDING' }
        ':\s+3\s' { return 'STOP_PENDING' }
        ':\s+4\s' { return 'RUNNING' }
        default { return 'OTHER' }
    }
}

# A previous run that did not get to its last step leaves a service behind; take it away first.
if ((Service-State) -ne 'ABSENT') {
    Say "== a stale `"$ServiceName`" exists; removing it"
    & sc.exe stop $ServiceName | Out-Null
    Start-Sleep -Seconds 2
    Nvs -Argv @('service', 'uninstall', $ServiceName) | Out-Null
}

Step 'install' {
    (Nvs -Argv @('service', 'install', $ServiceName, '--log-file', $ServiceLog, '--start', 'manual',
        '--config', $Config, '--',
        'serve', $Entry, '--listen', "127.0.0.1:$Port", '--no-init', '--config', $Config)) -eq 0
}

Step 'start' {
    $code = Nvs -Argv @('service', 'start', $ServiceName)
    if ($code -ne 0) { return $false }
    $up = Wait-Port 60
    Say "   port answers: $up; sc state: $(Service-State)"
    $up -and ((Service-State) -eq 'RUNNING')
}

Step 'request' {
    $health = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:$Port/healthz" -TimeoutSec 10
    $body = Invoke-WebRequest -UseBasicParsing -Uri "http://127.0.0.1:$Port/" -TimeoutSec 10
    Say "   /healthz: $($health.StatusCode); /: $($body.StatusCode) $($body.Content.Substring(0, [Math]::Min(40, $body.Content.Length)))"
    ($health.StatusCode -eq 200) -and ($body.StatusCode -eq 200)
}

Step 'status' {
    (Nvs -Argv @('service', 'status', $ServiceName, '--config', $Config)) -eq 0
}

Step 'stop' {
    $began = Get-Date
    $code = Nvs -Argv @('service', 'stop', $ServiceName)
    $end = (Get-Date).AddSeconds(60)
    do { Start-Sleep -Milliseconds 250; $state = Service-State } while ($state -ne 'STOPPED' -and $state -ne 'ABSENT' -and (Get-Date) -lt $end)
    Say "   sc state after stop: $state, after $([int]((Get-Date) - $began).TotalMilliseconds) ms"
    ($code -eq 0) -and ($state -eq 'STOPPED')
}

Step 'sc qc' {
    foreach ($line in (& sc.exe qc $ServiceName 2>&1)) { Say "   | $line" }
    $true
}

Step 'event log' {
    # The source has no message table, so a record's text is its one insertion string and not
    # `Message`, which is empty; the viewer shows the same string under "the following information".
    $events = @(Get-WinEvent -FilterHashtable @{ LogName = 'Application'; StartTime = $Began.AddSeconds(-5) } -ErrorAction SilentlyContinue |
        Where-Object ProviderName -eq $ServiceName | Sort-Object TimeCreated)
    if ($events.Count -eq 0) { Say '   no records under this source'; return $false }
    foreach ($event in $events) {
        Say ("   | {0:HH:mm:ss.fff} id {1} level {2}: {3}" -f $event.TimeCreated, $event.Id, $event.Level, (($event.Properties | ForEach-Object Value) -join ' '))
    }
    $true
}

Step 'service log' {
    if (Test-Path $ServiceLog) { foreach ($line in (Get-Content $ServiceLog)) { Say "   | $line" } } else { Say '   (no file)' }
    $true
}

Step 'uninstall' {
    (Nvs -Argv @('service', 'uninstall', $ServiceName)) -eq 0
}

Say "== $(if ($script:failed -eq 0) { 'PASS' } else { "FAIL ($script:failed step(s))" })"
Set-Content -Path $Status -Value $(if ($script:failed -eq 0) { 0 } else { 1 }) -Encoding ascii
exit $(if ($script:failed -eq 0) { 0 } else { 1 })
