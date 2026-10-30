# Closed historical caller-date failure. No replay of current launch/history authority.
. (Join-Path $PSScriptRoot 'seed-caller-date-geometry.ps1')
function Get-NativeCallerDateNames {
 return @{observation='driver-stop-observation.private.json';metadataCaller='driver-metadata-calls.actual.private.json';initialOwned='driver-B1-owned.private.json';frameOwned='B1-PLAY-owned-snapshot.private.json';beforeFrame='B1-PLAY-before-frame.private.json';sourceFrame='B1-PLAY-source-before.private.json';geometry='B1-PLAY-client-geometry.actual.private.json';geometryTerminal='B1-PLAY-client-geometry-terminal.actual.private.json';point='B1-PLAY-client-point.private.json';envelope='B1-PLAY-coordinate-proof.private.json';geometryConfiguration='geometry-owned-B1-PLAY/.agentic/project.json'}
}
function Get-NativeCallerDateProviderNames {
 $names=@{};foreach($i in 1..4){$n='call-'+$i.ToString('000');$names['call'+$i+'Attempt']=$n+'.attempt.private.json';$names['call'+$i+'Raw']=$n+'.raw.private.json'};return $names
}
function Assert-NativeCallerDateFailure($v,$a,$e,[string]$Output,[string]$Node,[string]$PowerShell){
 $r=$a.result;$l=$a.ledger;$p=$a.plan;$lease=$a.lease;$c=$a.controller;$o=$a.observation;$ready=$a.providerReady;$t=$a.providerTerminal;$owner=$a.claim.binding;$before=$p.historyBaseline;$cap=$v.actions.absoluteMaxReservedActions
 function Millis($X){Need ($X-is[string]);return [DateTimeOffset]::Parse($X).ToUnixTimeMilliseconds()}
 function Pos($X){Assert-NativeSeedInteger $X 1 9007199254740991}
 function Text($X){Need ($X-is[string]-and$X.Length-gt0-and$X.Length-le8192)}
 Assert-NativeCallerDateScalars $a
 foreach($obj in @($r,$l,$p,$lease,$c,$o,$ready,$t,$owner,$a.source,$a.claim,$a.launch,$a.protection,$a.owned)){
  foreach($prop in $obj.PSObject.Properties){if($prop.Value-is[array]){Need ($prop.Name-cin@('steps','batches','preclockOrder','forbidden','currentUnknowns','supervisorJobPids','ownedPids','processes','actualVendorCalls'))}}
 }
 Assert-NativeSeedFields $before @('overallAttempts','uiAttempts','reservedActions','sentActions');foreach($n in $before.PSObject.Properties){Assert-NativeSeedInteger $n.Value 0 9007199254740991}
 Need ($before.sentActions-le$before.reservedActions-and(Same $l.history $before)-and$null-eq$l.pending-and$l.uncertain-is[bool]-and!$l.uncertain)
 Need (Same $v.actions.currentRunContext ([pscustomobject]@{overallAttempts=$before.overallAttempts+1;uiAttempts=$before.uiAttempts+1}))
 Assert-NativeSeedInteger $cap ($before.reservedActions+1) ($before.reservedActions+8);Exact-Integer $p.absoluteMaxReservedActions $cap;Exact-Integer $lease.absoluteMaxReservedActions $cap
 Need (Same $r.history ([pscustomobject]@{guard=$before;actualContext=$v.actions.currentRunContext;absoluteMaxReservedActions=$cap;historicalUnsentReservations=$before.reservedActions-$before.sentActions;addedReservations=0;addedSentActions=0;refunded=0}))
 Need (Same $r.counts ([pscustomobject]@{vendorCalls=4;originalPngs=1;providersStarted=1;actions=0;activations=0}))
 Need (Same $r.steps ([pscustomobject]@{B1='STOPPED_BEFORE_INPUT_CALLER_DATE_SERIALIZATION';E1='NOT_STARTED';E2='NOT_STARTED'}))
 Need (Same $r.claims ([pscustomobject]@{seed=$false;manualSave=$false;campaign=$false;ordinaryExit=$false;native99Acceptance=$false;alphaAcceptance=$false;closedHistoryKindIssued=$false}))
 Assert-NativeSeedFields $r @('schemaVersion','kind','scenarioId','driver','status','verdict','failureCategory','sourceHead','history','counts','steps','originalInspection','geometry','failure','closure','protection','claims','evidence','atUtc')
 Assert-NativeSeedFields $o @('schemaVersion','kind','driver','scenario','atUtc','failureCategory','assertion','inspection','actualVendorCalls','inputs','reservations','begin','providerFinish','metadataFailure','noRetry','rootCallerSession','originalCallerCap','historyBaseline','actualContext','absoluteMaxReservedActions','ordinaryExit','seed','rootProtection')
 Exact-Integer $o.schemaVersion 1;Exact-Integer $o.inputs 0;Exact-Integer $o.reservations 0
 Need ($o.kind-is[string]-and$o.kind-ceq'ACTUAL_S8_DRIVER_PRE_INPUT_CALLER_SERIALIZATION_STOP'-and$o.driver-is[string]-and$o.driver-ceq$v.driver-and$o.scenario-is[string]-and[IO.Path]::GetFullPath($o.scenario)-ceq$v.scenario-and$o.failureCategory-is[string]-and$o.failureCategory-ceq$r.failureCategory-and$o.noRetry-is[bool]-and$o.noRetry-and$o.ordinaryExit-is[bool]-and!$o.ordinaryExit-and$o.seed-is[bool]-and!$o.seed-and$o.rootProtection-is[string]-and$o.rootProtection-ceq'PENDING'-and(Same $o.historyBaseline $before)-and(Same $o.actualContext $v.actions.currentRunContext))
 Exact-Integer $o.absoluteMaxReservedActions $cap;Exact-Integer $o.rootCallerSession $a.caller.session;Exact-Integer $o.originalCallerCap $p.callerAdmissionDeadlineMs
 $source=$a.source;Assert-NativeSeedFields $source @('schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256')
 $phase=Read-Ref ([pscustomobject]@{path=$source.planPath;sha256=$source.planSha256}) (Join-Path $v.scenario 'B1-plan.private.json')
 $phaseLease=Read-Ref ([pscustomobject]@{path=$source.leasePath;sha256=$source.leaseSha256}) (Join-Path $v.scenario 'B1-lease.private.json')
 Need ($p.batches-is[array]);foreach($batch in $p.batches){Need ($batch.id-is[string])};$b1=@($p.batches|Where-Object{$_.id-ceq'B1'});Need ($b1.Count-eq1)
 $expectedPhase=@{};foreach($prop in $p.PSObject.Properties){$expectedPhase[$prop.Name]=$prop.Value};$expectedPhase.campaign=$owner.campaign;$expectedPhase.batches=$b1
 $expectedLease=@{};foreach($prop in $lease.PSObject.Properties){$expectedLease[$prop.Name]=$prop.Value};$expectedLease.schemaVersion=6;$expectedLease.phase='DISPOSABLE_SEED_CREATION';$expectedLease.campaign=$owner.campaign;$expectedLease.planSha256=$source.planSha256
 Need ((Same $phase ([pscustomobject]$expectedPhase))-and(Same $phaseLease ([pscustomobject]$expectedLease)))
 Assert-NativeSeedFields $owner.campaign @('flow','windowId','rootClock','producerWitness','rootCallRecord','historyBaseline','maxReservedActions','launchPredecessor');Need (Same $owner.campaign.flow 'DISPOSABLE_SEED_CREATION')
 foreach($row in @(@($phase,@('scenarioId')),@($phaseLease,@('issuer','driver','phase','planSha256','supportPinsPath','supportPinsSha256','fixtureReceiptSha256')))){foreach($key in $row[1]){Need ($row[0].$key-is[string])}}
 Need ($phase.paths.scenario-is[string]-and$phase.source.qualifiedHead-is[string]-and$owner.planSha256-is[string]-and$owner.sourcePath-is[string])
 Need ((Same $phase.campaign $owner.campaign)-and(Same $phaseLease.campaign $owner.campaign)-and$owner.planSha256-ceq$source.planSha256-and$owner.sourcePath-ceq$e.source.path-and$phase.scenarioId-ceq$p.scenarioId-and$phase.paths.scenario-ceq$v.scenario-and$phase.source.qualifiedHead-ceq$v.sourceHead-and(Same $phase.historyBaseline $before)-and(Same $phase.seedCreation $p.seedCreation)-and(Same $owner.campaign.historyBaseline $before)-and(Same $owner.campaign.rootCallRecord $a.rootCall))
 Exact-Integer $phase.absoluteMaxReservedActions $cap;Exact-Integer $owner.campaign.maxReservedActions $cap;Exact-Integer $phaseLease.schemaVersion 6;Exact-Integer $phaseLease.maxActions 8;Exact-Integer $phaseLease.maxProxyInvocations 3;Exact-Integer $phaseLease.ownedGameHardTimeoutMs 480000
 Need ($phaseLease.authorized-is[bool]-and$phaseLease.authorized-and$phaseLease.issuer-is[string]-and$phaseLease.issuer-ceq'/root'-and$phaseLease.driver-is[string]-and$phaseLease.driver-ceq$v.driver-and$phaseLease.phase-is[string]-and$phaseLease.phase-ceq'DISPOSABLE_SEED_CREATION'-and$phaseLease.planSha256-ceq$source.planSha256-and$phaseLease.campaignLoadSaveAllowed-is[bool]-and!$phaseLease.campaignLoadSaveAllowed-and$phaseLease.supportPinsPath-ceq$e.supportPins.path-and$phaseLease.supportPinsSha256.ToLowerInvariant()-ceq$e.supportPins.sha256-and$phaseLease.fixtureReceiptSha256-ceq$lease.fixtureReceiptSha256-and(Same $phaseLease.seedCreation $p.seedCreation))
 Exact-Integer $phaseLease.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs
 $clock=$owner.campaign.rootClock;$birth=Millis $c.controllerCreationTime;$published=Millis $c.at;$outer=Millis $c.conservativeOuterDeadlineUtc;$end=Millis $c.controllerDeadlineUtc
 foreach($key in @('startMs','deadlineMs','effectiveDeadlineMs','sessionWatchdogMs','cleanupReserveMs','callerAdmissionDeadlineMs','originalCallerAdmissionDeadlineMs')){Assert-NativeSeedInteger $clock.$key 0 9007199254740991};foreach($key in @('recordKey','producerRecordKey')){Need ($clock.$key-is[string])}
 foreach($n in @($clock.startMs,$clock.deadlineMs,$clock.effectiveDeadlineMs,$p.callerAdmissionDeadlineMs,$l.providerClock.startMs,$l.providerClock.deadlineMs)){Assert-NativeSeedInteger $n 0 9007199254740991}
 Need ($clock.startMs-eq$a.rootCall.value.startMs-and$clock.recordKey-ceq$a.rootCall.key-and$clock.deadlineMs-eq($clock.startMs+480000)-and$clock.sessionWatchdogMs-eq480000-and$clock.cleanupReserveMs-eq30000-and$clock.callerAdmissionDeadlineMs-eq$p.callerAdmissionDeadlineMs-and$clock.originalCallerAdmissionDeadlineMs-eq$p.callerAdmissionDeadlineMs-and$lease.callerAdmissionDeadlineMs-eq$p.callerAdmissionDeadlineMs-and$clock.effectiveDeadlineMs-eq(@($clock.deadlineMs,$p.callerAdmissionDeadlineMs,$outer,$end)|Measure-Object -Minimum).Minimum-and$clock.startMs-le$birth-and$birth-le$published-and$published-lt$end-and$end-le$outer-and$outer-le($birth+480000)-and$l.providerClock.startMs-ge$published-and$l.providerClock.startMs-ge$clock.startMs-and$l.providerClock.deadlineMs-eq[math]::Min($l.providerClock.startMs+180000,$clock.effectiveDeadlineMs-30000)-and(Millis $ready.at)-ge$l.providerClock.startMs-and(Millis $t.at)-le$l.providerClock.deadlineMs)
 $prod=$owner.campaign.producerWitness
 foreach($key in @('producerRecordKey','controllerCreationTime','controllerKernelCreationFileTime','conservativeOuterDeadlineUtc','controllerDeadlineUtc')){Need ($prod.$key-is[string])};foreach($key in @('controllerPid','effectiveDeadlineMs','sessionWatchdogMs')){Assert-NativeSeedInteger $prod.$key 0 9007199254740991}
 Need ($clock.producerRecordKey-ceq('controller-start:'+$e.controller.sha256)-and$prod.producerRecordKey-ceq$clock.producerRecordKey-and$prod.controllerPid-eq$c.controllerPid-and$prod.controllerCreationTime-ceq$c.controllerCreationTime-and$prod.controllerKernelCreationFileTime-ceq$c.controllerKernelCreationFileTime-and$prod.conservativeOuterDeadlineUtc-ceq$c.conservativeOuterDeadlineUtc-and$prod.controllerDeadlineUtc-ceq$c.controllerDeadlineUtc-and$prod.effectiveDeadlineMs-eq$clock.effectiveDeadlineMs-and$prod.sessionWatchdogMs-eq480000)
 $predecessor=Read-Ref $owner.campaign.launchPredecessor (Join-Path $v.scenario 'campaign-postlaunch.private.json')
 Need (Same $predecessor ([pscustomobject]@{schemaVersion=1;kind='FINITE_CAMPAIGN_LAUNCH_PREDECESSOR';plan=$e.plan;lease=$e.lease;controller=$e.controller;rootClock=$clock;producerWitness=$prod;rootCallRecord=$a.rootCall;historyBaseline=$before;maxReservedActions=$cap}))
 foreach($child in @($ready.node,$ready.helper)){Pos $child.pid;Pos $child.parentPid;Need ((Millis $child.creationTime)-ge$published-and(Millis $child.creationTime)-le(Millis $ready.at))}
 Need ((Millis $ready.helper.creationTime)-ge(Millis $ready.node.creationTime))
 $tools=@('doctor','policy_status','get_window','screenshot');$posts=@();$raws=@();$attempts=@(Get-ChildItem -LiteralPath $Output -File|Where-Object{$_.Name-cmatch'^call-\d+\.attempt\.private\.json$'}|Sort-Object Name|ForEach-Object{$_.Name})
 Need ($o.actualVendorCalls-is[array]-and$o.actualVendorCalls.Count-eq4-and(Same $attempts @(1..4|ForEach-Object{'call-'+$_.ToString('000')+'.attempt.private.json'})))
 foreach($batch in @('E1','E2')){foreach($name in @(('guard-'+$batch+'-consumed.private.json'),($batch+'-authorization.private.json'))){Need (!(Test-Path -LiteralPath (Join-Path $v.scenario $name)))}}
 $previous=Millis $ready.at
 for($i=0;$i-lt4;$i++){
  $name='call-'+($i+1).ToString('000')+'.private.json';$post=Read-Ref $r.evidence.$name (Join-Path $Output $name);$attempt=$a.('call'+($i+1)+'Attempt');$raw=$a.('call'+($i+1)+'Raw');$row=$o.actualVendorCalls[$i]
  Assert-NativeSeedFields $row @('ordinal','batch','name','args','atMs');Exact-Integer $row.ordinal ($i+1);Pos $row.atMs
  $args=if($i-eq0){[pscustomobject]@{}}else{$post.args}
  foreach($key in @('runID','batch','callStarted','callCompleted','freshness')){Need ($post.$key-is[string])};foreach($key in @('runID','batch','sourceSha256','callStarted','innerProtocolID')){Need ($attempt.$key-is[string])}
  $rawErrorPresent=@($raw.PSObject.Properties.Name)-ccontains'isError'
  Need ($row.batch-is[string]-and$row.batch-ceq'B1'-and$row.name-is[string]-and$row.name-ceq$tools[$i]-and$row.atMs-ge(Millis $post.callCompleted)-and$row.atMs-le(Millis $t.at)-and(Same $row.args $args)-and$post.tool-is[string]-and$post.tool-ceq$tools[$i]-and$attempt.tool-is[string]-and$attempt.tool-ceq$tools[$i]-and$post.runID-ceq$ready.runID-and$attempt.runID-ceq$ready.runID-and$post.batch-ceq'B1'-and$attempt.batch-ceq'B1'-and$attempt.sourceSha256-ceq$e.source.sha256-and(Same $post.args $attempt.args)-and$attempt.callStarted-ceq$post.callStarted-and(Same $post.precedingActions @())-and$post.isError-is[bool]-and!$post.isError-and(!$rawErrorPresent-or($raw.isError-is[bool]-and!$raw.isError)))
  Pos $post.outerProtocolID;Exact-Integer $post.innerProtocolID ($i+1);Exact-Integer $attempt.outerProtocolID $post.outerProtocolID
  Need ($attempt.innerProtocolID-is[string]-and$attempt.innerProtocolID-ceq'UNKNOWN'-and$post.freshness-is[string]-and$post.freshness-ceq'UNKNOWN'-and(Millis $post.callStarted)-ge$previous-and(Millis $post.callCompleted)-ge(Millis $post.callStarted)-and(Millis $post.callCompleted)-le(Millis $t.at))
  $previous=Millis $post.callCompleted;$posts+=,$post;$raws+=,$raw
 }
 Assert-NativeSeedFields $o.begin @('args','raw','at');Need (Same $o.begin.args ([pscustomobject]@{consentMode='finite-explicit-consent';runID=$ready.runID;source=$e.source.path.Replace('\','/')}));Pos $o.begin.at;Need ($o.begin.at-ge(Millis $ready.at)-and$o.begin.at-le(Millis $posts[0].callStarted))
 $begun=Read-NativeCallerDatePacket $o.begin.raw;Assert-NativeSeedFields $begun @('runID','output','deadline');Need ($begun.runID-is[string]-and$begun.runID-ceq$ready.runID-and$begun.output-is[string]-and$begun.output-ceq$Output-and(Millis $begun.deadline)-ge(Millis $t.at)-and(Millis $begun.deadline)-le$l.providerClock.deadlineMs);Pos $o.providerFinish.at;Need ($o.providerFinish.at-ge(Millis $t.at)-and$o.providerFinish.at-le(Millis $o.atUtc))
 foreach($index in 0..2){Need ($raws[$index].structuredContent-is[pscustomobject])}
 Need ((Same $posts[0].args ([pscustomobject]@{include_remediation=$true}))-and(Same $posts[1].args ([pscustomobject]@{}))-and$raws[0].structuredContent.ok-is[bool]-and$raws[0].structuredContent.ok-and$raws[1].structuredContent.approval_required_for_all-is[bool]-and$raws[1].structuredContent.approval_required_for_all-and$raws[1].structuredContent.approval_token_configured-is[bool]-and!$raws[1].structuredContent.approval_token_configured)
 $window=$raws[2].structuredContent;Pos $window.pid;Pos $window.windowId;Pos $c.sessionId
 Assert-NativeSeedFields $window.bounds @('x','y','width','height');foreach($n in $window.bounds.PSObject.Properties){Assert-NativeSeedInteger $n.Value -9007199254740991 9007199254740991};Pos $window.bounds.width;Pos $window.bounds.height
 Need ($window.bundleId-is[string]-and$window.bundleId-ceq'java.exe'-and$window.isFocused-is[bool]-and$window.isFocused-and(Same $posts[2].args ([pscustomobject]@{window_id=$window.windowId}))-and(Same $posts[3].args ([pscustomobject]@{target_window_id=$window.windowId;quality=0;width=$window.bounds.width})))
 Exact-Integer $owner.campaign.windowId $window.windowId;Need (Same $e.initialOwned ([pscustomobject]@{path=$source.ownedReceiptPath;sha256=$source.ownedReceiptSha256}))
 $snapshots=@($a.initialOwned,$a.frameOwned,$a.owned);$games=@();foreach($snap in $snapshots){Need ($snap.processes-is[array]);foreach($process in $snap.processes){Need ($process-is[pscustomobject]-and$process.executable-is[string]);Assert-NativeSeedInteger $process.sessionId 1 9007199254740991};$rows=@($snap.processes|Where-Object{$_.executable-ceq(Join-Path $c.installation 'jre/bin/java.exe')});Need ($rows.Count-eq1);$games+=,$rows[0]}
 Assert-NativeSeedFields $a.initialOwned @('at','sourceSnapshot','processes');Assert-NativeSeedFields $a.initialOwned.sourceSnapshot @('path','sha256');Need ($a.initialOwned.sourceSnapshot.path-is[string]-and$a.initialOwned.sourceSnapshot.path-ceq$e.owned.path-and$a.initialOwned.sourceSnapshot.sha256-is[string]-and$a.initialOwned.sourceSnapshot.sha256-cmatch'^[a-f0-9]{64}$')
 $first=$games[0];Pos $first.pid;Pos $first.parentPid
 Need (Same $owner.ownedPids @($first.pid))
 Need ($first.kernelCreationFileTime-is[string]-and$first.kernelCreationFileTime-cmatch'^[1-9][0-9]*$'-and$first.pid-eq$window.pid-and$first.sessionId-eq$c.sessionId-and(Millis $first.creationTime)-ge$published-and(Millis $first.creationTime)-le(Millis $ready.at))
 for($i=0;$i-lt$games.Count;$i++){$x=$games[$i];$snap=$snapshots[$i];Need (($i-eq0-or((Same $snap.controllerPid $c.controllerPid)-and(Same $snap.controllerCreationTime $c.controllerCreationTime)-and(Same $snap.controllerKernelCreationFileTime $c.controllerKernelCreationFileTime)))-and$x.inSupervisorJob-is[bool]-and$x.inSupervisorJob-and$x.supervisorJobPids-is[array]);foreach($key in @('pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId')){Need (Same $x.$key $first.$key)};foreach($id in $x.supervisorJobPids){Pos $id};foreach($id in @($first.pid,$c.controllerPid,$x.parentPid)+@($c.supervisorJobPids)){Need ($x.supervisorJobPids-contains$id)};Need ($x.parentPid-ne$x.pid-and(Millis $x.creationTime)-le(Millis $snap.at));Need (($x.mainWindowHandle-is[string]-and$x.mainWindowHandle-cmatch'^[1-9][0-9]*$')-or$x.mainWindowHandle-is[long]-or$x.mainWindowHandle-is[int]);Need ([long]$x.mainWindowHandle-eq$window.windowId)}
 $prefix=Join-Path $Output 'capture-001';$image=$r.originalInspection
 Assert-NativeSeedFields $image @('file','sha256','dimensions','assertion','localPoint','screenPointFromActualGeometry','freshness','inputSent');Need ($image.file-is[string]-and$image.file-ceq'capture-001.png'-and$image.freshness-is[string]-and$image.freshness-ceq'UNKNOWN'-and$image.inputSent-is[bool]-and!$image.inputSent);Text $image.assertion
 $pngPath=$prefix+'.png';Assert-NativeSeedOrdinary $pngPath;$png=[IO.File]::ReadAllBytes($pngPath);Need ($png.Length-le33554432);$pngHash=(Get-FileHash -LiteralPath $pngPath).Hash.ToLowerInvariant()
 $meta=Read-Ref $r.evidence.'capture-001.metadata.private.json' ($prefix+'.metadata.private.json');$capture=Read-Ref $r.evidence.'capture-001.raw.private.json' ($prefix+'.raw.private.json') 33554432
 # Full PNG checks use the maintained strict parser in the native fixture.
 $module=Join-Path $PSScriptRoot '../direct-frames.mjs';$script='import {readFileSync} from "node:fs";import {pathToFileURL} from "node:url";const {completePng}=await import(pathToFileURL(process.argv[1]));process.stdout.write(JSON.stringify(completePng(readFileSync(process.argv[2]))));'
 $parsed=@(& $Node --input-type=module -e $script $module $pngPath);Need ($LASTEXITCODE-eq0);$dimensions=ConvertFrom-ZeroProviderObject ($parsed-join'')
 Need ($capture.content-is[array]);for($blockIndex=0;$blockIndex-lt$capture.content.Count;$blockIndex++){Need ($capture.content[$blockIndex]-is[pscustomobject])}
 $images=@($capture.content|Where-Object{$_.type-ceq'image'})
 Need ($capture.content-is[array]-and$images.Count-eq1-and$images[0]-is[pscustomobject]);foreach($key in @('type','mimeType','data')){Need ($images[0].$key-is[string])};foreach($key in @('freshness','rawPath','rawSha256','pngPath','pngSha256')){Need ($meta.$key-is[string])}
 Need ($pngHash-ceq$image.sha256-and(Same $r.evidence.'capture-001.png' ([pscustomobject]@{path=$pngPath;sha256=$pngHash}))-and(Same $capture $raws[3])-and[Convert]::ToBase64String([IO.File]::ReadAllBytes($e.call4Raw.path))-ceq[Convert]::ToBase64String([IO.File]::ReadAllBytes(($prefix+'.raw.private.json')))-and$images.Count-eq1-and$images[0].mimeType-ceq'image/png'-and$images[0].data-ceq[Convert]::ToBase64String($png)-and$meta.captureReceipt-is[bool]-and$meta.captureReceipt-and$meta.freshness-ceq'UNKNOWN'-and$meta.rawPath-ceq($prefix+'.raw.private.json')-and$meta.rawSha256-ceq$r.evidence.'capture-001.raw.private.json'.sha256-and$meta.pngPath-ceq$pngPath-and$meta.pngSha256-ceq$pngHash-and(Same $meta.window $window)-and(Same $meta.dimensions $dimensions)-and(Same $image.dimensions $dimensions)-and$dimensions.width-eq$window.bounds.width-and$dimensions.height-eq$window.bounds.height)
 foreach($key in @('tool','args','runID','batch','callStarted','callCompleted','precedingActions','outerProtocolID','innerProtocolID')){Need (Same $meta.$key $posts[3].$key)}
 Assert-NativeCallerDateGeometry $v $a $e $Output $Node $PowerShell $window $dimensions $posts
 $ids=@(@($c.controllerPid)+@($c.supervisorJobPids)+@($snapshots|ForEach-Object{$_.processes|ForEach-Object{$_.pid;$_.supervisorJobPids}})+@($owner.ownedPids)|Sort-Object -Unique);$absence=$a.pidAbsence
 Assert-NativeSeedFields $absence @('schemaVersion','kind','atUtc','sourceOwnedSnapshot','rows','allAbsent','zeroLiveOwnedGameProcesses','originalCallerSession','originalCallerTerminalReaped');Exact-Integer $absence.schemaVersion 1
 Need ($absence.kind-is[string]-and$absence.kind-ceq'ACTUAL_OWNED_TERMINAL_PID_ABSENCE'-and(Same $absence.sourceOwnedSnapshot $e.initialOwned)-and$absence.rows-is[array]-and(Same @($absence.rows.pid|Sort-Object) $ids)-and$absence.allAbsent-is[bool]-and$absence.allAbsent-and$absence.zeroLiveOwnedGameProcesses-is[bool]-and$absence.zeroLiveOwnedGameProcesses-and$absence.originalCallerTerminalReaped-is[bool]-and$absence.originalCallerTerminalReaped)
 foreach($row in $absence.rows){Assert-NativeSeedFields $row @('pid','absent');Pos $row.pid;Need ($row.absent-is[bool]-and$row.absent)};Exact-Integer $absence.originalCallerSession $a.caller.session
 Need ((Millis $absence.atUtc)-ge(Millis $t.at)-and(Millis $a.protection.at)-ge(Millis $t.at))
 $close=$r.closure;Assert-NativeSeedFields $close @('providerB1','rootCaller','ownedStopAtUtc','ownedPidAbsence','ordinaryExit','allOwnedTerminalReaped','originalCallerDeadlineMs')
 Need ((Same $close.providerB1 ([pscustomobject]@{verifierPid=$t.verifierPid;status=0;timedOut=$false;terminalObserved=$true;terminalReaped=$true;at=$t.at;verdict='PASS'}))-and(Same $close.rootCaller ([pscustomobject]@{session=$a.caller.session;terminalChunk=$a.caller.actual.chunk_id;exitCode=1;timedOut=$false;terminalReaped=$true;verdict='FAIL';reason='MAINTAINED_REQUESTED_OWNED_CLEANUP'}))-and$close.ownedPidAbsence-is[array]-and(Same @($close.ownedPidAbsence|Sort-Object) $ids)-and$close.ordinaryExit-is[bool]-and!$close.ordinaryExit-and$close.allOwnedTerminalReaped-is[bool]-and$close.allOwnedTerminalReaped)
 Exact-Integer $close.originalCallerDeadlineMs $p.callerAdmissionDeadlineMs;Need ((Millis $close.ownedStopAtUtc)-ge(Millis $t.at)-and(Millis $close.ownedStopAtUtc)-le(Millis $absence.atUtc)-and(Millis $close.ownedStopAtUtc)-le$clock.effectiveDeadlineMs-and(Millis $r.atUtc)-ge(Millis $a.protection.at))
 Need (Same $r.protection ([pscustomobject]@{protectedOriginalFiles=182;protectedSaveFiles=178;originalAndBackupHashesMatch=$true;saveCensusUnchanged=$true;privateAutosaveEnabled=$false;privateSaveEntries=0;liveGameProcesses=0;originalWrites=$false;restoration='UNNECESSARY_ORIGINALS_UNCHANGED';sharedPreference='Registry64/String/true unchanged'}))
 $exclude=@('result','rootCall','preference','supportPins','owned','helper','geometryConfiguration','geometryRunner','initialOwned','frameOwned','beforeFrame')+@((Get-NativeCallerDateProviderNames).Keys);$aliases=@($e.PSObject.Properties.Name|Where-Object{$_-cnotin$exclude});$expected=@()
 foreach($key in $aliases){$name=[IO.Path]::GetFileName($e.$key.path);$expected+=,$name;Need (Same $r.evidence.$name $e.$key)}
 $expected+=@('caller-terminal.actual.private.json','progress.pre-native-preparation.original.private.json','capture-001.png','capture-001.metadata.private.json','capture-001.raw.private.json')+@(1..4|ForEach-Object{'call-'+$_.ToString('000')+'.private.json'});Assert-NativeSeedFields $r.evidence $expected
 foreach($name in @('caller-terminal.actual.private.json','progress.pre-native-preparation.original.private.json')){$data=Read-Ref $r.evidence.$name (Join-Path $v.scenario $name);if($name-ceq'caller-terminal.actual.private.json'){Need (Same $data $a.caller.actual)}}
 Assert-NativeCallerDatePrivateCaller $v $a $e $PowerShell
}
function Assert-NativeCallerDatePrivateCaller($v,$a,$e,[string]$PowerShell){
 $p=$a.plan;$c=$a.controller
 function Historic([string]$Name){$pin=$p.preclockHelpers.$Name;Assert-NativeSeedFields $pin @('path','sha256');Need ($pin.path-is[string]-and$pin.path-ceq(Join-Path $v.scenario $Name)-and$pin.sha256-is[string]-and$pin.sha256-cmatch'^[a-f0-9]{64}$');Assert-NativeSeedOrdinary $pin.path;Need ((Get-FileHash -LiteralPath $pin.path).Hash-ieq$pin.sha256);return $pin}
 $caller=Historic 'launch-owned-caller.private.ps1';$stager=Historic 'record-root-call-and-stage-B1.private.ps1';$orchestration=Historic 'launch-and-stage.orchestration.proposed.private.js';Need (Same (Historic 'construct-actual-phase-inputs.private.mjs') $e.helper)
 foreach($row in @(@($p.historyAdmission,'root-history-baseline.admitted.private.json'),@($p.fixtureBinding,'fixture-binding.private.json'))){$pin=$row[0];Assert-NativeSeedFields $pin @('path','sha256');Need ($pin.path-is[string]-and$pin.path-ceq(Join-Path $v.scenario $row[1])-and$pin.sha256-is[string]-and$pin.sha256-cmatch'^[a-f0-9]{64}$');Assert-NativeSeedOrdinary $pin.path;Need ((Get-FileHash -LiteralPath $pin.path).Hash-ieq$pin.sha256)}
 Need ((Same $p.historyAdmission $a.lease.historyAdmission)-and$p.rootCallContract.path-ceq$e.rootCall.path-and$p.rootCallContract.key-ceq$a.rootCall.key)
 $path=Join-Path $v.scenario 'root-postlaunch-mechanical-delegation.private.json';$pin=[pscustomobject]@{path=$path;sha256=(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()};$d=Read-Ref $pin $path
 foreach($key in @('kind','issuer','driver','planSha256','leaseSha256','helperSha256','callerSha256','stagerSha256','orchestrationSha256','historyAdmissionSha256')){Need ($d.$key-is[string])};Exact-Integer $d.schemaVersion 1
 Need ($d.schemaVersion-eq1-and$d.kind-ceq'ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION'-and$d.issuer-ceq'/root'-and$d.driver-ceq$v.driver-and$d.planSha256-ceq$e.plan.sha256-and$d.leaseSha256-ceq$e.lease.sha256-and$d.helperSha256-ceq$e.helper.sha256-and$d.callerSha256-ceq$caller.sha256-and$d.stagerSha256-ceq$stager.sha256-and$d.orchestrationSha256-ceq$orchestration.sha256-and$d.historyAdmissionSha256-ceq$p.historyAdmission.sha256-and(Same $d.allowedModes @('init','phase B1')))
 function Q([string]$X){return "'"+$X.Replace("'","''")+"'"}
 $cmd='& '+(Q $PowerShell)+' -NoProfile -NonInteractive -File '+(Q $caller.path)+' -PlanSha256 '+(Q $e.plan.sha256)+' -LeaseSha256 '+(Q $e.lease.sha256)+' -DelegationSha256 '+(Q $pin.sha256)+' -HelperSha256 '+(Q $e.helper.sha256)+' -HistoryAdmissionSha256 '+(Q $p.historyAdmission.sha256)
 Need ($a.rootCall.value.args.cmd-is[string]-and$a.rootCall.value.args.cmd-ceq$cmd);$outer=$a.launch.result.steps[0]
 Need ($outer.command-is[string]-and$outer.command-ceq$PowerShell-and(Same $outer.args @('-NoProfile','-NonInteractive','-File',(Join-Path $v.sourceWorkspace 'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',$v.scenario,'-BindingPath',$p.fixtureBinding.path,'-BindingSha256',$p.fixtureBinding.sha256,'-OuterDeadlineUtc',$c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000'))-and$outer.cwd-ceq'.')
}
function Assert-NativeCallerDateScalars($a){
 function Scalars($Object,[string[]]$Strings=@(),[string[]]$Numbers=@(),[string[]]$Booleans=@()){
  Need ($Object-is[pscustomobject])
  foreach($key in $Strings){Need ($Object.$key-is[string])};foreach($key in $Numbers){Assert-NativeSeedInteger $Object.$key 0 9007199254740991};foreach($key in $Booleans){Need ($Object.$key-is[bool])}
 }
 Scalars $a.result @('kind','scenarioId','driver','status','verdict','failureCategory','sourceHead') @('schemaVersion')
 Scalars $a.ledger @('kind','controllerSha256','rootRecordKey','b1ClaimSha256') @('schemaVersion') @('uncertain')
 Scalars $a.ledger.provider @('batch','runID','sourceSha256')
 Scalars $a.plan @('scenarioId') @('absoluteMaxReservedActions','callerAdmissionDeadlineMs');Scalars $a.plan.paths @('scenario');Scalars $a.plan.source @('qualifiedHead')
 Scalars $a.plan.ownership @('driver') @() @('onlyGameInputDriver')
 Scalars $a.plan.campaignControl @('profile') @('schemaVersion');Scalars $a.lease.campaignControl @('profile') @('schemaVersion')
 Scalars $a.plan.rootCallContract @('path','key');Scalars $a.caller @('kind')
 Scalars $a.lease @('scenarioId','driver','issuer','planSha256','supportPinsPath','supportPinsSha256') @('absoluteMaxReservedActions','callerAdmissionDeadlineMs') @('authorized')
 Scalars $a.controller @('controllerCreationTime','controllerKernelCreationFileTime','conservativeOuterDeadlineUtc','controllerDeadlineUtc','rootLeaseSha256','installation','at') @('controllerPid','sessionId')
 Scalars $a.rootCall @('key');Scalars $a.rootCall.value @() @('startMs');Scalars $a.rootCall.value.args @('cmd','workdir');Scalars $a.rootCall.value.raw @() @('session_id')
 Scalars $a.source @('scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256') @('schemaVersion')
 Scalars $a.claim @('sourcePath','sourceSha256');Scalars $a.claim.binding @('scenario','batch','workspaceRoot','runID','sourceSha256','originalPlanSha256','controllerSha256','sourcePath','planSha256') @('schemaVersion')
 Scalars $a.providerReady @('at','runID','batch','sourceSha256','consentMode','query') @('schemaVersion','startupTimeoutMs')
 foreach($child in @($a.providerReady.node,$a.providerReady.helper)){Scalars $child @('creationTime','executable') @('pid','parentPid','sessionId')}
 Scalars $a.providerTerminal @('at','verdict','terminalReceipt') @('verifierPid','verifierStatus') @('terminalObserved','cleanupUnknown','terminalBoundToChild','closeFailed','overflow','spawnFailed')
 Scalars $a.protection @('at','phase') @('protectedOriginalFiles','protectedSaveFiles','privateSaveEntries','liveGameProcesses') @('allOriginalAndBackupHashesMatch','saveCensusUnchanged','originalWrites','privateAutosaveEnabled')
 Scalars $a.protection.originalAutosave @() @('maximumSlots');Exact-Integer $a.protection.originalAutosave.maximumSlots 3
 foreach($pin in $a.supportPins.pins){Assert-NativeSeedFields $pin @('path','sha256');Scalars $pin @('path','sha256');Assert-NativeSeedHash $pin.sha256}
}
function Assert-NativeCallerDateIdentityShapes($Controller,$Ready){
 foreach($value in @(@{ids=$Controller.supervisorJobPids},@{ids=$Ready.supervisorJobPids})){
  Need ($value.ids-is[array]-and$value.ids.Count-gt0);$seen=[Collections.Generic.HashSet[long]]::new()
  foreach($id in $value.ids){Assert-NativeSeedInteger $id 1 9007199254740991;Need ($seen.Add([long]$id))}
 }
 Need ($Controller.controllerKernelCreationFileTime-is[string]-and$Controller.controllerKernelCreationFileTime-cmatch'^[1-9][0-9]*$')
}
function Assert-NativeCallerDateProviderShape($Terminal,$Spool,$Launch){
 foreach($rows in @(@{value=$Terminal.steps},@{value=$Spool.result.steps},@{value=$Launch.result.steps})){
  Need ($rows.value-is[array]-and$rows.value.Count-eq1);$step=$rows.value[0]
  foreach($key in @('command','cwd','processOwnership')){Need ($step.$key-is[string])};Need ($step.args-is[array]);foreach($arg in $step.args){Need ($arg-is[string])}
 }
 foreach($x in @(@{value=$Spool.command},@{value=$Spool.result.verdict},@{value=$Terminal.verdict},@{value=$Launch.result.verdict})){Need ($x.value-is[string])}
}
