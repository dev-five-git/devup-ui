param(
  [Parameter(Mandatory=$true)][string]$Program,
  [string]$Arguments = '',
  [string]$Directory = (Get-Location).Path,
  [Parameter(Mandatory=$true)][string]$Log,
  [int]$TimeoutSeconds = 3600,
  [switch]$RealBuild
)
$ErrorActionPreference = 'Stop'
$env:CI = $null
$env:CARGO_BUILD_JOBS = '2'
if (Test-Path -LiteralPath "$PSScriptRoot\W72-node\node.exe") { $env:PATH = "$PSScriptRoot\W72-node;$env:PATH" }
if ($env:RUST_MIN_STACK -or $env:RUSTFLAGS -match 'stack-size|STACK:') { throw 'Default stack required' }
$os = Get-CimInstance Win32_OperatingSystem
$freeGiB = $os.FreePhysicalMemory / 1MB
if ($RealBuild -and $freeGiB -lt 7) { throw "Real build needs 7 GiB free; found $freeGiB" }
$info = New-Object System.Diagnostics.ProcessStartInfo
$info.FileName = (Get-Command $Program).Source
$info.Arguments = $Arguments
$info.WorkingDirectory = $Directory
$info.UseShellExecute = $false
$info.RedirectStandardOutput = $true
$info.RedirectStandardError = $true
$process = New-Object System.Diagnostics.Process
$process.StartInfo = $info
$watch = [Diagnostics.Stopwatch]::StartNew()
[void]$process.Start()
$stdout = $process.StandardOutput.ReadToEndAsync()
$stderr = $process.StandardError.ReadToEndAsync()
$started = $process.StartTime
$peakRss = 0L
$peakPrivate = 0L
$samples = 0
$lastBeat = 0
$reason = 'exit'
try {
  while (-not $process.HasExited) {
    $all = @(Get-CimInstance Win32_Process | Where-Object { $_.ProcessId -eq $process.Id -or $_.CreationDate -ge $started })
    if ($process.HasExited) { break }
    $birth = @{}
    foreach ($p in $all) { $birth[[int]$p.ProcessId] = $p.CreationDate }
    $known = [Collections.Generic.HashSet[int]]::new()
    [void]$known.Add($process.Id)
    do {
      $added = 0
      foreach ($p in $all) {
        if ($known.Contains([int]$p.ParentProcessId) -and $birth.ContainsKey([int]$p.ParentProcessId) -and $p.CreationDate -ge $birth[[int]$p.ParentProcessId] -and $known.Add([int]$p.ProcessId)) { $added++ }
      }
    } while ($added -gt 0)
    $tree = @($all | Where-Object { $known.Contains([int]$_.ProcessId) })
    $rss = [long]($tree | Measure-Object WorkingSetSize -Sum).Sum
    $private = [long]($tree | Measure-Object PrivatePageCount -Sum).Sum
    $peakRss = [Math]::Max($peakRss, $rss)
    $peakPrivate = [Math]::Max($peakPrivate, $private)
    $samples++
    if ($rss -gt 6GB -or $private -gt 6GB) { $reason = 'memory-cap'; break }
    if ($watch.Elapsed.TotalSeconds -gt $TimeoutSeconds) { $reason = 'timeout'; break }
    if ($watch.Elapsed.TotalSeconds - $lastBeat -gt 290) {
      if ($env:W72_ORCA_FROM -and $env:W72_ORCA_TASK_ID -and $env:W72_ORCA_DISPATCH_ID) {
        & orca orchestration send --from $env:W72_ORCA_FROM --type heartbeat --subject alive --task-id $env:W72_ORCA_TASK_ID --dispatch-id $env:W72_ORCA_DISPATCH_ID --phase implementing
      }
      $lastBeat = $watch.Elapsed.TotalSeconds
    }
    Start-Sleep -Milliseconds 500
  }
  if ($reason -ne 'exit') {
    foreach ($member in ($tree | Sort-Object CreationDate -Descending)) {
      $current = Get-CimInstance Win32_Process -Filter "ProcessId = $($member.ProcessId)"
      if ($current -and $current.CreationDate -eq $member.CreationDate) { Stop-Process -Id $member.ProcessId -Force -ErrorAction SilentlyContinue }
    }
  }
  $process.WaitForExit()
  [IO.File]::WriteAllText($Log, $stdout.Result + "`n--- STDERR ---`n" + $stderr.Result)
  $result = [ordered]@{
    program = $Program; arguments = $Arguments; directory = $Directory
    elapsedMs = $watch.Elapsed.TotalMilliseconds; exitCode = $process.ExitCode; reason = $reason
    freeGiBAtStart = $freeGiB; peakTreeRssBytes = $peakRss; peakTreePrivateBytes = $peakPrivate
    samples = $samples; sampledEveryMs = 500; capBytes = 6GB
    watchdogVersion = 'current-tree-v2-root-included'
    capProcessTree = if ($reason -eq 'memory-cap') { @($tree | Select-Object ProcessId,ParentProcessId,Name,CreationDate,WorkingSetSize,PrivatePageCount) } else { @() }
  }
  $json = $result | ConvertTo-Json
  [IO.File]::WriteAllText($Log + '.json', $json)
  $json
  if ($reason -ne 'exit') { exit 125 }
  exit $process.ExitCode
} finally {
  if (-not $process.HasExited) { $process.Kill() }
  $process.Dispose()
}
