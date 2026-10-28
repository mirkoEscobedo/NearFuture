# Modern generated-seed provenance. V1 keeps its exact legacy reader and bounds.
function Read-NativeModernSeedJson([string]$Path,[string]$Hash,[long]$Maximum=262144) {
 $value=Read-NativeSeedJson $Path $Hash $Maximum
 $text=[Text.UTF8Encoding]::new($false,$true).GetString([IO.File]::ReadAllBytes($Path))
 $document=[Text.Json.JsonDocument]::Parse($text)
 function Assert-UniqueSeedJson($Element) {
  if($Element.ValueKind-eq[Text.Json.JsonValueKind]::Object){
   $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
   foreach($p in $Element.EnumerateObject()){if(!$seen.Add($p.Name)){throw 'Duplicate modern seed JSON field refused'};Assert-UniqueSeedJson $p.Value}
  }elseif($Element.ValueKind-eq[Text.Json.JsonValueKind]::Array){foreach($e in $Element.EnumerateArray()){Assert-UniqueSeedJson $e}}
 }
 try{Assert-UniqueSeedJson $document.RootElement}finally{$document.Dispose()}
 return $value
}
function Assert-NativeModernSeedPins($Pins,$Binding) {
 Assert-NativeSeedFields $Pins @('pins')
 if($Pins.pins-isnot[array]-or$Pins.pins.Count-lt1-or$Pins.pins.Count-gt1024){throw 'Bounded complete modern seed pins required'}
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
 foreach($row in $Pins.pins){
  Assert-NativeSeedFields $row @('path','sha256');Assert-NativeSeedHash $row.sha256
  if($row.path-isnot[string]-or![IO.Path]::IsPathFullyQualified($row.path)-or![StringComparer]::Ordinal.Equals([IO.Path]::GetFullPath($row.path),$row.path)-or!$seen.Add($row.path)){throw 'Unique canonical modern seed pin required'}
  Assert-NativeSeedOrdinary $row.path
  if((Get-Item -LiteralPath $row.path).PSIsContainer-or(Get-FileHash -LiteralPath $row.path -Algorithm SHA256).Hash-ine$row.sha256){throw 'Modern seed source pin changed'}
 }
 foreach($name in @('fixture-inputs.ps1','seed-fixture.ps1','seed-provenance-v2.ps1','seed-creation-history.ps1','seed-failed-attempt.ps1','seed-known-play-attempt.ps1','seed-wrong-scene-attempt.ps1', 'seed-capture-linkage-attempt.ps1', 'seed-caller-date-attempt.ps1', 'seed-caller-date-geometry.ps1', 'seed-pre-start-margin-attempt.ps1','read-owned-client-geometry.ps1', 'seed-zero-provider-attempt.ps1', 'seed-zero-provider-terminal.ps1','seed-preference.ps1','prepare-fixture.ps1','protection-check.ps1')){
  if(!$seen.Contains([IO.Path]::GetFullPath((Join-Path $PSScriptRoot $name)))){throw 'Complete modern seed tooling closure required'}
 }
 foreach($name in @('seed-creation-custody.mjs','seed-creation-history.mjs','seed-failed-attempt.mjs','seed-known-play-attempt.mjs','seed-wrong-scene-attempt.mjs', 'seed-capture-linkage-attempt.mjs', 'seed-caller-date-attempt.mjs', 'seed-caller-date-geometry.mjs', 'seed-pre-start-margin-attempt.mjs', 'seed-pre-start-margin-records.mjs', 'seed-pre-start-margin-geometry.mjs', 'seed-zero-provider-attempt.mjs', 'seed-zero-provider-terminal.mjs','client-coordinate.mjs','direct-frames.mjs','seed-wizard-proof.mjs','seed-creation-output.mjs','campaign-proof.mjs','campaign-authority.mjs')){
  if(!$seen.Contains([IO.Path]::GetFullPath((Join-Path $PSScriptRoot ('../'+$name))))){throw 'Complete modern seed proof closure required'}
 }
}
function Read-NativeModernSeedSource($Binding,$Manifest,[string]$ManifestSha256,$CurrentPins,[string]$CurrentPinsSha256) {
 $manifestPath=Join-Path $Binding.fixtureRoot 'seed-manifest.private.json'
 $m=Read-NativeModernSeedJson $manifestPath $ManifestSha256 16384
 Assert-NativeSeedFields $m @('schemaVersion','kind','sourceWorkspace','sourceScenario','sourceEvidenceScenario','sourceRun','saveDirectory','gameVersion','files','provenance')
 Assert-NativeSeedInteger $m.schemaVersion 2 2
 if($m.kind-cne'NATIVE99_DISPOSABLE_SEED_MANIFEST_V2'-or$m.sourceWorkspace-isnot[string]-or![IO.Path]::IsPathFullyQualified($m.sourceWorkspace)-or[IO.Path]::GetFullPath($m.sourceWorkspace)-cne$m.sourceWorkspace){throw 'Canonical modern seed workspace required'}
 Assert-NativeSeedOrdinary $m.sourceWorkspace
 foreach($name in @('.git','.agentic/project.json','tools/starsector-proxy/direct-mcp.mjs')){Assert-NativeSeedOrdinary (Join-Path $m.sourceWorkspace $name)}
 Assert-NativeSeedInteger $m.sourceRun 1 1000000
 foreach($key in @('sourceScenario','sourceEvidenceScenario')){if($m.$key-isnot[string]-or$m.$key-cnotmatch'^[A-Za-z0-9][A-Za-z0-9_-]{1,127}$'){throw 'Contained modern seed scenario required'}}
 if($m.saveDirectory-isnot[string]-or$m.saveDirectory-cnotmatch'^save_[A-Za-z0-9_]{1,128}$'-or$m.gameVersion-cne'0.98a-RC8'){throw 'Closed modern seed identity required'}
 $scenarios=Join-Path $m.sourceWorkspace '.tmp/starsector-proxy/scenarios'
 $source=[IO.Path]::GetFullPath((Join-Path $scenarios $m.sourceScenario));$evidence=[IO.Path]::GetFullPath((Join-Path $scenarios $m.sourceEvidenceScenario))
 foreach($root in @($source,$evidence)){
  if([IO.Path]::GetDirectoryName($root)-cne[IO.Path]::GetFullPath($scenarios)-or(Test-NativeSeedSamePath $root $Binding.fixtureRoot)){throw 'Distinct contained modern seed source required'}
  Assert-NativeSeedOrdinary $root
 }
 Assert-NativeSeedFields $m.provenance @('fixtureReceipt','rootCall','closeout','protection','launch')
 $paths=@{fixtureReceipt=Join-Path $source 'protected-fixture-receipt.private.json';rootCall=Join-Path $evidence 'root-call.actual.private.json';closeout=Join-Path $evidence 'seed-creation-closeout.private.json';protection=Join-Path $evidence 'after-smoke-protection.private.json';launch=Join-Path $evidence 'launch-supervisor.private.json'}
 $records=@{}
 foreach($key in $paths.Keys){
  $pin=$m.provenance.$key;Assert-NativeSeedFields $pin @('bytes','sha256');Assert-NativeSeedInteger $pin.bytes 2 262144
  if((Get-Item -LiteralPath $paths[$key] -Force).Length-ne$pin.bytes){throw 'Modern seed provenance size changed'}
  $records[$key]=Read-NativeModernSeedJson $paths[$key] $pin.sha256
 }
 $f=$records.fixtureReceipt;$p=$records.protection;$c=$records.closeout;$l=$records.launch;$r=$records.rootCall
 Assert-NativeSeedFields $c @('schemaVersion','kind','scenarioId','toolingHead','rootSessionId','allOwnedHandlesTerminalReaped','actions','cleanup','protection','files','evidence')
 if($c.schemaVersion-ne1-or$c.kind-cne'ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT'-or$c.scenarioId-cne$m.sourceScenario-or$c.toolingHead-isnot[string]-or$c.toolingHead-cnotmatch'^[a-f0-9]{40}$'-or$c.allOwnedHandlesTerminalReaped-isnot[bool]-or!$c.allOwnedHandlesTerminalReaped){throw 'Closed completed modern seed provenance required'}
 $fixed=@{fixtureReceipt=$paths.fixtureReceipt;rootCall=$paths.rootCall;protection=$paths.protection;launch=$paths.launch;plan=Join-Path $source 'plan.json';lease=Join-Path $source 'native-lane-lease.private.json';controller=Join-Path $source 'controller-start.private.json';ledger=Join-Path $source 'guard-launch-history.private.json';preference=Join-Path $source 'seed-creation-preference.private.json';historyAdmission=Join-Path $source 'root-history-baseline.admitted.private.json'}
 Assert-NativeSeedFields $c.evidence @($fixed.Keys)
 $graph=@{}
 foreach($key in $fixed.Keys){
  $pin=$c.evidence.$key;Assert-NativeSeedFields $pin @('path','sha256')
  if($pin.path-cne$fixed[$key]){throw 'Fixed modern seed evidence path required'}
  $graph[$key]=Read-NativeModernSeedJson $pin.path $pin.sha256
  if($key-in@('fixtureReceipt','rootCall','protection','launch')-and$pin.sha256-ine$m.provenance.$key.sha256){throw 'Modern seed evidence linkage changed'}
 }
 $plan=$graph.plan;$lease=$graph.lease;$controller=$graph.controller;$ledger=$graph.ledger
 if($plan.scenarioId-cne$m.sourceScenario-or$plan.paths.scenario-cne$source-or$plan.source.qualifiedHead-cne$c.toolingHead-or$plan.campaignControl.schemaVersion-ne4-or$plan.campaignControl.profile-cne'DISPOSABLE_SEED_CREATION'-or$lease.planSha256-ine$c.evidence.plan.sha256-or$controller.rootLeaseSha256-ine$c.evidence.lease.sha256-or$ledger.controllerSha256-ine$c.evidence.controller.sha256-or$controller.installation-cne(Join-Path $source 'installation')){throw 'Modern seed launch identity changed'}
 $sourceBindingPin=$plan.fixtureBinding
 Assert-NativeSeedFields $sourceBindingPin @('path','sha256')
 if($sourceBindingPin.path-cne(Join-Path $source 'fixture-binding.private.json')){throw 'Fixed modern source binding required'}
 $sourceBinding=Read-NativeModernSeedJson $sourceBindingPin.path $sourceBindingPin.sha256 16384
 if($sourceBinding.schemaVersion-ne3-or$sourceBinding.fixtureProfile-cne'VANILLA_UNCOMPRESSED_GENERATION'-or$sourceBinding.fixtureRoot-cne$source-or$sourceBinding.driverRole-cne'/root/recovery_game_driver'-or(Test-NativeSeedSamePath $sourceBinding.sourceInstallation $source)){throw 'Private generated source binding required'}
 Assert-NativeSeedCreationHistory $plan $lease $source
 Assert-NativeSeedFields $r @('key','value');Assert-NativeSeedFields $r.value @('startMs','args','raw')
 Assert-NativeSeedInteger $r.value.raw.session_id 1 2147483647;Assert-NativeSeedInteger $r.value.startMs 1 9007199254740991
 if($r.value.args.workdir-cne$m.sourceWorkspace-or$r.key-cne$ledger.rootRecordKey-or$c.rootSessionId-ne$r.value.raw.session_id){throw 'Actual modern root wrapper required'}
 Assert-NativeSeedFields $c.actions @('inheritedBaseline','actualGuard','currentRunContext','absoluteMaxReservedActions','clicks','activations','providers')
 foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){
  Assert-NativeSeedInteger $c.actions.inheritedBaseline.$key $plan.historyBaseline.$key $plan.historyBaseline.$key
  Assert-NativeSeedInteger $c.actions.actualGuard.$key $ledger.history.$key $ledger.history.$key
 }
 foreach($key in @('overallAttempts','uiAttempts')){Assert-NativeSeedInteger $c.actions.currentRunContext.$key ($plan.historyBaseline.$key+1) ($plan.historyBaseline.$key+1)}
 if($c.actions.currentRunContext.overallAttempts-ne$m.sourceRun-or$ledger.history.reservedActions-ne($plan.historyBaseline.reservedActions+8)-or$ledger.history.sentActions-ne($plan.historyBaseline.sentActions+8)-or$ledger.uncertain-isnot[bool]-or$ledger.uncertain-or$ledger.pending.wizardStep.stage-cne'START_GAME'-or$ledger.pending.status-cne'AWAITING_AFTER'){throw 'Known completed seed Start and carried history required'}
 foreach($pair in @(@('absoluteMaxReservedActions',$plan.absoluteMaxReservedActions),@('clicks',8),@('activations',0),@('providers',3))){Assert-NativeSeedInteger $c.actions.($pair[0]) $pair[1] $pair[1]}
 if($f.schemaVersion-ne1-or$f.phase-cne'FROZEN_PREPARATION_ONLY'-or$f.privateSaveRootEmpty-isnot[bool]-or!$f.privateSaveRootEmpty-or$f.nativeLaunched-isnot[bool]-or$f.nativeLaunched-or$f.gameLaunched-isnot[bool]-or$f.gameLaunched-or$f.originalWrites-isnot[bool]-or$f.originalWrites-or$f.backupVerified-isnot[bool]-or!$f.backupVerified-or$f.originalFilesVerified-ne182-or$f.originalSaveFiles-ne178-or$f.logBaseline.existed-isnot[bool]-or$f.logBaseline.existed-or$f.logBaseline.bytes-ne0-or$f.installation-cne(Join-Path $source 'installation')-or$f.privateSaveRoot-cne(Join-Path $source 'installation/saves')){throw 'Modern empty preparation provenance required'}
 if($p.phase-cne'after-smoke'-or$p.protectedOriginalFiles-ne182-or$p.protectedSaveFiles-ne178-or$p.allOriginalAndBackupHashesMatch-isnot[bool]-or!$p.allOriginalAndBackupHashesMatch-or$p.saveCensusUnchanged-isnot[bool]-or!$p.saveCensusUnchanged-or$p.liveGameProcesses-ne0-or$p.originalWrites-isnot[bool]-or$p.originalWrites-or$p.privateAutosaveEnabled-isnot[bool]-or$p.privateAutosaveEnabled-or$p.privateSaveEntries-ne2-or$p.originalAutosave.enabled-isnot[bool]-or!$p.originalAutosave.enabled-or$p.originalAutosave.maximumSlots-ne3){throw 'Modern source protection required'}
 foreach($key in @('protectedOriginalFiles','protectedSaveFiles','liveGameProcesses','privateSaveEntries','allOriginalAndBackupHashesMatch','saveCensusUnchanged','originalWrites','privateAutosaveEnabled')){if($c.protection.$key.GetType()-ne$p.$key.GetType()-or$c.protection.$key-ne$p.$key){throw 'Modern source closeout protection mismatch'}}
 Assert-NativeSeedPreferenceValue $p.sharedPreference
 Assert-NativeSeedPreferenceValue $c.protection.sharedPreference
 foreach($key in $graph.preference.PSObject.Properties.Name){if($p.sharedPreference.$key.GetType()-ne$graph.preference.$key.GetType()-or$p.sharedPreference.$key-cne$graph.preference.$key){throw 'Modern source preference changed'}}
 if($l.schemaVersion-ne1-or$l.command-cne'verify'-or$l.ok-isnot[bool]-or!$l.ok-or@($l.result.steps).Count-ne1-or$l.result.verdict-cnotin@('PASS','FAIL')){throw 'Modern source actual supervisor required'}
 $step=$l.result.steps[0]
 Assert-NativeSeedInteger $step.status 0 1
 if($step.timedOut-isnot[bool]-or$step.timedOut-or$step.processOwnership-cne'windows-job-object'-or(($step.status-eq0)-ne($l.result.verdict-ceq'PASS'))-or$c.cleanup.status-ne$step.status-or$c.cleanup.verdict-cne$l.result.verdict-or$c.cleanup.timedOut-isnot[bool]-or$c.cleanup.timedOut-or$c.cleanup.processOwnership-cne'windows-job-object'-or$c.cleanup.zeroLiveGameProcesses-isnot[bool]-or!$c.cleanup.zeroLiveGameProcesses){throw 'Modern source terminal owned cleanup required'}
 $files=@($m.files);if($files.Count-ne2){throw 'Exact modern seed file census required'}
 $names=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal);$total=0L
 foreach($row in $files){
  Assert-NativeSeedFields $row @('relative','bytes','sha256')
  if($row.relative-isnot[string]-or$row.relative-cnotin@('campaign.xml','descriptor.xml')-or!$names.Add($row.relative)){throw 'Closed modern seed files required'}
  Assert-NativeSeedInteger $row.bytes 1 16777216;Assert-NativeSeedHash $row.sha256;$total+=$row.bytes
  $matches=@($c.files|Where-Object {$_.relative-ceq$row.relative-and$_.bytes-eq$row.bytes-and$_.sha256-ieq$row.sha256})
  if($matches.Count-ne1){throw 'Sealed modern seed file census changed'}
 }
 if($total-gt16777216-or@($c.files).Count-ne2){throw 'Bounded modern seed census required'}
 $saveRoot=Join-Path $source 'installation/saves';Assert-NativeSeedOrdinary $saveRoot
 $entries=@(Get-ChildItem -LiteralPath $saveRoot -Force)
 if($entries.Count-ne2-or(@($entries.Name|Sort-Object)-join',')-cne(@('common',$m.saveDirectory|Sort-Object)-join',')){throw 'Exact modern generated output required'}
 $common=Join-Path $saveRoot 'common';Assert-NativeSeedOrdinary $common
 $commonEntries=@(Get-ChildItem -LiteralPath $common -Force)
 if($commonEntries.Count-ne1-or$commonEntries[0].Name-cne'core_shared_settings.json.data'-or$commonEntries[0].LinkType-or$commonEntries[0].Length-ne41-or[IO.File]::ReadAllText($commonEntries[0].FullName)-cne'{"campaignHelpPopupsOptionChecked": true}'){throw 'Exact modern common preference required'}
 $save=Join-Path $saveRoot $m.saveDirectory;$actual=@(Get-NativeSeedFiles $save $files)
 foreach($row in $actual){if((Get-Item -LiteralPath (Join-Path $save $row.relative) -Force).LinkType){throw 'Modern seed links refused'}}
 $xml=Read-NativeSeedXml $save $m.saveDirectory $m.gameVersion
 $mod=Get-Content -LiteralPath (Join-Path $Binding.fixtureRoot 'installation/mods/near_future/mod_info.json') -Raw|ConvertFrom-Json -DateKind String
 if($mod.gameVersion-cne$m.gameVersion){throw 'Modern target game version mismatch'}
 $node=(Get-Command node -CommandType Application -TotalCount 1).Source
 $nodePins=@($CurrentPins.pins|Where-Object {$_.path-is[string]-and(Test-NativeSeedSamePath $_.path $node)})
 if($nodePins.Count-ne1-or(Get-FileHash -LiteralPath $node -Algorithm SHA256).Hash-ine$nodePins[0].sha256){throw 'Pinned modern seed proof runtime required'}
 $prefix=([IO.Path]::GetFullPath((Join-Path $source 'installation')).Replace('\','/').TrimEnd('/')+'/')
 $live=@(Get-CimInstance Win32_Process -OperationTimeoutSec 10|Where-Object {$_.ExecutablePath-and$_.ExecutablePath.Replace('\','/').StartsWith($prefix,[StringComparison]::OrdinalIgnoreCase)})
 if($live.Count){throw 'Modern seed source game still live'}
 $closeoutHash=([string]$m.provenance.closeout.sha256).ToLowerInvariant()
 $currentPinsPath=Join-Path $Binding.fixtureRoot 'source-pins.private.json'
 $powershell=(Get-Command pwsh -CommandType Application -TotalCount 1).Source
 $supervisor=Join-Path $Binding.repositoryRoot 'node_modules/workspace-template-win32-x64/bin/workspace-template.exe'
 $currentPinsHash=$CurrentPinsSha256.ToLowerInvariant()
 $output=@(& $node (Join-Path $PSScriptRoot '../seed-creation-custody.mjs') $source $closeoutHash $currentPinsPath $currentPinsHash $powershell $supervisor)
 if($LASTEXITCODE-ne0){throw 'Modern completed wizard custody refused'}
 $custody=($output-join"`n")|ConvertFrom-Json -DateKind String
 if($custody.ok-isnot[bool]-or!$custody.ok-or$custody.kind-cne'COMPLETED_SEED_WIZARD_CUSTODY'-or$custody.inputs-ne8){throw 'Modern completed wizard proof required'}
 return [pscustomobject]@{manifest=$m;manifestSha256=$ManifestSha256.ToUpperInvariant();sourceSave=$save;files=$actual;xml=$xml}
}
