param(
    [Parameter(Mandatory)][string]$Scenario,
    [ValidateRange(10, 300)][int]$DurationSeconds = 60,
    [ValidateRange(1, 10)][int]$SampleSeconds = 2
)
$ErrorActionPreference = 'Stop'
$calendarLogical = [int](Get-CimInstance Win32_ComputerSystem).NumberOfLogicalProcessors
$calendarRoots = @(Get-Process -Name calendar-app)
if ($calendarRoots.Count -ne 1) { throw 'Debe haber exactamente una instancia de Calendario.' }
$calendarRootId = $calendarRoots[0].Id
$calendarRootStart = $calendarRoots[0].StartTime.ToUniversalTime().ToString('o')
$calendarKnown = [System.Collections.Generic.HashSet[int]]::new()
[void]$calendarKnown.Add($calendarRootId)
$calendarNames = @{}
function Get-CalendarCounters {
    $calendarTable = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name)
    do {
        $calendarAdded = $false
        foreach ($calendarRow in $calendarTable) {
            if ($calendarKnown.Contains([int]$calendarRow.ParentProcessId) -and $calendarKnown.Add([int]$calendarRow.ProcessId)) { $calendarAdded = $true }
        }
    } while ($calendarAdded)
    $calendarCounters = @{}
    $calendarResident = 0L
    $calendarPrivate = 0L
    foreach ($calendarRow in $calendarTable) {
        if (-not $calendarKnown.Contains([int]$calendarRow.ProcessId)) { continue }
        try {
            $calendarProcess = Get-Process -Id $calendarRow.ProcessId -ErrorAction Stop
            $calendarKey = '{0}:{1}' -f $calendarProcess.Id, $calendarProcess.StartTime.ToUniversalTime().Ticks
            $calendarCounters[$calendarKey] = $calendarProcess.TotalProcessorTime.TotalSeconds
            $calendarNames[$calendarKey] = $calendarProcess.ProcessName
            $calendarResident += $calendarProcess.WorkingSet64
            $calendarPrivate += $calendarProcess.PrivateMemorySize64
        } catch { continue }
    }
    [pscustomobject]@{Counters=$calendarCounters; WorkingSetSumMiB=$calendarResident/1MB; PrivateCommitMiB=$calendarPrivate/1MB}
}
$calendarOutput = Join-Path $PSScriptRoot '../docs/performance'
New-Item -ItemType Directory -Force -Path $calendarOutput | Out-Null
$calendarStart = (Get-Date).ToUniversalTime()
$calendarBaseline = Get-CalendarCounters
$calendarPrevious = $calendarBaseline.Counters
$calendarWatch = [Diagnostics.Stopwatch]::StartNew()
$calendarLastElapsed = 0.0
$calendarSamples = [System.Collections.Generic.List[object]]::new()
$calendarCpuTotal = 0.0
Write-Output "Midiendo $Scenario durante aproximadamente $DurationSeconds segundos; incluye todos los descendientes de Calendario."
while ($calendarWatch.Elapsed.TotalSeconds -lt $DurationSeconds) {
    Start-Sleep -Milliseconds ([int]([Math]::Min($SampleSeconds, $DurationSeconds-$calendarWatch.Elapsed.TotalSeconds)*1000))
    $calendarCurrent = Get-CalendarCounters
    $calendarElapsed = $calendarWatch.Elapsed.TotalSeconds
    $calendarInterval = $calendarElapsed - $calendarLastElapsed
    $calendarDelta = 0.0
    foreach ($calendarKey in $calendarCurrent.Counters.Keys) {
        if ($calendarPrevious.ContainsKey($calendarKey)) {
            $calendarDelta += [Math]::Max(0.0, $calendarCurrent.Counters[$calendarKey] - $calendarPrevious[$calendarKey])
        } else {
            # A child born during the measurement contributes its CPU since birth.
            $calendarDelta += $calendarCurrent.Counters[$calendarKey]
        }
    }
    $calendarCpuTotal += $calendarDelta
    $calendarSamples.Add([pscustomobject]@{
        ElapsedSeconds=[Math]::Round($calendarElapsed,3)
        IntervalSeconds=[Math]::Round($calendarInterval,3)
        CpuPercentTotalMachine=100*$calendarDelta/$calendarInterval/$calendarLogical
        CpuPercentOneCore=100*$calendarDelta/$calendarInterval
        WorkingSetSumMiB=$calendarCurrent.WorkingSetSumMiB
        PrivateCommitMiB=$calendarCurrent.PrivateCommitMiB
        ProcessCount=$calendarCurrent.Counters.Count
    })
    $calendarPrevious = $calendarCurrent.Counters
    $calendarLastElapsed = $calendarElapsed
}
$calendarWatch.Stop()
$calendarSummary = [pscustomobject]@{
    Scenario=$Scenario
    StartedUtc=$calendarStart.ToString('o')
    RootId=$calendarRootId
    RootStartUtc=$calendarRootStart
    DurationSeconds=[Math]::Round($calendarLastElapsed,3)
    LogicalProcessors=$calendarLogical
    SampleCount=$calendarSamples.Count
    CpuSeconds=$calendarCpuTotal
    AverageCpuPercent=100*$calendarCpuTotal/$calendarLastElapsed/$calendarLogical
    MaximumSampleCpuPercent=($calendarSamples | Measure-Object CpuPercentTotalMachine -Maximum).Maximum
    AverageWorkingSetSumMiB=($calendarSamples | Measure-Object WorkingSetSumMiB -Average).Average
    AveragePrivateCommitMiB=($calendarSamples | Measure-Object PrivateCommitMiB -Average).Average
    ProcessNames=$calendarNames
    Notes='CPU normalizada sobre toda la capacidad del equipo, incluyendo WebView2. Los picos son promedios por intervalo (~2 s), no instantáneos. WorkingSetSum puede contar páginas compartidas más de una vez. No mide GPU ni consumo eléctrico. Procesos que terminan entre muestras pueden dejar CPU sin contabilizar.'
}
$calendarSamples | Export-Csv -LiteralPath (Join-Path $calendarOutput "$Scenario.csv") -NoTypeInformation -Encoding utf8
$calendarSummary | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $calendarOutput "$Scenario.json") -Encoding utf8
$calendarSummary | ConvertTo-Json -Depth 5

