param(
  [int]$Port = 5188
)

$ErrorActionPreference = "SilentlyContinue"
$stopped = @()

$listenerPids = @(Get-NetTCPConnection -LocalPort $Port -State Listen | Select-Object -ExpandProperty OwningProcess -Unique)
foreach ($pid in $listenerPids) {
  cmd /c "taskkill /PID $pid /T /F" | Out-Null
  if ($LASTEXITCODE -eq 0) { $stopped += "port:$Port pid:$pid" }
}

$names = @("quan-voice", "quan-voice.exe")
foreach ($name in $names) {
  $procs = @(Get-Process -Name $name)
  foreach ($p in $procs) {
    cmd /c "taskkill /PID $($p.Id) /T /F" | Out-Null
    if ($LASTEXITCODE -eq 0) { $stopped += "name:$($p.ProcessName) pid:$($p.Id)" }
  }
}

if ($stopped.Count -eq 0) {
  Write-Host "No dev processes found."
} else {
  Write-Host ("Stopped: " + ($stopped -join ", "))
}
