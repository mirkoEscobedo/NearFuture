# Read-only four-call, zero-input capture-linkage failure custody.
function Assert-NativeCaptureLinkageFailure($v,$a,$e,[string]$Output,[string]$Node,[string]$PowerShell) {
 $r=$a.result;$l=$a.ledger;$p=$a.plan;$c=$a.controller;$o=$a.observation;$ready=$a.providerReady;$terminal=$a.providerTerminal;$owner=$a.claim.binding;$before=$p.historyBaseline
 function Link-Millis($Value){Need ($Value-is[string]);return [DateTimeOffset]::Parse($Value).ToUnixTimeMilliseconds()}
 function Link-Integer($Value){Assert-NativeSeedInteger $Value 1 9007199254740991}
 function Link-Strings($Value,[string[]]$Keys){foreach($key in $Keys){Need ($Value.$key-is[string])}}
 Link-Strings $v @('kind','issuer','driver','scenario','sourceWorkspace','sourceHead')
 Need ($terminal.steps-is[array]-and$a.providerSpool.result.steps-is[array]-and$a.launch.result.steps-is[array])
 Exact-Integer $ready.schemaVersion 1;Exact-Integer $ready.startupTimeoutMs 20000;Exact-Integer $a.providerSpool.schemaVersion 1
 Need ((Same $ready.node.sessionId $c.sessionId)-and(Same $ready.helper.sessionId $c.sessionId))
 Exact-Integer $a.caller.session $a.rootCall.value.raw.session_id
 Exact-Integer $a.protection.protectedOriginalFiles 182;Exact-Integer $a.protection.protectedSaveFiles 178;Exact-Integer $a.protection.originalAutosave.maximumSlots 3
 Link-Strings $a.launch @('command');Link-Strings $a.launch.result @('verdict');Link-Strings $a.providerSpool @('command');Link-Strings $a.providerSpool.result @('verdict')
 foreach($pin in $e.PSObject.Properties.Value){Link-Strings $pin @('path','sha256')}
 foreach($row in @(@($r,'kind,status,verdict,failureCategory,scenarioId,driver,sourceHead'),@($l,'kind,rootRecordKey,controllerSha256,b1ClaimSha256'),@($p,'scenarioId'),@($p.paths,'scenario'),@($p.source,'qualifiedHead'),@($p.campaignControl,'profile'),@($p.ownership,'driver'),@($a.lease,'scenarioId,driver,issuer,supportPinsPath,supportPinsSha256'),@($a.lease.campaignControl,'profile'),@($c,'rootLeaseSha256,installation,controllerCreationTime,controllerKernelCreationFileTime'),@($a.rootCall,'key'),@($a.rootCall.value.args,'workdir'),@($a.source,'scenario,batch,planPath,planSha256,leasePath,leaseSha256,ownedReceiptPath,ownedReceiptSha256'),@($a.claim,'sourcePath,sourceSha256'),@($owner,'batch,scenario,workspaceRoot,runID,sourcePath,sourceSha256,planSha256,originalPlanSha256,controllerSha256'),@($ready,'at,runID,batch,sourceSha256,consentMode,query'),@($terminal,'at,verdict,terminalReceipt'))){Link-Strings $row[0] ($row[1]-split',')}
 Assert-NativeSeedFields $before @('overallAttempts','uiAttempts','reservedActions','sentActions')
 foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){Assert-NativeSeedInteger $before.$key 0 9007199254740991}
 Need ($before.sentActions-le$before.reservedActions-and(Same $l.history $before)-and$null-eq$l.pending-and(Same $l.uncertain $false))
 Need ((Same $v.actions.currentRunContext ([pscustomobject]@{overallAttempts=$before.overallAttempts+1;uiAttempts=$before.uiAttempts+1}))-and(Same $r.counts ([pscustomobject]@{reservedThisRun=0;sentThisRun=0;clicks=0;activations=0;providers=1;vendorCalls=4;originals=1;geometryModeAttempts=1;geometryChildrenLaunched=0;mappingModes=0;pointChecks=0}))-and(Same $r.providerStatus ([pscustomobject]@{B1='TERMINAL_PASS_REAPED';E1='NOT_STARTED';E2='NOT_STARTED'}))-and(Same $r.seed ([pscustomobject]@{created=$false;modernCloseoutCreated=$false;stabilityModeExecuted=$false;semanticAcceptance='NOT_RUN'})))
 Exact-Integer $p.absoluteMaxReservedActions $r.absoluteMaxReservedActions;Exact-Integer $a.lease.absoluteMaxReservedActions $r.absoluteMaxReservedActions;Exact-Integer $v.actions.absoluteMaxReservedActions $r.absoluteMaxReservedActions;Exact-Integer $r.historicalUnsentReservations ($before.reservedActions-$before.sentActions)
 Assert-NativeSeedFields $o @('schemaVersion','kind','scenarioId','driver','atUtc','sourceHead','verdict','failureCategory','actualRunContext','actualGuardHistory','actions','B1','helperInvocation','smallestReadOnlyMismatch','E1','E2','ledger','rootCall','seedClaim','cleanup','allPriorFailuresRetained')
 Link-Strings $o @('kind','scenarioId','driver','atUtc','sourceHead','verdict','failureCategory','E1','E2','cleanup');Exact-Integer $o.schemaVersion 1
 Need ($o.kind-ceq'ACTUAL_DRIVER_PRE_INPUT_CAPTURE_LINKAGE_STOP'-and$o.failureCategory-ceq'PRIVATE_CAPTURE_LINKAGE_REFUSAL_BEFORE_INPUT'-and$o.verdict-ceq'FAIL'-and$o.scenarioId-ceq$p.scenarioId-and$o.driver-ceq$v.driver-and$o.sourceHead-ceq$v.sourceHead-and(Same $o.actualRunContext $r.actualRunContext)-and(Same $o.actualGuardHistory $before)-and(Same $o.actions ([pscustomobject]@{clicks=0;activations=0;reservations=0;pending=$null;uncertain=$false}))-and$o.E1-ceq'NOT_STARTED'-and$o.E2-ceq'NOT_STARTED'-and(Same $o.seedClaim $false)-and(Same $o.allPriorFailuresRetained $true)-and$o.cleanup-ceq'ROOT_OWNS_PENDING_ACTUAL_TERMINAL_PROTECTION'-and(Same $o.ledger $e.ledger)-and(Same $o.rootCall $e.rootCall))
 Assert-NativeSeedFields $o.B1 @('runId','calls','ready','terminal','original')
 Need ((Same $o.B1.runId $ready.runID)-and(Same $o.B1.ready $e.providerReady)-and(Same $o.B1.terminal $e.providerTerminal))
 $source=$a.source;Assert-NativeSeedFields $source @('schemaVersion','scenario','batch','planPath','planSha256','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256')
 $phase=Read-Ref ([pscustomobject]@{path=$source.planPath;sha256=$source.planSha256}) (Join-Path $v.scenario 'B1-plan.private.json')
 $phaseLease=Read-Ref ([pscustomobject]@{path=$source.leasePath;sha256=$source.leaseSha256}) (Join-Path $v.scenario 'B1-lease.private.json')
 Need ((Same $phase.campaign $owner.campaign)-and(Same $phaseLease.campaign $owner.campaign)-and(Same $owner.planSha256 $source.planSha256)-and(Same $owner.sourcePath $e.source.path))
 Need ((Same $phase.scenarioId $p.scenarioId)-and(Same $phase.paths.scenario $v.scenario)-and(Same $phase.source.qualifiedHead $v.sourceHead)-and(Same $phase.historyBaseline $before)-and(Same $phase.absoluteMaxReservedActions $p.absoluteMaxReservedActions)-and(Same $phase.seedCreation $p.seedCreation)-and(Same $owner.campaign.historyBaseline $before)-and(Same $owner.campaign.maxReservedActions $p.absoluteMaxReservedActions)-and(Same $owner.campaign.rootCallRecord $a.rootCall))
 Need ((Same $phaseLease.schemaVersion 6)-and(Same $phaseLease.authorized $true)-and(Same $phaseLease.issuer '/root')-and(Same $phaseLease.driver $v.driver)-and(Same $phaseLease.phase 'DISPOSABLE_SEED_CREATION')-and(Same $phaseLease.planSha256 $source.planSha256)-and(Same $phaseLease.maxActions 8)-and(Same $phaseLease.maxProxyInvocations 3)-and(Same $phaseLease.ownedGameHardTimeoutMs 480000)-and(Same $phaseLease.campaignLoadSaveAllowed $false)-and(Same $phaseLease.supportPinsPath $e.supportPins.path)-and$phaseLease.supportPinsSha256-is[string]-and$phaseLease.supportPinsSha256.ToLowerInvariant()-ceq$e.supportPins.sha256-and(Same $phaseLease.fixtureReceiptSha256 $a.lease.fixtureReceiptSha256)-and(Same $phaseLease.seedCreation $p.seedCreation)-and(Same $phaseLease.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs))
 $clock=$owner.campaign.rootClock;$birth=Link-Millis $c.controllerCreationTime;$published=Link-Millis $c.at;$outer=Link-Millis $c.conservativeOuterDeadlineUtc;$end=Link-Millis $c.controllerDeadlineUtc
 foreach($number in @($clock.startMs,$clock.deadlineMs,$clock.effectiveDeadlineMs,$p.callerAdmissionDeadlineMs,$l.providerClock.startMs,$l.providerClock.deadlineMs,$a.rootCall.value.startMs)){Link-Integer $number}
 Exact-Integer $clock.sessionWatchdogMs 480000;Exact-Integer $clock.cleanupReserveMs 30000
 Need ((Same $clock.recordKey $a.rootCall.key)-and(Same $clock.startMs $a.rootCall.value.startMs)-and$clock.deadlineMs-eq($clock.startMs+480000)-and(Same $clock.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs)-and(Same $clock.originalCallerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs)-and(Same $a.lease.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs)-and$clock.effectiveDeadlineMs-eq(@($clock.deadlineMs,$p.callerAdmissionDeadlineMs,$outer,$end)|Measure-Object -Minimum).Minimum-and$clock.startMs-le$birth-and$birth-le$published-and$published-lt$end-and$end-le$outer-and$outer-le($birth+480000))
 Need ($l.providerClock.startMs-ge$published-and$l.providerClock.startMs-ge$clock.startMs-and$l.providerClock.deadlineMs-eq[Math]::Min($l.providerClock.startMs+180000,$clock.effectiveDeadlineMs-30000)-and(Link-Millis $ready.at)-ge$l.providerClock.startMs-and(Link-Millis $terminal.at)-le$l.providerClock.deadlineMs)
 $producer=$owner.campaign.producerWitness
 Need ((Same $clock.producerRecordKey ('controller-start:'+$e.controller.sha256))-and(Same $producer.producerRecordKey $clock.producerRecordKey)-and(Same $producer.controllerPid $c.controllerPid)-and(Same $producer.controllerCreationTime $c.controllerCreationTime)-and(Same $producer.controllerKernelCreationFileTime $c.controllerKernelCreationFileTime)-and(Same $producer.conservativeOuterDeadlineUtc $c.conservativeOuterDeadlineUtc)-and(Same $producer.controllerDeadlineUtc $c.controllerDeadlineUtc)-and(Same $producer.effectiveDeadlineMs $clock.effectiveDeadlineMs)-and(Same $producer.sessionWatchdogMs 480000))
 foreach($child in @($ready.node,$ready.helper)){Link-Integer $child.pid;Link-Integer $child.parentPid;Link-Strings $child @('creationTime','executable');Need ((Link-Millis $child.creationTime)-ge$published-and(Link-Millis $child.creationTime)-le(Link-Millis $ready.at))}
 $tools=@('doctor','policy_status','get_window','screenshot');$posts=@();$raws=@();$previous=Link-Millis $ready.at
 $attempts=@(Get-ChildItem -LiteralPath $Output -File|Where-Object Name -cmatch '^call-\d+\.attempt\.private\.json$'|Sort-Object Name|ForEach-Object Name)
 Need ($o.B1.calls-is[array]-and$o.B1.calls.Count-eq4-and(Same $attempts @(1..4|ForEach-Object{'call-'+$_.ToString('000')+'.attempt.private.json'})))
 foreach($name in (Get-ChildItem -LiteralPath $v.scenario).Name){Need ($name-cnotmatch'^(?:guard-E[12]-consumed|E[12]-authorization)\.private\.json$'-and$name-cnotmatch'^B1-PLAY-(owned-snapshot|before-frame|source-before|client-point|client-geometry|coordinate-proof|point|geometry)'-and$name-cne'geometry-owned-B1-PLAY')}
 for($i=0;$i-lt4;$i++){
  $row=$o.B1.calls[$i];Assert-NativeSeedFields $row @('post','raw','attempt');$prefix=Join-Path $Output ('call-'+($i+1).ToString('000'))
  $post=Read-Ref $row.post ($prefix+'.private.json');$attempt=Read-Ref $row.attempt ($prefix+'.attempt.private.json');$raw=Read-Ref $row.raw ($prefix+'.raw.private.json') 33554432
  Link-Strings $post @('tool','runID','batch','callStarted','callCompleted','freshness');Link-Strings $attempt @('tool','runID','batch','sourceSha256','callStarted')
  Need ($post.tool-ceq$tools[$i]-and$attempt.tool-ceq$tools[$i]-and(Same $post.runID $ready.runID)-and(Same $attempt.runID $ready.runID)-and$post.batch-ceq'B1'-and$attempt.batch-ceq'B1'-and(Same $attempt.sourceSha256 $e.source.sha256)-and(Same $post.args $attempt.args)-and(Same $attempt.callStarted $post.callStarted)-and(Same $post.precedingActions @())-and(Same $post.isError $false)-and$post.freshness-ceq'UNKNOWN')
  if(@($raw.PSObject.Properties.Name)-contains'isError'){Need (Same $raw.isError $false)}
  Link-Integer $post.outerProtocolID;Exact-Integer $post.innerProtocolID ($i+1);Exact-Integer $attempt.outerProtocolID $post.outerProtocolID;Need (Same $attempt.innerProtocolID 'UNKNOWN')
  $start=Link-Millis $post.callStarted;$stop=Link-Millis $post.callCompleted;Need ($start-ge$previous-and$stop-ge$start-and$stop-le(Link-Millis $terminal.at));$previous=$stop;$posts+=,$post;$raws+=,$raw
 }
 Need ((Same $posts[0].args ([pscustomobject]@{include_remediation=$true}))-and(Same $posts[1].args ([pscustomobject]@{}))-and(Same $raws[0].structuredContent.ok $true)-and(Same $raws[1].structuredContent.approval_required_for_all $true)-and(Same $raws[1].structuredContent.approval_token_configured $false))
 $window=$raws[2].structuredContent;Link-Integer $window.pid;Link-Integer $window.windowId;Link-Integer $c.sessionId
 Need ((Same $window.bundleId 'java.exe')-and(Same $window.isFocused $true));Assert-NativeSeedFields $window.bounds @('x','y','width','height')
 foreach($key in @('x','y')){Assert-NativeSeedInteger $window.bounds.$key -9007199254740991 9007199254740991};Link-Integer $window.bounds.width;Link-Integer $window.bounds.height
 Need ((Same $posts[2].args ([pscustomobject]@{window_id=$window.windowId}))-and(Same $posts[3].args ([pscustomobject]@{target_window_id=$window.windowId;quality=0;width=$window.bounds.width}))-and(Same $owner.campaign.windowId $window.windowId))
 $initial=Read-Ref ([pscustomobject]@{path=$source.ownedReceiptPath;sha256=$source.ownedReceiptSha256}) (Join-Path $v.scenario 'driver-B1-owned.private.json')
 Need ($initial.processes-is[array]-and$a.owned.processes-is[array]);$gameExe=Join-Path $c.installation 'jre/bin/java.exe'
 $firstRows=@($initial.processes|Where-Object{Same $_.executable $gameExe});$lastRows=@($a.owned.processes|Where-Object{Same $_.executable $gameExe})
 Need ($firstRows.Count-eq1-and$lastRows.Count-eq1);$first=$firstRows[0]
 Link-Integer $first.pid;Link-Integer $first.parentPid;Link-Strings $first @('kernelCreationFileTime','creationTime','executable')
 Need ($first.kernelCreationFileTime-cmatch'^[1-9][0-9]*$'-and(Same $first.pid $window.pid)-and$first.executable-ceq(Join-Path $c.installation 'jre/bin/java.exe')-and(Same $first.sessionId $c.sessionId)-and(Link-Millis $first.creationTime)-ge$published-and(Link-Millis $first.creationTime)-le(Link-Millis $ready.at))
 foreach($row in @($first,$lastRows[0])){
  foreach($key in @('pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId')){Need (Same $row.$key $first.$key)}
  Need ((Same $row.inSupervisorJob $true)-and$row.supervisorJobPids-is[array]);foreach($id in $row.supervisorJobPids){Link-Integer $id};foreach($id in (@($first.pid,$c.controllerPid)+@($c.supervisorJobPids))){Need ($row.supervisorJobPids-contains$id)}
  if($row.mainWindowHandle-is[string]){Need ($row.mainWindowHandle-cmatch'^[1-9][0-9]*$')}else{Link-Integer $row.mainWindowHandle};Need ([long]$row.mainWindowHandle-eq$window.windowId)
 }
 $original=$o.B1.original;Assert-NativeSeedFields $original @('png','metadata','raw','originalInspected','nativeDimensions','visibleAssertion','freshness','windowId','pid','isFocused')
 Need ((Same $original.originalInspected $true)-and(Same $original.freshness 'UNKNOWN')-and(Same $original.isFocused $true)-and(Same $original.windowId $window.windowId)-and(Same $original.pid $window.pid)-and(Same $original.visibleAssertion 'Launcher Play visible'))
 $prefix=Join-Path $Output 'capture-001';$m=Read-Ref $original.metadata ($prefix+'.metadata.private.json');$raw=Read-Ref $original.raw ($prefix+'.raw.private.json') 33554432
 Assert-NativeSeedFields $original.png @('path','sha256');Need (Same $original.png.path ($prefix+'.png'));Assert-NativeSeedOrdinary $original.png.path
 Need ((Get-Item -LiteralPath $original.png.path).Length-le33554432-and(Get-FileHash -LiteralPath $original.png.path).Hash-ieq$original.png.sha256)
 $bytes=[IO.File]::ReadAllBytes($original.png.path);$captureBytes=[IO.File]::ReadAllBytes($original.raw.path);$callBytes=[IO.File]::ReadAllBytes($o.B1.calls[3].raw.path)
 Need ([Convert]::ToBase64String($captureBytes)-ceq[Convert]::ToBase64String($callBytes)-and(Same $raw $raws[3])-and$raw.content-is[array]);$images=@($raw.content|Where-Object{Same $_.type 'image'})
 Need ($images.Count-eq1-and(Same $images[0].mimeType 'image/png')-and(Same $images[0].data ([Convert]::ToBase64String($bytes)))-and(Same $m.captureReceipt $true)-and(Same $m.freshness 'UNKNOWN')-and(Same $m.rawPath $original.raw.path)-and(Same $m.rawSha256 $original.raw.sha256)-and(Same $m.pngPath $original.png.path)-and(Same $m.pngSha256 $original.png.sha256)-and(Same $m.window $window))
 $module=Join-Path $PSScriptRoot '../direct-frames.mjs';$script='import {readFileSync} from "node:fs";import {pathToFileURL} from "node:url";const {completePng}=await import(pathToFileURL(process.argv[1]));process.stdout.write(JSON.stringify(completePng(readFileSync(process.argv[2]))));'
 $parsed=@(& $Node --input-type=module -e $script $module $original.png.path);Need ($LASTEXITCODE-eq0);$dimensions=($parsed-join'')|ConvertFrom-Json -DateKind String
 Need ((Same $m.dimensions $dimensions)-and(Same $original.nativeDimensions @($dimensions.width,$dimensions.height))-and(Same $dimensions.width $window.bounds.width)-and(Same $dimensions.height $window.bounds.height))
 foreach($key in @('tool','args','runID','batch','callStarted','callCompleted','precedingActions','outerProtocolID','innerProtocolID')){Need (Same $m.$key $posts[3].$key)}
 Need (Same $r.original ([pscustomobject]@{originalInspected=$true;dimensions=$original.nativeDimensions;pngSha256=$original.png.sha256;visibleAssertion=$original.visibleAssertion;isFocused=$true;windowId=$window.windowId;pid=$window.pid;freshness='UNKNOWN';inputSent=$false}))
 Need (Same $o.smallestReadOnlyMismatch ([pscustomobject]@{predicate='m.rawPath===raw.path';automaticMetadataRawPath=$original.raw.path;selectedCallRawPath=$o.B1.calls[3].raw.path;automaticRaw=$original.raw;callRaw=$o.B1.calls[3].raw;noSourceOrHelperChange=$true;noRetryOrTimeoutChange=$true}))
 Assert-NativeSeedFields $e.helper @('path','sha256');Need ((Same $e.helper.path (Join-Path $v.scenario 'construct-actual-phase-inputs.private.mjs'))-and(Same $p.driverMechanicalScope.metadataHelper $e.helper));Assert-NativeSeedOrdinary $e.helper.path
 Need ((Get-Item -LiteralPath $e.helper.path).Length-le262144-and(Get-FileHash -LiteralPath $e.helper.path).Hash-ieq$e.helper.sha256-and[IO.File]::ReadAllText($e.helper.path).Contains('m.rawPath===raw.path'))
 $packet=$a.metadataCaller;Need ($packet-is[array]-and$packet.Count-eq1);$entry=$packet[0];Assert-NativeSeedFields $entry @('label','args','raw','polls')
 Exact-Integer $entry.raw.exit_code 1;Need ((Same $entry.label 'B1-PLAY')-and(Same $entry.polls @())-and@($entry.raw.PSObject.Properties.Name)-cnotcontains'session_id'-and$entry.raw.output-is[string]-and[Text.Encoding]::UTF8.GetByteCount($entry.raw.output)-le8192-and$entry.raw.output.Contains('Error: Actual automatic/raw/source linkage required')-and$entry.raw.output.Contains('at selectedCapture')-and$entry.raw.output.Contains('Geometry failed; no mapping/input'))
 Need (($entry.raw.wall_time_seconds-is[double]-or$entry.raw.wall_time_seconds-is[long]-or$entry.raw.wall_time_seconds-is[int])-and[double]::IsFinite($entry.raw.wall_time_seconds)-and$entry.raw.wall_time_seconds-ge0)
 $cmd=$entry.args.cmd;Need ($cmd-is[string]-and$cmd.Length-le8192);$timeMatch=[regex]::Match($cmd,' capture-001 ([0-9]+);if\(\$LASTEXITCODE-ne0\)');$point=[regex]::Match($cmd,' capture-001 ([0-9]+) ([0-9]+) ([0-9]+) \$g.geometry.path');Need ($timeMatch.Success-and$point.Success)
 $inspected=[long]$timeMatch.Groups[1].Value;Need ($inspected-ge$previous-and$inspected-le(Link-Millis $terminal.at)-and$point.Groups[3].Value-ceq$timeMatch.Groups[1].Value-and[long]$point.Groups[1].Value-lt$dimensions.width-and[long]$point.Groups[2].Value-lt$dimensions.height)
 $h=$e.helper.path.Replace('\','/');$n=$Node.Replace('\','/');$run=$ready.runID;$x=$point.Groups[1].Value;$y=$point.Groups[2].Value
 $expected='$ErrorActionPreference=''Stop'';$h='''+$h+''';$n='''+$n+''';$gRaw=& $n $h geometry B1-PLAY '+$run+' call-004 capture-001 '+$inspected+';if($LASTEXITCODE-ne0){throw ''Geometry failed; no mapping/input''};$g=$gRaw|ConvertFrom-Json;$mRaw=& $n $h play-point '+$run+' call-004 capture-001 '+$x+' '+$y+' '+$inspected+' $g.geometry.path $g.geometry.sha256 $g.terminal.sha256;if($LASTEXITCODE-ne0){throw ''Play mapping failed; no input''};$m=$mRaw|ConvertFrom-Json;$cRaw=& $n $h point-check B1-PLAY $m.coordinateProof.point.sha256 $m.coordinateProof.envelope.sha256;if($LASTEXITCODE-ne0){throw ''Point-check failed; no input''};$c=$cRaw|ConvertFrom-Json;$geo=Get-Content -LiteralPath $g.geometry.path -Raw|ConvertFrom-Json;[ordered]@{check=$c;geometryObservedAtMs=[DateTimeOffset]::Parse($geo.observedAtUtc).ToUnixTimeMilliseconds()}|ConvertTo-Json -Depth 12 -Compress'
 Need ($cmd-ceq$expected-and(Same $o.helperInvocation ([pscustomobject]@{modeAttempted='geometry B1-PLAY';toolChunkId=$entry.raw.chunk_id;status=1;message='Actual automatic/raw/source linkage required';actualCallerPacket=$e.metadataCaller;helper=$e.helper;geometryChildLaunched=$false;mappingAttempted=$false;pointCheckAttempted=$false;nativeInputAttempted=$false;timingFailure=$false}))-and(Same $r.helperFailure ([pscustomobject]@{mode='geometry B1-PLAY';actualToolChunk=$entry.raw.chunk_id;actualExitCode=1;message='Actual automatic/raw/source linkage required';predicate='m.rawPath===raw.path';automaticRawBasename='capture-001.raw.private.json';callRawBasename='call-004.raw.private.json';bothRawSha256=$original.raw.sha256;geometryChildLaunched=$false;timingFailure=$false;noRetryOrTimeoutChange=$true;noCauseFixedClaim=$true}))-and(Link-Millis $o.atUtc)-ge(Link-Millis $terminal.at)-and(Link-Millis $o.atUtc)-le(Link-Millis $a.protection.at))
# PRIVATE PROPOSED SNIPPET ONLY. Existing newcase variables/functions in scope.
# Use historical $powershell from parent core, not Get-Command/current runtime.
function Link-PrivatePin([string]$Name){
 $pin=$p.preclockHelpers.$Name;$path=Join-Path $v.scenario $Name
 Assert-NativeSeedFields $pin @('path','sha256');Link-Strings $pin @('path','sha256');Assert-NativeSeedHash $pin.sha256
 Need (Same $pin.path $path);Assert-NativeSeedOrdinary $path
 Need ((Get-Item -LiteralPath $path).Length-le262144-and(Get-FileHash -LiteralPath $path).Hash.ToLowerInvariant()-ceq$pin.sha256)
 return $pin
}
$callerPin=Link-PrivatePin 'launch-owned-caller.private.ps1'
$stagerPin=Link-PrivatePin 'record-root-call-and-stage-B1.private.ps1'
$orchestrationPin=Link-PrivatePin 'launch-and-stage.orchestration.proposed.private.js'
Need (Same (Link-PrivatePin 'construct-actual-phase-inputs.private.mjs') $e.helper)
$history=$p.historyAdmission;$binding=$p.fixtureBinding
foreach($row in @(@($history,'root-history-baseline.admitted.private.json'),@($binding,'fixture-binding.private.json'))){
 $pin=$row[0];Assert-NativeSeedFields $pin @('path','sha256');Link-Strings $pin @('path','sha256');Assert-NativeSeedHash $pin.sha256
 Need (Same $pin.path (Join-Path $v.scenario $row[1]));Assert-NativeSeedOrdinary $pin.path
 Need ((Get-Item -LiteralPath $pin.path).Length-le262144-and(Get-FileHash -LiteralPath $pin.path).Hash.ToLowerInvariant()-ceq$pin.sha256)
}
Need ((Same $history $a.lease.historyAdmission)-and(Same $p.rootCallContract.path $e.rootCall.path)-and(Same $p.rootCallContract.key $a.rootCall.key))
$delegationPath=Join-Path $v.scenario 'root-postlaunch-mechanical-delegation.private.json'
Assert-NativeSeedOrdinary $delegationPath;Need ((Get-Item -LiteralPath $delegationPath).Length-le262144)
$delegationPin=[pscustomobject]@{path=$delegationPath;sha256=(Get-FileHash -LiteralPath $delegationPath).Hash.ToLowerInvariant()}
$d=Read-Ref $delegationPin $delegationPath
Exact-Integer $d.schemaVersion 1
foreach($key in @('kind','issuer','driver','planSha256','leaseSha256','helperSha256','callerSha256','stagerSha256','orchestrationSha256','historyAdmissionSha256')){Need ($d.$key-is[string])}
Need ((Same $d.kind 'ROOT_BOUNDED_POSTLAUNCH_MECHANICAL_DELEGATION')-and(Same $d.issuer '/root')-and(Same $d.driver $v.driver)-and
 (Same $d.planSha256 $e.plan.sha256)-and(Same $d.leaseSha256 $e.lease.sha256)-and(Same $d.helperSha256 $e.helper.sha256)-and
 (Same $d.callerSha256 $callerPin.sha256)-and(Same $d.stagerSha256 $stagerPin.sha256)-and(Same $d.orchestrationSha256 $orchestrationPin.sha256)-and
 (Same $d.historyAdmissionSha256 $history.sha256)-and(Same $d.allowedModes @('init','phase B1')))
function Link-PSQuote([string]$Value){return "'"+$Value.Replace("'","''")+"'"}
$expectedRootCmd='& '+(Link-PSQuote $powershell)+' -NoProfile -NonInteractive -File '+(Link-PSQuote $callerPin.path)+
 ' -PlanSha256 '+(Link-PSQuote $e.plan.sha256)+' -LeaseSha256 '+(Link-PSQuote $e.lease.sha256)+
 ' -DelegationSha256 '+(Link-PSQuote $delegationPin.sha256)+' -HelperSha256 '+(Link-PSQuote $e.helper.sha256)+
 ' -HistoryAdmissionSha256 '+(Link-PSQuote $history.sha256)
Need ($a.rootCall.value.args.cmd-is[string]-and$a.rootCall.value.args.cmd-ceq$expectedRootCmd)
$expectedOuterArgs=@('-NoProfile','-NonInteractive','-File',(Join-Path $v.sourceWorkspace 'tools/starsector-proxy/fixture/native-controller.ps1'),
 '-EvidenceRoot',$v.scenario,'-BindingPath',$binding.path,'-BindingSha256',$binding.sha256,
 '-OuterDeadlineUtc',$c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000')
$outerStep=$a.launch.result.steps[0]
Need ((Same $outerStep.command $powershell)-and(Same $outerStep.args $expectedOuterArgs)-and(Same $outerStep.cwd '.'))
Need (Same $a.result.evidence.actualMetadataCalls $e.metadataCaller)

}
