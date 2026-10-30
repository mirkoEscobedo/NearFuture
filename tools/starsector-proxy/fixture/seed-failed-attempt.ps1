# Historical closed failure custody only; no process, input, clock or seed authority.
. (Join-Path $PSScriptRoot 'seed-known-play-attempt.ps1')
. (Join-Path $PSScriptRoot 'seed-wrong-scene-attempt.ps1')
. (Join-Path $PSScriptRoot 'seed-capture-linkage-attempt.ps1')
. (Join-Path $PSScriptRoot 'seed-caller-date-attempt.ps1')
. (Join-Path $PSScriptRoot 'seed-zero-provider-terminal.ps1')
. (Join-Path $PSScriptRoot 'seed-snapshot-busy-attempt.ps1')
function Read-NativeFailedSeedAttempt($Receipt,$PreviousLedger,$LedgerPin) {
 return Read-NativeSeedFailedAttemptCore $Receipt $PreviousLedger $LedgerPin 'ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY'
}
function Read-NativeKnownPlayFailedSeedAttempt($Receipt,$PreviousLedger,$LedgerPin) {
 return Read-NativeSeedFailedAttemptCore $Receipt $PreviousLedger $LedgerPin 'ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY'
}
function Read-NativeCaptureStopFailedSeedAttempt($Receipt,$PreviousLedger,$LedgerPin) {
 return Read-NativeSeedFailedAttemptCore $Receipt $PreviousLedger $LedgerPin 'ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY'
}
function Read-NativeWrongSceneFailedSeedAttempt($Receipt,$PreviousLedger,$LedgerPin) {
 return Read-NativeSeedFailedAttemptCore $Receipt $PreviousLedger $LedgerPin 'ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY'
}
function Read-NativeCaptureLinkageFailedSeedAttempt($Receipt,$PreviousLedger,$LedgerPin) {
 return Read-NativeSeedFailedAttemptCore $Receipt $PreviousLedger $LedgerPin 'ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY'
}
function Read-NativeCallerDateFailedSeedAttempt($Receipt,$PreviousLedger,$LedgerPin) {
 return Read-NativeSeedFailedAttemptCore $Receipt $PreviousLedger $LedgerPin 'ROOT_CLOSED_PRE_INPUT_CALLER_DATE_FAILED_SEED_ATTEMPT_HISTORY'
}
function Read-NativeSeedFailedAttemptCore($Receipt,$PreviousLedger,$LedgerPin,[string]$ExpectedKind) {
 function Need($Ok){if(!$Ok){throw 'Closed no-input failed seed attempt custody required'}}
 function Exact-Integer($Value,[long]$Expected){Assert-NativeSeedInteger $Value $Expected $Expected}
 function Same($A,$B){
  if($null-eq$A-or$null-eq$B){return ($null-eq$A-and$null-eq$B)}
  # Provider-produced strings can carry a PSObject wrapper (for example Join-Path).
  # Compare primitive values before inspecting object properties.
  if($A-is[bool]-or$B-is[bool]){return ($A-is[bool]-and$B-is[bool]-and$A-eq$B)}
  if($A-is[string]-or$B-is[string]){return ($A-is[string]-and$B-is[string]-and$A-ceq$B)}
  if($A-is[array]-or$B-is[array]){if($A-isnot[array]-or$B-isnot[array]-or$A.Count-ne$B.Count){return $false};for($i=0;$i-lt$A.Count;$i++){if(!(Same $A[$i] $B[$i])){return $false}};return $true}
  if($A-is[pscustomobject]-or$B-is[pscustomobject]){if($A-isnot[pscustomobject]-or$B-isnot[pscustomobject]-or(@($A.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($B.PSObject.Properties.Name|Sort-Object)-join',')){return $false};foreach($k in $A.PSObject.Properties.Name){if(!(Same $A.$k $B.$k)){return $false}};return $true}
  return ($A-eq$B)
 }
 function Read-Ref($Pin,[string]$Expected,[long]$Maximum=262144){
  Assert-NativeSeedFields $Pin @('path','sha256');Need ($Pin.path-is[string]-and$Pin.path-ceq$Expected-and$Pin.sha256-is[string]-and$Pin.sha256-cmatch'^[a-f0-9]{64}$')
  if($callerDate-or$observedStop){Assert-NativeSeedOrdinary $Expected;$bytes=[IO.File]::ReadAllBytes($Expected);Need ($bytes.Length-ge2-and$bytes.Length-le$Maximum-and[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()-ceq$Pin.sha256);return ConvertFrom-ZeroProviderObject ([Text.UTF8Encoding]::new($false,$true).GetString($bytes))}
  return (Read-NativeModernSeedJson $Expected $Pin.sha256 $Maximum)
 }
 function Critical($Step){return [pscustomobject]@{command=$Step.command;args=$Step.args;cwd=$Step.cwd;processOwnership=$Step.processOwnership;status=$Step.status;timedOut=$Step.timedOut}}
 $callerDate=$ExpectedKind-ceq'ROOT_CLOSED_PRE_INPUT_CALLER_DATE_FAILED_SEED_ATTEMPT_HISTORY'
 $observedStop=($ExpectedKind-ceq'ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY'-and$Receipt.scenario-is[string]-and$Receipt.evidence.observation.path-ceq(Join-Path $Receipt.scenario 'driver-stop-observation.private.json'))
 $snapshotBusy=$false
 if($observedStop){$rawStop=Read-Ref $Receipt.evidence.observation (Join-Path $Receipt.scenario 'driver-stop-observation.private.json');$snapshotBusy=($rawStop.kind-is[string]-and$rawStop.kind-ceq'ACTUAL_S12_DRIVER_PRE_INPUT_POINTCHECK_EBUSY_STOP')}
 $observedFocus=$observedStop-and!$snapshotBusy
 $linkage=$ExpectedKind-ceq'ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY';$v=$Receipt;$driver='/root/recovery_game_driver';$captureStop=$ExpectedKind-ceq'ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY';$wrongScene=$ExpectedKind-ceq'ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY';$known=($ExpectedKind-ceq'ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY'-or$captureStop-or$wrongScene)
 Assert-NativeSeedFields $v @('schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence')
 Assert-NativeSeedInteger $v.schemaVersion 1 1
 Need ($ExpectedKind-cin@('ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_PRE_INPUT_CALLER_DATE_FAILED_SEED_ATTEMPT_HISTORY')-and$v.kind-is[string]-and$v.kind-ceq$ExpectedKind-and$v.issuer-is[string]-and$v.issuer-ceq'/root'-and$v.driver-is[string]-and$v.driver-ceq$driver-and$v.allOwnedHandlesTerminalReaped-is[bool]-and$v.allOwnedHandlesTerminalReaped)
 foreach($path in @($v.scenario,$v.sourceWorkspace)){Need ($path-is[string]-and[IO.Path]::IsPathFullyQualified($path)-and[IO.Path]::GetFullPath($path)-ceq$path)}
 $scenario=$v.scenario;Need ([IO.Path]::GetDirectoryName($scenario)-ceq(Join-Path $v.sourceWorkspace '.tmp/starsector-proxy/scenarios')-and$v.sourceHead-is[string]-and$v.sourceHead-cmatch'^[a-f0-9]{40}$')
 Assert-NativeSeedFields $v.actions @('actualGuard','currentRunContext','absoluteMaxReservedActions')
 Assert-NativeSeedFields $v.actions.currentRunContext @('overallAttempts','uiAttempts')
 $names=@{result='driver-flow-result.private.json';ledger='guard-launch-history.private.json';plan='plan.json';lease='native-lane-lease.private.json';controller='controller-start.private.json';owned='owned-processes-current.private.json';rootCall='root-call.actual.private.json';caller='launch-caller-terminal.actual.private.json';launch='launch-supervisor.private.json';protection='after-smoke-protection.private.json';pidAbsence='owned-terminal-pid-absence.actual.private.json';preference='seed-creation-preference.private.json';source='B1-authorization.private.json';claim='guard-B1-consumed.private.json';observation='driver-observation-stop.private.json'}
 $names.supportPins='frozen-support-pins.private.json'
 if($observedFocus){$names.observation='driver-stop-observation.private.json'}
 $providerNames=@{providerReady='guarded-http-ready.private.json';providerTerminal='owned-provider-terminal.private.json';providerSpool='owned-provider.stdout.private.txt';providerStderr='owned-provider.stderr.private.txt'}
 $secondProvider=@()
 if($wrongScene){$names.e1Source='E1-authorization.private.json';$names.e1Claim='guard-E1-consumed.private.json';$names.e1Plan='E1-plan.private.json';$names.e1Lease='E1-lease.private.json';$names.e1Owned='driver-E1-owned.private.json';$names.b1Menu='B1-menu-inspected.private.json';$secondProvider=@('e1ProviderReady','e1ProviderTerminal','e1ProviderSpool','e1ProviderStderr')}
 if($linkage){$names.metadataCaller='driver-metadata-calls.actual.private.json';$secondProvider=@('helper')}
 if($callerDate){$more=Get-NativeCallerDateNames;foreach($key in $more.Keys){$names[$key]=$more[$key]};$secondProvider=@('helper','geometryRunner')+@(Get-NativeCallerDateProviderNames).Keys}
 if($snapshotBusy){$more=Get-NativeSnapshotBusyNames;foreach($key in $more.Keys){$names[$key]=$more[$key]};$secondProvider=@('helper','geometryRunner')+@(Get-NativeSnapshotBusyProviderNames).Keys}
 Assert-NativeSeedFields $v.evidence (@($names.Keys)+@($providerNames.Keys)+$secondProvider)
 $e=$v.evidence;$a=@{};foreach($key in $names.Keys){$a[$key]=Read-Ref $e.$key (Join-Path $scenario $names[$key])}
 if($linkage){
  $packetBytes=[IO.File]::ReadAllBytes($e.metadataCaller.path);Need ($packetBytes.Length-le262144-and[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($packetBytes)).ToLowerInvariant()-ceq$e.metadataCaller.sha256)
  $a.metadataCaller=[Text.UTF8Encoding]::new($false,$true).GetString($packetBytes)|ConvertFrom-Json -Depth 100 -DateKind String -NoEnumerate
 }
 $r=$a.result;$l=$a.ledger;$p=$a.plan;$lease=$a.lease;$c=$a.controller;$owned=$a.owned;$rootCall=$a.rootCall;$caller=$a.caller;$launch=$a.launch;$protection=$a.protection;$preference=$a.preference;$source=$a.source;$claim=$a.claim;$o=$a.observation
 if($observedFocus){$o=Get-NativeObservedFocusView $v $a}
 Exact-Integer $r.schemaVersion 1;Exact-Integer $l.schemaVersion 2;Exact-Integer $p.campaignControl.schemaVersion 4;Exact-Integer $lease.campaignControl.schemaVersion 4
 Exact-Integer $source.schemaVersion 6;Exact-Integer $claim.binding.schemaVersion 6
 Need ((Same $e.ledger $LedgerPin)-and(Same $l $PreviousLedger)-and$l.schemaVersion-eq2-and$l.kind-ceq'STARSECTOR_FINITE_LAUNCH_HISTORY'-and($known-or$null-eq$l.pending)-and$l.uncertain-is[bool]-and!$l.uncertain-and(Same $l.history $v.actions.actualGuard)-and($known-or(Same $l.history $p.historyBaseline)))
 Need ($p.scenarioId-ceq[IO.Path]::GetFileName($scenario)-and$p.paths.scenario-ceq$scenario-and$p.source.qualifiedHead-ceq$v.sourceHead-and$p.campaignControl.profile-ceq'DISPOSABLE_SEED_CREATION'-and$p.campaignControl.schemaVersion-eq4-and$p.ownership.driver-ceq$driver-and$p.ownership.onlyGameInputDriver-is[bool]-and$p.ownership.onlyGameInputDriver)
 Need ($lease.planSha256.ToLowerInvariant()-ceq$e.plan.sha256-and$lease.scenarioId-ceq$p.scenarioId-and$lease.driver-ceq$driver-and$lease.authorized-is[bool]-and$lease.authorized-and$lease.issuer-ceq'/root'-and$lease.campaignControl.profile-ceq$p.campaignControl.profile-and$lease.campaignControl.schemaVersion-eq4-and$c.rootLeaseSha256.ToLowerInvariant()-ceq$e.lease.sha256-and$l.controllerSha256-ceq$e.controller.sha256)
 Need ($rootCall.key-ceq$l.rootRecordKey-and$rootCall.value.args.workdir-ceq$v.sourceWorkspace);Assert-NativeSeedInteger $rootCall.value.raw.session_id 1 9007199254740991
 $inherited=if($known){$p.historyBaseline}else{$l.history};$category=if($linkage){'PRIVATE_CAPTURE_LINKAGE_REFUSAL_BEFORE_INPUT'}elseif($wrongScene){'POSTCONDITION_FAILED'}elseif($captureStop){'CAPTURE_UNCERTAIN'}elseif($known){'PRIVATE_FRAME_CUSTODY_REFUSAL_AFTER_KNOWN_PLAY'}else{'FOCUS_BLOCKED'}
 if($callerDate){$category='CALLER_METADATA_SERIALIZATION_FAILED'}
 if($snapshotBusy){$category='PRIVATE_POINT_CHECK_LIVE_SNAPSHOT_READ_EBUSY'}
 Need (($callerDate-or((Same $r.actualGuardHistory $l.history)-and(Same $r.inheritedGuardBaseline $inherited)-and(Same $r.actualRunContext $v.actions.currentRunContext)))-and$r.schemaVersion-eq1-and$r.kind-ceq'ACTUAL_FINITE_DRIVER_FLOW_RESULT'-and$r.status-ceq'CLOSED_FAIL'-and$r.verdict-ceq'FAIL'-and$r.failureCategory-ceq$category-and$r.scenarioId-ceq$p.scenarioId-and$r.driver-ceq$driver-and$r.sourceHead-ceq$v.sourceHead)
 if(!$callerDate){
 $expectedCalls=if($snapshotBusy){4}else{5}
 Need ($known-or$linkage-or((Same $r.actions ([pscustomobject]@{reservedThisRun=0;sentThisRun=0;clicks=0;activations=0}))-and(Same $r.providers ([pscustomobject]@{started=1;B1='TERMINAL_PASS_REAPED';E1='NOT_STARTED';E2='NOT_STARTED';calls=$expectedCalls}))))
 Assert-NativeSeedInteger $r.absoluteMaxReservedActions ($l.history.reservedActions+1) ($l.history.reservedActions+8)
 Need ($r.absoluteMaxReservedActions-eq$p.absoluteMaxReservedActions-and$r.absoluteMaxReservedActions-eq$lease.absoluteMaxReservedActions-and$r.absoluteMaxReservedActions-eq$v.actions.absoluteMaxReservedActions-and$r.historicalUnsentReservations-eq($l.history.reservedActions-$l.history.sentActions))
 $aliases=@{observation='observation';callerTerminal='caller';launchSupervisor='launch';protection='protection';pidAbsence='pidAbsence';ledger='ledger';plan='plan';lease='lease'}
 foreach($key in $aliases.Keys){Need (Same $r.evidence.$key $e.($aliases[$key]))}
 }
 $provider=if($wrongScene){$claim.binding}else{$l.provider}
 Need ($provider.batch-ceq'B1'-and$provider.runID-is[string]-and$provider.runID-cmatch'^[A-Za-z0-9_-]+$'-and$source.schemaVersion-eq6-and$source.scenario-ceq$scenario-and$source.batch-ceq'B1'-and$provider.sourceSha256-ceq$e.source.sha256-and$l.b1ClaimSha256-ceq$e.claim.sha256-and$claim.sourcePath-ceq$e.source.path-and$claim.sourceSha256-ceq$e.source.sha256)
 Need ($claim.binding.schemaVersion-eq6-and$claim.binding.batch-ceq'B1'-and$claim.binding.scenario-ceq$scenario-and$claim.binding.workspaceRoot-ceq$v.sourceWorkspace-and$claim.binding.runID-ceq$provider.runID-and$claim.binding.sourceSha256-ceq$e.source.sha256-and$claim.binding.originalPlanSha256-ceq$e.plan.sha256-and$claim.binding.controllerSha256-ceq$e.controller.sha256)
 $output=Join-Path $v.sourceWorkspace ('.tmp/starsector-proxy/'+$provider.runID)
 if($callerDate){$more=Get-NativeCallerDateProviderNames;foreach($key in $more.Keys){$a[$key]=Read-Ref $e.$key (Join-Path $output $more[$key]) 33554432}}
 if($snapshotBusy){$more=Get-NativeSnapshotBusyProviderNames;foreach($key in $more.Keys){$a[$key]=Read-Ref $e.$key (Join-Path $output $more[$key]) 33554432}}
 foreach($key in $providerNames.Keys){
  $path=Join-Path $output $providerNames[$key]
  if($key-ceq'providerStderr'){Assert-NativeSeedFields $e.$key @('path','sha256');Need ($e.$key.path-ceq$path);Assert-NativeSeedOrdinary $path;Need ((Get-Item -LiteralPath $path).Length-eq0-and(Get-FileHash -LiteralPath $path).Hash-ieq$e.$key.sha256)}else{$a[$key]=Read-Ref $e.$key $path}
 }
 $ready=$a.providerReady;$t=$a.providerTerminal;$spool=$a.providerSpool;$step=$t.steps[0]
 if($callerDate-or$snapshotBusy){Assert-NativeCallerDateIdentityShapes $c $ready;Assert-NativeCallerDateProviderShape $t $spool $launch}
 Exact-Integer $ready.schemaVersion 1;Exact-Integer $ready.startupTimeoutMs 20000;Exact-Integer $spool.schemaVersion 1
 Exact-Integer $t.verifierStatus 0;Exact-Integer $step.status 0
 Assert-NativeSeedFields $a.supportPins @('pins')
 $supportSame=if($callerDate){$aliased=Read-Ref $p.source.supportPins (Join-Path $scenario 'source-pins.private.json');(Same $aliased $a.supportPins)-and[Convert]::ToBase64String([IO.File]::ReadAllBytes($p.source.supportPins.path))-ceq[Convert]::ToBase64String([IO.File]::ReadAllBytes($e.supportPins.path))}else{Same $p.source.supportPins $e.supportPins}
 Need ($a.supportPins.pins-is[array]-and$a.supportPins.pins.Count-le1024-and$supportSame-and$lease.supportPinsPath-ceq$e.supportPins.path-and$lease.supportPinsSha256.ToLowerInvariant()-ceq$e.supportPins.sha256)
 function Historical-Executable([string]$Name){
  $pins=@($a.supportPins.pins|Where-Object{$_.path-is[string]-and[IO.Path]::GetFileName($_.path).ToLowerInvariant()-ceq$Name});Need ($pins.Count-eq1);$pin=$pins[0]
  Assert-NativeSeedFields $pin @('path','sha256');Assert-NativeSeedHash $pin.sha256;Need ([IO.Path]::IsPathFullyQualified($pin.path)-and[IO.Path]::GetFullPath($pin.path)-ceq$pin.path)
  Assert-NativeSeedOrdinary $pin.path;Need ((Get-Item -LiteralPath $pin.path).Length-le134217728-and(Get-FileHash -LiteralPath $pin.path).Hash-ieq$pin.sha256);return $pin.path
 }
 $node=Historical-Executable 'node.exe';$powershell=Historical-Executable 'pwsh.exe'
 foreach($key in @('terminalObserved','terminalBoundToChild')){Need ($t.$key-is[bool]-and$t.$key)}
 foreach($key in @('cleanupUnknown','closeFailed','overflow','spawnFailed')){Need ($t.$key-is[bool]-and!$t.$key)}
 Need ($t.verifierStatus-eq0-and$null-eq$t.signal-and$t.verdict-ceq'PASS')
 Need ($t.terminalReceipt-ceq$e.providerTerminal.path-and$t.steps.Count-eq1)
 Need ($step.status-eq0-and$step.timedOut-is[bool]-and!$step.timedOut)
 Need ($step.command-ceq$node-and$step.processOwnership-ceq'windows-job-object'-and$step.cwd-ceq'.')
 Need (Same $step.args @((Join-Path $v.sourceWorkspace 'tools/starsector-proxy/guarded-http-child.mjs')))
 Need ($spool.schemaVersion-eq1-and$spool.command-ceq'verify'-and$spool.ok-is[bool]-and$spool.ok-and$spool.result.verdict-ceq'PASS'-and$spool.result.steps.Count-eq1-and(Same (Critical $spool.result.steps[0]) (Critical $step)))
 Need ($ready.schemaVersion-eq1-and$ready.runID-ceq$provider.runID-and$ready.batch-ceq'B1'-and$ready.sourceSha256-ceq$e.source.sha256-and$ready.startupTimeoutMs-eq20000-and$ready.consentMode-ceq'finite-explicit-consent'-and$ready.node.executable-ceq$node-and$ready.helper.executable-ceq$powershell-and$ready.node.parentPid-eq$t.verifierPid-and$ready.helper.parentPid-eq$ready.node.pid-and$ready.node.pid-ne$ready.helper.pid-and$ready.node.sessionId-eq$c.sessionId-and$ready.helper.sessionId-eq$c.sessionId-and$ready.supervisorJobPids-contains$ready.node.pid-and$ready.supervisorJobPids-contains$ready.helper.pid-and$ready.query-ceq'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)'-and[DateTimeOffset]::Parse($t.at)-ge[DateTimeOffset]::Parse($ready.at))
 Assert-NativeSeedFields $caller @('schemaVersion','kind','session','launchRaw','launchPolls','actual','terminalReaped')
 Exact-Integer $caller.schemaVersion 1
 Need ($caller.schemaVersion-eq1-and$caller.kind-ceq'ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL'-and$caller.session-eq$rootCall.value.raw.session_id-and$caller.terminalReaped-is[bool]-and$caller.terminalReaped-and(Same $caller.launchRaw $rootCall.value.raw)-and$caller.launchPolls-is[array]-and$caller.launchPolls.Count-ge1-and$caller.launchPolls.Count-le256)
 $chain=@($caller.launchRaw)+@($caller.launchPolls);$bytes=0;$text=''
 for($i=0;$i-lt$chain.Count;$i++){
  $raw=$chain[$i];Need ($raw.output-is[string]-and($raw.wall_time_seconds-is[int]-or$raw.wall_time_seconds-is[long]-or$raw.wall_time_seconds-is[double]-or$raw.wall_time_seconds-is[decimal])-and[double]::IsFinite($raw.wall_time_seconds)-and$raw.wall_time_seconds-ge0)
  $n=[Text.Encoding]::UTF8.GetByteCount($raw.output);$bytes+=$n;Need ($n-le65536-and$bytes-le131072);$text+=$raw.output
  if($i-lt$chain.Count-1){Need ($raw.session_id-eq$caller.session-and$null-eq$raw.exit_code)}else{Exact-Integer $raw.exit_code 1;Need (@($raw.PSObject.Properties.Name)-cnotcontains'session_id'-and(Same $raw $caller.actual))}
 }
 $returned=if($callerDate-or$snapshotBusy){ConvertFrom-ZeroProviderObject $text}else{$text|ConvertFrom-Json -DateKind String};$ls=$launch.result.steps[0];$returnedStep=if($returned.steps-is[array]){$returned.steps[0]}else{$returned.steps}
 if($callerDate){Need ($returned.verdict-is[string])}
 Exact-Integer $launch.schemaVersion 1;Exact-Integer $ls.status 1
 $selected=[pscustomobject]@{status=$ls.status;timedOut=$ls.timedOut;processOwnership=$ls.processOwnership;stdout=$ls.stdout;stderr=$ls.stderr}
 Need ($launch.schemaVersion-eq1-and$launch.command-ceq'verify'-and$launch.ok-is[bool]-and$launch.ok-and$launch.result.verdict-ceq'FAIL'-and$launch.result.steps.Count-eq1-and$ls.status-eq1-and$ls.timedOut-is[bool]-and!$ls.timedOut-and$ls.processOwnership-ceq'windows-job-object'-and$ls.stderr-is[string]-and$ls.stderr.Contains('Driver requested owned cleanup; native success not established')-and$returned.verdict-ceq'FAIL'-and($returned.steps-isnot[array]-or$returned.steps.Count-eq1)-and(Same $returnedStep $selected))
 Need ((Same $p.seedCreation.preference $e.preference)-and(Same $p.seedCreation $lease.seedCreation)-and(Same $preference (Get-NativeSeedPreferenceWitness $p $lease $scenario)))
 Exact-Integer $protection.privateSaveEntries 0;Exact-Integer $protection.liveGameProcesses 0
 Need ($protection.phase-ceq'after-smoke'-and$protection.protectedOriginalFiles-eq182-and$protection.protectedSaveFiles-eq178-and$protection.privateSaveEntries-eq0-and$protection.liveGameProcesses-eq0-and$protection.originalAutosave.maximumSlots-eq3-and(Same $protection.sharedPreference $preference))
 foreach($key in @('allOriginalAndBackupHashesMatch','saveCensusUnchanged')){Need ($protection.$key-is[bool]-and$protection.$key)}
 foreach($key in @('originalWrites','privateAutosaveEnabled')){Need ($protection.$key-is[bool]-and!$protection.$key)}
 Need ($protection.originalAutosave.enabled-is[bool]-and$protection.originalAutosave.enabled)
 Need ($owned.controllerPid-eq$c.controllerPid-and$owned.controllerCreationTime-ceq$c.controllerCreationTime-and$owned.controllerKernelCreationFileTime-ceq$c.controllerKernelCreationFileTime-and$owned.processes-is[array]-and$claim.binding.ownedPids-is[array]-and$claim.binding.ownedPids.Count-gt0-and$c.installation-ceq(Join-Path $scenario 'installation'))
 $absence=$a.pidAbsence;$expected=@($c.controllerPid)+@($c.supervisorJobPids)+@($owned.processes|ForEach-Object{$_.pid;$_.supervisorJobPids})+@($claim.binding.ownedPids)+@($t.verifierPid,$ready.node.pid,$ready.helper.pid)+@($ready.supervisorJobPids)
 if($callerDate){Assert-NativeCallerDateFailure $v $a $e $output $node $powershell;return $v.actions}
 Need ($absence.allAbsent-is[bool]-and$absence.allAbsent-and$absence.liveRows-is[array]-and$absence.liveRows.Count-eq0-and$absence.queriedExactRecordedPids-is[array])
 foreach($id in $expected){Assert-NativeSeedInteger $id 1 9007199254740991;Need ($absence.queriedExactRecordedPids-contains$id)}
 Need ([DateTimeOffset]::Parse($absence.atUtc)-ge[DateTimeOffset]::Parse($t.at)-and[DateTimeOffset]::Parse($protection.at)-ge[DateTimeOffset]::Parse($t.at))
 if($linkage){Assert-NativeCaptureLinkageFailure $v $a $e $output $node $powershell;return $v.actions}
 if($wrongScene){Assert-NativeWrongSceneFailureCustody $v $a $e $output $node $powershell;return $v.actions}
 if($known){Assert-NativeKnownPlayFailureCustody $v $a $e $output $returned $captureStop;return $v.actions}
 if($snapshotBusy){Assert-NativeSnapshotBusyFailure $v $a $e $output $node $powershell;return $v.actions}
 Exact-Integer $o.schemaVersion 1;Exact-Integer $o.inputActions 0;Exact-Integer $o.activations 0;Exact-Integer $o.providersStarted 1;Exact-Integer $o.providersClosedKnown 1
 $observationKind=if($observedFocus){'ACTUAL_S11_DRIVER_FOCUS_BLOCKED_STOP'}else{'ACTUAL_DRIVER_NO_INPUT_FOCUS_STOP'}
 Need ($o.calls-is[array]-and$o.calls.Count-eq5-and$o.schemaVersion-eq1-and$o.kind-ceq$observationKind-and$o.category-ceq'FOCUS_BLOCKED'-and$o.verdict-ceq'FAIL'-and$o.driver-ceq$driver-and$o.scenarioId-ceq$p.scenarioId-and$o.sourceHead-ceq$v.sourceHead-and$o.inputActions-eq0-and$o.activations-eq0-and$o.providersStarted-eq1-and$o.providersClosedKnown-eq1-and(Same $o.currentRunContext $r.actualRunContext)-and(Same $o.actualGuardHistory $l.history))
 $tools=if($observedFocus){@('doctor','policy_status','get_window','screenshot','get_frontmost_app')}else{@('doctor','policy_status','get_window','get_frontmost_app','screenshot')};$attempts=@(Get-ChildItem -LiteralPath $output -File|Where-Object{$_.Name-cmatch'^call-\d+\.attempt\.private\.json$'}|Sort-Object Name|ForEach-Object{$_.Name})
 Need (Same $attempts @(1..5|ForEach-Object{'call-'+$_.ToString('000')+'.attempt.private.json'}))
 foreach($batch in @('E1','E2')){Need (!(Test-Path -LiteralPath (Join-Path $scenario ('guard-'+$batch+'-consumed.private.json'))))}
 $rawCalls=@();$posts=@()
 for($i=0;$i-lt5;$i++){
  $row=$o.calls[$i];Assert-NativeSeedFields $row @('completed','attempt','raw');$prefix=Join-Path $output ('call-'+($i+1).ToString('000'))
  $post=Read-Ref $row.completed ($prefix+'.private.json');$attempt=Read-Ref $row.attempt ($prefix+'.attempt.private.json');$raw=Read-Ref $row.raw ($prefix+'.raw.private.json') 8388608;$rawCalls+=,$raw;$posts+=,$post
  Need ($post.tool-ceq$tools[$i]-and$attempt.tool-ceq$tools[$i]-and$post.runID-ceq$ready.runID-and$attempt.runID-ceq$ready.runID-and$post.batch-ceq'B1'-and$attempt.batch-ceq'B1'-and$attempt.sourceSha256-ceq$e.source.sha256-and(Same $post.args $attempt.args)-and$post.precedingActions-is[array]-and$post.precedingActions.Count-eq0-and$post.isError-is[bool]-and!$post.isError-and$raw.isError-ne$true)
  Need ([DateTimeOffset]::Parse($post.callStarted)-ge[DateTimeOffset]::Parse($ready.at)-and[DateTimeOffset]::Parse($post.callCompleted)-ge[DateTimeOffset]::Parse($post.callStarted)-and[DateTimeOffset]::Parse($post.callCompleted)-le[DateTimeOffset]::Parse($t.at))
 }
 if($observedFocus){Assert-NativeObservedFocusCustody $v $a $rawCalls $posts $output}
 $frontIndex=if($observedFocus){4}else{3};$window=$rawCalls[2].structuredContent;$front=$rawCalls[$frontIndex].structuredContent.app
 $targets=@($owned.processes|Where-Object{$_.pid-eq$window.pid});Need ($targets.Count-eq1);$target=$targets[0]
 Need ($target.inSupervisorJob-is[bool]-and$target.inSupervisorJob-and$target.sessionId-eq$c.sessionId-and$target.executable-ceq(Join-Path $c.installation 'jre/bin/java.exe')-and$target.supervisorJobPids-contains$window.pid-and$claim.binding.ownedPids-contains$window.pid-and$claim.binding.campaign.windowId-eq$window.windowId-and$absence.queriedExactRecordedPids-contains$window.pid)
 foreach($id in $c.supervisorJobPids){Need ($target.supervisorJobPids-contains$id)}
 Need ($window.isFocused-is[bool]-and!$window.isFocused-and$window.windowId-eq$o.capture.windowId-and$window.pid-eq$o.capture.pid-and$front.pid-ne$window.pid-and$front.pid-eq$o.frontmost.pid)
 return $v.actions
}

# A local view of one immutable raw record. No source artifact is rewritten.
function Get-NativeObservedFocusView($v,$a) {
 $o=$a.observation;$r=$a.result;$w=$o.window;$front=$o.foreground.raw.structuredContent.app
 Exact-Integer $a.plan.sessionWatchdogMs 1800000;Exact-Integer $a.lease.sessionWatchdogMs 1800000
 $cap=$a.plan.callerAdmissionDeadlineMs;$start=$a.rootCall.value.startMs
 Assert-NativeSeedInteger $cap 1 9007199254740991;Exact-Integer $a.lease.callerAdmissionDeadlineMs $cap;Assert-NativeSeedInteger $start 1 9007199254740991
 Need ($start-lt$cap-and$cap-$start-le1800000)
 foreach($name in @('kind','driver','scenario','failureCategory','assertion','rootProtection')){Need ($o.$name-is[string])}
 foreach($name in @('kind','scenarioId','driver','sourceHead','status','verdict','failureCategory')){Need ($r.$name-is[string])}
 foreach($stamp in @($o.atUtc,$r.atUtc)){Need ($stamp-is[string]);$null=[DateTimeOffset]::Parse($stamp)}
 Assert-NativeSeedFields $o @('schemaVersion','kind','driver','scenario','atUtc','failureCategory','assertion','window','foreground','callCount','originalImages','inputCount','reservationsAdded','activations','geometryModes','metadataModes','actualVendorCalls','begin','providerFinish','historyBaseline','actualContext','absoluteMaxReservedActions','originalCallerSession','originalCallerDeadlineMs','retry','ordinaryExit','campaign','seed','rootProtection')
 Assert-NativeSeedFields $r @('schemaVersion','kind','scenarioId','driver','sourceHead','status','verdict','failureCategory','inheritedGuardBaseline','actualGuardHistory','actualRunContext','absoluteMaxReservedActions','historicalUnsentReservations','actions','providers','counts','actualCallOrder','focus','visibleAssertion','steps','timing','closure','protection','readerCompatibility','claims','evidence','calls','atUtc')
 Exact-Integer $o.schemaVersion 1;foreach($pair in @(@('callCount',5),@('originalImages',1),@('inputCount',0),@('reservationsAdded',0),@('activations',0),@('geometryModes',0),@('metadataModes',0))){Exact-Integer $o.($pair[0]) $pair[1]}
 Need ($o.kind-ceq'ACTUAL_S11_DRIVER_FOCUS_BLOCKED_STOP'-and$o.driver-ceq'/root/recovery_game_driver'-and$o.scenario-is[string]-and[IO.Path]::IsPathFullyQualified($o.scenario)-and[IO.Path]::GetFullPath($o.scenario)-ceq$v.scenario-and$o.failureCategory-ceq'FOCUS_BLOCKED'-and$o.rootProtection-ceq'PENDING')
 Need ((Same @($o.retry,$o.ordinaryExit,$o.campaign,$o.seed) @($false,$false,$false,$false))-and$r.calls-is[array]-and$r.calls.Count-eq5-and$o.actualVendorCalls-is[array]-and$o.actualVendorCalls.Count-eq5-and(Same $o.historyBaseline $r.inheritedGuardBaseline)-and(Same $o.actualContext $r.actualRunContext))
 Exact-Integer $o.absoluteMaxReservedActions $r.absoluteMaxReservedActions;Exact-Integer $o.originalCallerSession $a.caller.session;Exact-Integer $o.originalCallerDeadlineMs $a.plan.callerAdmissionDeadlineMs
 Need ($o.assertion-is[string]-and$o.assertion.Length-gt0-and$o.assertion.Length-le4096)
 Assert-NativeSeedFields $o.foreground @('raw','at','reason');Need ($o.foreground.reason-is[string]-and$o.foreground.reason.Length-le4096)
 Assert-NativeSeedFields $o.begin @('args','raw','at');Assert-NativeSeedFields $o.begin.args @('consentMode','runID','source');Assert-NativeSeedFields $o.providerFinish @('raw','at')
 Need ((Same $r.counts ([pscustomobject]@{originalPngs=1;metadataModes=0;geometryModes=0}))-and(Same $r.steps ([pscustomobject]@{B1='FOCUS_BLOCKED_BEFORE_INPUT';E1='NOT_STARTED';E2='NOT_STARTED'})))
 Need (Same $r.claims ([pscustomobject]@{campaign=$false;seed=$false;stability=$false;ordinaryExit=$false;native99Acceptance=$false;alphaAcceptance=$false;newTypedHistoryKind=$false}))
 Need (Same $r.timing ([pscustomobject]@{rootWatchdogMs=$a.plan.sessionWatchdogMs;inputMarginMs=20000;cleanupReserveMs=30000;originalCallerDeadlineMs=$a.plan.callerAdmissionDeadlineMs;clockRenewal=$false;retry=$false}))
 Assert-NativeSeedInteger $w.pid 1 9007199254740991;Assert-NativeSeedInteger $w.windowId 1 9007199254740991;Assert-NativeSeedInteger $front.pid 1 9007199254740991
 Need ($front.bundleId-is[string])
 Assert-NativeSeedInteger $w.bounds.width 1 32768;Assert-NativeSeedInteger $w.bounds.height 1 32768
 Need ($w.isFocused-is[bool]-and!$w.isFocused-and(Same $r.focus ([pscustomobject]@{ownedPid=$w.pid;ownedHwnd=$w.windowId;isFocused=$false;foregroundApp=$front.bundleId;foregroundPid=$front.pid})))
 Need (Same $r.visibleAssertion ([pscustomobject]@{launcherPlayVisible=$true;inputPrecondition=$false;originalNativeDimensions=[pscustomobject]@{width=$w.bounds.width;height=$w.bounds.height};captureReceipt=$true;freshness='UNKNOWN';activationAllowed=$false}))
 $compatibility=$r.readerCompatibility
 Assert-NativeSeedFields $compatibility @('currentNoInputFocusReader','expectedCallOrder','actualOrderPreserved','currentReaderSource','observationActualKind','canonicalObservationAliasCreated','reason')
 foreach($name in @('currentNoInputFocusReader','observationActualKind')){Need ($compatibility.$name-is[string])}
 Need ($compatibility.currentNoInputFocusReader-ceq'NOT_COMPATIBLE_WITH_ACTUAL_ORDER_OR_OBSERVATION_SHAPE'-and(Same $compatibility.expectedCallOrder @('doctor','policy_status','get_window','get_frontmost_app','screenshot'))-and$compatibility.actualOrderPreserved-is[bool]-and$compatibility.actualOrderPreserved-and$compatibility.observationActualKind-ceq$o.kind-and$compatibility.canonicalObservationAliasCreated-is[bool]-and!$compatibility.canonicalObservationAliasCreated)
 foreach($name in @('currentReaderSource','reason')){Need ($compatibility.$name-is[string]-and$compatibility.$name.Length-le4096)}
 Assert-NativeSeedFields $r.closure @('provider','rootCaller','maintainedStopToolChunk','forcedOwnedCleanup','ordinaryExit','allOwnedHandlesTerminalReaped','knownPidsAbsent')
 Need ((Same @($r.closure.forcedOwnedCleanup,$r.closure.ordinaryExit,$r.closure.allOwnedHandlesTerminalReaped) @($true,$false,$true))-and$r.closure.maintainedStopToolChunk-is[string]-and$r.closure.maintainedStopToolChunk.Length-gt0-and$r.closure.maintainedStopToolChunk.Length-le128)
 return [pscustomobject]@{schemaVersion=$o.schemaVersion;kind=$o.kind;scenarioId=$r.scenarioId;category=$o.failureCategory;verdict=$r.verdict;driver=$o.driver;sourceHead=$r.sourceHead;currentRunContext=$o.actualContext;actualGuardHistory=$o.historyBaseline;inputActions=$o.inputCount;activations=$o.activations;providersStarted=$r.providers.started;providersClosedKnown=$(if($r.closure.provider.terminalReaped-is[bool]-and$r.closure.provider.terminalReaped){1}else{0});calls=$r.calls;capture=$w;frontmost=$front}
}

function Assert-NativeObservedFocusCustody($v,$a,$raw,$posts,[string]$output) {
 $o=$a.observation;$r=$a.result;$t=$a.providerTerminal;$caller=$a.caller;$tools=@('doctor','policy_status','get_window','screenshot','get_frontmost_app')
 foreach($post in $posts){Need ($post.callStarted-is[string]-and$post.callCompleted-is[string]);$null=[DateTimeOffset]::Parse($post.callStarted);$null=[DateTimeOffset]::Parse($post.callCompleted)}
 foreach($stamp in @($a.providerReady.at,$t.at,$t.deadline,$a.protection.at,$a.pidAbsence.atUtc)){
  Need ($stamp-is[string]);$null=[DateTimeOffset]::Parse($stamp)
 }
 $readyMs=[DateTimeOffset]::Parse($a.providerReady.at).ToUnixTimeMilliseconds();$closedMs=[DateTimeOffset]::Parse($t.at).ToUnixTimeMilliseconds();$deadlineMs=[DateTimeOffset]::Parse($t.deadline).ToUnixTimeMilliseconds()
 Need ($a.rootCall.value.startMs-le$readyMs-and$closedMs-le$deadlineMs-and$deadlineMs-le$a.plan.callerAdmissionDeadlineMs-30000-and$deadlineMs-$readyMs-le180000)
 Need ((Same $o.window $raw[2].structuredContent)-and(Same $o.foreground.raw $raw[4])-and(Same $r.actualCallOrder $tools))
 Need (Same @($posts[0].args,$posts[1].args,$posts[2].args,$posts[3].args,$posts[4].args) @([pscustomobject]@{include_remediation=$true},[pscustomobject]@{},[pscustomobject]@{window_id=$o.window.windowId},[pscustomobject]@{target_window_id=$o.window.windowId;width=$o.window.bounds.width;quality=0},[pscustomobject]@{}))
 $previous=[long]0
 for($i=0;$i-lt5;$i++){
  $row=$o.actualVendorCalls[$i];Assert-NativeSeedFields $row @('ordinal','name','args','atMs');Exact-Integer $row.ordinal ($i+1);Assert-NativeSeedInteger $row.atMs 0 9007199254740991
  Need ($row.name-is[string])
  # The pinned doctor schema supplies true for the caller's omitted option.
  $auditArgs=if($i-eq0){[pscustomobject]@{}}else{$posts[$i].args}
  Need ($row.name-ceq$tools[$i]-and(Same $row.args $auditArgs)-and$row.atMs-ge[DateTimeOffset]::Parse($posts[$i].callCompleted).ToUnixTimeMilliseconds()-and$row.atMs-gt$previous-and$row.atMs-le[DateTimeOffset]::Parse($t.at).ToUnixTimeMilliseconds());$previous=$row.atMs
 }
 function Packet($packet){
  Assert-NativeSeedFields $packet @('content');Need ($packet.content-is[array]-and$packet.content.Count-eq1);Assert-NativeSeedFields $packet.content[0] @('type','text')
  Need ($packet.content[0].type-is[string]-and$packet.content[0].type-ceq'text'-and$packet.content[0].text-is[string]-and[Text.Encoding]::UTF8.GetByteCount($packet.content[0].text)-le262144)
  return ConvertFrom-ZeroProviderObject $packet.content[0].text
 }
 $begin=Packet $o.begin.raw;Assert-NativeSeedFields $begin @('runID','output','deadline')
 Need ($begin.runID-is[string]-and$begin.deadline-is[string]-and$o.begin.args.consentMode-is[string]-and$o.begin.args.runID-is[string])
 Need ($begin.runID-ceq$a.providerReady.runID-and$begin.output-is[string]-and[IO.Path]::IsPathFullyQualified($begin.output)-and[IO.Path]::GetFullPath($begin.output)-ceq$output-and$begin.deadline-ceq$t.deadline-and(Same (Packet $o.providerFinish.raw) $t))
 $null=[DateTimeOffset]::Parse($begin.deadline)
 Need ($o.begin.args.consentMode-ceq'finite-explicit-consent'-and$o.begin.args.runID-ceq$begin.runID-and$o.begin.args.source-is[string]-and[IO.Path]::IsPathFullyQualified($o.begin.args.source)-and[IO.Path]::GetFullPath($o.begin.args.source)-ceq$v.evidence.source.path)
 foreach($stamp in @($o.begin.at,$o.foreground.at,$o.providerFinish.at)){Assert-NativeSeedInteger $stamp 0 9007199254740991}
 Need ($o.begin.at-ge[DateTimeOffset]::Parse($a.providerReady.at).ToUnixTimeMilliseconds()-and$o.begin.at-le[DateTimeOffset]::Parse($posts[0].callStarted).ToUnixTimeMilliseconds()-and$o.foreground.at-ge$o.actualVendorCalls[4].atMs-and$o.foreground.at-le[DateTimeOffset]::Parse($t.at).ToUnixTimeMilliseconds()-and$o.providerFinish.at-ge[DateTimeOffset]::Parse($t.at).ToUnixTimeMilliseconds()-and[DateTimeOffset]::Parse($o.atUtc).ToUnixTimeMilliseconds()-ge$o.providerFinish.at-and[DateTimeOffset]::Parse($r.atUtc)-ge[DateTimeOffset]::Parse($o.atUtc))
 Need (Same $r.closure.provider ([pscustomobject]@{verifierPid=$t.verifierPid;status=0;timedOut=$false;verdict='PASS';terminalReaped=$true;at=$t.at}))
 Need (Same $r.closure.rootCaller ([pscustomobject]@{session=$caller.session;originalRawChunk=$caller.launchRaw.chunk_id;terminalRawChunk=$caller.actual.chunk_id;status=1;timedOut=$false;verdict='FAIL';terminalReaped=$true;kind=$caller.kind;pollCount=$caller.launchPolls.Count}))
 Need (Same $r.closure.knownPidsAbsent $a.pidAbsence.queriedExactRecordedPids)
 $p=$a.protection
 Need (Same $r.protection ([pscustomobject]@{protectedOriginalFiles=$p.protectedOriginalFiles;protectedSaveFiles=$p.protectedSaveFiles;allOriginalAndBackupHashesMatch=$p.allOriginalAndBackupHashesMatch;saveCensusUnchanged=$p.saveCensusUnchanged;originalWrites=$p.originalWrites;originalAutosave=$p.originalAutosave;privateAutosaveEnabled=$p.privateAutosaveEnabled;privateSaveEntries=$p.privateSaveEntries;liveGameProcesses=$p.liveGameProcesses;sharedPreference='Registry64/String/true unchanged';restoration=$p.restoration}))
}
