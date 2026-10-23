param([Parameter(Mandatory)][string]$BindingPath,[Parameter(Mandatory)][string]$BindingSha256,[Parameter(Mandatory)][string]$EvidenceRoot,[switch]$OwnershipProbeOnly,[string]$OuterDeadlineUtc,[ValidateRange(480000,43200000)][long]$SessionWatchdogMs=480000)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'fixture-inputs.ps1')
. (Join-Path $PSScriptRoot 'publish-owned-snapshot.ps1')
. (Join-Path $PSScriptRoot 'session-watchdog.ps1')
$binding=Read-NativeFixtureBinding $BindingPath $BindingSha256
if(!$OwnershipProbeOnly){$SessionWatchdogMs=Read-NativeSessionWatchdog $EvidenceRoot $SessionWatchdogMs}
& (Join-Path $PSScriptRoot 'protection-check.ps1') -BindingPath $BindingPath -BindingSha256 $BindingSha256 -Phase admit-probe -EvidenceRoot $EvidenceRoot
$installation=Join-Path $binding.fixtureRoot 'installation'
$receiptPath=Join-Path $binding.fixtureRoot 'protected-fixture-receipt.private.json'
if(Test-Path -LiteralPath "$EvidenceRoot/controller-start.private.json"){throw 'Refuse duplicate native launch'}
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
using System.Text;
using Microsoft.Win32.SafeHandles;
public sealed class NFRecoveryIdentity {
 public long Pid;
 public string KernelCreationFileTime;
 public string Executable;
 public int SessionId;
 public bool Alive;
}
public static class NFRecoveryJob {
 [DllImport("kernel32.dll", SetLastError=true)]
 static extern bool QueryInformationJobObject(IntPtr job, int kind, IntPtr buffer, uint length, out uint returned);
 [DllImport("kernel32.dll", SetLastError=true)]
 static extern SafeProcessHandle OpenProcess(uint access, bool inherit, uint pid);
 [DllImport("kernel32.dll", SetLastError=true)]
 static extern uint GetProcessId(SafeProcessHandle process);
 [DllImport("kernel32.dll", SetLastError=true)]
 static extern bool GetProcessTimes(SafeProcessHandle process,out long creation,out long exit,out long kernel,out long user);
 [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
 static extern bool QueryFullProcessImageName(SafeProcessHandle process,uint flags,StringBuilder path,ref uint capacity);
 [DllImport("kernel32.dll", SetLastError=true)]
 static extern bool ProcessIdToSessionId(uint pid,out uint session);
 [DllImport("kernel32.dll", SetLastError=true)]
 static extern uint WaitForSingleObject(SafeProcessHandle process,uint milliseconds);
 [DllImport("kernel32.dll", SetLastError=true)]
 static extern bool TerminateProcess(SafeProcessHandle process,uint exitCode);
 static void NativeError() { throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error()); }
 static SafeProcessHandle Open(long pid,bool terminate) {
  if(pid<1 || pid>Int32.MaxValue) throw new InvalidOperationException("Positive process ID required");
  var handle=OpenProcess(0x1000u|0x100000u|(terminate?1u:0u),false,(uint)pid);
  if(handle.IsInvalid) { int error=Marshal.GetLastWin32Error();handle.Dispose();throw new System.ComponentModel.Win32Exception(error); }
  return handle;
 }
 public static SafeProcessHandle OpenMetadata(long pid) { return Open(pid,false); }
 public static SafeProcessHandle OpenCandidate(long pid) { return Open(pid,true); }
 public static bool IsAlive(SafeProcessHandle handle) {
  uint state=WaitForSingleObject(handle,0);
  if(state==258) return true;
  if(state==0) return false;
  NativeError();return false;
 }
 public static NFRecoveryIdentity Identity(SafeProcessHandle handle) {
  uint pid=GetProcessId(handle);
  if(pid==0) NativeError();
  var value=new NFRecoveryIdentity { Pid=pid,Alive=false };
  if(!IsAlive(handle)) return value;
  long birth,exit,kernel,user;
  if(!GetProcessTimes(handle,out birth,out exit,out kernel,out user)) NativeError();
  if(birth<=0) throw new InvalidOperationException("Positive kernel creation FILETIME required");
  uint capacity=32768;
  var image=new StringBuilder((int)capacity);
  if(!QueryFullProcessImageName(handle,0,image,ref capacity)) NativeError();
  uint session;
  if(!ProcessIdToSessionId(pid,out session)) NativeError();
  if(session>Int32.MaxValue) throw new InvalidOperationException("Bounded session ID required");
  value.KernelCreationFileTime=birth.ToString(System.Globalization.CultureInfo.InvariantCulture);
  value.Executable=image.ToString();value.SessionId=(int)session;
  value.Alive=IsAlive(handle);
  return value;
 }
 public static bool TerminateAndWait(SafeProcessHandle handle,int maximumWaitMilliseconds) {
  if(maximumWaitMilliseconds<1 || maximumWaitMilliseconds>10000) throw new InvalidOperationException("Bounded same-handle wait required");
  if(!IsAlive(handle)) throw new InvalidOperationException("Candidate already exited");
  if(!TerminateProcess(handle,137)) NativeError();
  uint state=WaitForSingleObject(handle,(uint)maximumWaitMilliseconds);
  if(state==0) return true;
  if(state==258) return false;
  NativeError();return false;
 }
 public static long[] CurrentSupervisorPids() {
  int capacity=256;
  IntPtr buffer=Marshal.AllocHGlobal(8+capacity*IntPtr.Size);
  try {
   uint returned;
   if(!QueryInformationJobObject(IntPtr.Zero,3,buffer,(uint)(8+capacity*IntPtr.Size),out returned))
    throw new System.ComponentModel.Win32Exception(Marshal.GetLastWin32Error());
   int assigned=Marshal.ReadInt32(buffer,0), count=Marshal.ReadInt32(buffer,4);
   if(assigned<1 || count!=assigned || count>capacity) throw new InvalidOperationException("Incomplete supervisor process list");
   long[] pids=new long[count];
   for(int i=0;i<count;i++) pids[i]=Marshal.ReadIntPtr(buffer,8+i*IntPtr.Size).ToInt64();
   return pids;
  } finally { Marshal.FreeHGlobal(buffer); }
 }
}
'@
$jobPids=[NFRecoveryJob]::CurrentSupervisorPids()
if($PID-notin$jobPids){throw 'Controller absent from inherited supervisor job list'}
if($OwnershipProbeOnly){
 if(Test-Path -LiteralPath "$EvidenceRoot/job-ownership-probe.private.json"){throw 'Preserve existing ownership probe'}
 [ordered]@{at=Get-Date -Format o;controllerPid=$PID;currentSupervisorJobPids=$jobPids;query='QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)';complete=$true;nativeLaunched=$false}|ConvertTo-Json|Set-Content "$EvidenceRoot/job-ownership-probe.private.json"
 exit 0
}
if([string]::IsNullOrWhiteSpace($OuterDeadlineUtc)){throw 'Conservative runner outer deadline required'}
$conservativeOuterDeadline=[DateTime]::ParseExact($OuterDeadlineUtc,'o',[Globalization.CultureInfo]::InvariantCulture,[Globalization.DateTimeStyles]::RoundtripKind)
$remainingOuter=($conservativeOuterDeadline-[DateTime]::UtcNow).TotalMilliseconds
if($conservativeOuterDeadline.Kind-ne[DateTimeKind]::Utc-or$remainingOuter-lt60000-or$remainingOuter-gt$SessionWatchdogMs){throw 'Bounded UTC runner admission required'}
function Assert-OwnedHookOrdinary([string]$Path) {
 $item=Get-Item -LiteralPath $Path -Force
 while($null-ne$item){
  if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'HOOK_REPARSE_PATH'}
  $item=if($item-is[IO.FileInfo]){$item.Directory}else{$item.Parent}
 }
}
function Read-OwnedHookJson([string]$Path,[long]$MaximumBytes) {
 Assert-OwnedHookOrdinary $Path
 $file=Get-Item -LiteralPath $Path -Force
 if($file-isnot[IO.FileInfo]-or$file.Length-lt1-or$file.Length-gt$MaximumBytes){throw 'HOOK_JSON_BYTE_BOUND'}
 return ([IO.File]::ReadAllText($Path,[Text.UTF8Encoding]::new($false,$true))|ConvertFrom-Json -DateKind String)
}
function Assert-OwnedHookFields($Value,[string[]]$Names) {
 if($Value-isnot[pscustomobject]-or(@($Value.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($Names|Sort-Object)-join',')){throw 'HOOK_CLOSED_FIELDS'}
}
function Assert-OwnedHookInteger($Value,[long]$Minimum=0) {
 if(($Value-isnot[int]-and$Value-isnot[long])-or$Value-lt$Minimum-or$Value-gt[int]::MaxValue){throw 'HOOK_INTEGER_REQUIRED'}
}
function Assert-OwnedHookHash($Value) {
 if($Value-isnot[string]-or$Value-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'HOOK_HASH_REQUIRED'}
}
function Assert-OwnedHookKernelBirth($Value) {
 if($Value-isnot[string]-or$Value-cnotmatch'^[1-9][0-9]{0,19}$'){throw 'HOOK_KERNEL_BIRTH_STRING'}
 $null=[UInt64]::Parse($Value,[Globalization.NumberStyles]::None,[Globalization.CultureInfo]::InvariantCulture)
}
function Test-OwnedHookPath([string]$Actual,[string]$Expected) {
 return [StringComparer]::OrdinalIgnoreCase.Equals([IO.Path]::GetFullPath($Actual),[IO.Path]::GetFullPath($Expected))
}
function Write-OwnedHookExclusive([string]$Path,$Value) {
 Assert-OwnedHookOrdinary ([IO.Path]::GetDirectoryName($Path))
 $bytes=[Text.UTF8Encoding]::new($false).GetBytes(($Value|ConvertTo-Json -Depth 10 -Compress))
 if($bytes.Length-gt65536){throw 'HOOK_RECEIPT_BYTE_BOUND'}
 $stream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
 try{$stream.Write($bytes,0,$bytes.Length)}finally{$stream.Dispose()}
}
function Assert-OwnedHookTime($State) {
 $now=[DateTime]::UtcNow
 foreach($key in @('ControllerStartedUtc','ControllerDeadlineUtc','OuterDeadlineUtc')){
  if($State.$key-isnot[DateTime]-or$State.$key.Kind-ne[DateTimeKind]::Utc){throw 'HOOK_UTC_DEADLINE_REQUIRED'}
 }
 if($now-lt$State.ControllerStartedUtc-or($State.ControllerDeadlineUtc-$State.ControllerStartedUtc).TotalMilliseconds-gt($State.SessionWatchdogMs-10000)-or($State.OuterDeadlineUtc-$State.ControllerStartedUtc).TotalMilliseconds-gt$State.SessionWatchdogMs){throw 'HOOK_ORIGINAL_DEADLINE_REQUIRED'}
 $remaining=[Math]::Min(($State.ControllerDeadlineUtc-$now).TotalMilliseconds,($State.OuterDeadlineUtc-$now).TotalMilliseconds)
 if($remaining-lt60000-or($remaining-10000)-lt30000){throw 'HOOK_REMAINING_BOUND'}
}
function Read-OwnedHookBindings($State,$Request) {
 foreach($path in @($State.EvidenceRoot,$State.Installation,$State.SourcePinsPath,$State.FixtureReceiptPath)){Assert-OwnedHookOrdinary $path}
 if([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($State.EvidenceRoot))-ine[IO.Path]::GetFullPath($binding.scenariosRoot)){throw 'HOOK_CALLER_ROOT'}
 $planPath=Join-Path $State.EvidenceRoot 'plan.json'
 $leasePath=Join-Path $State.EvidenceRoot 'native-lane-lease.private.json'
 $plan=Read-OwnedHookJson $planPath 65536
 $lease=Read-OwnedHookJson $leasePath 8192
 foreach($key in @('LeaseSHA256','PlanSHA256','FixtureSHA256','SourcePinsSHA256')){Assert-OwnedHookHash $State.$key}
 foreach($pair in @(@($leasePath,$State.LeaseSHA256),@($planPath,$State.PlanSHA256),@($State.FixtureReceiptPath,$State.FixtureSHA256),@($State.SourcePinsPath,$State.SourcePinsSHA256))){
  Assert-OwnedHookOrdinary $pair[0]
  if((Get-FileHash -LiteralPath $pair[0] -Algorithm SHA256).Hash-ine$pair[1]){throw 'HOOK_FROZEN_BINDING_CHANGED'}
 }
 Assert-OwnedHookInteger $lease.schemaVersion 1
 if($lease.schemaVersion-ne1-or$lease.scenarioId-isnot[string]-or$lease.scenarioId-cne$binding.fixtureId-or$lease.authorized-isnot[bool]-or!$lease.authorized-or$lease.phase-isnot[string]-or$lease.phase-cne'CAMPAIGN_BINDING_ONLY'-or$lease.driver-isnot[string]-or$lease.driver-cne$binding.driverRole){throw 'HOOK_LEASE_AUTHORITY'}
 if($plan.ownership.driver-isnot[string]-or$plan.ownership.driver-cne$lease.driver-or$plan.ownership.onlyGameInputDriver-isnot[bool]-or!$plan.ownership.onlyGameInputDriver){throw 'HOOK_PLAN_AUTHORITY'}
 foreach($spec in @($lease.ownedNodeKill,$plan.ownedNodeKill)){
  Assert-OwnedHookFields $spec @('case','caseNonce')
  if($spec.case-isnot[string]-or$spec.case-cne'KILL_ONE_OWNED_NODE'-or$spec.caseNonce-isnot[string]-or$spec.caseNonce-cnotmatch'^[a-f0-9]{32}$'){throw 'HOOK_EXPLICIT_OPT_IN'}
 }
 if($Request.caseNonce-isnot[string]-or$Request.caseNonce-cnotmatch'^[a-f0-9]{32}$'-or$lease.ownedNodeKill.caseNonce-cne$plan.ownedNodeKill.caseNonce-or$Request.caseNonce-cne$lease.ownedNodeKill.caseNonce){throw 'HOOK_CASE_NONCE'}
 foreach($pair in @(@('planSha256','PlanSHA256'),@('fixtureReceiptSha256','FixtureSHA256'),@('sourcePinsSha256','SourcePinsSHA256'))){
  Assert-OwnedHookHash $lease.($pair[0]);Assert-OwnedHookHash $Request.($pair[0])
  if($lease.($pair[0])-ine$State.($pair[1])-or$Request.($pair[0])-ine$State.($pair[1])){throw 'HOOK_REQUEST_BINDING'}
 }
 $sources=Read-OwnedHookJson $State.SourcePinsPath 262144
 $pins=@($sources.pins)
 if($pins.Count-lt1-or$pins.Count-gt256){throw 'HOOK_SOURCE_PIN_CAP'}
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
 $controllerFound=$false
 foreach($pin in $pins){
  Assert-OwnedHookFields $pin @('path','bytes','sha256')
  if($pin.path-isnot[string]-or!$seen.Add([IO.Path]::GetFullPath($pin.path))-or($pin.bytes-isnot[int]-and$pin.bytes-isnot[long])-or$pin.bytes-lt1-or$pin.bytes-gt67108864){throw 'HOOK_SOURCE_PIN_REQUIRED'}
  Assert-OwnedHookHash $pin.sha256;Assert-OwnedHookOrdinary $pin.path
  if((Get-Item -LiteralPath $pin.path).Length-ne$pin.bytes-or(Get-FileHash -LiteralPath $pin.path -Algorithm SHA256).Hash-ine$pin.sha256){throw 'HOOK_SOURCE_BYTES_CHANGED'}
  if(Test-OwnedHookPath $pin.path (Join-Path $PSScriptRoot 'native-controller.ps1')){$controllerFound=$true}
 }
 if(!$controllerFound){throw 'HOOK_CONTROLLER_SOURCE_CUSTODY'}
 return [pscustomobject]@{planSha256=$State.PlanSHA256;leaseSha256=$State.LeaseSHA256;fixtureReceiptSha256=$State.FixtureSHA256;sourcePinsSha256=$State.SourcePinsSHA256;caseNonce=$Request.caseNonce}
}
function Assert-OwnedHookEligible($State,$Request,$OwnedRows,$Identity=$null) {
 Assert-OwnedHookTime $State
 $expectedNode=Join-Path $State.Installation 'diagnostic/nf-ipc-node.exe'
 $expectedParent=Join-Path $State.Installation 'jre/bin/java.exe'
 foreach($pair in @(@($expectedNode,$State.NodeExecutableSHA256),@($expectedParent,$State.ParentExecutableSHA256))){
  Assert-OwnedHookHash $pair[1];Assert-OwnedHookOrdinary $pair[0]
  if((Get-FileHash -LiteralPath $pair[0] -Algorithm SHA256).Hash-ine$pair[1]){throw 'HOOK_EXECUTABLE_DISK_CUSTODY'}
 }
 foreach($key in @('nodePid','directGamePid','controllerPid')){Assert-OwnedHookInteger $Request.$key 1}
 Assert-OwnedHookInteger $Request.nodeSessionId
 foreach($key in @('nodeCreationTime','directGameCreationTime','controllerCreationTime','nodeExecutable')){
  if($Request.$key-isnot[string]-or[string]::IsNullOrWhiteSpace($Request.$key)-or$Request.$key.Length-gt4096){throw 'HOOK_IDENTITY_STRING'}
 }
 Assert-OwnedHookKernelBirth $Request.nodeKernelCreationFileTime
 if($Request.controllerPid-ne$PID-or$State.ControllerPid-ne$PID-or$Request.controllerCreationTime-cne$State.ControllerCreationTime-or!(Test-OwnedHookPath $Request.nodeExecutable $expectedNode)){throw 'HOOK_CONTROLLER_OR_EXECUTABLE'}
 $rows=@($OwnedRows)
 if($rows.Count-lt2-or$rows.Count-gt256){throw 'HOOK_OWNED_ROW_CAP'}
 $nodes=@($rows|Where-Object {$_.pid-eq$Request.nodePid})
 $parents=@($rows|Where-Object {$_.pid-eq$Request.directGamePid})
 if($nodes.Count-ne1-or$parents.Count-ne1-or$Request.nodePid-eq$Request.directGamePid){throw 'HOOK_UNIQUE_CURRENT_CHAIN'}
 $node=$nodes[0];$parent=$parents[0]
 foreach($row in @($node,$parent)){
  if($row.inSupervisorJob-isnot[bool]-or!$row.inSupervisorJob-or$row.sessionId-ne$Request.nodeSessionId){throw 'HOOK_OWNED_ROW_PROOF'}
  Assert-OwnedHookKernelBirth $row.kernelCreationFileTime
 }
 if($node.parentPid-ne$parent.pid-or$node.creationTime-cne$Request.nodeCreationTime-or$node.kernelCreationFileTime-cne$Request.nodeKernelCreationFileTime-or$parent.creationTime-cne$Request.directGameCreationTime-or!(Test-OwnedHookPath $node.executable $expectedNode)-or!(Test-OwnedHookPath $parent.executable $expectedParent)){throw 'HOOK_CURRENT_CHAIN_MISMATCH'}
 $job=[NFRecoveryJob]::CurrentSupervisorPids()
 foreach($pidValue in @($PID,$node.pid,$parent.pid)){if($pidValue-notin$job){throw 'HOOK_CURRENT_JOB_MEMBERSHIP'}}
 $current=@(Get-CimInstance Win32_Process -Filter "ProcessId=$($Request.nodePid)" -OperationTimeoutSec 10)
 $currentParent=@(Get-CimInstance Win32_Process -Filter "ProcessId=$($Request.directGamePid)" -OperationTimeoutSec 10)
 $currentController=@(Get-CimInstance Win32_Process -Filter "ProcessId=$PID" -OperationTimeoutSec 10)
 if($current.Count-ne1-or$currentParent.Count-ne1-or$currentController.Count-ne1){throw 'HOOK_CURRENT_CIM_REQUIRED'}
 $c=$current[0];$p=$currentParent[0]
 if($c.ParentProcessId-ne$p.ProcessId-or$c.CreationDate.ToString('o')-cne$Request.nodeCreationTime-or$p.CreationDate.ToString('o')-cne$Request.directGameCreationTime-or$currentController[0].CreationDate.ToString('o')-cne$State.ControllerCreationTime-or$c.SessionId-ne$Request.nodeSessionId-or$p.SessionId-ne$Request.nodeSessionId-or$c.SessionId-ne[Diagnostics.Process]::GetCurrentProcess().SessionId-or!(Test-OwnedHookPath $c.ExecutablePath $expectedNode)-or!(Test-OwnedHookPath $p.ExecutablePath $expectedParent)){throw 'HOOK_CURRENT_CIM_MISMATCH'}
 if($null-ne$Identity){
  if(!$Identity.Alive-or$Identity.Pid-ne$Request.nodePid-or$Identity.KernelCreationFileTime-cne$Request.nodeKernelCreationFileTime-or$Identity.SessionId-ne$Request.nodeSessionId-or!(Test-OwnedHookPath $Identity.Executable $expectedNode)){throw 'HOOK_HELD_KERNEL_IDENTITY'}
 }
 return ,$job
}
function Invoke-OwnedNodeKillRequest($State,$OwnedRows) {
 if($State.RequestConsumed-isnot[bool]){throw 'HOOK_CONSUMPTION_STATE_REQUIRED'}
 if($State.RequestConsumed){throw 'HOOK_ALREADY_CONSUMED'}
 $requestPath=Join-Path $State.EvidenceRoot 'kill-one-owned-node.request.private.json'
 if(!(Test-Path -LiteralPath $requestPath)){return [pscustomobject]@{status='NO_REQUEST';consumed=$false}}
 $State.RequestConsumed=$true
 $candidatePath=Join-Path $State.EvidenceRoot 'owned-node-kill-candidate.private.json'
 $terminalPath=Join-Path $State.EvidenceRoot 'kill-one-owned-node.terminal.private.json'
 $handle=$null;$request=$null;$requestHash=$null;$invoked=$false
 try{
  Assert-OwnedHookOrdinary $State.EvidenceRoot
  foreach($path in @($candidatePath,$terminalPath)){if(Test-Path -LiteralPath $path){throw 'HOOK_EXISTING_RECEIPT'}}
  $request=Read-OwnedHookJson $requestPath 8192
  $requestHash=(Get-FileHash -LiteralPath $requestPath -Algorithm SHA256).Hash
  Assert-OwnedHookFields $request @('schemaVersion','case','caseNonce','nodePid','nodeCreationTime','nodeKernelCreationFileTime','nodeExecutable','nodeSessionId','directGamePid','directGameCreationTime','controllerPid','controllerCreationTime','planSha256','fixtureReceiptSha256','sourcePinsSha256')
  Assert-OwnedHookInteger $request.schemaVersion 1
  if($request.schemaVersion-ne1-or$request.case-isnot[string]-or$request.case-cne'KILL_ONE_OWNED_NODE'){throw 'HOOK_REQUEST_CASE'}
  $bindings=Read-OwnedHookBindings $State $request
  $null=Assert-OwnedHookEligible $State $request $OwnedRows
  # This is the sole termination-capable handle; never reopen or retarget it.
  $handle=[NFRecoveryJob]::OpenCandidate($request.nodePid)
  $identity=[NFRecoveryJob]::Identity($handle)
  $job=Assert-OwnedHookEligible $State $request $OwnedRows $identity
  if(![NFRecoveryJob]::IsAlive($handle)){throw 'HOOK_CANDIDATE_EXITED'}
  $candidate=[ordered]@{schemaVersion=1;case='KILL_ONE_OWNED_NODE';caseNonce=$request.caseNonce;status='VALIDATED_SAME_HANDLE';atUtc=[DateTime]::UtcNow.ToString('o');nodePid=$identity.Pid;nodeKernelCreationFileTime=$identity.KernelCreationFileTime;nodeExecutable=$identity.Executable;nodeSessionId=$identity.SessionId;nodeCreationTime=$request.nodeCreationTime;directGamePid=$request.directGamePid;directGameCreationTime=$request.directGameCreationTime;controllerPid=$PID;controllerCreationTime=$State.ControllerCreationTime;requestSHA256=$requestHash;bindings=$bindings;currentImmediateJobPids=$job}
  Write-OwnedHookExclusive $candidatePath $candidate
  $candidateHash=(Get-FileHash -LiteralPath $candidatePath -Algorithm SHA256).Hash
  $null=Read-OwnedHookBindings $State $request
  Assert-OwnedHookOrdinary $requestPath;Assert-OwnedHookOrdinary $candidatePath
  if((Get-Item -LiteralPath $requestPath).Length-gt8192-or(Get-FileHash -LiteralPath $requestPath -Algorithm SHA256).Hash-cne$requestHash-or(Get-FileHash -LiteralPath $candidatePath -Algorithm SHA256).Hash-cne$candidateHash){throw 'HOOK_CLAIM_BYTES_CHANGED'}
  $identity=[NFRecoveryJob]::Identity($handle)
  $null=Assert-OwnedHookEligible $State $request $OwnedRows $identity
  Assert-OwnedHookTime $State
  $invoked=$true
  $exited=[NFRecoveryJob]::TerminateAndWait($handle,10000)
  if(!$exited){throw 'HOOK_SAME_HANDLE_WAIT_UNKNOWN'}
  $terminal=[ordered]@{schemaVersion=1;case='KILL_ONE_OWNED_NODE';caseNonce=$request.caseNonce;status='TERMINATED';nodePid=$request.nodePid;nodeKernelCreationFileTime=$request.nodeKernelCreationFileTime;requestSHA256=$requestHash;candidateSHA256=$candidateHash;sameHandleExitObserved=$true;terminationInvocationAttempted=$true;atUtc=[DateTime]::UtcNow.ToString('o');bindings=$bindings}
  Write-OwnedHookExclusive $terminalPath $terminal
  $State|Add-Member -NotePropertyName ConsumedRequestSHA256 -NotePropertyValue $requestHash -Force
  return [pscustomobject]$terminal
 }catch{
  $failure=$_.Exception.Message
  if(!(Test-Path -LiteralPath $terminalPath)){
   $record=[ordered]@{schemaVersion=1;case='KILL_ONE_OWNED_NODE';status=if($invoked){'UNKNOWN_TERMINATION'}else{'REFUSED'};reason=$failure.Substring(0,[Math]::Min(512,$failure.Length));requestSHA256=$requestHash;requestHashMissingMeans='NOT_READ_OR_NOT_ADMITTED_WITHIN_8192_BYTE_BOUND';terminationInvocationAttempted=$invoked;sameHandleExitObserved=$false;atUtc=[DateTime]::UtcNow.ToString('o')}
   Write-OwnedHookExclusive $terminalPath $record
  }
  throw
 }finally{if($null-ne$handle){$handle.Dispose()}}
}
function Current-Owned {
 $prefix=[IO.Path]::GetFullPath($installation).Replace('\','/')+'/'
 $rows=@(Get-CimInstance Win32_Process -OperationTimeoutSec 10|Where-Object {$_.ExecutablePath-and$_.ExecutablePath.Replace('\','/').StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)})
 $result=@()
 $currentJobPids=[NFRecoveryJob]::CurrentSupervisorPids()
 if($PID-notin$currentJobPids){throw 'Controller left inherited supervisor job'}
 foreach($row in $rows){
  $process=Get-Process -Id $row.ProcessId -ErrorAction SilentlyContinue
  if(!$process){continue}
  $handle=$null
  try{
   if([long]$row.ProcessId-notin$currentJobPids){throw 'Private installation process absent from this supervisor job'}
   $handle=[NFRecoveryJob]::OpenMetadata($row.ProcessId)
   $identity=[NFRecoveryJob]::Identity($handle)
   if(!$identity.Alive){continue}
   $confirmed=@(Get-CimInstance Win32_Process -Filter "ProcessId=$($row.ProcessId)" -OperationTimeoutSec 10)
   if($confirmed.Count-ne1){if(![NFRecoveryJob]::IsAlive($handle)){continue};throw 'Private process census identity unavailable'}
   if($confirmed[0].CreationDate.ToString('o')-cne$row.CreationDate.ToString('o')-or$confirmed[0].ParentProcessId-ne$row.ParentProcessId-or!(Test-OwnedHookPath $confirmed[0].ExecutablePath $identity.Executable)){throw 'Private process census identity changed'}
   if($identity.Pid-ne$row.ProcessId-or$identity.SessionId-ne[Diagnostics.Process]::GetCurrentProcess().SessionId-or$row.CreationDate.ToUniversalTime()-lt$script:started-or!(Test-OwnedHookPath $row.ExecutablePath $identity.Executable)){throw 'Private process identity outside owned launch interval/session'}
   $process.Refresh()
   $mainWindowHandle=$process.MainWindowHandle.ToInt64().ToString([Globalization.CultureInfo]::InvariantCulture)
   $mainWindowObservedAtUtc=[DateTime]::UtcNow.ToString('o')
   $cpu=$process.CPU;$memory=$process.WorkingSet64
   if(![NFRecoveryJob]::IsAlive($handle)){continue}
   $result+=[ordered]@{pid=[int]$row.ProcessId;parentPid=[int]$row.ParentProcessId;creationTime=$row.CreationDate.ToString('o');kernelCreationFileTime=$identity.KernelCreationFileTime;executable=$identity.Executable;sessionId=$identity.SessionId;inSupervisorJob=$true;supervisorJobPids=$currentJobPids;cpuSeconds=$cpu;workingSetBytes=$memory;mainWindowHandle=$mainWindowHandle;mainWindowObservedAtUtc=$mainWindowObservedAtUtc}
  }finally{if($null-ne$handle){$handle.Dispose()};$process.Dispose()}
 }
 return ,$result
}
$existing=@(Get-CimInstance Win32_Process|Where-Object {$_.ExecutablePath-and($_.ExecutablePath.Replace('\','/').StartsWith(([IO.Path]::GetFullPath($installation).Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase)-or$_.ExecutablePath.Replace('\','/').StartsWith(($binding.sourceInstallation.Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase))})
if($existing.Count-ne0){throw 'Game installation already has processes'}
if(![Environment]::UserInteractive){throw 'Signed-in interactive desktop required'}
$launchAdmission=& (Join-Path $PSScriptRoot 'protection-check.ps1') -BindingPath $BindingPath -BindingSha256 $BindingSha256 -Phase admit-launch -EvidenceRoot $EvidenceRoot
$script:started=[DateTime]::UtcNow
$controllerRow=@(Get-CimInstance Win32_Process -Filter "ProcessId=$PID" -OperationTimeoutSec 10)
if($controllerRow.Count-ne1){throw 'Actual controller birth required'}
$controllerHandle=[NFRecoveryJob]::OpenMetadata($PID)
try{$controllerIdentity=[NFRecoveryJob]::Identity($controllerHandle);if(!$controllerIdentity.Alive){throw 'Actual live controller required'}}finally{$controllerHandle.Dispose()}
$sourcePinsPath=Join-Path $binding.fixtureRoot 'source-pins.private.json'
$hookState=[pscustomobject]@{EvidenceRoot=$EvidenceRoot;Installation=$installation;SourcePinsPath=$sourcePinsPath;FixtureReceiptPath=$receiptPath;ControllerPid=$PID;ControllerCreationTime=$controllerRow[0].CreationDate.ToString('o');ControllerKernelCreationFileTime=$controllerIdentity.KernelCreationFileTime;SessionWatchdogMs=$SessionWatchdogMs;ControllerStartedUtc=$script:started;ControllerDeadlineUtc=$script:started.AddMilliseconds($SessionWatchdogMs-10000);OuterDeadlineUtc=$conservativeOuterDeadline;RequestConsumed=$false;LeaseSHA256=(Get-FileHash -LiteralPath "$EvidenceRoot/native-lane-lease.private.json" -Algorithm SHA256).Hash;PlanSHA256=(Get-FileHash -LiteralPath "$EvidenceRoot/plan.json" -Algorithm SHA256).Hash;FixtureSHA256=(Get-FileHash -LiteralPath $receiptPath -Algorithm SHA256).Hash;SourcePinsSHA256=(Get-FileHash -LiteralPath $sourcePinsPath -Algorithm SHA256).Hash;NodeExecutableSHA256=(Get-FileHash -LiteralPath "$installation/diagnostic/nf-ipc-node.exe" -Algorithm SHA256).Hash;ParentExecutableSHA256=(Get-FileHash -LiteralPath "$installation/jre/bin/java.exe" -Algorithm SHA256).Hash}
$env:NF_PERFORMANCE_TRACE_ID=[Guid]::NewGuid().ToString('N')
$env:NF_PERFORMANCE_OBSERVER='true'
$controllerStart=[ordered]@{at=Get-Date -Format o;controllerPid=$PID;controllerCreationTime=$hookState.ControllerCreationTime;controllerKernelCreationFileTime=$hookState.ControllerKernelCreationFileTime;sessionWatchdogMs=$SessionWatchdogMs;conservativeOuterDeadlineUtc=$hookState.OuterDeadlineUtc.ToString('o');controllerDeadlineUtc=$hookState.ControllerDeadlineUtc.ToString('o');outerDeadlineSource='RUNNER_BEFORE_PROJECT_PREPARATION';sessionId=[Diagnostics.Process]::GetCurrentProcess().SessionId;supervisorJobPids=$jobPids;traceId=$env:NF_PERFORMANCE_TRACE_ID;observer=$env:NF_PERFORMANCE_OBSERVER;installation=$installation;hardTimeoutMs=$SessionWatchdogMs;timeoutMeaning='SAFETY_WATCHDOG_NOT_PRODUCT_DEADLINE';fixtureReceiptSha256=$hookState.FixtureSHA256;sourcePinsSha256=$hookState.SourcePinsSHA256;rootLeaseSha256=$hookState.LeaseSHA256}
if($null-ne$launchAdmission){
 if($launchAdmission.rootLeaseSha256-ine$hookState.LeaseSHA256){throw 'Validated menu launch lease changed'}
 $controllerStart.launchGrant=$launchAdmission.launchGrant
}
$controllerStart|ConvertTo-Json -Depth 6|Set-Content "$EvidenceRoot/controller-start.private.json"
# Visible interactive launcher is the authorized native test surface.
$launcher=Start-Process -FilePath "$installation/starsector.exe" -WorkingDirectory $installation -WindowStyle Normal -PassThru
$samples=[Collections.Generic.List[object]]::new()
$seen=$false
while([DateTime]::UtcNow-lt$hookState.ControllerDeadlineUtc){
 if(Test-Path -LiteralPath "$EvidenceRoot/stop-owned-controller"){throw 'Driver requested owned cleanup; native success not established'}
 $rows=Current-Owned
 if($rows.Count){$seen=$true}
 $sample=[ordered]@{at=Get-Date -Format o;controllerPid=$PID;controllerCreationTime=$hookState.ControllerCreationTime;controllerKernelCreationFileTime=$hookState.ControllerKernelCreationFileTime;sessionWatchdogMs=$SessionWatchdogMs;conservativeOuterDeadlineUtc=$hookState.OuterDeadlineUtc.ToString('o');controllerDeadlineUtc=$hookState.ControllerDeadlineUtc.ToString('o');outerDeadlineSource='RUNNER_BEFORE_PROJECT_PREPARATION';launcherPid=$launcher.Id;launcherExited=$launcher.HasExited;processes=$rows}
 $samples.Add($sample)
 $null=Publish-OwnedSnapshot -Path "$EvidenceRoot/owned-processes-current.private.json" -Snapshot $sample
 $requestPath="$EvidenceRoot/kill-one-owned-node.request.private.json"
 if(!$hookState.RequestConsumed-and(Test-Path -LiteralPath $requestPath)){
  $null=Invoke-OwnedNodeKillRequest -State $hookState -OwnedRows $rows
 }elseif($hookState.RequestConsumed){
  Assert-OwnedHookOrdinary $requestPath
  if((Get-Item -LiteralPath $requestPath).Length-gt8192-or(Get-FileHash -LiteralPath $requestPath -Algorithm SHA256).Hash-cne$hookState.ConsumedRequestSHA256){throw 'Consumed one-shot request changed; no second dispatch'}
 }
 if($seen-and$launcher.HasExited-and$rows.Count-eq0){
  $samples|ConvertTo-Json -Depth 8|Set-Content "$EvidenceRoot/owned-process-samples.private.json"
  [ordered]@{at=Get-Date -Format o;observedProcesses=$true;privateProcessesAbsent=$true;normalQuitEvidence='REQUIRES_DRIVER_VERIFICATION';providerCleanup='PENDING_TERMINAL_SUPERVISOR_EVIDENCE';launcherExitCode=$launcher.ExitCode;remainingPrivateProcesses=0}|ConvertTo-Json|Set-Content "$EvidenceRoot/controller-terminal.private.json"
  exit 0
 }
 Start-Sleep -Milliseconds 1000
}
Write-OwnedHookExclusive "$EvidenceRoot/controller-watchdog.private.json" ([ordered]@{schemaVersion=1;status='INCOMPLETE_AUTOMATION_WATCHDOG';productOutcome='NOT_ESTABLISHED';sessionWatchdogMs=$SessionWatchdogMs;atUtc=[DateTime]::UtcNow.ToString('o')})
exit 2

