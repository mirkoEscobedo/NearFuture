param([Parameter(Mandatory)][string]$EvidenceRoot,[Parameter(Mandatory)][int]$NodePid,[Parameter(Mandatory)][string]$BindingPath,[Parameter(Mandatory)][string]$BindingSha256)
$ErrorActionPreference='Stop'
& (Join-Path $PSScriptRoot 'fixture/protection-check.ps1') -BindingPath $BindingPath -BindingSha256 $BindingSha256 -Phase admit-probe -EvidenceRoot $EvidenceRoot
if($NodePid-le0-or$NodePid-eq$PID){throw 'Distinct current Node parent required'}
$rows=@(Get-CimInstance Win32_Process -Filter "ProcessId = $NodePid OR ProcessId = $PID" -OperationTimeoutSec 10)
if($rows.Count-ne2){throw 'Complete current child and probe identity required'}
if(@($rows|Where-Object {$_.ProcessId-eq$PID-and$_.ParentProcessId-eq$NodePid}).Count-ne1){throw 'Current query helper must be a direct child of the admitted Node process'}
$identities=@($rows|ForEach-Object {
 if(!$_.ExecutablePath-or!$_.CreationDate){throw 'Complete current process identity required'}
 [ordered]@{pid=[int]$_.ProcessId;parentPid=[int]$_.ParentProcessId;creationTime=$_.CreationDate.ToUniversalTime().ToString('o');executable=$_.ExecutablePath;sessionId=[int]$_.SessionId}
})
$output=Join-Path $EvidenceRoot 'process-identities.private.json'
$stream=[IO.File]::Open($output,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
try{
 $bytes=[Text.Encoding]::UTF8.GetBytes(($identities|ConvertTo-Json -Depth 5))
 $stream.Write($bytes,0,$bytes.Length)
}finally{$stream.Dispose()}
# This unchanged helper only queries its inherited supervisor Job in this mode.
& (Join-Path $PSScriptRoot 'fixture/native-controller.ps1') -BindingPath $BindingPath -BindingSha256 $BindingSha256 -EvidenceRoot $EvidenceRoot -OwnershipProbeOnly
