param([string]$Config)
$ErrorActionPreference = 'Stop'
$job = $null
$result = 125
$reason = 'setup_error'
$clean = $false
$script:outputClosed = $false
function Record($Event, $Data) {
    if ($script:outputClosed) { return }
    try { [Console]::WriteLine('NEXT-EVENT ' + (@{ event = $Event; data = $Data } | ConvertTo-Json -Depth 6 -Compress)) }
    catch [System.IO.IOException] { $script:outputClosed = $true }
}
try {
    $settings = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Config)) | ConvertFrom-Json
    $owner = [Diagnostics.Process]::GetProcessById($settings.owner)
    $ownerHandle = $owner.Handle
    $caller = [Diagnostics.Process]::GetProcessById($settings.caller)
    $callerHandle = $caller.Handle
    Add-Type -Path (Join-Path $PSScriptRoot 'webpack-job.cs')
    $env:DEVUP_NEXT_FIXTURE_ROOT = $settings.root
    $job = New-Object WebpackJob
    if ($owner.HasExited) { throw 'Owner exited before launch' }
    $job.Launch($settings.command, $settings.cwd)
    $clock = [Diagnostics.Stopwatch]::StartNew()
    $deadline = 110000
    Record 'launch' @{ root = $settings.root; pid = $job.RootPid }
    while ($true) {
        $members = @($job.Members())
        Record 'members' @{ pids = $members }
        if ($job.RootExited) { $result = $job.ExitCode; $reason = 'driver_exit'; break }
        if ($owner.HasExited -or $caller.HasExited) { $result = 125; $reason = 'owner_exit'; break }
        if ($clock.ElapsedMilliseconds -ge $deadline) { $result = 124; $reason = 'deadline'; break }
        if (($settings.fault -eq 'post-start-timeout') -and (Test-Path -LiteralPath (Join-Path $settings.root 'ready'))) {
            $deadline = [Math]::Min($deadline, $clock.ElapsedMilliseconds + 500)
        }
        Start-Sleep -Milliseconds 100
    }
} catch {
    Record 'error' @{ message = $_.Exception.Message }
} finally {
    if ($null -ne $job) {
        try {
            $job.KillAndWait()
            Record 'tree_exit' @{ pids = @($job.Observed); active = @($job.Members()); reason = $reason }
            $clean = $true
        } catch { $result = 125; Record 'cleanup_error' @{ message = $_.Exception.Message } }
        finally { $job.Dispose() }
    }
    if ($clean) {
        & $settings.node (Join-Path $PSScriptRoot 'webpack-remove.cjs') $settings.root
        if ($LASTEXITCODE -ne 0) { throw 'Fixture removal failed' }
        Record 'cleanup' @{ root = $settings.root; removed = $true; exitCode = $result }
    }
    if ($null -ne $owner) { $owner.Dispose() }
    if ($null -ne $caller) { $caller.Dispose() }
}
exit $result
