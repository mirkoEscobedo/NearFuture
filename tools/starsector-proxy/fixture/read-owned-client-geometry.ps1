param(
 [Parameter(Mandatory=$true)][string]$EvidenceRoot,
 [Parameter(Mandatory=$true)][long]$WindowId,
 [Parameter(Mandatory=$true)][int]$ExpectedX,
 [Parameter(Mandatory=$true)][int]$ExpectedY,
 [Parameter(Mandatory=$true)][int]$ExpectedWidth,
 [Parameter(Mandatory=$true)][int]$ExpectedHeight
)
$ErrorActionPreference='Stop'
$base=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../../.tmp/starsector-proxy/scenarios'))
if([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($EvidenceRoot))-ine$base){throw 'Caller-owned scenario root required'}
foreach($name in @('controller-start.private.json','owned-processes-current.private.json','plan.json','native-lane-lease.private.json')){
 $path=Join-Path $EvidenceRoot $name
 $item=Get-Item -LiteralPath $path -Force
 while($null-ne$item){if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary owned metadata required'};$item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}}
}
$start=Get-Content -LiteralPath "$EvidenceRoot/controller-start.private.json" -Raw|ConvertFrom-Json -DateKind String
$sample=Get-Content -LiteralPath "$EvidenceRoot/owned-processes-current.private.json" -Raw|ConvertFrom-Json -DateKind String
$lease=Get-Content -LiteralPath "$EvidenceRoot/native-lane-lease.private.json" -Raw|ConvertFrom-Json
if($lease.authorized-ne$true-or(Get-FileHash -LiteralPath "$EvidenceRoot/plan.json").Hash-ine$lease.planSha256-or$sample.controllerPid-ne$start.controllerPid-or$sample.controllerKernelCreationFileTime-cne$start.controllerKernelCreationFileTime){throw 'Current owned launch binding required'}
$now=[DateTime]::UtcNow
$age=($now-[DateTime]::Parse($sample.at).ToUniversalTime()).TotalMilliseconds
if($age-lt0-or$age-gt5000-or$now-ge[DateTime]::Parse($start.conservativeOuterDeadlineUtc).ToUniversalTime()){throw 'Fresh in-bound owned sample required'}
$rows=@($sample.processes|Where-Object {$_.mainWindowHandle-ceq$WindowId.ToString([Globalization.CultureInfo]::InvariantCulture)})
if($rows.Count-ne1-or$rows[0].inSupervisorJob-ne$true){throw 'Exact current owned HWND required'}
$row=$rows[0]
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class NFSmokeClientGeometry {
 [StructLayout(LayoutKind.Sequential)] public struct RECT { public int Left,Top,Right,Bottom; }
 [StructLayout(LayoutKind.Sequential)] public struct POINT { public int X,Y; }
 [DllImport("user32.dll",SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)]
 public static extern bool GetWindowRect(IntPtr h,out RECT r);
 [DllImport("user32.dll",SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)]
 public static extern bool GetClientRect(IntPtr h,out RECT r);
 [DllImport("user32.dll",SetLastError=true)] [return:MarshalAs(UnmanagedType.Bool)]
 public static extern bool ClientToScreen(IntPtr h,ref POINT p);
 [DllImport("user32.dll",SetLastError=true)]
 public static extern uint GetWindowThreadProcessId(IntPtr h,out uint p);
}
'@
$h=[IntPtr]::new($WindowId);[uint32]$ownerPid=0
if([NFSmokeClientGeometry]::GetWindowThreadProcessId($h,[ref]$ownerPid)-eq0-or$ownerPid-ne$row.pid){throw 'HWND owner changed'}
$outer=[NFSmokeClientGeometry+RECT]::new();$client=[NFSmokeClientGeometry+RECT]::new();$origin=[NFSmokeClientGeometry+POINT]::new()
if(![NFSmokeClientGeometry]::GetWindowRect($h,[ref]$outer)-or![NFSmokeClientGeometry]::GetClientRect($h,[ref]$client)-or![NFSmokeClientGeometry]::ClientToScreen($h,[ref]$origin)){throw 'Current client geometry unavailable'}
if($outer.Left-ne$ExpectedX-or$outer.Top-ne$ExpectedY-or$outer.Right-$outer.Left-ne$ExpectedWidth-or$outer.Bottom-$outer.Top-ne$ExpectedHeight){throw 'MCP outer coordinates differ; no transform admitted'}
$current=@(Get-CimInstance Win32_Process -Filter "ProcessId=$ownerPid" -OperationTimeoutSec 10)
if($current.Count-ne1-or$current[0].CreationDate.ToString('o')-cne$row.creationTime-or$current[0].SessionId-ne$row.sessionId-or$current[0].ExecutablePath.Replace('\','/')-ine$row.executable.Replace('\','/')){throw 'Owned process identity changed'}
if([NFSmokeClientGeometry]::GetWindowThreadProcessId($h,[ref]$ownerPid)-eq0-or$ownerPid-ne$row.pid){throw 'HWND owner changed after geometry'}
$outerAfter=[NFSmokeClientGeometry+RECT]::new();$clientAfter=[NFSmokeClientGeometry+RECT]::new();$originAfter=[NFSmokeClientGeometry+POINT]::new()
if(![NFSmokeClientGeometry]::GetWindowRect($h,[ref]$outerAfter)-or![NFSmokeClientGeometry]::GetClientRect($h,[ref]$clientAfter)-or![NFSmokeClientGeometry]::ClientToScreen($h,[ref]$originAfter)){throw 'Final geometry unavailable'}
foreach($field in @('Left','Top','Right','Bottom')){if($outerAfter.$field-ne$outer.$field-or$clientAfter.$field-ne$client.$field){throw 'Window geometry changed; reacquire'}}
if($originAfter.X-ne$origin.X-or$originAfter.Y-ne$origin.Y){throw 'Client origin changed; reacquire'}
$now=[DateTime]::UtcNow
$age=($now-[DateTime]::Parse($sample.at).ToUniversalTime()).TotalMilliseconds
if($age-lt0-or$age-gt5000){throw 'Owned sample expired during geometry observation'}
foreach($deadline in @($start.conservativeOuterDeadlineUtc,$start.controllerDeadlineUtc)){if($now-ge[DateTime]::Parse($deadline).ToUniversalTime()){throw 'Owned deadline expired during geometry observation'}}
if($client.Right-$client.Left-lt1-or$client.Bottom-$client.Top-lt1-or$client.Right-$client.Left-gt$ExpectedWidth-or$client.Bottom-$client.Top-gt$ExpectedHeight){throw 'Client rectangle inconsistent'}
[ordered]@{schemaVersion=1;scope='READ_ONLY_OWNED_HWND_CLIENT_GEOMETRY';windowId=$WindowId;pid=$ownerPid;creationTime=$row.creationTime;controllerPid=$start.controllerPid;observedAtUtc=[DateTime]::UtcNow.ToString('o');outerBounds=@{x=$outer.Left;y=$outer.Top;width=($outer.Right-$outer.Left);height=($outer.Bottom-$outer.Top)};clientOriginScreen=@{x=$origin.X;y=$origin.Y};clientSize=@{width=($client.Right-$client.Left);height=($client.Bottom-$client.Top)};offsetFromOuter=@{x=($origin.X-$outer.Left);y=($origin.Y-$outer.Top)};mcpOuterBoundsMatched=$true;inputGranted=$false;originMeaning='Current ClientToScreen(hwnd,0,0), matching upstream7.1.0 GetDC(hwnd) client-image origin; reacquire if window/focus changes.'}|ConvertTo-Json -Depth 6

