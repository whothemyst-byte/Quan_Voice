param(
  [int]$Last = 200
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

$total = @($rows | ForEach-Object { [double]$_.total_ms })
$releaseToFirst = @($rows | ForEach-Object { [double]([math]::Max(0, $_.first_token_ts - $_.release_ts)) })
$transcribe = @($rows | ForEach-Object { [double]$_.transcribe_ms })
$inject = @($rows | ForEach-Object { [double]$_.inject_ms })

Write-Host ("Rows: " + $rows.Count)
Write-Host ("total_ms               p50={0:N1}  p95={1:N1}" -f (Get-Percentile $total 50), (Get-Percentile $total 95))
Write-Host ("release_to_first_ms    p50={0:N1}  p95={1:N1}" -f (Get-Percentile $releaseToFirst 50), (Get-Percentile $releaseToFirst 95))
Write-Host ("transcribe_ms          p50={0:N1}  p95={1:N1}" -f (Get-Percentile $transcribe 50), (Get-Percentile $transcribe 95))
Write-Host ("inject_ms              p50={0:N1}  p95={1:N1}" -f (Get-Percentile $inject 50), (Get-Percentile $inject 95))
