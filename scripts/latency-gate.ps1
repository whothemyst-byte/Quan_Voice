param(
  [int]$Last = 300,
  [double]$P50TargetMs = 350,
  [double]$P95TargetMs = 900
)

$metricsPath = Join-Path $env:LOCALAPPDATA "QuanVoice\metrics\transcription_metrics.jsonl"
if (-not (Test-Path $metricsPath)) {
  Write-Host "Metrics file not found: $metricsPath"
  exit 1
}

$rows = Get-Content $metricsPath |
  Where-Object { $_.Trim().Length -gt 0 } |
  Select-Object -Last $Last |
  ForEach-Object {
    try { $_ | ConvertFrom-Json } catch { $null }
  } |
  Where-Object { $_ -ne $null }

if (-not $rows -or $rows.Count -eq 0) {
  Write-Host "No valid metric rows found."
  exit 1
}

function Get-Percentile([double[]]$values, [double]$p) {
  if (-not $values -or $values.Count -eq 0) { return [double]::NaN }
  $sorted = $values | Sort-Object
  if ($sorted.Count -eq 1) { return [double]$sorted[0] }
  $rank = ($p / 100.0) * ($sorted.Count - 1)
  $low = [math]::Floor($rank)
  $high = [math]::Ceiling($rank)
  if ($low -eq $high) { return [double]$sorted[$low] }
  $weight = $rank - $low
  return ([double]$sorted[$low] * (1.0 - $weight)) + ([double]$sorted[$high] * $weight)
}

$releaseToInject = @($rows | ForEach-Object { [double]([math]::Max(0, $_.inject_done_ts - $_.release_ts)) })
$p50 = Get-Percentile $releaseToInject 50
$p95 = Get-Percentile $releaseToInject 95

Write-Host ("Rows: " + $rows.Count)
Write-Host ("release_to_inject_ms p50={0:N1} p95={1:N1}" -f $p50, $p95)

$byModel = $rows | Group-Object model
foreach ($group in $byModel) {
  $vals = @($group.Group | ForEach-Object { [double]([math]::Max(0, $_.inject_done_ts - $_.release_ts)) })
  $m50 = Get-Percentile $vals 50
  $m95 = Get-Percentile $vals 95
  Write-Host ("model={0} count={1} p50={2:N1} p95={3:N1}" -f $group.Name, $group.Count, $m50, $m95)
}

$pass = ($p50 -le $P50TargetMs) -and ($p95 -le $P95TargetMs)
if ($pass) {
  Write-Host "GATE: PASS"
  exit 0
}

Write-Host ("GATE: FAIL (targets p50<={0}ms, p95<={1}ms)" -f $P50TargetMs, $P95TargetMs)
exit 2
