# Historical known Play custody. Caller supplies the already-verified terminal graph.
function Assert-NativeKnownPlayFailureCustody($Receipt,$Values,$Evidence,[string]$Output,$Returned) {
 $v=$Receipt;$a=$Values;$e=$Evidence;$r=$a.result;$l=$a.ledger;$p=$a.plan;$c=$a.controller
 $o=$a.observation;$ready=$a.providerReady;$terminal=$a.providerTerminal;$action=$l.pending;$before=$p.historyBaseline
 function Millis($Value){Need ($Value-is[string]);return [DateTimeOffset]::Parse($Value).ToUnixTimeMilliseconds()}
 function Strings($Value,[string[]]$Fields){foreach($field in $Fields){Need ($Value.$field-is[string])}}
 function Raw-Result($Value){if(@($Value.PSObject.Properties.Name)-ccontains'isError'){Need ($Value.isError-is[bool]-and!$Value.isError)}}
 function Known-Window($Window){
  Assert-NativeSeedInteger $Window.windowId 1 9007199254740991;Assert-NativeSeedInteger $Window.pid 1 9007199254740991
  Strings $Window @('bundleId');Need ($Window.bundleId-ceq'java.exe'-and$Window.isFocused-is[bool]-and$Window.isFocused)
  Assert-NativeSeedFields $Window.bounds @('x','y','width','height')
  foreach($key in @('x','y')){Assert-NativeSeedInteger $Window.bounds.$key -9007199254740991 9007199254740991}
  foreach($key in @('width','height')){Assert-NativeSeedInteger $Window.bounds.$key 1 9007199254740991}
 }
 Strings $v @('kind','issuer','driver','scenario','sourceWorkspace','sourceHead')
 foreach($key in $e.PSObject.Properties.Name){Strings $e.$key @('path','sha256')}
 Strings $r @('kind','status','verdict','failureCategory','scenarioId','driver','sourceHead')
 Strings $p @('scenarioId');Strings $p.paths @('scenario');Strings $p.source @('qualifiedHead');Strings $p.campaignControl @('profile');Strings $p.ownership @('driver')
 Strings $l @('kind','controllerSha256','b1ClaimSha256');Strings $l.provider @('batch','runID','sourceSha256')
 Strings $a.lease @('scenarioId','driver','issuer','supportPinsPath');Strings $a.lease.campaignControl @('profile')
 Strings $c @('installation','controllerCreationTime','controllerKernelCreationFileTime')
 Assert-NativeSeedInteger $c.sessionId 0 9007199254740991
 Strings $a.owned @('controllerCreationTime','controllerKernelCreationFileTime')
 Strings $a.source @('scenario','batch','ownedReceiptPath','ownedReceiptSha256');Strings $a.claim @('sourcePath','sourceSha256')
 Strings $a.claim.binding @('batch','scenario','workspaceRoot','runID','sourceSha256','originalPlanSha256','controllerSha256')
 Strings $a.rootCall @('key');Strings $a.caller @('kind')
 Strings $ready @('at','batch','runID','sourceSha256','consentMode','query');Strings $ready.node @('executable');Strings $ready.helper @('executable')
 Strings $terminal @('at','verdict','terminalReceipt');Strings $terminal.steps[0] @('command','cwd','processOwnership')
 Strings $a.providerSpool @('command');Strings $a.providerSpool.result @('verdict')
 Strings $a.launch @('command');Strings $a.launch.result @('verdict');Strings $a.launch.result.steps[0] @('processOwnership')
 Strings $Returned @('verdict')
 Strings $a.protection @('at','phase');Strings $a.pidAbsence @('atUtc')
 Exact-Integer $ready.node.sessionId $c.sessionId;Exact-Integer $ready.helper.sessionId $c.sessionId
 Assert-NativeSeedFields $before @('overallAttempts','uiAttempts','reservedActions','sentActions')
 foreach($key in $before.PSObject.Properties.Name){Assert-NativeSeedInteger $before.$key 0 9007199254740991}
 Need ($before.sentActions-le$before.reservedActions)
 $guard=[pscustomobject]@{overallAttempts=$before.overallAttempts;uiAttempts=$before.uiAttempts;reservedActions=($before.reservedActions+1);sentActions=($before.sentActions+1)}
 $context=[pscustomobject]@{overallAttempts=($before.overallAttempts+1);uiAttempts=($before.uiAttempts+1)}
 Need ((Same $l.history $guard)-and(Same $r.inheritedGuardBaseline $before)-and(Same $v.actions.currentRunContext $context))
 Assert-NativeSeedInteger $r.absoluteMaxReservedActions ($before.reservedActions+1) ($before.reservedActions+8)
 Assert-NativeSeedFields $action @('batch','runID','sourceSha256','reservationID','tool','args','status','wizardStep','sentAtMs','knownResultAtMs')
 Strings $action @('batch','runID','sourceSha256','reservationID','tool','status');Strings $action.args @('focus_strategy')
 if(@($action.args.PSObject.Properties.Name)-ccontains'target_app'){
  Assert-NativeSeedFields $action.args @('coordinate','target_window_id','focus_strategy','target_app');Strings $action.args @('target_app');Need ($action.args.target_app-ceq'java.exe')
 }else{Assert-NativeSeedFields $action.args @('coordinate','target_window_id','focus_strategy')}
 Need ($action.batch-ceq'B1'-and$action.runID-ceq$ready.runID-and$action.sourceSha256-ceq$e.source.sha256-and$action.reservationID-ceq($e.controller.sha256+':'+$l.history.reservedActions)-and$action.tool-ceq'left_click'-and$action.status-ceq'AWAITING_AFTER'-and(Same $action.wizardStep ([pscustomobject]@{ordinal=1;stage='PLAY'}))-and$action.args.focus_strategy-ceq'strict')
 Exact-Integer $action.wizardStep.ordinal 1
 Assert-NativeSeedInteger $action.sentAtMs 0 9007199254740991;Assert-NativeSeedInteger $action.knownResultAtMs $action.sentAtMs 9007199254740991
 Need ($action.args.coordinate-is[array]-and$action.args.coordinate.Count-eq2)
 foreach($number in $action.args.coordinate){Assert-NativeSeedInteger $number -9007199254740991 9007199254740991}
 foreach($key in @('resolvedB1','resolvedE1','resolvedE2')){Need (@($l.PSObject.Properties.Name)-cnotcontains$key)}
 Need ((Same $r.providers ([pscustomobject]@{started=1;B1='TERMINAL_PASS_REAPED';E1='NOT_STARTED';E2='NOT_STARTED';calls=14})))
 Exact-Integer $o.schemaVersion 1
 Strings $o @('kind','category','verdict','driver','scenarioId','sourceHead','E1','E2')
 Need ($o.kind-ceq'ACTUAL_DRIVER_HANDOFF_CUSTODY_STOP'-and$o.category-ceq$r.failureCategory-and$o.verdict-ceq'FAIL'-and$o.driver-ceq$v.driver-and$o.scenarioId-ceq$p.scenarioId-and$o.sourceHead-ceq$v.sourceHead-and(Same $o.actualRunContext $r.actualRunContext)-and(Same $o.guardHistory $l.history)-and$o.E1-ceq'NOT_STARTED'-and$o.E2-ceq'NOT_STARTED')
 Need ($o.seedClaim-is[bool]-and!$o.seedClaim-and$o.allHistoryRetained-is[bool]-and$o.allHistoryRetained-and(Same $o.providerTerminal $e.providerTerminal)-and(Same $o.ledger $e.ledger)-and(Same $o.rootCall $e.rootCall))
 $tools=@('doctor','policy_status','get_window','screenshot','get_frontmost_app','get_window','left_click','get_frontmost_app','list_windows','get_window','screenshot','get_frontmost_app','get_window','screenshot')
 Need ($o.calls-is[array]-and$o.calls.Count-eq14-and$o.originals-is[array]-and$o.originals.Count-eq3)
 $attempts=@(Get-ChildItem -LiteralPath $Output -File|Where-Object{$_.Name-cmatch'^call-\d+\.attempt\.private\.json$'}|Sort-Object Name|ForEach-Object{$_.Name})
 Need (Same $attempts @(1..14|ForEach-Object{'call-'+$_.ToString('000')+'.attempt.private.json'}))
 foreach($batch in @('E1','E2')){Need (!(Test-Path -LiteralPath (Join-Path $v.scenario ('guard-'+$batch+'-consumed.private.json'))))}
 $posts=@();$raws=@();$previous=Millis $ready.at
 for($i=0;$i-lt14;$i++){
  $row=$o.calls[$i];Assert-NativeSeedFields $row @('post','raw','attempt');$prefix=Join-Path $Output ('call-'+($i+1).ToString('000'))
  foreach($key in @('post','raw','attempt')){Strings $row.$key @('path','sha256')}
  $post=Read-Ref $row.post ($prefix+'.private.json');$attempt=Read-Ref $row.attempt ($prefix+'.attempt.private.json');$raw=Read-Ref $row.raw ($prefix+'.raw.private.json') 33554432
  Raw-Result $raw
  Strings $post @('tool','runID','batch','callStarted','callCompleted');Strings $attempt @('tool','runID','batch','sourceSha256','callStarted')
  Need ($post.tool-ceq$tools[$i]-and$attempt.tool-ceq$tools[$i]-and$post.runID-ceq$ready.runID-and$attempt.runID-ceq$ready.runID-and$post.batch-ceq'B1'-and$attempt.batch-ceq'B1'-and$attempt.sourceSha256-ceq$e.source.sha256-and(Same $post.args $attempt.args)-and$attempt.callStarted-ceq$post.callStarted-and$post.isError-is[bool]-and!$post.isError-and$raw.isError-ne$true)
  $start=Millis $post.callStarted;$end=Millis $post.callCompleted
  Need ($start-ge$previous-and$end-ge$start-and$end-le(Millis $terminal.at))
  $preceding=if($i-le6){,@()}else{,@($posts[6])};Need (Same $post.precedingActions $preceding)
  $previous=$end;$posts+=,$post;$raws+=,$raw
 }
 $click=$posts[6];$ack=$raws[6].content
 Need ((Same $click.args $action.args)-and(Millis $click.callStarted)-le$action.sentAtMs-and$action.knownResultAtMs-le(Millis $click.callCompleted)-and$ack-is[array]-and$ack.Count-eq1-and(Same $ack[0] ([pscustomobject]@{type='text';text=('Clicked ('+($action.args.coordinate-join', ')+')')})))
 Need ($raws[0].structuredContent.ok-is[bool]-and$raws[0].structuredContent.ok-and$raws[1].structuredContent.approval_required_for_all-is[bool]-and$raws[1].structuredContent.approval_required_for_all-and$raws[1].structuredContent.approval_token_configured-is[bool]-and!$raws[1].structuredContent.approval_token_configured)
 $old=$raws[2].structuredContent;$current=$raws[12].structuredContent
 Known-Window $old;Known-Window $current;Exact-Integer $action.args.target_window_id $old.windowId
 Assert-NativeSeedInteger $current.windowId 1 9007199254740991
 Need ($old.windowId-eq$a.claim.binding.campaign.windowId-and$action.args.target_window_id-eq$old.windowId-and$current.pid-eq$old.pid-and$old.bundleId-ceq'java.exe'-and$current.bundleId-ceq'java.exe'-and$old.isFocused-is[bool]-and$old.isFocused-and$current.isFocused-is[bool]-and$current.isFocused)
 foreach($i in @(4,7,11)){Exact-Integer $raws[$i].structuredContent.app.pid $old.pid;Strings $raws[$i].structuredContent.app @('bundleId');Need ($raws[$i].structuredContent.app.bundleId-ceq'java.exe')}
 foreach($pair in @(@(2,$old),@(5,$old),@(9,$current),@(12,$current))){
  $i=$pair[0];$window=$pair[1];$actual=$raws[$i].structuredContent
  Known-Window $actual;Exact-Integer $actual.windowId $window.windowId;Exact-Integer $actual.pid $old.pid
  Need ((Same $posts[$i].args ([pscustomobject]@{window_id=$window.windowId}))-and$actual.windowId-eq$window.windowId-and$actual.pid-eq$old.pid-and$actual.bundleId-ceq'java.exe'-and$actual.isFocused-is[bool]-and$actual.isFocused-and(Same $actual.bounds $window.bounds))
 }
 $listed=$raws[8].structuredContent.windows;Need ($listed-is[array])
 $matched=@($listed|Where-Object{($_.windowId-is[int]-or$_.windowId-is[long])-and($_.pid-is[int]-or$_.pid-is[long])-and$_.windowId-eq$current.windowId-and$_.pid-eq$old.pid})
 Need ($matched.Count-eq1);Known-Window $matched[0]
 $initial=Read-Ref ([pscustomobject]@{path=$a.source.ownedReceiptPath;sha256=$a.source.ownedReceiptSha256}) (Join-Path $v.scenario 'driver-B1-owned.private.json')
 $firstRows=@($initial.processes|Where-Object{$_.pid-eq$old.pid});$lastRows=@($a.owned.processes|Where-Object{$_.pid-eq$old.pid})
 Need ($firstRows.Count-eq1-and$lastRows.Count-eq1);$first=$firstRows[0];$last=$lastRows[0]
 Strings $first @('creationTime','kernelCreationFileTime','executable');Strings $last @('creationTime','kernelCreationFileTime','executable')
 Exact-Integer $first.sessionId $c.sessionId;Exact-Integer $last.sessionId $c.sessionId
 Assert-NativeSeedInteger $first.parentPid 1 9007199254740991;Assert-NativeSeedInteger $old.pid 1 9007199254740991
 Need ([long]$first.mainWindowHandle-eq$old.windowId-and[long]$last.mainWindowHandle-eq$current.windowId-and$a.claim.binding.ownedPids-contains$old.pid-and$first.creationTime-is[string]-and$first.kernelCreationFileTime-is[string]-and$first.kernelCreationFileTime-cmatch'^[1-9][0-9]*$'-and$first.executable-ceq(Join-Path $c.installation 'jre/bin/java.exe')-and$first.sessionId-eq$c.sessionId)
 $null=Millis $first.creationTime
 foreach($key in @('pid','parentPid','creationTime','kernelCreationFileTime','executable','sessionId')){Need (Same $first.$key $last.$key)}
 foreach($row in @($first,$last)){
  Need ($row.inSupervisorJob-is[bool]-and$row.inSupervisorJob-and$row.supervisorJobPids-is[array])
  foreach($pidValue in @($c.controllerPid,$old.pid)+@($c.supervisorJobPids)){Need ($row.supervisorJobPids-contains$pidValue)}
 }
 Need ($a.pidAbsence.queriedExactRecordedPids-contains$old.pid)
 $expectedActions=[pscustomobject]@{reservedThisRun=1;sentThisRun=1;clicks=1;activations=0;stage='PLAY';coordinate=$action.args.coordinate;originalWindow=$old.windowId;replacementWindow=$current.windowId;ownedPid=$old.pid;status='AWAITING_AFTER';uncertain=$false}
 $expectedObservation=[pscustomobject]@{clicks=1;activations=0;coordinate=$action.args.coordinate;targetWindow=$old.windowId;pid=$old.pid;replacementWindow=$current.windowId;knownInput=$true;ledgerStatus='AWAITING_AFTER';uncertain=$false}
 Need ((Same $r.actions $expectedActions)-and(Same $o.actions $expectedObservation))
 Need ($action.args.coordinate[0]-ge$old.bounds.x-and$action.args.coordinate[0]-lt($old.bounds.x+$old.bounds.width)-and$action.args.coordinate[1]-ge$old.bounds.y-and$action.args.coordinate[1]-lt($old.bounds.y+$old.bounds.height))
 $pngPaths=@();$callIndexes=@(3,10,13)
 for($i=0;$i-lt3;$i++){
  $row=$o.originals[$i];Assert-NativeSeedFields $row @('png','metadata','raw','originalInspected','freshness')
  Strings $row @('freshness');foreach($key in @('png','metadata','raw')){Strings $row.$key @('path','sha256')}
  Need ($row.originalInspected-is[bool]-and$row.originalInspected-and$row.freshness-ceq'UNKNOWN')
  $prefix=Join-Path $Output ('capture-'+($i+1).ToString('000'));$metadata=Read-Ref $row.metadata ($prefix+'.metadata.private.json');$raw=Read-Ref $row.raw ($prefix+'.raw.private.json') 33554432
  Strings $metadata @('tool','runID','batch','callStarted','callCompleted','pngPath','pngSha256','rawPath','rawSha256');Raw-Result $raw
  Assert-NativeSeedFields $row.png @('path','sha256');Need ($row.png.path-ceq($prefix+'.png'));Assert-NativeSeedOrdinary $row.png.path
  Need ((Get-Item -LiteralPath $row.png.path).Length-le33554432-and(Get-FileHash -LiteralPath $row.png.path).Hash-ieq$row.png.sha256)
  $bytes=[IO.File]::ReadAllBytes($row.png.path);$post=$posts[$callIndexes[$i]];$window=if($i-eq0){$old}else{$current}
  Need ($raw.content-is[array]);$images=@($raw.content|Where-Object{$_.type-is[string]-and$_.type-ceq'image'})
  Need ($images.Count-eq1);Strings $images[0] @('type','mimeType','data')
  Known-Window $metadata.window;Exact-Integer $metadata.window.windowId $window.windowId;Exact-Integer $metadata.window.pid $old.pid
  Exact-Integer $post.args.quality 0;Exact-Integer $post.args.width $window.bounds.width;Exact-Integer $post.args.target_window_id $window.windowId
  Need ((Same $raw $raws[$callIndexes[$i]])-and$images.Count-eq1-and$images[0].mimeType-ceq'image/png'-and$images[0].data-ceq[Convert]::ToBase64String($bytes))
  Need ($metadata.captureReceipt-is[bool]-and$metadata.captureReceipt-and$metadata.pngPath-ceq$row.png.path-and$metadata.pngSha256-ceq$row.png.sha256-and$metadata.rawPath-ceq$row.raw.path-and$metadata.rawSha256-ceq$row.raw.sha256-and$metadata.tool-ceq'screenshot'-and$metadata.runID-ceq$ready.runID-and$metadata.batch-ceq'B1')
  Need ($metadata.callStarted-ceq$post.callStarted-and$metadata.callCompleted-ceq$post.callCompleted-and(Same $metadata.precedingActions $post.precedingActions)-and(Same $metadata.args $post.args)-and$post.args.target_window_id-eq$window.windowId-and$post.args.quality-eq0-and$post.args.width-eq$window.bounds.width-and$metadata.window.windowId-eq$window.windowId-and$metadata.window.pid-eq$old.pid-and$metadata.window.isFocused-is[bool]-and$metadata.window.isFocused-and(Same $metadata.window.bounds $window.bounds))
  $pngPaths+=,$row.png.path
 }
 # Reuse the maintained complete PNG parser only; history/action/process checks above remain PowerShell.
 $module=Join-Path $PSScriptRoot '../direct-frames.mjs';$node=Historical-Executable 'node.exe'
 $script='import {readFileSync} from "node:fs";import {pathToFileURL} from "node:url";const {completePng}=await import(pathToFileURL(process.argv[1]));process.stdout.write(JSON.stringify(process.argv.slice(2).map(p=>completePng(readFileSync(p)))));'
 $parsed=@(& $node --input-type=module -e $script $module @pngPaths);Need ($LASTEXITCODE-eq0)
 $dimensions=($parsed-join'')|ConvertFrom-Json -DateKind String;Need ($dimensions.Count-eq3)
 for($i=0;$i-lt3;$i++){$window=if($i-eq0){$old}else{$current};$metadata=Read-Ref $o.originals[$i].metadata (Join-Path $Output ('capture-'+($i+1).ToString('000')+'.metadata.private.json'));Need ((Same $metadata.dimensions $dimensions[$i])-and$dimensions[$i].width-eq$window.bounds.width-and$dimensions[$i].height-eq$window.bounds.height)}
}
