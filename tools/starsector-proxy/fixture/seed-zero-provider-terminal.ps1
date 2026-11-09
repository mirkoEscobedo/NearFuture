# Uses the strict local Need/Same/Exact-Integer helpers of the zero-provider reader.
function ConvertFrom-ZeroProviderObject([string]$Text) {
 $document=[Text.Json.JsonDocument]::Parse($Text)
 function Assert-ZeroProviderFields($Element){
  if($Element.ValueKind-eq[Text.Json.JsonValueKind]::Object){
   $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
   foreach($prop in $Element.EnumerateObject()){if(!$seen.Add($prop.Name)){throw 'Duplicate zero-provider JSON field refused'};Assert-ZeroProviderFields $prop.Value}
  }elseif($Element.ValueKind-eq[Text.Json.JsonValueKind]::Array){foreach($child in $Element.EnumerateArray()){Assert-ZeroProviderFields $child}}
 }
 try{if($document.RootElement.ValueKind-ne[Text.Json.JsonValueKind]::Object){throw 'Zero-provider JSON object required'};Assert-ZeroProviderFields $document.RootElement}finally{$document.Dispose()}
 return $Text|ConvertFrom-Json -Depth 100 -DateKind String -NoEnumerate
}
function Read-NativeZeroProviderJson([string]$Path,[string]$Hash) {
 Assert-NativeSeedHash $Hash;if($Hash-cnotmatch'^[a-f0-9]{64}$'){throw 'Canonical zero-provider digest required'}
 Assert-NativeSeedOrdinary $Path;$item=Get-Item -LiteralPath $Path -Force
 if($item.PSIsContainer-or$item.Length-lt2-or$item.Length-gt262144){throw 'Bounded zero-provider JSON required'}
 $bytes=[IO.File]::ReadAllBytes($Path)
 if($bytes.Length-ne$item.Length-or[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes)).ToLowerInvariant()-cne$Hash){throw 'Zero-provider JSON digest changed'}
 return ConvertFrom-ZeroProviderObject ([Text.UTF8Encoding]::new($false,$true).GetString($bytes))
}
function Read-ZeroProviderStream($First,$Polls,[long]$Status) {
 Need ($Polls-is[array]-and$Polls.Count-le256);$rows=@($First)+@($Polls);$text='';$session=$First.session_id
 for($i=0;$i-lt$rows.Count;$i++){
  $r=$rows[$i];Need ($r.output-is[string]-and($r.wall_time_seconds-is[int]-or$r.wall_time_seconds-is[long]-or$r.wall_time_seconds-is[double]-or$r.wall_time_seconds-is[decimal])-and[double]::IsFinite($r.wall_time_seconds)-and$r.wall_time_seconds-ge0)
  Need ([Text.Encoding]::UTF8.GetByteCount($r.output)-le65536);$text+=$r.output;Need ([Text.Encoding]::UTF8.GetByteCount($text)-le131072)
  if($i-eq$rows.Count-1){Exact-Integer $r.exit_code $Status;Need (@($r.PSObject.Properties.Name)-cnotcontains'session_id')}
  else{Assert-NativeSeedInteger $session 1 9007199254740991;Exact-Integer $r.session_id $session;Need ($null-eq$r.exit_code)}
 }
 return $text
}
function Assert-NativeZeroProviderTerminal($v,$a,$e,[string]$powershell) {
 $rootCall=$a.rootCall;$caller=$a.caller;$launch=$a.launch;$x=$a.combined;$f=$a.frozen;$z=$a.freezeSupervisor;$p=$a.plan;$c=$a.controller
 Assert-NativeSeedFields $x @('launch','stage','stagePolls','stop','stopPolls','launchPolls','error','b1Admit','remainingStageSession','remainingLaunchSession')
 Assert-NativeSeedFields $x.stage @('args','raw');Assert-NativeSeedFields $x.error @('name','message');Assert-NativeSeedFields $z @('args','raw','polls','priorRejectedCall')
 Strings $launch @('command');Strings $x.error @('name','message');Strings $x.stage.args @('cmd','workdir')
 Strings $f @('issuer','planSha256','leaseSha256','delegationSha256','helperSha256','historyAdmissionSha256')
 Assert-NativeSeedFields $caller @('schemaVersion','kind','session','launchRaw','launchPolls','actual','terminalReaped')
 Strings $caller @('kind')
 Exact-Integer $caller.schemaVersion 1;Exact-Integer $caller.session $rootCall.value.raw.session_id
 Need ($caller.kind-ceq'ACTUAL_ORIGINAL_LAUNCH_CALLER_TERMINAL'-and$caller.terminalReaped-is[bool]-and$caller.terminalReaped-and(Same $caller.launchRaw $rootCall.value.raw)-and$caller.launchPolls-is[array]-and$caller.launchPolls.Count-ge1-and(Same $caller.actual $caller.launchPolls[-1]))
 $returned=ConvertFrom-ZeroProviderObject (Read-ZeroProviderStream $caller.launchRaw $caller.launchPolls 1);$step=$launch.result.steps[0]
 Strings $step @('command','cwd','stdout','stderr','processOwnership');Strings $launch.result @('verdict');Strings $returned @('verdict')
 Exact-Integer $launch.schemaVersion 1;Exact-Integer $step.status 1
 $selected=[pscustomobject]@{status=$step.status;timedOut=$step.timedOut;processOwnership=$step.processOwnership;stdout=$step.stdout;stderr=$step.stderr}
 $returnedStep=if($returned.steps-is[array]){$returned.steps[0]}else{$returned.steps}
 Need ($launch.command-ceq'verify'-and$launch.ok-is[bool]-and$launch.ok-and$launch.result.verdict-ceq'FAIL'-and$launch.result.steps.Count-eq1-and$step.timedOut-is[bool]-and!$step.timedOut-and$step.processOwnership-ceq'windows-job-object'-and$step.command-ceq$powershell-and$step.cwd-ceq'.'-and$step.stderr.Contains('Driver requested owned cleanup; native success not established')-and$returned.verdict-ceq'FAIL'-and(Same $returned.blocked @())-and($returned.steps-isnot[array]-or$returned.steps.Count-eq1)-and(Same $returnedStep $selected))
 Need (Same $step.args @('-NoProfile','-NonInteractive','-File',(Join-Path $v.sourceWorkspace 'tools/starsector-proxy/fixture/native-controller.ps1'),'-EvidenceRoot',$v.scenario,'-BindingPath',$p.fixtureBinding.path,'-BindingSha256',$p.fixtureBinding.sha256,'-OuterDeadlineUtc',$c.conservativeOuterDeadlineUtc,'-SessionWatchdogMs','480000'))
 Need ($x.error.name-ceq'Error'-and$x.error.message-ceq'Actual B1 stager not known terminal exit0; no provider'-and$null-eq$x.b1Admit-and$null-eq$x.remainingStageSession-and$null-eq$x.remainingLaunchSession-and(Same $x.launch $rootCall)-and(Same $x.launchPolls $caller.launchPolls))
 Assert-NativeSeedInteger $x.stage.raw.session_id 1 9007199254740991;Need ($x.stage.raw.session_id-ne$caller.session)
 $stageOutput=Read-ZeroProviderStream $x.stage.raw $x.stagePolls 1
 Need ($stageOutput.Contains('Error: Campaign authority refused: actual controller clock and identity required')-and$stageOutput.Contains('Actual unclaimed schema6 B1 admission refused; no retry'))
 $null=Read-ZeroProviderStream $x.stop $x.stopPolls 0
 function Q($Value){return "'"+([string]$Value).Replace("'","''")+"'"}
 $callerScript=$p.preclockHelpers.'launch-owned-caller.private.ps1';$stager=$p.preclockHelpers.'record-root-call-and-stage-B1.private.ps1'
 Need ($callerScript.path-ceq(Join-Path $v.scenario 'launch-owned-caller.private.ps1')-and$stager.path-ceq(Join-Path $v.scenario 'record-root-call-and-stage-B1.private.ps1'))
 $expected='& '+(Q $powershell)+' -NoProfile -NonInteractive -File '+(Q $callerScript.path)+' -PlanSha256 '+(Q $e.plan.sha256)+' -LeaseSha256 '+(Q $e.lease.sha256)+' -DelegationSha256 '+(Q $f.delegationSha256)+' -HelperSha256 '+(Q $f.helperSha256)+' -HistoryAdmissionSha256 '+(Q $f.historyAdmissionSha256)
 Need ($rootCall.value.args.workdir-ceq$v.sourceWorkspace-and$rootCall.value.args.cmd-ceq$expected)
 # ConvertTo-Json escapes some characters differently from JSON.stringify. Decode the
 # single quoted RootCallJson argument and compare exact structured data instead.
 $prefix='& '+(Q $powershell)+' -NoProfile -NonInteractive -File '+(Q $stager.path)+' -RootCallJson '
 $suffix=' -HelperSha256 '+(Q $f.helperSha256)+' -HistoryAdmissionSha256 '+(Q $f.historyAdmissionSha256)+' -DelegationSha256 '+(Q $f.delegationSha256)
 $cmd=$x.stage.args.cmd;Need ($x.stage.args.workdir-ceq$v.sourceWorkspace-and$cmd.StartsWith($prefix)-and$cmd.EndsWith($suffix))
 $argument=$cmd.Substring($prefix.Length,$cmd.Length-$prefix.Length-$suffix.Length);Need ($argument.StartsWith("'")-and$argument.EndsWith("'"))
 $embedded=ConvertFrom-ZeroProviderObject ($argument.Substring(1,$argument.Length-2).Replace("''","'"))
 Need ((Q ($embedded|ConvertTo-Json -Depth 100 -Compress -EscapeHandling Default))-ceq$argument-and(Same $embedded $rootCall))
 $report=ConvertFrom-ZeroProviderObject (Read-ZeroProviderStream $z.raw $z.polls 0);$fs=$report.result.steps[0]
 Exact-Integer $report.schemaVersion 1;Exact-Integer $fs.status 0
 Strings $report @('command');Strings $report.result @('verdict');Strings $fs @('command','cwd','processOwnership','stdout')
 $freeze=$p.preclockHelpers.'root-freeze-and-preflight.seed.proposed.private.ps1';$loaded=$p.source.loadedFrontendWitness
 Need ($fs.args-is[array]-and$fs.args.Count-eq13);foreach($arg in $fs.args){Need ($arg-is[string])};Need ([IO.Path]::IsPathFullyQualified($fs.args[3])-and[IO.Path]::IsPathFullyQualified($fs.args[10]))
 $freezeArgs=@($fs.args);$freezeArgs[3]=[IO.Path]::GetFullPath($freezeArgs[3]);$freezeArgs[10]=[IO.Path]::GetFullPath($freezeArgs[10])
 Need ($freeze.path-ceq(Join-Path $v.scenario 'root-freeze-and-preflight.seed.proposed.private.ps1')-and$fs.cwd-ceq'.'-and(Same $freezeArgs @('-NoProfile','-NonInteractive','-File',$freeze.path,'-RootAdmitted','-ProcedureSha256',$freeze.sha256,'-PreclockIndexSha256',$e.preclockIndex.sha256,'-LoadedFrontendWitnessPath',$loaded.path,'-LoadedFrontendWitnessSha256',$loaded.sha256)))
 Need ($report.command-ceq'verify'-and$report.ok-is[bool]-and$report.ok-and$report.result.verdict-ceq'PASS'-and$report.result.steps.Count-eq1-and$fs.timedOut-is[bool]-and!$fs.timedOut-and$fs.processOwnership-ceq'windows-job-object'-and$fs.command-is[string]-and[IO.Path]::IsPathFullyQualified($fs.command)-and[IO.Path]::GetFullPath($fs.command)-ceq$powershell-and(Same (ConvertFrom-ZeroProviderObject $fs.stdout) $f))
 Exact-Integer $f.schemaVersion 1;Exact-Integer $f.actualPsAdmissionExit 0;Exact-Integer $f.actualWatchdogMs 480000
 Exact-Integer $f.callerAdmissionDeadlineMs $p.callerAdmissionDeadlineMs;Exact-Integer $f.cleanupStartNoLaterMs ($p.callerAdmissionDeadlineMs-30000)
 Need ($f.issuer-ceq'/root'-and$f.planSha256-ceq$e.plan.sha256-and$f.leaseSha256-ceq$e.lease.sha256-and$f.historyAdmissionSha256-ceq$p.historyAdmission.sha256-and$f.callerAdmissionDeadlineMs-eq$p.callerAdmissionDeadlineMs-and$f.cleanupStartNoLaterMs-eq($p.callerAdmissionDeadlineMs-30000)-and$f.gameStarted-is[bool]-and!$f.gameStarted-and$f.providerStarted-is[bool]-and!$f.providerStarted-and(Same $f.actualPsAdmissionOutput @())-and$f.helperSha256-ceq$p.preclockHelpers.'construct-actual-phase-inputs.private.mjs'.sha256)
}
