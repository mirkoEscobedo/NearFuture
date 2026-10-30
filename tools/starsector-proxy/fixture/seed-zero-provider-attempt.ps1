# Distinct historical custody only: no provider, input, clock or seed authority.
. (Join-Path $PSScriptRoot 'seed-zero-provider-terminal.ps1')
function Read-NativeZeroProviderFailedSeedAttempt($v,$Ledger,$LedgerPin) {
 function Need($Ok){if(!$Ok){throw 'Closed zero-provider controller-clock failed seed custody required'}}
 function Exact-Integer($Value,[long]$Expected){Assert-NativeSeedInteger $Value $Expected $Expected}
 function Strings($Object,[string[]]$Fields){Need ($Object-is[pscustomobject]);foreach($field in $Fields){Need ($Object.$field-is[string])}}
 function Same($A,$B){
  if($null-eq$A-or$null-eq$B){return ($null-eq$A-and$null-eq$B)}
  if($A-is[bool]-or$B-is[bool]){return ($A-is[bool]-and$B-is[bool]-and$A-eq$B)}
  if($A-is[string]-or$B-is[string]){return ($A-is[string]-and$B-is[string]-and$A-ceq$B)}
  if($A-is[array]-or$B-is[array]){if($A-isnot[array]-or$B-isnot[array]-or$A.Count-ne$B.Count){return $false};for($i=0;$i-lt$A.Count;$i++){if(!(Same $A[$i] $B[$i])){return $false}};return $true}
  if($A-is[pscustomobject]-or$B-is[pscustomobject]){if($A-isnot[pscustomobject]-or$B-isnot[pscustomobject]-or(@($A.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($B.PSObject.Properties.Name|Sort-Object)-join',')){return $false};foreach($k in $A.PSObject.Properties.Name){if(!(Same $A.$k $B.$k)){return $false}};return $true}
  if($A-is[ValueType]-or$B-is[ValueType]){return ($A-is[ValueType]-and$B-is[ValueType]-and$A-eq$B)}
  return $false
 }
 function Read-Ref($Pin,[string]$Expected){Assert-NativeSeedFields $Pin @('path','sha256');Strings $Pin @('path','sha256');Need ($Pin.path-ceq$Expected-and$Pin.sha256-cmatch'^[a-f0-9]{64}$');return Read-NativeZeroProviderJson $Expected $Pin.sha256}
 function Check-File($Pin,[long]$Maximum){
  Assert-NativeSeedFields $Pin @('path','sha256');Strings $Pin @('path','sha256');Assert-NativeSeedHash $Pin.sha256;Assert-NativeSeedOrdinary $Pin.path
  $item=Get-Item -LiteralPath $Pin.path -Force;Need (!$item.PSIsContainer-and$item.Length-le$Maximum-and(Get-FileHash -LiteralPath $Pin.path -Algorithm SHA256).Hash-ieq$Pin.sha256)
 }
 Assert-NativeSeedFields $v @('schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence')
 Exact-Integer $v.schemaVersion 1
 Need ($v.kind-ceq'ROOT_CLOSED_ZERO_PROVIDER_CONTROLLER_CLOCK_FAILED_SEED_ATTEMPT_HISTORY'-and$v.issuer-ceq'/root'-and$v.driver-ceq'/root/recovery_game_driver'-and$v.allOwnedHandlesTerminalReaped-is[bool]-and$v.allOwnedHandlesTerminalReaped)
 foreach($path in @($v.scenario,$v.sourceWorkspace)){Need ($path-is[string]-and[IO.Path]::IsPathFullyQualified($path)-and[IO.Path]::GetFullPath($path)-ceq$path)}
 $s=$v.scenario;Need ([IO.Path]::GetDirectoryName($s)-ceq(Join-Path $v.sourceWorkspace '.tmp/starsector-proxy/scenarios')-and$v.sourceHead-is[string]-and$v.sourceHead-cmatch'^[a-f0-9]{40}$')
 Assert-NativeSeedFields $v.actions @('actualGuard','currentRunContext','absoluteMaxReservedActions')
 $names=@{plan='plan.json';lease='native-lane-lease.private.json';controller='controller-start.private.json';owned='owned-processes-current.private.json';rootCall='root-call.actual.private.json';caller='launch-caller-terminal.actual.private.json';launch='launch-supervisor.private.json';protection='after-smoke-protection.private.json';pidAbsence='poststop-pid-absence.actual.private.json';preference='seed-creation-preference.private.json';supportPins='frozen-support-pins.private.json';source='B1-authorization.private.json';phasePlan='B1-plan.private.json';phaseLease='B1-lease.private.json';initialOwned='driver-B1-owned.private.json';predecessor='campaign-postlaunch.private.json';frozen='root-final-prelaunch-actual.private.json';freezeSupervisor='root-freeze-supervisor.actual.private.json';combined='combined-launch-stage.actual.private.json';ledger='guard-launch-history.private.json';preclockIndex='seed-graph-preclock-status.proposed.private.json'}
 Assert-NativeSeedFields $v.evidence @($names.Keys);$e=$v.evidence;$a=@{};foreach($k in $names.Keys){$a[$k]=Read-Ref $e.$k (Join-Path $s $names[$k])}
 $p=$a.plan;$l=$a.lease;$c=$a.controller;$owned=$a.owned;$rootCall=$a.rootCall;$source=$a.source;$bp=$a.phasePlan;$bl=$a.phaseLease;$initial=$a.initialOwned;$pre=$a.predecessor;$protect=$a.protection;$abs=$a.pidAbsence;$g=$a.ledger;$guard=$g.history;$ctx=$v.actions.currentRunContext
 Strings $v @('kind','issuer','driver','scenario','sourceWorkspace','sourceHead');Strings $p @('scenarioId');Strings $p.paths @('scenario');Strings $p.source @('qualifiedHead');Strings $p.ownership @('driver')
 Strings $l @('scenarioId','issuer','driver','phase','planSha256','supportPinsPath','supportPinsSha256');Strings $g @('kind','rootRecordKey','controllerSha256')
 Strings $c @('at','controllerCreationTime','controllerKernelCreationFileTime','conservativeOuterDeadlineUtc','controllerDeadlineUtc','installation','rootLeaseSha256')
 Strings $rootCall @('key');Strings $rootCall.value.args @('cmd','workdir');Strings $source @('scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256')
 Strings $pre @('kind');Strings $pre.rootClock @('recordKey','producerRecordKey');Strings $owned @('at','controllerCreationTime','controllerKernelCreationFileTime');Strings $initial @('at');Strings $initial.sourceSnapshot @('path','sha256');Strings $protect @('at','phase');Strings $abs @('atUtc')
 Assert-NativeSeedFields $g @('schemaVersion','kind','rootRecordKey','controllerSha256','history','pending','uncertain');Exact-Integer $g.schemaVersion 2
 Need ($g.kind-ceq'STARSECTOR_FINITE_LAUNCH_HISTORY'-and$null-eq$g.pending-and$g.uncertain-is[bool]-and!$g.uncertain-and(Same $g $Ledger)-and(Same $e.ledger $LedgerPin))
 Assert-NativeSeedFields $guard @('overallAttempts','uiAttempts','reservedActions','sentActions');foreach($k in $guard.PSObject.Properties.Name){Assert-NativeSeedInteger $guard.$k 0 9007199254740991}
 Need ($guard.sentActions-le$guard.reservedActions-and(Same $guard $p.historyBaseline)-and(Same $guard $v.actions.actualGuard))
 Assert-NativeSeedFields $ctx @('overallAttempts','uiAttempts');Exact-Integer $ctx.overallAttempts ($guard.overallAttempts+1);Exact-Integer $ctx.uiAttempts ($guard.uiAttempts+1)
 Assert-NativeSeedInteger $v.actions.absoluteMaxReservedActions ($guard.reservedActions+1) ($guard.reservedActions+8)
 Exact-Integer $p.absoluteMaxReservedActions $v.actions.absoluteMaxReservedActions;Exact-Integer $l.absoluteMaxReservedActions $p.absoluteMaxReservedActions
 Exact-Integer $p.campaignControl.schemaVersion 4;Exact-Integer $l.schemaVersion 1
 Need ($p.scenarioId-ceq[IO.Path]::GetFileName($s)-and$p.paths.scenario-ceq$s-and$p.source.qualifiedHead-ceq$v.sourceHead-and$p.ownership.driver-ceq$v.driver-and$p.ownership.onlyGameInputDriver-is[bool]-and$p.ownership.onlyGameInputDriver-and(Same $p.campaignControl ([pscustomobject]@{schemaVersion=4;profile='DISPOSABLE_SEED_CREATION'}))-and(Same $l.campaignControl $p.campaignControl)-and$l.scenarioId-ceq$p.scenarioId-and$l.authorized-is[bool]-and$l.authorized-and$l.issuer-ceq'/root'-and$l.driver-ceq$v.driver-and$l.phase-ceq'CAMPAIGN_BINDING_ONLY'-and$l.planSha256.ToLowerInvariant()-ceq$e.plan.sha256-and$c.rootLeaseSha256.ToLowerInvariant()-ceq$e.lease.sha256-and$g.controllerSha256-ceq$e.controller.sha256-and$rootCall.key-ceq$g.rootRecordKey-and$rootCall.value.args.workdir-ceq$v.sourceWorkspace-and$c.installation-ceq(Join-Path $s 'installation'))
 Need ((Same $p.source.supportPins $e.supportPins)-and$l.supportPinsPath-ceq$e.supportPins.path-and$l.supportPinsSha256.ToLowerInvariant()-ceq$e.supportPins.sha256)
 Assert-NativeSeedFields $a.supportPins @('pins');Need ($a.supportPins.pins-is[array]-and$a.supportPins.pins.Count-le1024)
 foreach($prop in $p.preclockHelpers.PSObject.Properties){$pin=$prop.Value;Check-File $pin 262144;Need ($pin.path-ceq(Join-Path $s $prop.Name))}
 Need (Same $p.historyAdmission $l.historyAdmission);$null=Read-Ref $p.historyAdmission (Join-Path $s 'root-history-baseline.admitted.private.json')
 $delegationPath=Join-Path $s 'root-postlaunch-mechanical-delegation.private.json';$null=Read-Ref ([pscustomobject]@{path=$delegationPath;sha256=$a.frozen.delegationSha256}) $delegationPath
 $loaded=$p.source.loadedFrontendWitness;Need ($loaded.path-is[string]-and[IO.Path]::IsPathFullyQualified($loaded.path)-and[IO.Path]::GetFullPath($loaded.path)-ceq$loaded.path);$null=Read-Ref $loaded $loaded.path
 $shells=@($a.supportPins.pins|Where-Object {$_.path-is[string]-and[IO.Path]::GetFileName($_.path).ToLowerInvariant()-ceq'pwsh.exe'});Need ($shells.Count-eq1);$shell=$shells[0]
 Need ([IO.Path]::IsPathFullyQualified($shell.path)-and[IO.Path]::GetFullPath($shell.path)-ceq$shell.path-and$shell.sha256-is[string]-and$shell.sha256-cmatch'^[a-fA-F0-9]{64}$');Check-File $shell 134217728
 $birth=[DateTimeOffset]::Parse($c.controllerCreationTime).ToUnixTimeMilliseconds();$published=[DateTimeOffset]::Parse($c.at).ToUnixTimeMilliseconds();$outer=[DateTimeOffset]::Parse($c.conservativeOuterDeadlineUtc).ToUnixTimeMilliseconds();$inner=[DateTimeOffset]::Parse($c.controllerDeadlineUtc).ToUnixTimeMilliseconds();$clock=$pre.rootClock
 Assert-NativeSeedInteger $rootCall.value.startMs 1 9007199254740991;Assert-NativeSeedInteger $p.callerAdmissionDeadlineMs 1 9007199254740991
 Exact-Integer $c.sessionWatchdogMs 480000;Exact-Integer $c.hardTimeoutMs 480000;Assert-NativeSeedInteger $c.controllerPid 1 9007199254740991;Assert-NativeSeedInteger $c.sessionId 0 9007199254740991
 Need ($p.callerAdmissionDeadlineMs-gt$published-and$rootCall.value.startMs-le$birth-and$birth-le$published-and$published-lt$outer-and$outer-lt$inner-and$outer-le($birth+480000)-and$c.controllerKernelCreationFileTime-is[string]-and$c.controllerKernelCreationFileTime-cmatch'^[1-9][0-9]*$')
 Exact-Integer $l.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs;Need ($p.callerAdmissionDeadlineMs-le($rootCall.value.startMs+480000))
 Exact-Integer $pre.schemaVersion 1
 Need ($pre.kind-ceq'FINITE_CAMPAIGN_LAUNCH_PREDECESSOR'-and(Same $pre.plan $e.plan)-and(Same $pre.lease $e.lease)-and(Same $pre.controller $e.controller)-and(Same $pre.rootCallRecord $rootCall)-and(Same $pre.historyBaseline $guard))
 Exact-Integer $pre.maxReservedActions $p.absoluteMaxReservedActions;Exact-Integer $clock.startMs $rootCall.value.startMs;Exact-Integer $clock.deadlineMs ($clock.startMs+480000);Exact-Integer $clock.sessionWatchdogMs 480000;Exact-Integer $clock.cleanupReserveMs 30000
 Exact-Integer $clock.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs;Exact-Integer $clock.originalCallerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs;Exact-Integer $clock.effectiveDeadlineMs ([Math]::Min([Math]::Min($clock.deadlineMs,$p.callerAdmissionDeadlineMs),[Math]::Min($outer,$inner)))
 Need ($clock.recordKey-ceq$rootCall.key-and$clock.producerRecordKey-ceq('controller-start:'+$e.controller.sha256))
 Need (Same $pre.producerWitness ([pscustomobject]@{producerRecordKey=$clock.producerRecordKey;controllerPid=$c.controllerPid;controllerCreationTime=$c.controllerCreationTime;controllerKernelCreationFileTime=$c.controllerKernelCreationFileTime;conservativeOuterDeadlineUtc=$c.conservativeOuterDeadlineUtc;controllerDeadlineUtc=$c.controllerDeadlineUtc;effectiveDeadlineMs=$clock.effectiveDeadlineMs;sessionWatchdogMs=480000}))
 Assert-NativeSeedFields $source @('schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256');Exact-Integer $source.schemaVersion 6
 Need ($source.scenario-ceq$s-and$source.batch-ceq'B1'-and$source.planPath-ceq$e.phasePlan.path-and$source.planSha256-ceq$e.phasePlan.sha256-and$source.leasePath-ceq$e.phaseLease.path-and$source.leaseSha256-ceq$e.phaseLease.sha256-and$source.ownedReceiptPath-ceq$e.initialOwned.path-and$source.ownedReceiptSha256-ceq$e.initialOwned.sha256)
 $campaign=$bp.campaign;Assert-NativeSeedFields $campaign @('flow','windowId','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions','launchPredecessor');Strings $campaign @('flow');Assert-NativeSeedInteger $campaign.windowId 1 9007199254740991
 Need ($campaign.flow-ceq'DISPOSABLE_SEED_CREATION'-and(Same $campaign.launchPredecessor $e.predecessor));foreach($k in @('rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions')){Need (Same $campaign.$k $pre.$k)}
 $expected=$p|ConvertTo-Json -Depth 100|ConvertFrom-Json -Depth 100 -DateKind String;$expected|Add-Member campaign $campaign;$expected.batches=@($p.batches|Where-Object {$_.id-ceq'B1'});Need (Same $bp $expected)
 $expected=$l|ConvertTo-Json -Depth 100|ConvertFrom-Json -Depth 100 -DateKind String;$expected.schemaVersion=6;$expected.phase='DISPOSABLE_SEED_CREATION';$expected.planSha256=$e.phasePlan.sha256;$expected|Add-Member campaign $campaign;Need (Same $bl $expected)
 Need ($bp.batches.Count-eq1-and$bp.batches[0].runID-is[string]-and$bp.batches[0].runID-cmatch'^[A-Za-z0-9_-]+$')
 Need (@(Get-ChildItem -LiteralPath (Join-Path $v.sourceWorkspace '.tmp/starsector-proxy') -Force|Where-Object {$_.Name-ieq$bp.batches[0].runID}).Count-eq0)
 Need ($owned.controllerPid-eq$c.controllerPid-and$owned.controllerCreationTime-ceq$c.controllerCreationTime-and$owned.controllerKernelCreationFileTime-ceq$c.controllerKernelCreationFileTime-and$owned.processes-is[array]-and$initial.processes-is[array]-and$initial.processes.Count-eq1-and$initial.sourceSnapshot.path-ceq$e.owned.path-and$initial.sourceSnapshot.sha256-is[string]-and$initial.sourceSnapshot.sha256-cmatch'^[a-f0-9]{64}$')
 $java=$initial.processes[0];$javaRows=@($owned.processes|Where-Object {$_.pid-eq$java.pid});Need ($javaRows.Count-eq1)
 foreach($row in @($java)+@($owned.processes)){Strings $row @('creationTime','kernelCreationFileTime','executable');Assert-NativeSeedInteger $row.pid 1 9007199254740991;Assert-NativeSeedInteger $row.parentPid 1 9007199254740991;Assert-NativeSeedInteger $row.sessionId 0 9007199254740991;Need ($row.supervisorJobPids-is[array]);foreach($id in $row.supervisorJobPids){Assert-NativeSeedInteger $id 1 9007199254740991}}
 Exact-Integer $owned.controllerPid $c.controllerPid;Need ($java.parentPid-ne$java.pid-and$java.supervisorJobPids-contains$java.parentPid-and$java.kernelCreationFileTime-cmatch'^[1-9][0-9]*$'-and($java.mainWindowHandle-is[string]-or$java.mainWindowHandle-is[int]-or$java.mainWindowHandle-is[long]))
 foreach($k in @('pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId','inSupervisorJob','supervisorJobPids','mainWindowHandle')){Need (Same $java.$k $javaRows[0].$k)}
 Need ($java.inSupervisorJob-is[bool]-and$java.inSupervisorJob-and$java.sessionId-eq$c.sessionId-and$java.executable-ceq(Join-Path $c.installation 'jre/bin/java.exe')-and[string]$java.mainWindowHandle-ceq[string]$campaign.windowId-and[DateTimeOffset]::Parse($java.creationTime).ToUnixTimeMilliseconds()-ge$published-and$c.supervisorJobPids-is[array]-and$c.supervisorJobPids-contains$c.controllerPid-and$java.supervisorJobPids-contains$java.pid)
 foreach($id in $c.supervisorJobPids){Need ($java.supervisorJobPids-contains$id)}
 $javaBirth=[DateTimeOffset]::Parse($java.creationTime).ToUnixTimeMilliseconds();$initialAt=[DateTimeOffset]::Parse($initial.at).ToUnixTimeMilliseconds();$currentAt=[DateTimeOffset]::Parse($owned.at).ToUnixTimeMilliseconds();$protectedAt=[DateTimeOffset]::Parse($protect.at).ToUnixTimeMilliseconds();$absentAt=[DateTimeOffset]::Parse($abs.atUtc).ToUnixTimeMilliseconds()
 Need ($javaBirth-le$initialAt-and$javaBirth-le$currentAt);foreach($time in @($initialAt,$currentAt)){Need ($time-ge$published-and$time-le$protectedAt-and$time-le$absentAt)}
 Need ((Same $p.seedCreation $l.seedCreation)-and(Same $p.seedCreation.preference $e.preference)-and(Same $a.preference (Get-NativeSeedPreferenceWitness $p $l $s)))
 foreach($pair in @(@('protectedOriginalFiles',182),@('protectedSaveFiles',178),@('liveGameProcesses',0),@('privateSaveEntries',0))){Exact-Integer $protect.($pair[0]) $pair[1]}
 Need ($protect.phase-ceq'after-smoke'-and(Same $protect.originalAutosave ([pscustomobject]@{enabled=$true;maximumSlots=3}))-and(Same $protect.sharedPreference $a.preference))
 foreach($k in @('allOriginalAndBackupHashesMatch','saveCensusUnchanged')){Need ($protect.$k-is[bool]-and$protect.$k)};foreach($k in @('originalWrites','privateAutosaveEnabled')){Need ($protect.$k-is[bool]-and!$protect.$k)}
 $ids=@(@($c.controllerPid)+@($c.supervisorJobPids)+@($owned.processes|ForEach-Object{$_.pid;$_.supervisorJobPids})|Sort-Object -Unique)
 Assert-NativeSeedFields $abs @('atUtc','expectedPids','absentPids','noLiveOwnedPids','providerStarted','nativeInputs','forcedOwnedCleanup','ordinaryExitAccepted')
 Need ($abs.expectedPids-is[array]);foreach($id in $abs.expectedPids){Assert-NativeSeedInteger $id 1 9007199254740991}
 Need ((Same @($abs.expectedPids|Sort-Object) $ids)-and(Same $abs.absentPids $abs.expectedPids));Exact-Integer $abs.nativeInputs 0
 foreach($k in @('noLiveOwnedPids','forcedOwnedCleanup')){Need ($abs.$k-is[bool]-and$abs.$k)};foreach($k in @('providerStarted','ordinaryExitAccepted')){Need ($abs.$k-is[bool]-and!$abs.$k)}
 Need ([DateTimeOffset]::Parse($abs.atUtc).ToUnixTimeMilliseconds()-ge$published-and[DateTimeOffset]::Parse($protect.at).ToUnixTimeMilliseconds()-ge$published)
 Need (@(Get-ChildItem -LiteralPath $s|Where-Object {$_.Name-cmatch'^(guard-(B1|E1|E2)-consumed|capture-|call-|geometry-|owned-provider|guarded-http|supervision-(B1|E1|E2))'}).Count-eq0)
 Assert-NativeZeroProviderTerminal $v $a $e $shell.path
 return $v.actions
}
