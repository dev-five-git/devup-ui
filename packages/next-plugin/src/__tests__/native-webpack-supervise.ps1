param([string]$Config)
$ErrorActionPreference = 'Stop'
$job = $null
$owner = $null
$caller = $null
$result = 125
$reason = 'setup_error'
try {
    $settings = [Text.Encoding]::UTF8.GetString([Convert]::FromBase64String($Config)) | ConvertFrom-Json
    $owner = [Diagnostics.Process]::GetProcessById($settings.owner)
    $ownerHandle = $owner.Handle
    $caller = [Diagnostics.Process]::GetProcessById($settings.caller)
    $callerHandle = $caller.Handle
    Add-Type -Path (Join-Path $PSScriptRoot 'webpack-job.cs')
    $job = New-Object WebpackJob
    if ($owner.HasExited -or $caller.HasExited) { throw 'Native command owner exited before launch' }
    $job.Launch($settings.command, $settings.cwd)
    [Console]::WriteLine('NATIVE-LIFECYCLE ' + (@{ event = 'launch'; pid = $job.RootPid } | ConvertTo-Json -Compress))
    $clock = [Diagnostics.Stopwatch]::StartNew()
    while ($true) {
        $members = @($job.Members())
        if ($job.RootExited) { $result = $job.ExitCode; $reason = 'command_exit'; break }
        if ($owner.HasExited -or $caller.HasExited) { $result = 125; $reason = 'owner_exit'; break }
        if ($clock.ElapsedMilliseconds -ge $settings.deadline) { $result = 124; $reason = 'deadline'; break }
        Start-Sleep -Milliseconds 100
    }
} catch {
    [Console]::Error.WriteLine($_.Exception.ToString())
} finally {
    if ($null -ne $job) {
        try {
            $job.KillAndWait()
            [Console]::WriteLine('NATIVE-LIFECYCLE ' + (@{ event = 'tree_exit'; pids = @($job.Observed); active = @($job.Members()); reason = $reason; exitCode = $result } | ConvertTo-Json -Compress))
        } catch { $result = 125; [Console]::Error.WriteLine($_.Exception.ToString()) }
        finally { $job.Dispose() }
    }
    if ($null -ne $owner) { $owner.Dispose() }
    if ($null -ne $caller) { $caller.Dispose() }
}
exit $result
