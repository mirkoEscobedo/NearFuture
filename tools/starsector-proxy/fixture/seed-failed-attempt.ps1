# Historical closed failure custody only; no process, input, clock or seed authority.
. (Join-Path $PSScriptRoot 'seed-known-play-attempt.ps1')
. (Join-Path $PSScriptRoot 'seed-wrong-scene-attempt.ps1')
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
  Assert-NativeSeedFields $Pin @('path','sha256');Need ($Pin.path-ceq$Expected-and$Pin.sha256-is[string]-and$Pin.sha256-cmatch'^[a-f0-9]{64}$')
  return (Read-NativeModernSeedJson $Expected $Pin.sha256 $Maximum)
 }
 function Critical($Step){return [pscustomobject]@{command=$Step.command;args=$Step.args;cwd=$Step.cwd;processOwnership=$Step.processOwnership;status=$Step.status;timedOut=$Step.timedOut}}
 $v=$Receipt;$driver='/root/recovery_game_driver';$captureStop=$ExpectedKind-ceq'ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY';$wrongScene=$ExpectedKind-ceq'ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY';$known=($ExpectedKind-ceq'ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY'-or$captureStop-or$wrongScene)
 Assert-NativeSeedFields $v @('schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence')
 Assert-NativeSeedInteger $v.schemaVersion 1 1
 Need ($ExpectedKind-cin@('ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY')-and$v.kind-ceq$ExpectedKind-and$v.issuer-ceq'/root'-and$v.driver-ceq$driver-and$v.allOwnedHandlesTerminalReaped-is[bool]-and$v.allOwnedHandlesTerminalReaped)
 foreach($path in @($v.scenario,$v.sourceWorkspace)){Need ($path-is[string]-and[IO.Path]::IsPathFullyQualified($path)-and[IO.Path]::GetFullPath($path)-ceq$path)}
 $scenario=$v.scenario;Need ([IO.Path]::GetDirectoryName($scenario)-ceq(Join-Path $v.sourceWorkspace '.tmp/starsector-proxy/scenarios')-and$v.sourceHead-is[string]-and$v.sourceHead-cmatch'^[a-f0-9]{40}$')
 Assert-NativeSeedFields $v.actions @('actualGuard','currentRunContext','absoluteMaxReservedActions')
 Assert-NativeSeedFields $v.actions.currentRunContext @('overallAttempts','uiAttempts')
 $names=@{result='driver-flow-result.private.json';ledger='guard-launch-history.private.json';plan='plan.json';lease='native-lane-lease.private.json';controller='controller-start.private.json';owned='owned-processes-current.private.json';rootCall='root-call.actual.private.json';caller='launch-caller-terminal.actual.private.json';launch='launch-supervisor.private.json';protection='after-smoke-protection.private.json';pidAbsence='owned-terminal-pid-absence.actual.private.json';preference='seed-creation-preference.private.json';source='B1-authorization.private.json';claim='guard-B1-consumed.private.json';observation='driver-observation-stop.private.json'}
 $names.supportPins='frozen-support-pins.private.json'
 $providerNames=@{providerReady='guarded-http-ready.private.json';providerTerminal='owned-provider-terminal.private.json';providerSpool='owned-provider.stdout.private.txt';providerStderr='owned-provider.stderr.private.txt'}
 $secondProvider=@()
 if($wrongScene){$names.e1Source='E1-authorization.private.json';$names.e1Claim='guard-E1-consumed.private.json';$names.e1Plan='E1-plan.private.json';$names.e1Lease='E1-lease.private.json';$names.e1Owned='driver-E1-owned.private.json';$names.b1Menu='B1-menu-inspected.private.json';$secondProvider=@('e1ProviderReady','e1ProviderTerminal','e1ProviderSpool','e1ProviderStderr')}
 Assert-NativeSeedFields $v.evidence (@($names.Keys)+@($providerNames.Keys)+$secondProvider)
 $e=$v.evidence;$a=@{};foreach($key in $names.Keys){$a[$key]=Read-Ref $e.$key (Join-Path $scenario $names[$key])}
 $r=$a.result;$l=$a.ledger;$p=$a.plan;$lease=$a.lease;$c=$a.controller;$owned=$a.owned;$rootCall=$a.rootCall;$caller=$a.caller;$launch=$a.launch;$protection=$a.protection;$preference=$a.preference;$source=$a.source;$claim=$a.claim;$o=$a.observation
 Exact-Integer $r.schemaVersion 1;Exact-Integer $l.schemaVersion 2;Exact-Integer $p.campaignControl.schemaVersion 4;Exact-Integer $lease.campaignControl.schemaVersion 4
 Exact-Integer $source.schemaVersion 6;Exact-Integer $claim.binding.schemaVersion 6
 Need ((Same $e.ledger $LedgerPin)-and(Same $l $PreviousLedger)-and$l.schemaVersion-eq2-and$l.kind-ceq'STARSECTOR_FINITE_LAUNCH_HISTORY'-and($known-or$null-eq$l.pending)-and$l.uncertain-is[bool]-and!$l.uncertain-and(Same $l.history $v.actions.actualGuard)-and($known-or(Same $l.history $p.historyBaseline)))
 Need ($p.scenarioId-ceq[IO.Path]::GetFileName($scenario)-and$p.paths.scenario-ceq$scenario-and$p.source.qualifiedHead-ceq$v.sourceHead-and$p.campaignControl.profile-ceq'DISPOSABLE_SEED_CREATION'-and$p.campaignControl.schemaVersion-eq4-and$p.ownership.driver-ceq$driver-and$p.ownership.onlyGameInputDriver-is[bool]-and$p.ownership.onlyGameInputDriver)
 Need ($lease.planSha256.ToLowerInvariant()-ceq$e.plan.sha256-and$lease.scenarioId-ceq$p.scenarioId-and$lease.driver-ceq$driver-and$lease.authorized-is[bool]-and$lease.authorized-and$lease.issuer-ceq'/root'-and$lease.campaignControl.profile-ceq$p.campaignControl.profile-and$lease.campaignControl.schemaVersion-eq4-and$c.rootLeaseSha256.ToLowerInvariant()-ceq$e.lease.sha256-and$l.controllerSha256-ceq$e.controller.sha256)
 Need ($rootCall.key-ceq$l.rootRecordKey-and$rootCall.value.args.workdir-ceq$v.sourceWorkspace);Assert-NativeSeedInteger $rootCall.value.raw.session_id 1 9007199254740991
 $inherited=if($known){$p.historyBaseline}else{$l.history};$category=if($wrongScene){'POSTCONDITION_FAILED'}elseif($captureStop){'CAPTURE_UNCERTAIN'}elseif($known){'PRIVATE_FRAME_CUSTODY_REFUSAL_AFTER_KNOWN_PLAY'}else{'FOCUS_BLOCKED'}
 Need ((Same $r.actualGuardHistory $l.history)-and(Same $r.inheritedGuardBaseline $inherited)-and(Same $r.actualRunContext $v.actions.currentRunContext)-and$r.schemaVersion-eq1-and$r.kind-ceq'ACTUAL_FINITE_DRIVER_FLOW_RESULT'-and$r.status-ceq'CLOSED_FAIL'-and$r.verdict-ceq'FAIL'-and$r.failureCategory-ceq$category-and$r.scenarioId-ceq$p.scenarioId-and$r.driver-ceq$driver-and$r.sourceHead-ceq$v.sourceHead)
 Need ($known-or((Same $r.actions ([pscustomobject]@{reservedThisRun=0;sentThisRun=0;clicks=0;activations=0}))-and(Same $r.providers ([pscustomobject]@{started=1;B1='TERMINAL_PASS_REAPED';E1='NOT_STARTED';E2='NOT_STARTED';calls=5}))))
 Assert-NativeSeedInteger $r.absoluteMaxReservedActions ($l.history.reservedActions+1) ($l.history.reservedActions+8)
 Need ($r.absoluteMaxReservedActions-eq$p.absoluteMaxReservedActions-and$r.absoluteMaxReservedActions-eq$lease.absoluteMaxReservedActions-and$r.absoluteMaxReservedActions-eq$v.actions.absoluteMaxReservedActions-and$r.historicalUnsentReservations-eq($l.history.reservedActions-$l.history.sentActions))
 $aliases=@{observation='observation';callerTerminal='caller';launchSupervisor='launch';protection='protection';pidAbsence='pidAbsence';ledger='ledger';plan='plan';lease='lease'}
 foreach($key in $aliases.Keys){Need (Same $r.evidence.$key $e.($aliases[$key]))}
 $provider=if($wrongScene){$claim.binding}else{$l.provider}
 Need ($provider.batch-ceq'B1'-and$provider.runID-is[string]-and$provider.runID-cmatch'^[A-Za-z0-9_-]+$'-and$source.schemaVersion-eq6-and$source.scenario-ceq$scenario-and$source.batch-ceq'B1'-and$provider.sourceSha256-ceq$e.source.sha256-and$l.b1ClaimSha256-ceq$e.claim.sha256-and$claim.sourcePath-ceq$e.source.path-and$claim.sourceSha256-ceq$e.source.sha256)
 Need ($claim.binding.schemaVersion-eq6-and$claim.binding.batch-ceq'B1'-and$claim.binding.scenario-ceq$scenario-and$claim.binding.workspaceRoot-ceq$v.sourceWorkspace-and$claim.binding.runID-ceq$provider.runID-and$claim.binding.sourceSha256-ceq$e.source.sha256-and$claim.binding.originalPlanSha256-ceq$e.plan.sha256-and$claim.binding.controllerSha256-ceq$e.controller.sha256)
 $output=Join-Path $v.sourceWorkspace ('.tmp/starsector-proxy/'+$provider.runID)
 foreach($key in $providerNames.Keys){
  $path=Join-Path $output $providerNames[$key]
  if($key-ceq'providerStderr'){Assert-NativeSeedFields $e.$key @('path','sha256');Need ($e.$key.path-ceq$path);Assert-NativeSeedOrdinary $path;Need ((Get-Item -LiteralPath $path).Length-eq0-and(Get-FileHash -LiteralPath $path).Hash-ieq$e.$key.sha256)}else{$a[$key]=Read-Ref $e.$key $path}
 }
 $ready=$a.providerReady;$t=$a.providerTerminal;$spool=$a.providerSpool;$step=$t.steps[0]
 Exact-Integer $ready.schemaVersion 1;Exact-Integer $ready.startupTimeoutMs 20000;Exact-Integer $spool.schemaVersion 1
 Exact-Integer $t.verifierStatus 0;Exact-Integer $step.status 0
 Assert-NativeSeedFields $a.supportPins @('pins')
 Need ($a.supportPins.pins-is[array]-and$a.supportPins.pins.Count-le1024-and(Same $p.source.supportPins $e.supportPins)-and$lease.supportPinsPath-ceq$e.supportPins.path-and$lease.supportPinsSha256.ToLowerInvariant()-ceq$e.supportPins.sha256)
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
 $returned=$text|ConvertFrom-Json -DateKind String;$ls=$launch.result.steps[0];$returnedStep=if($returned.steps-is[array]){$returned.steps[0]}else{$returned.steps}
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
 Need ($absence.allAbsent-is[bool]-and$absence.allAbsent-and$absence.liveRows-is[array]-and$absence.liveRows.Count-eq0-and$absence.queriedExactRecordedPids-is[array])
 foreach($id in $expected){Assert-NativeSeedInteger $id 1 9007199254740991;Need ($absence.queriedExactRecordedPids-contains$id)}
 Need ([DateTimeOffset]::Parse($absence.atUtc)-ge[DateTimeOffset]::Parse($t.at)-and[DateTimeOffset]::Parse($protection.at)-ge[DateTimeOffset]::Parse($t.at))
 if($wrongScene){Assert-NativeWrongSceneFailureCustody $v $a $e $output $node $powershell;return $v.actions}
 if($known){Assert-NativeKnownPlayFailureCustody $v $a $e $output $returned $captureStop;return $v.actions}
 Exact-Integer $o.schemaVersion 1;Exact-Integer $o.inputActions 0;Exact-Integer $o.activations 0;Exact-Integer $o.providersStarted 1;Exact-Integer $o.providersClosedKnown 1
 Need ($o.calls-is[array]-and$o.calls.Count-eq5-and$o.schemaVersion-eq1-and$o.kind-ceq'ACTUAL_DRIVER_NO_INPUT_FOCUS_STOP'-and$o.category-ceq'FOCUS_BLOCKED'-and$o.verdict-ceq'FAIL'-and$o.driver-ceq$driver-and$o.scenarioId-ceq$p.scenarioId-and$o.sourceHead-ceq$v.sourceHead-and$o.inputActions-eq0-and$o.activations-eq0-and$o.providersStarted-eq1-and$o.providersClosedKnown-eq1-and(Same $o.currentRunContext $r.actualRunContext)-and(Same $o.actualGuardHistory $l.history))
 $tools=@('doctor','policy_status','get_window','get_frontmost_app','screenshot');$attempts=@(Get-ChildItem -LiteralPath $output -File|Where-Object{$_.Name-cmatch'^call-\d+\.attempt\.private\.json$'}|Sort-Object Name|ForEach-Object{$_.Name})
 Need (Same $attempts @(1..5|ForEach-Object{'call-'+$_.ToString('000')+'.attempt.private.json'}))
 foreach($batch in @('E1','E2')){Need (!(Test-Path -LiteralPath (Join-Path $scenario ('guard-'+$batch+'-consumed.private.json'))))}
 $rawCalls=@()
 for($i=0;$i-lt5;$i++){
  $row=$o.calls[$i];Assert-NativeSeedFields $row @('completed','attempt','raw');$prefix=Join-Path $output ('call-'+($i+1).ToString('000'))
  $post=Read-Ref $row.completed ($prefix+'.private.json');$attempt=Read-Ref $row.attempt ($prefix+'.attempt.private.json');$raw=Read-Ref $row.raw ($prefix+'.raw.private.json') 8388608;$rawCalls+=,$raw
  Need ($post.tool-ceq$tools[$i]-and$attempt.tool-ceq$tools[$i]-and$post.runID-ceq$ready.runID-and$attempt.runID-ceq$ready.runID-and$post.batch-ceq'B1'-and$attempt.batch-ceq'B1'-and$attempt.sourceSha256-ceq$e.source.sha256-and(Same $post.args $attempt.args)-and$post.precedingActions-is[array]-and$post.precedingActions.Count-eq0-and$post.isError-is[bool]-and!$post.isError-and$raw.isError-ne$true)
  Need ([DateTimeOffset]::Parse($post.callStarted)-ge[DateTimeOffset]::Parse($ready.at)-and[DateTimeOffset]::Parse($post.callCompleted)-ge[DateTimeOffset]::Parse($post.callStarted)-and[DateTimeOffset]::Parse($post.callCompleted)-le[DateTimeOffset]::Parse($t.at))
 }
 $window=$rawCalls[2].structuredContent;$front=$rawCalls[3].structuredContent.app
 $targets=@($owned.processes|Where-Object{$_.pid-eq$window.pid});Need ($targets.Count-eq1);$target=$targets[0]
 Need ($target.inSupervisorJob-is[bool]-and$target.inSupervisorJob-and$target.sessionId-eq$c.sessionId-and$target.executable-ceq(Join-Path $c.installation 'jre/bin/java.exe')-and$target.supervisorJobPids-contains$window.pid-and$claim.binding.ownedPids-contains$window.pid-and$claim.binding.campaign.windowId-eq$window.windowId-and$absence.queriedExactRecordedPids-contains$window.pid)
 foreach($id in $c.supervisorJobPids){Need ($target.supervisorJobPids-contains$id)}
 Need ($window.isFocused-is[bool]-and!$window.isFocused-and$window.windowId-eq$o.capture.windowId-and$window.pid-eq$o.capture.pid-and$front.pid-ne$window.pid-and$front.pid-eq$o.frontmost.pid)
 return $v.actions
}
