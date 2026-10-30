# Historical two-click wrong-scene custody only. Parent owns terminal validation helpers.
function Assert-NativeWrongSceneFailureCustody($Receipt,$Values,$Evidence,[string]$Output,[string]$Node,[string]$PowerShell) {
 $v=$Receipt;$a=$Values;$e=$Evidence;$r=$a.result;$l=$a.ledger;$p=$a.plan;$c=$a.controller;$o=$a.observation
 $source=$a.e1Source;$claim=$a.e1Claim;$owner=$claim.binding;$phase=$a.e1Plan;$phaseLease=$a.e1Lease;$before=$p.historyBaseline;$pending=$l.pending
 function Wrong-Strings($Value,[string[]]$Fields){foreach($field in $Fields){Need ($Value.$field-is[string])}}
 Wrong-Strings $v @('kind','issuer','driver','scenario','sourceWorkspace','sourceHead')
 Wrong-Strings $r @('kind','status','verdict','failureCategory','assertionResult','scenarioId','driver','sourceHead')
 Wrong-Strings $o @('kind','category','verdict','scenarioId','driver','sourceHead','E2')
 Exact-Integer $o.schemaVersion 1;Assert-NativeSeedInteger $o.ownedPid 1 9007199254740991
 Wrong-Strings $source @('scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256')
 Wrong-Strings $claim @('sourcePath','sourceSha256')
 Wrong-Strings $owner @('batch','sourcePath','sourceSha256','scenario','workspaceRoot','planSha256','originalPlanSha256','controllerSha256','runID')
 # These guards apply only to this new historical kind. PowerShell collection and
 # numeric coercion must not broaden the JavaScript reader's primitive contract.
 foreach($pin in $e.psobject.Properties.Value){Wrong-Strings $pin @('path','sha256')}
 Wrong-Strings $l @('kind','rootRecordKey','controllerSha256','b1ClaimSha256')
 Wrong-Strings $p @('scenarioId');Wrong-Strings $p.paths @('scenario');Wrong-Strings $p.source @('qualifiedHead')
 Wrong-Strings $p.campaignControl @('profile');Wrong-Strings $p.ownership @('driver')
 Wrong-Strings $a.lease @('scenarioId','driver','issuer','supportPinsPath','supportPinsSha256')
 Wrong-Strings $a.lease.campaignControl @('profile')
 Wrong-Strings $c @('rootLeaseSha256','installation')
 Wrong-Strings $a.rootCall @('key');Wrong-Strings $a.rootCall.value.args @('workdir')
 Wrong-Strings $a.source @('scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256')
 Wrong-Strings $a.claim @('sourcePath','sourceSha256')
 Wrong-Strings $a.claim.binding @('batch','scenario','workspaceRoot','runID','sourcePath','sourceSha256','planSha256','originalPlanSha256','controllerSha256')
 Wrong-Strings $a.providerReady @('at','runID','batch','sourceSha256','consentMode','query')
 Wrong-Strings $a.providerReady.node @('executable','creationTime');Wrong-Strings $a.providerReady.helper @('executable','creationTime')
 Wrong-Strings $a.providerTerminal @('at','deadline','verdict','terminalReceipt')
 Need ($a.providerTerminal.steps-is[array]-and$a.providerSpool.result.steps-is[array])
 Wrong-Strings $a.providerTerminal.steps[0] @('command','cwd','processOwnership')
 Wrong-Strings $a.providerSpool @('command');Wrong-Strings $a.providerSpool.result @('verdict')
 Wrong-Strings $a.caller @('kind');Exact-Integer $a.caller.session $a.rootCall.value.raw.session_id
 Wrong-Strings $a.launch @('command');Wrong-Strings $a.launch.result @('verdict')
 Need ($a.launch.result.steps-is[array]);Wrong-Strings $a.launch.result.steps[0] @('processOwnership')
 $returned=(@($a.caller.launchRaw.output)+@($a.caller.launchPolls|ForEach-Object{$_.output})-join'')|ConvertFrom-Json -DateKind String
 Wrong-Strings $returned @('verdict')
 Wrong-Strings $a.protection @('phase');Exact-Integer $a.protection.protectedOriginalFiles 182
 Exact-Integer $a.protection.protectedSaveFiles 178;Exact-Integer $a.protection.originalAutosave.maximumSlots 3
 Wrong-Strings $a.owned @('controllerCreationTime','controllerKernelCreationFileTime');Exact-Integer $a.owned.controllerPid $c.controllerPid
 Exact-Integer $v.actions.absoluteMaxReservedActions $r.absoluteMaxReservedActions
 Exact-Integer $r.historicalUnsentReservations ($l.history.reservedActions-$l.history.sentActions)
 Wrong-Strings $phase @('scenarioId');Wrong-Strings $phase.paths @('scenario');Wrong-Strings $phase.source @('qualifiedHead')
 Wrong-Strings $phaseLease @('issuer','driver','phase','planSha256','supportPinsPath','supportPinsSha256','fixtureReceiptSha256')
 Assert-NativeSeedInteger $c.sessionId 1 9007199254740991
 Need ($c.controllerKernelCreationFileTime-is[string]-and$c.controllerKernelCreationFileTime-cmatch'^[1-9][0-9]*$')
 Exact-Integer $p.absoluteMaxReservedActions $r.absoluteMaxReservedActions;Exact-Integer $a.lease.absoluteMaxReservedActions $r.absoluteMaxReservedActions
 Exact-Integer $phase.absoluteMaxReservedActions $r.absoluteMaxReservedActions;Exact-Integer $phase.campaign.maxReservedActions $r.absoluteMaxReservedActions
 function Wrong-Millis($Value){Need ($Value-is[string]);return [DateTimeOffset]::Parse($Value).ToUnixTimeMilliseconds()}
 function Wrong-Window($Window){
  Assert-NativeSeedInteger $Window.windowId 1 9007199254740991;Assert-NativeSeedInteger $Window.pid 1 9007199254740991
  Need ($Window.bundleId-is[string]-and$Window.bundleId-ceq'java.exe'-and$Window.isFocused-is[bool]-and$Window.isFocused)
  Assert-NativeSeedFields $Window.bounds @('x','y','width','height')
  foreach($key in @('x','y')){Assert-NativeSeedInteger $Window.bounds.$key -9007199254740991 9007199254740991}
  foreach($key in @('width','height')){Assert-NativeSeedInteger $Window.bounds.$key 1 9007199254740991}
 }
 function Wrong-Handle($Value,[long]$Expected){
  if($Value-is[string]){Need ($Value-cmatch'^[1-9][0-9]*$');Need ([long]::Parse($Value,[Globalization.CultureInfo]::InvariantCulture)-eq$Expected)}else{Exact-Integer $Value $Expected}
 }
 Assert-NativeSeedFields $before @('overallAttempts','uiAttempts','reservedActions','sentActions')
 foreach($key in $before.psobject.Properties.Name){Assert-NativeSeedInteger $before.$key 0 9007199254740991}
 Need ($before.sentActions-le$before.reservedActions)
 $guard=[pscustomobject]@{overallAttempts=$before.overallAttempts;uiAttempts=$before.uiAttempts;reservedActions=($before.reservedActions+2);sentActions=($before.sentActions+2)}
 $context=[pscustomobject]@{overallAttempts=($before.overallAttempts+1);uiAttempts=($before.uiAttempts+1)}
 Need ((Same $l.history $guard)-and(Same $v.actions.currentRunContext $context)-and$r.assertionResult-ceq'FAIL')
 Need (Same $r.counts ([pscustomobject]@{reservedThisRun=2;sentThisRun=2;clicks=2;activations=0;providers=2;B1Calls=11;E1Calls=10;totalCalls=21;originals=4;B1WaitMs=10000;B1OptionalRecaptureUsed=0}))
 Need (Same $r.providerStatus ([pscustomobject]@{B1='TERMINAL_PASS_REAPED';E1='TERMINAL_PASS_REAPED';E2='NOT_STARTED'}))
 Need (Same $r.seed ([pscustomobject]@{created=$false;modernCloseoutCreated=$false;stabilityModeExecuted=$false;semanticAcceptance='NOT_RUN'}))
 Assert-NativeSeedFields $source @('schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256')
 Exact-Integer $source.schemaVersion 6;Assert-NativeSeedFields $claim @('sourcePath','sourceSha256','binding');Exact-Integer $owner.schemaVersion 6
 Need ($source.scenario-ceq$v.scenario-and$source.batch-ceq'E1'-and$source.planPath-ceq$e.e1Plan.path-and$source.planSha256-ceq$e.e1Plan.sha256-and$source.leasePath-ceq$e.e1Lease.path-and$source.leaseSha256-ceq$e.e1Lease.sha256-and$source.ownedReceiptPath-ceq$e.e1Owned.path-and$source.ownedReceiptSha256-ceq$e.e1Owned.sha256)
 Need ($claim.sourcePath-ceq$e.e1Source.path-and$claim.sourceSha256-ceq$e.e1Source.sha256-and$owner.batch-ceq'E1'-and$owner.sourcePath-ceq$e.e1Source.path-and$owner.sourceSha256-ceq$e.e1Source.sha256-and$owner.scenario-ceq$v.scenario-and$owner.workspaceRoot-ceq$v.sourceWorkspace-and$owner.planSha256-ceq$e.e1Plan.sha256-and$owner.originalPlanSha256-ceq$e.plan.sha256-and$owner.controllerSha256-ceq$e.controller.sha256-and$owner.runID-is[string]-and$owner.runID-cmatch'^[A-Za-z0-9_-]+$')
 Need (Same $l.provider ([pscustomobject]@{batch='E1';runID=$owner.runID;sourceSha256=$e.e1Source.sha256}))
 Need ($phase.scenarioId-ceq$p.scenarioId-and$phase.paths.scenario-ceq$v.scenario-and$phase.source.qualifiedHead-ceq$v.sourceHead-and(Same $phase.historyBaseline $before)-and$phase.absoluteMaxReservedActions-eq$p.absoluteMaxReservedActions-and(Same $phase.seedCreation $p.seedCreation)-and(Same $phase.campaign $owner.campaign)-and(Same $phase.campaign $phaseLease.campaign)-and(Same $phase.campaign.historyBaseline $before)-and$phase.campaign.maxReservedActions-eq$p.absoluteMaxReservedActions-and(Same $phase.campaign.rootCallRecord $a.rootCall))
 Exact-Integer $phase.campaign.rootClock.sessionWatchdogMs 480000;Exact-Integer $phaseLease.schemaVersion 6
 Need ($phaseLease.authorized-is[bool]-and$phaseLease.authorized-and$phaseLease.issuer-ceq'/root'-and$phaseLease.driver-ceq$v.driver-and$phaseLease.phase-ceq'DISPOSABLE_SEED_CREATION'-and$phaseLease.planSha256-ceq$e.e1Plan.sha256)
 Exact-Integer $phaseLease.maxActions 8;Exact-Integer $phaseLease.maxProxyInvocations 3;Exact-Integer $phaseLease.ownedGameHardTimeoutMs 480000
 Need ($phaseLease.campaignLoadSaveAllowed-is[bool]-and!$phaseLease.campaignLoadSaveAllowed-and$phaseLease.supportPinsPath-ceq$e.supportPins.path-and$phaseLease.supportPinsSha256.ToLowerInvariant()-ceq$e.supportPins.sha256-and$phaseLease.fixtureReceiptSha256-ceq$a.lease.fixtureReceiptSha256-and(Same $phaseLease.seedCreation $p.seedCreation))
 $menuPin=[pscustomobject]@{path=$e.b1Menu.path;sha256=$e.b1Menu.sha256}
 Need ((Same $phase.b1MenuReceipt $menuPin)-and(Same $phase.b1MenuReceipt $phaseLease.b1MenuReceipt))
 $clock=$owner.campaign.rootClock;$producer=$owner.campaign.producerWitness
 Wrong-Strings $clock @('recordKey','producerRecordKey')
 Wrong-Strings $producer @('producerRecordKey','controllerCreationTime','controllerKernelCreationFileTime','conservativeOuterDeadlineUtc','controllerDeadlineUtc')
 Exact-Integer $producer.controllerPid $c.controllerPid
 Exact-Integer $a.rootCall.value.startMs $clock.startMs
 $birth=Wrong-Millis $c.controllerCreationTime;$published=Wrong-Millis $c.at;$outer=Wrong-Millis $c.conservativeOuterDeadlineUtc;$ended=Wrong-Millis $c.controllerDeadlineUtc
 foreach($n in @($clock.startMs,$clock.deadlineMs,$clock.effectiveDeadlineMs,$p.callerAdmissionDeadlineMs)){Assert-NativeSeedInteger $n 1 9007199254740991}
 Exact-Integer $clock.deadlineMs ($clock.startMs+480000);Exact-Integer $clock.sessionWatchdogMs 480000;Exact-Integer $clock.cleanupReserveMs 30000
 Exact-Integer $clock.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs;Exact-Integer $clock.originalCallerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs;Exact-Integer $a.lease.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs
 $effective=@($clock.deadlineMs,$p.callerAdmissionDeadlineMs,$outer,$ended)|Measure-Object -Minimum;Exact-Integer $clock.effectiveDeadlineMs $effective.Minimum
 Need ($clock.recordKey-ceq$a.rootCall.key-and$clock.startMs-eq$a.rootCall.value.startMs-and$clock.startMs-le$birth-and$birth-le$published-and$published-lt$ended-and$ended-le$outer-and$outer-le($birth+480000))
 Need ($clock.producerRecordKey-ceq('controller-start:'+$e.controller.sha256)-and$producer.producerRecordKey-ceq$clock.producerRecordKey-and$producer.controllerPid-eq$c.controllerPid-and$producer.controllerCreationTime-ceq$c.controllerCreationTime-and$producer.controllerKernelCreationFileTime-ceq$c.controllerKernelCreationFileTime-and$producer.conservativeOuterDeadlineUtc-ceq$c.conservativeOuterDeadlineUtc-and$producer.controllerDeadlineUtc-ceq$c.controllerDeadlineUtc)
 Exact-Integer $producer.effectiveDeadlineMs $clock.effectiveDeadlineMs;Exact-Integer $producer.sessionWatchdogMs 480000
 # Independently verify historical B1 -> E1 phase, full process lineage and original menu custody.
 $b1=$a.claim.binding;$play=$l.resolvedB1;$menu=$a.b1Menu
 Wrong-Strings $play @('status','tool','batch','runID','sourceSha256','reservationID','menuReceiptSha256')
 Wrong-Strings $pending @('batch','runID','sourceSha256','reservationID','tool','status')
 Wrong-Strings $menu @('kind','scenario','flow','rootRecordKey','controllerSha256')
 Wrong-Strings $menu.b1 @('sourceSha256','claimSha256','runID','reservationID')
 foreach($pair in @(@('planPath','B1-plan.private.json'),@('leasePath','B1-lease.private.json'),@('ownedReceiptPath','driver-B1-owned.private.json'))){$key=$pair[0];$pin=[pscustomobject]@{path=$a.source.$key;sha256=$a.source.($key.Replace('Path','Sha256'))};$null=Read-Ref $pin (Join-Path $v.scenario $pair[1])}
 $initialOwned=Read-Ref ([pscustomobject]@{path=$a.source.ownedReceiptPath;sha256=$a.source.ownedReceiptSha256}) (Join-Path $v.scenario 'driver-B1-owned.private.json')
 $b1Phase=Read-Ref ([pscustomobject]@{path=$a.source.planPath;sha256=$a.source.planSha256}) (Join-Path $v.scenario 'B1-plan.private.json')
 $b1Campaign=$b1.campaign|ConvertTo-Json -Depth 100|ConvertFrom-Json -DateKind String;$b1Campaign.windowId=$owner.campaign.windowId
 Need ((Same $b1Campaign $owner.campaign)-and(Same $b1Phase.campaign $b1.campaign)-and$b1.planSha256-ceq$a.source.planSha256)
 Need ($a.e1Owned.processes-is[array]-and$a.e1Owned.processes.Count-gt0-and$owner.ownedPids-is[array])
 foreach($snapshot in @($initialOwned,$a.e1Owned,$a.owned)){
  $observedRows=@($snapshot.processes|Where-Object{Same $_.pid $o.ownedPid});Need ($observedRows.Count-eq1)
 }
 Need ($b1.ownedPids-contains$o.ownedPid-and$owner.ownedPids-contains$o.ownedPid)
 foreach($current in $a.e1Owned.processes){
  $originals=@($initialOwned.processes|Where-Object{$_.pid-eq$current.pid});Need ($originals.Count-eq1);$original=$originals[0]
  $lastRows=@($a.owned.processes|Where-Object{$_.pid-eq$current.pid});Need ($lastRows.Count-eq1);$lastOwned=$lastRows[0]
  Assert-NativeSeedInteger $original.pid 1 9007199254740991;Assert-NativeSeedInteger $original.parentPid 1 9007199254740991
  Wrong-Strings $original @('executable');Exact-Integer $original.sessionId $c.sessionId
  Assert-NativeSeedInteger $b1.campaign.windowId 1 9007199254740991;Assert-NativeSeedInteger $owner.campaign.windowId 1 9007199254740991
  Wrong-Handle $original.mainWindowHandle $b1.campaign.windowId;Wrong-Handle $current.mainWindowHandle $owner.campaign.windowId;Wrong-Handle $lastOwned.mainWindowHandle $owner.campaign.windowId
  foreach($key in @('pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId')){Need ((Same $original.$key $current.$key)-and(Same $original.$key $lastOwned.$key))}
  Need ($original.creationTime-is[string]-and$original.kernelCreationFileTime-is[string]-and$original.kernelCreationFileTime-cmatch'^[1-9][0-9]*$'-and$original.parentPid-gt0-and$original.executable-ceq(Join-Path $c.installation 'jre/bin/java.exe')-and$current.sessionId-eq$c.sessionId-and$owner.ownedPids-contains$current.pid-and$b1.ownedPids-contains$current.pid)
  Need ((Wrong-Millis $original.creationTime)-ge$published-and(Wrong-Millis $original.creationTime)-le$play.knownResultAtMs)
  foreach($row in @($original,$current,$lastOwned)){Need ($row.inSupervisorJob-is[bool]-and$row.inSupervisorJob-and$row.supervisorJobPids-is[array]);foreach($id in $row.supervisorJobPids){Assert-NativeSeedInteger $id 1 9007199254740991};foreach($id in @($c.controllerPid,$current.pid)+@($c.supervisorJobPids)){Need ($row.supervisorJobPids-contains$id)}}
 }
 Assert-NativeSeedFields $menu @('schemaVersion','kind','scenario','flow','rootRecordKey','controllerSha256','b1','frame','inspection','provider')
 Exact-Integer $menu.schemaVersion 1
 Need ($menu.kind-ceq'STARSECTOR_B1_MAIN_MENU_INSPECTION'-and$menu.scenario-ceq$v.scenario-and$menu.flow-ceq'DISPOSABLE_SEED_CREATION'-and$menu.rootRecordKey-ceq$a.rootCall.key-and$menu.controllerSha256-ceq$e.controller.sha256-and$menu.b1.sourceSha256-ceq$e.source.sha256-and$menu.b1.claimSha256-ceq$e.claim.sha256-and$menu.b1.runID-ceq$b1.runID-and$menu.b1.reservationID-ceq$play.reservationID-and$play.menuReceiptSha256-ceq$e.b1Menu.sha256)
 Need ($play.status-ceq'AWAITING_AFTER'-and$play.tool-ceq'left_click'-and$play.batch-ceq'B1'-and$play.runID-ceq$b1.runID-and$play.sourceSha256-ceq$e.source.sha256-and$play.args.target_window_id-eq$b1.campaign.windowId-and$play.reservationID-ceq($e.controller.sha256+':'+($before.reservedActions+1))-and(Same $play.wizardStep ([pscustomobject]@{ordinal=1;stage='PLAY'})))
 Assert-NativeSeedFields $pending @('batch','runID','sourceSha256','reservationID','tool','args','status','wizardStep','sentAtMs','knownResultAtMs')
 Need ($pending.batch-ceq'E1'-and$pending.runID-ceq$owner.runID-and$pending.sourceSha256-ceq$e.e1Source.sha256-and$pending.reservationID-ceq($e.controller.sha256+':'+$l.history.reservedActions)-and$pending.tool-ceq'left_click'-and$pending.status-ceq'AWAITING_AFTER'-and(Same $pending.wizardStep ([pscustomobject]@{ordinal=1;stage='NEW_GAME'}))-and(Same $pending.args.coordinate $menu.inspection.newGameCoordinate))
 foreach($key in @('resolvedE1','resolvedE2')){Need (@($l.psobject.Properties.Name)-cnotcontains$key)}
 Need ($null-eq$l.resolvedActions-or(Same $l.resolvedActions @()))
 $expectedMenu=[pscustomobject]@{path=$e.b1Menu.path;sha256=$e.b1Menu.sha256;newGameCoordinate=$menu.inspection.newGameCoordinate};Need (Same $owner.b1MenuReceipt $expectedMenu)
 $secondOutput=Join-Path $v.sourceWorkspace ('.tmp/starsector-proxy/'+$owner.runID)
 $ready=Read-Ref $e.e1ProviderReady (Join-Path $secondOutput 'guarded-http-ready.private.json')
 $terminal=Read-Ref $e.e1ProviderTerminal (Join-Path $secondOutput 'owned-provider-terminal.private.json')
 $spool=Read-Ref $e.e1ProviderSpool (Join-Path $secondOutput 'owned-provider.stdout.private.txt')
 Exact-Integer $spool.schemaVersion 1;Wrong-Strings $spool @('command');Wrong-Strings $spool.result @('verdict')
 Need ($terminal.steps-is[array]-and$spool.result.steps-is[array])
 Wrong-Strings $ready @('at','batch','runID','sourceSha256','consentMode','query')
 Wrong-Strings $ready.node @('executable','creationTime');Wrong-Strings $ready.helper @('executable','creationTime')
 Wrong-Strings $terminal @('at','deadline','verdict','terminalReceipt')
 Exact-Integer $ready.schemaVersion 1;Exact-Integer $ready.startupTimeoutMs 20000
 Exact-Integer $ready.node.parentPid $terminal.verifierPid;Exact-Integer $ready.helper.parentPid $ready.node.pid
 Exact-Integer $ready.node.sessionId $c.sessionId;Exact-Integer $ready.helper.sessionId $c.sessionId
 Assert-NativeSeedFields $e.e1ProviderStderr @('path','sha256');$stderr=Join-Path $secondOutput 'owned-provider.stderr.private.txt';Need ($e.e1ProviderStderr.path-ceq$stderr);Assert-NativeSeedOrdinary $stderr
 Need ((Get-Item -LiteralPath $stderr).Length-eq0-and(Get-FileHash -LiteralPath $stderr).Hash-ieq$e.e1ProviderStderr.sha256)
 foreach($key in @('terminalObserved','terminalBoundToChild')){Need ($terminal.$key-is[bool]-and$terminal.$key)}
 foreach($key in @('cleanupUnknown','closeFailed','overflow','spawnFailed')){Need ($terminal.$key-is[bool]-and!$terminal.$key)}
 Exact-Integer $terminal.verifierStatus 0;Need ($null-eq$terminal.signal-and$terminal.verdict-ceq'PASS'-and$terminal.terminalReceipt-ceq$e.e1ProviderTerminal.path-and$terminal.steps.Count-eq1)
 $step=$terminal.steps[0];Wrong-Strings $step @('command','cwd','processOwnership');Exact-Integer $step.status 0;Need ($step.timedOut-is[bool]-and!$step.timedOut-and$step.command-ceq$Node-and$step.cwd-ceq'.'-and$step.processOwnership-ceq'windows-job-object'-and(Same $step.args @((Join-Path $v.sourceWorkspace 'tools/starsector-proxy/guarded-http-child.mjs'))))
 Need ($spool.schemaVersion-eq1-and$spool.command-ceq'verify'-and$spool.ok-is[bool]-and$spool.ok-and$spool.result.verdict-ceq'PASS'-and$spool.result.steps.Count-eq1-and(Same (Critical $spool.result.steps[0]) (Critical $step)))
 Need ($ready.schemaVersion-eq1-and$ready.batch-ceq'E1'-and$ready.runID-ceq$owner.runID-and$ready.sourceSha256-ceq$e.e1Source.sha256-and$ready.startupTimeoutMs-eq20000-and$ready.consentMode-ceq'finite-explicit-consent'-and$ready.node.executable-ceq$Node-and$ready.helper.executable-ceq$PowerShell-and$ready.node.parentPid-eq$terminal.verifierPid-and$ready.helper.parentPid-eq$ready.node.pid-and$ready.node.pid-ne$ready.helper.pid-and$ready.node.sessionId-eq$c.sessionId-and$ready.helper.sessionId-eq$c.sessionId-and$ready.supervisorJobPids-contains$ready.node.pid-and$ready.supervisorJobPids-contains$ready.helper.pid-and$ready.query-ceq'QueryInformationJobObject(NULL,JobObjectBasicProcessIdList)')
 Need ((Wrong-Millis $ready.at)-ge(Wrong-Millis $a.providerTerminal.at)-and(Wrong-Millis $terminal.at)-ge(Wrong-Millis $ready.at)-and(Wrong-Millis $o.atUtc)-ge(Wrong-Millis $terminal.at)-and(Wrong-Millis $o.atUtc)-le(Wrong-Millis $a.protection.at)-and(Wrong-Millis $a.pidAbsence.atUtc)-ge(Wrong-Millis $terminal.at))
 Need ((Wrong-Millis $ready.node.creationTime)-ge$l.providerClock.startMs-and(Wrong-Millis $ready.helper.creationTime)-ge(Wrong-Millis $ready.node.creationTime)-and(Wrong-Millis $ready.helper.creationTime)-le(Wrong-Millis $ready.at)-and(Wrong-Millis $terminal.at)-le(Wrong-Millis $terminal.deadline)-and(Wrong-Millis $terminal.deadline)-le$l.providerClock.deadlineMs)
 Assert-NativeSeedInteger $l.providerClock.startMs 1 9007199254740991;Exact-Integer $l.providerClock.deadlineMs ([Math]::Min(($l.providerClock.startMs+180000),($owner.campaign.rootClock.effectiveDeadlineMs-30000)))
 foreach($id in @($terminal.verifierPid,$ready.node.pid,$ready.helper.pid)+@($ready.supervisorJobPids)+@($owner.ownedPids)){Assert-NativeSeedInteger $id 1 9007199254740991;Need ($a.pidAbsence.queriedExactRecordedPids-contains$id)}
 Need ($o.schemaVersion-eq1-and$o.kind-ceq'ACTUAL_DRIVER_WRONG_SCENE_STOP'-and$o.category-ceq'POSTCONDITION_FAILED'-and$o.verdict-ceq'FAIL'-and$o.scenarioId-ceq$p.scenarioId-and$o.driver-ceq$v.driver-and$o.sourceHead-ceq$v.sourceHead-and(Same $o.actualRunContext $r.actualRunContext)-and(Same $o.actualGuardHistory $l.history))
 foreach($pair in @(@('activations',0),@('totalCalls',21),@('totalOriginals',4),@('B1LoadingWaitMs',10000),@('B1OptionalRecaptureUsed',0))){Exact-Integer $o.($pair[0]) $pair[1]}
 Need ($o.E2-ceq'NOT_STARTED'-and$o.seedClaim-is[bool]-and!$o.seedClaim-and$o.allPriorFailuresRetained-is[bool]-and$o.allPriorFailuresRetained-and(Same $o.ledger $e.ledger)-and(Same $o.rootCall $e.rootCall)-and(Same $o.B1MenuProof $menuPin)-and$o.providers-is[array]-and$o.providers.Count-eq2)
 function Read-WrongSceneCalls($Row,[string[]]$Tools,[string]$Out,$Ready,$Terminal,$SourcePin,$Action){
  Wrong-Strings $Row @('batch')
  Assert-NativeSeedFields $Row @('batch','ready','terminal','calls','originals');Need ($Row.batch-ceq$Ready.batch-and$Row.calls-is[array]-and$Row.calls.Count-eq$Tools.Count-and$Row.originals-is[array]-and$Row.originals.Count-eq2)
  $expectedReady=if($Row.batch-ceq'B1'){$e.providerReady}else{$e.e1ProviderReady};$expectedTerminal=if($Row.batch-ceq'B1'){$e.providerTerminal}else{$e.e1ProviderTerminal};Need ((Same $Row.ready $expectedReady)-and(Same $Row.terminal $expectedTerminal))
  $attempts=@(Get-ChildItem -LiteralPath $Out -File|Where-Object{$_.Name-cmatch'^call-\d+\.attempt\.private\.json$'}|Sort-Object Name|ForEach-Object{$_.Name});Need (Same $attempts @(1..$Tools.Count|ForEach-Object{'call-'+$_.ToString('000')+'.attempt.private.json'}))
  $posts=@();$raws=@();$last=Wrong-Millis $Ready.at
  for($i=0;$i-lt$Tools.Count;$i++){
   $ref=$Row.calls[$i];Assert-NativeSeedFields $ref @('post','raw','attempt');$prefix=Join-Path $Out ('call-'+($i+1).ToString('000'))
   $post=Read-Ref $ref.post ($prefix+'.private.json');$attempt=Read-Ref $ref.attempt ($prefix+'.attempt.private.json');$raw=Read-Ref $ref.raw ($prefix+'.raw.private.json') 33554432
   Wrong-Strings $post @('tool','runID','batch','callStarted','callCompleted')
   Wrong-Strings $attempt @('tool','runID','batch','sourceSha256','callStarted')
   $preceding=if($i-le6){,@()}else{,@($posts[6])}
   Need ($post.tool-ceq$Tools[$i]-and$attempt.tool-ceq$Tools[$i]-and$post.runID-ceq$Ready.runID-and$attempt.runID-ceq$Ready.runID-and$post.batch-ceq$Row.batch-and$attempt.batch-ceq$Row.batch-and$attempt.sourceSha256-ceq$SourcePin.sha256-and(Same $attempt.args $post.args)-and$attempt.callStarted-ceq$post.callStarted-and$post.isError-is[bool]-and!$post.isError)
   if(@($raw.psobject.Properties.Name)-ccontains'isError'){Need ($raw.isError-is[bool]-and!$raw.isError)}
   Need ((Wrong-Millis $post.callStarted)-ge$last-and(Wrong-Millis $post.callCompleted)-ge(Wrong-Millis $post.callStarted)-and(Wrong-Millis $post.callCompleted)-le(Wrong-Millis $Terminal.at)-and(Same $post.precedingActions $preceding))
   $last=Wrong-Millis $post.callCompleted;$posts+=,$post;$raws+=,$raw
  }
  Need ($raws[0].structuredContent.ok-is[bool]-and$raws[0].structuredContent.ok-and$raws[1].structuredContent.approval_required_for_all-is[bool]-and$raws[1].structuredContent.approval_required_for_all-and$raws[1].structuredContent.approval_token_configured-is[bool]-and!$raws[1].structuredContent.approval_token_configured)
  Assert-NativeSeedFields $Action.args @('coordinate','target_app','target_window_id','focus_strategy');Need ($Action.args.target_app-ceq'java.exe'-and$Action.args.focus_strategy-ceq'strict'-and(Same $posts[6].args $Action.args))
  Wrong-Strings $Action.args @('target_app','focus_strategy')
  Need (Same $raws[6].content @([pscustomobject]@{type='text';text=('Clicked ('+($Action.args.coordinate-join', ')+')')}))
  Assert-NativeSeedInteger $Action.sentAtMs 1 9007199254740991;Assert-NativeSeedInteger $Action.knownResultAtMs $Action.sentAtMs 9007199254740991
  Need ((Wrong-Millis $posts[6].callStarted)-le$Action.sentAtMs-and$Action.knownResultAtMs-le(Wrong-Millis $posts[6].callCompleted))
  $lastWindowIndex=if($Row.batch-ceq'B1'){9}else{8};$original=$raws[2].structuredContent;$current=$raws[$lastWindowIndex].structuredContent;Wrong-Window $original;Wrong-Window $current
  Exact-Integer $Action.args.target_window_id $original.windowId
  Need ($original.pid-eq$current.pid-and$original.pid-eq$o.ownedPid-and$Action.args.target_window_id-eq$original.windowId)
  foreach($i in @(4,7)){Exact-Integer $raws[$i].structuredContent.app.pid $original.pid;Wrong-Strings $raws[$i].structuredContent.app @('bundleId');Need ($raws[$i].structuredContent.app.bundleId-ceq'java.exe')}
  foreach($i in @(2,5,$lastWindowIndex)){$w=if($i-eq$lastWindowIndex){$current}else{$original};Wrong-Window $raws[$i].structuredContent;Need ((Same $raws[$i].structuredContent $w)-and(Same $posts[$i].args ([pscustomobject]@{window_id=$w.windowId})))}
  if($Row.batch-ceq'B1'){$listed=@($raws[8].structuredContent.windows|Where-Object{Same $_ $current});Need ($listed.Count-eq1-and((Wrong-Millis $posts[10].callStarted)-(Wrong-Millis $posts[6].callCompleted))-ge10000)}else{Need (Same $original $current)}
  Need ($Action.args.coordinate-is[array]-and$Action.args.coordinate.Count-eq2);foreach($n in $Action.args.coordinate){Assert-NativeSeedInteger $n -9007199254740991 9007199254740991}
  Need ($Action.args.coordinate[0]-ge$original.bounds.x-and$Action.args.coordinate[0]-lt($original.bounds.x+$original.bounds.width)-and$Action.args.coordinate[1]-ge$original.bounds.y-and$Action.args.coordinate[1]-lt($original.bounds.y+$original.bounds.height))
  $pngPaths=@();$indexes=@(3,($Tools.Count-1))
  for($i=0;$i-lt2;$i++){
   $ref=$Row.originals[$i];Assert-NativeSeedFields $ref @('png','metadata','raw','originalInspected','freshness');Need ($ref.originalInspected-is[bool]-and$ref.originalInspected-and$ref.freshness-ceq'UNKNOWN')
   Wrong-Strings $ref @('freshness')
   $prefix=Join-Path $Out ('capture-'+($i+1).ToString('000'));Assert-NativeSeedFields $ref.png @('path','sha256');Need ($ref.png.path-ceq($prefix+'.png'));Assert-NativeSeedOrdinary $ref.png.path
   Need ((Get-Item -LiteralPath $ref.png.path).Length-le33554432-and(Get-FileHash -LiteralPath $ref.png.path).Hash-ieq$ref.png.sha256)
   $meta=Read-Ref $ref.metadata ($prefix+'.metadata.private.json');$raw=Read-Ref $ref.raw ($prefix+'.raw.private.json') 33554432;$images=@($raw.content|Where-Object{$_.type-ceq'image'});$w=if($i-eq0){$original}else{$current};$post=$posts[$indexes[$i]]
   foreach($image in $images){Wrong-Strings $image @('type','mimeType','data')}
   Wrong-Strings $meta @('pngPath','pngSha256','rawPath','rawSha256','runID','batch','tool','callStarted','callCompleted')
   Need ((Same $raw $raws[$indexes[$i]])-and$images.Count-eq1-and$images[0].mimeType-ceq'image/png'-and$images[0].data-ceq[Convert]::ToBase64String([IO.File]::ReadAllBytes($ref.png.path)))
   Need ($meta.captureReceipt-is[bool]-and$meta.captureReceipt-and$meta.pngPath-ceq$ref.png.path-and$meta.pngSha256-ceq$ref.png.sha256-and$meta.rawPath-ceq$ref.raw.path-and$meta.rawSha256-ceq$ref.raw.sha256-and$meta.runID-ceq$Ready.runID-and$meta.batch-ceq$Row.batch-and$meta.tool-ceq'screenshot'-and$meta.callStarted-ceq$post.callStarted-and$meta.callCompleted-ceq$post.callCompleted-and(Same $meta.args $post.args)-and(Same $meta.precedingActions $post.precedingActions)-and(Same $meta.window $w))
   Need (Same $post.args ([pscustomobject]@{target_window_id=$w.windowId;quality=0;width=$w.bounds.width}));$pngPaths+=,$ref.png.path
  }
  $module=Join-Path $PSScriptRoot '../direct-frames.mjs';$script='import {readFileSync} from "node:fs";import {pathToFileURL} from "node:url";const {completePng}=await import(pathToFileURL(process.argv[1]));process.stdout.write(JSON.stringify(process.argv.slice(2).map(p=>completePng(readFileSync(p)))));'
  $parsed=@(& $Node --input-type=module -e $script $module @pngPaths);Need ($LASTEXITCODE-eq0);$dimensions=($parsed-join'')|ConvertFrom-Json -DateKind String;Need ($dimensions.Count-eq2)
  for($i=0;$i-lt2;$i++){$w=if($i-eq0){$original}else{$current};$meta=Read-Ref $Row.originals[$i].metadata (Join-Path $Out ('capture-'+($i+1).ToString('000')+'.metadata.private.json'));Need ((Same $meta.dimensions $dimensions[$i])-and$dimensions[$i].width-eq$w.bounds.width-and$dimensions[$i].height-eq$w.bounds.height)}
  return [pscustomobject]@{original=$original;current=$current;posts=$posts;raws=$raws}
 }
 $toolsB1=@('doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','list_windows','get_window','screenshot')
 $toolsE1=@('doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','get_window','screenshot')
 $first=Read-WrongSceneCalls $o.providers[0] $toolsB1 $Output $a.providerReady $a.providerTerminal $e.source $play
 $second=Read-WrongSceneCalls $o.providers[1] $toolsE1 $secondOutput $ready $terminal $e.e1Source $pending
 Need ((Same $first.current $second.original)-and$o.launcherWindow-eq$first.original.windowId-and$o.gameWindow-eq$second.current.windowId)
 Exact-Integer $o.launcherWindow $first.original.windowId;Exact-Integer $o.gameWindow $second.current.windowId
 $summary=$r.originals;Need ($summary.allFourInspected-is[bool]-and$summary.allFourInspected-and$summary.nativeLauncherWidth-eq$first.original.bounds.width-and$summary.nativeGameWidth-eq$second.current.bounds.width-and$summary.captureReceipt-is[bool]-and$summary.captureReceipt-and$summary.freshness-ceq'UNKNOWN'-and$summary.wrongScenePngSha256-ceq$o.providers[1].originals[1].png.sha256-and$summary.actualVisualTransitionProven-is[bool]-and$summary.actualVisualTransitionProven-and$summary.scenarioSuccess-is[bool]-and!$summary.scenarioSuccess)
 Exact-Integer $summary.nativeLauncherWidth $first.original.bounds.width;Exact-Integer $summary.nativeGameWidth $second.current.bounds.width
 Wrong-Strings $summary @('freshness','wrongScenePngSha256')
 Need (!(Test-Path -LiteralPath (Join-Path $v.scenario 'guard-E2-consumed.private.json')))
 Need ($r.actions-is[array]-and$r.actions.Count-eq2-and$o.actions-is[array]-and$o.actions.Count-eq2)
 $actions=@($play,$pending)
 for($i=0;$i-lt2;$i++){$action=$actions[$i];$result=$r.actions[$i];$observed=$o.actions[$i];$stage=if($i-eq0){'PLAY'}else{'NEW_GAME'};$assertion=if($i-eq0){'PASS'}else{'FAIL'}
  Wrong-Strings $result @('batch','guardStage','assertion');Wrong-Strings $observed @('batch')
  Exact-Integer $result.reservationOrdinal ($before.reservedActions+$i+1);Exact-Integer $observed.reservationOrdinal $result.reservationOrdinal
  Need ($result.batch-ceq$action.batch-and$result.guardStage-ceq$stage-and$result.reservationOrdinal-eq($before.reservedActions+$i+1)-and(Same $result.args $action.args)-and$result.acknowledged-is[bool]-and$result.acknowledged-and$result.assertion-ceq$assertion-and$observed.batch-ceq$action.batch-and(Same $observed.args $action.args)-and$observed.acknowledged-is[bool]-and$observed.acknowledged-and$observed.reservationOrdinal-eq$result.reservationOrdinal)}
 Need ($r.actions[1].ledgerStatus-ceq'AWAITING_AFTER'-and$r.actions[1].uncertain-is[bool]-and!$r.actions[1].uncertain-and$o.actions[1].expectedAssertion-ceq'FAIL')
 Wrong-Strings $r.actions[1] @('ledgerStatus');Wrong-Strings $o.actions[1] @('expectedAssertion')
 # The copied B1 handoff original must equal the exact original observed after Play.
 $frame=$menu.frame;Assert-NativeSeedFields $frame @('pngPath','pngSha256','metadataPath','metadataSha256','capturedAtMs')
 Wrong-Strings $frame @('pngPath','pngSha256','metadataPath','metadataSha256')
 Assert-NativeSeedInteger $frame.capturedAtMs 1 9007199254740991
 $png=Join-Path $v.scenario 'B1-after/F1.original.png';$metaPath=Join-Path $v.scenario 'B1-after/capture-001.metadata.private.json';$rawPath=Join-Path $v.scenario 'B1-after/capture-001.raw.private.json'
 Need ($frame.pngPath-ceq$png-and$frame.metadataPath-ceq$metaPath-and$frame.pngSha256-ceq$o.providers[0].originals[1].png.sha256);Assert-NativeSeedOrdinary $png
 Need ((Get-FileHash -LiteralPath $png).Hash-ieq$frame.pngSha256)
 $meta=Read-Ref ([pscustomobject]@{path=$metaPath;sha256=$frame.metadataSha256}) $metaPath;$raw=Read-NativeModernSeedJson $rawPath (Get-FileHash -LiteralPath $rawPath).Hash 33554432
 Wrong-Strings $meta @('rawPath','rawSha256','pngPath','pngSha256','runID','batch','sourceSha256','reservationID')
 Exact-Integer $meta.capturedAtMs $frame.capturedAtMs
 Need ((Same $raw $first.raws[10])-and$meta.captureReceipt-is[bool]-and$meta.captureReceipt-and$meta.rawPath-ceq$rawPath-and$meta.rawSha256-ieq(Get-FileHash -LiteralPath $rawPath).Hash-and$meta.pngPath-ceq$png-and$meta.pngSha256-ceq$frame.pngSha256-and$meta.runID-ceq$b1.runID-and$meta.batch-ceq'B1'-and$meta.sourceSha256-ceq$e.source.sha256-and$meta.reservationID-ceq$play.reservationID-and$meta.capturedAtMs-eq$frame.capturedAtMs-and$frame.capturedAtMs-eq(Wrong-Millis $first.posts[10].callCompleted)-and(Same $meta.window $first.current))
 Assert-NativeSeedFields $menu.inspection @('issuer','driver','inspectedAtMs','state','newGameCoordinate');Need ($menu.inspection.issuer-ceq'/root'-and$menu.inspection.driver-ceq$v.driver-and$menu.inspection.state-ceq'STARSECTOR_MAIN_MENU')
 Wrong-Strings $menu.inspection @('issuer','driver','state')
 Assert-NativeSeedInteger $menu.inspection.inspectedAtMs $frame.capturedAtMs 9007199254740991
 Assert-NativeSeedFields $menu.provider @('readySha256','terminalSha256','observedClosedAtMs','providerReaped')
 Wrong-Strings $menu.provider @('readySha256','terminalSha256')
 Need ($menu.provider.readySha256-ceq$e.providerReady.sha256-and$menu.provider.terminalSha256-ceq$e.providerTerminal.sha256-and$menu.provider.providerReaped-is[bool]-and$menu.provider.providerReaped)
 $closed=Wrong-Millis $a.providerTerminal.at;Assert-NativeSeedInteger $menu.provider.observedClosedAtMs $closed 9007199254740991
 Need ($menu.inspection.inspectedAtMs-le$l.providerClock.startMs-and$menu.provider.observedClosedAtMs-le$l.providerClock.startMs)
 Need ($frame.capturedAtMs-ge$play.knownResultAtMs-and$closed-ge$frame.capturedAtMs-and$closed-le(Wrong-Millis $a.providerTerminal.deadline)-and(Wrong-Millis $a.providerTerminal.deadline)-le$play.providerClock.deadlineMs-and$menu.provider.observedClosedAtMs-le(Wrong-Millis $o.atUtc)-and$menu.inspection.inspectedAtMs-le(Wrong-Millis $o.atUtc))
 Exact-Integer $play.providerClock.deadlineMs ([Math]::Min(($play.providerClock.startMs+180000),($owner.campaign.rootClock.effectiveDeadlineMs-30000)))
 Assert-NativeSeedInteger $play.providerClock.startMs $clock.startMs $play.sentAtMs
 Need ((Wrong-Millis $a.providerReady.at)-ge$play.providerClock.startMs-and(Wrong-Millis $a.providerReady.at)-le$play.sentAtMs-and
  (Wrong-Millis $a.providerReady.node.creationTime)-ge$play.providerClock.startMs-and(Wrong-Millis $a.providerReady.node.creationTime)-le$closed-and
  (Wrong-Millis $a.providerReady.helper.creationTime)-ge(Wrong-Millis $a.providerReady.node.creationTime)-and(Wrong-Millis $a.providerReady.helper.creationTime)-le$closed)
}
