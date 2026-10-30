param([Parameter(Mandatory)][string]$BindingPath,[Parameter(Mandatory)][string]$BindingSha256,[string]$Phase,[string]$EvidenceRoot,[string]$StageSeedAuthorityPath,[string]$StageSeedAuthoritySha256)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'fixture-inputs.ps1')
. (Join-Path $PSScriptRoot 'seed-fixture.ps1')
. (Join-Path $PSScriptRoot 'menu-launch.ps1')
$binding=Read-NativeFixtureBinding $BindingPath $BindingSha256
if([string]::IsNullOrWhiteSpace($EvidenceRoot)){$EvidenceRoot=$binding.fixtureRoot}
if($Phase-cnotin@('before-smoke','after-smoke','admit-launch','admit-probe','admit-seed-stage','validate-runtime')){throw 'Explicit protection phase required'}
if($Phase-cne'admit-seed-stage'-and($PSBoundParameters.ContainsKey('StageSeedAuthorityPath')-or$PSBoundParameters.ContainsKey('StageSeedAuthoritySha256'))){throw 'Seed stage authority is restricted to staging admission'}
$prepared=$binding.fixtureRoot
$source=$binding.sourceInstallation
function Assert-OrdinaryPath([string]$Path) {
 $item=Get-Item -LiteralPath $Path -Force
 while($null-ne$item){
  if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary contained fixture path required'}
  $item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}
 }
}
function Assert-ScenarioRoot([string]$Path) {
 $base=[IO.Path]::GetFullPath($binding.scenariosRoot)
 if([string]::IsNullOrWhiteSpace($Path)-or[IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($Path))-ine$base){throw 'Existing caller-owned scenario root required'}
 Assert-OrdinaryPath $Path
 if(!(Get-Item -LiteralPath $Path -Force).PSIsContainer){throw 'Existing caller-owned scenario root required'}
}
function Read-RuntimeManifest([string]$Path) {
 Assert-OrdinaryPath $Path
 if((Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash-ine$binding.runtimeManifestSha256){throw 'Qualified runtime manifest binding changed'}
 Assert-OrdinaryPath $Path
 if((Get-Item -LiteralPath $Path).Length-gt16384){throw 'Bounded candidate runtime manifest required'}
 $runtime=Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json
 if((@($runtime.PSObject.Properties.Name|Sort-Object)-join',')-cne'artifacts,candidateHead,kind,schemaVersion'){throw 'Closed candidate runtime schema required'}
 if($runtime.schemaVersion-isnot[long]-and$runtime.schemaVersion-isnot[int]){throw 'Candidate runtime schema required'}
 if($runtime.schemaVersion-ne1-or$runtime.kind-cne'NATIVE99_CAMPAIGN_BINDING_RUNTIME_INPUTS'-or$runtime.candidateHead-cne$binding.candidateHead){throw 'Exact candidate runtime manifest required'}
 $names=@('adapter-0.1.0.jar','bcprov-jdk18on-1.81.jar','contract-0.1.0.jar','ipc-0.1.0.jar','protobuf-java-4.31.1.jar','wire-0.1.0.jar')
 $expected=@($names|ForEach-Object {'mods/near_future/jars/'+$_})+@('diagnostic/nf-ipc-node.exe')
 $rows=@($runtime.artifacts)
 if($rows.Count-ne7){throw 'Exactly six candidate jars and one maintained node required'}
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
 foreach($row in $rows){
  if((@($row.PSObject.Properties.Name|Sort-Object)-join',')-cne'bytes,relative,sha256,source'){throw 'Closed candidate artifact schema required'}
  if($row.relative-isnot[string]-or$row.relative-cnotin$expected-or!$seen.Add($row.relative)){throw 'Closed candidate runtime paths required'}
  $source=if($row.relative-ceq'diagnostic/nf-ipc-node.exe'){(Join-Path $binding.qualifiedRuntimeRoot 'diagnostic/nf-ipc-node.exe')}else{((Join-Path $binding.qualifiedRuntimeRoot 'jars')+[IO.Path]::DirectorySeparatorChar)+[IO.Path]::GetFileName($row.relative)}
  if($row.source-isnot[string]-or[IO.Path]::GetFullPath($row.source)-cne[IO.Path]::GetFullPath($source)){throw 'Candidate runtime source path refused'}
  if(($row.bytes-isnot[int]-and$row.bytes-isnot[long])-or$row.bytes-lt1-or$row.bytes-gt67108864-or$row.sha256-isnot[string]-or$row.sha256-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Candidate runtime pin required'}
  Assert-OrdinaryPath $source
  if((Get-Item -LiteralPath $source).Length-ne$row.bytes-or(Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash-ine$row.sha256){throw 'Candidate runtime bytes changed'}
 }
 return $runtime
}
function Read-NativeLease([string]$Path) {
 if(!(Test-Path -LiteralPath $Path -PathType Leaf)){throw 'Explicit current bounded driver lease required'}
 Assert-OrdinaryPath $Path
 try{$lease=Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json}catch{throw 'Explicit current bounded driver lease required'}
 if($lease.phase-ceq'MENU_SMOKE_ONLY'){return (Read-NativeMenuLaunchLease $Path $binding $BindingPath $BindingSha256)}
 if(($lease.schemaVersion-isnot[int]-and$lease.schemaVersion-isnot[long])-or$lease.schemaVersion-ne1-or$lease.scenarioId-isnot[string]-or$lease.scenarioId-cne$binding.fixtureId){throw 'Exact campaign binding lease required'}
 foreach($key in @('planSha256','fixtureReceiptSha256')){if($lease.$key-isnot[string]-or$lease.$key-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Bound campaign binding lease required'}}
 if($lease.authorized-isnot[bool]-or$lease.authorized-ne$true-or$lease.driver-isnot[string]-or$lease.driver-cne$binding.driverRole-or$lease.phase-isnot[string]-or$lease.phase-cne'CAMPAIGN_BINDING_ONLY'){throw 'Explicit current bounded driver lease required'}
 if(@($lease.PSObject.Properties.Name)-ccontains'ownedNodeKill'){
  $spec=$lease.ownedNodeKill
  if($spec-isnot[pscustomobject]-or(@($spec.PSObject.Properties.Name|Sort-Object)-join',')-cne'case,caseNonce'-or$spec.case-isnot[string]-or$spec.case-cne'KILL_ONE_OWNED_NODE'-or$spec.caseNonce-isnot[string]-or$spec.caseNonce-cnotmatch'^[a-f0-9]{32}$'-or$lease.sourcePinsSha256-isnot[string]-or$lease.sourcePinsSha256-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Closed explicit one-shot owned-node authority required'}
 }
 return $lease
}
function Assert-OwnedNodeKillFreshState([string]$Root) {
 Assert-OrdinaryPath $Root
 foreach($name in @('owned-node-kill-candidate.private.json','kill-one-owned-node.request.private.json','kill-one-owned-node.terminal.private.json')){
  if(Test-Path -LiteralPath (Join-Path $Root $name)){throw 'Prior owned-node request or receipt refuses fresh native launch'}
 }
}
function Read-FrozenReceipt([string]$Path,[string]$Installation) {
 Assert-OrdinaryPath $Path
 $receipt=Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json
 foreach($pair in @(@('schemaVersion',1),@('originalFilesVerified',182),@('originalSaveFiles',178))){
  $value=$receipt.($pair[0]);if(($value-isnot[int]-and$value-isnot[long])-or$value-ne$pair[1]){throw 'Guarded frozen fixture receipt required'}
 }
 foreach($pair in @(@('originalWrites',$false),@('backupVerified',$true),@('privateAutosaveEnabled',$false),@('privateSaveRootEmpty',$true),@('nativeLaunched',$false),@('gameLaunched',$false),@('nodeVaultInitialized',$true))){
  $value=$receipt.($pair[0]);if($value-isnot[bool]-or$value-ne$pair[1]){throw 'Guarded frozen fixture receipt required'}
 }
 if($receipt.phase-cne'FROZEN_PREPARATION_ONLY'-or$receipt.nativeAcceptance-cne'UNMEASURED'){throw 'Guarded frozen fixture receipt required'}
 foreach($pair in @(@($receipt.installation,$Installation),@($receipt.privateSaveRoot,"$Installation/saves"),@($receipt.logBaseline.path,"$Installation/starsector-core/starsector.log"))){
  if($pair[0]-isnot[string]-or[IO.Path]::GetFullPath($pair[0])-ine[IO.Path]::GetFullPath($pair[1])){throw 'Frozen fixture path mismatch'}
 }
 if($receipt.logBaseline.existed-isnot[bool]-or$receipt.logBaseline.existed-ne$false-or($receipt.logBaseline.bytes-isnot[int]-and$receipt.logBaseline.bytes-isnot[long])-or$receipt.logBaseline.bytes-ne0){throw 'Guarded frozen fixture receipt required'}
 foreach($key in @('enabledByDefault','featureEnabled')){if($receipt.originalAutosave.$key-isnot[bool]-or$receipt.originalAutosave.$key-ne$true){throw 'Guarded frozen fixture receipt required'}}
 if(($receipt.originalAutosave.maximumSlots-isnot[int]-and$receipt.originalAutosave.maximumSlots-isnot[long])-or$receipt.originalAutosave.maximumSlots-ne3){throw 'Guarded frozen fixture receipt required'}
 foreach($key in @('classSha256','agentSha256','vmparamsSha256','jfcSha256','runtimeManifestSha256','modInfoSha256','enabledModsSha256','settingsSha256','vaultInitializationSha256')){if($receipt.$key-isnot[string]-or$receipt.$key-cnotmatch'^[a-fA-F0-9]{64}$'){throw 'Frozen fixture digest required'}}
 if($receipt.classSha256-ine$binding.presentationClassSha256){throw 'Frozen presentation class pin changed'}
 return $receipt
}
function Assert-FrozenProvenance($Receipt,[string]$Prepared) {
 Assert-NativeRecordingRecipe $binding $Receipt
 $manifest="$Prepared/runtime-artifacts.private.json"
 $runtime=Read-RuntimeManifest $manifest
 if((Get-FileHash -LiteralPath $manifest -Algorithm SHA256).Hash-ine$Receipt.runtimeManifestSha256){throw 'Frozen runtime manifest changed'}
 foreach($row in $runtime.artifacts){
  $copy=Join-Path "$Prepared/installation" $row.relative
  Assert-OrdinaryPath $copy
  if((Get-Item -LiteralPath $copy).Length-ne$row.bytes-or(Get-FileHash -LiteralPath $copy -Algorithm SHA256).Hash-ine$row.sha256){throw 'Frozen staged runtime changed'}
 }
 Assert-OrdinaryPath "$Prepared/diagnostic-vault"
 $init=Get-Content -LiteralPath "$Prepared/vault-initialization.private.json" -Raw|ConvertFrom-Json
 if(($init.status-isnot[int]-and$init.status-isnot[long])-or$init.status-ne0-or$init.command-cne'diagnostic/nf-ipc-node.exe'-or$init.scope-cne'OWNER_PRIVATE_VAULT_INITIALIZATION_ONLY'){throw 'Actual successful private vault initialization required'}
 $mods=@(Get-ChildItem -LiteralPath "$Prepared/installation/mods" -Directory -Force)
 if($mods.Count-ne1-or$mods[0].Name-cne'near_future'){throw 'Unexpected private mods'}
 $jars=@(Get-ChildItem -LiteralPath "$Prepared/installation/mods/near_future/jars" -Force)
 if($jars.Count-ne6){throw 'Unexpected staged runtime jars'}
 foreach($pair in @(@('modInfoSha256',"$Prepared/installation/mods/near_future/mod_info.json"),@('enabledModsSha256',"$Prepared/installation/mods/enabled_mods.json"),@('settingsSha256',"$Prepared/installation/starsector-core/data/config/settings.json"),@('vaultInitializationSha256',"$Prepared/vault-initialization.private.json"),@('vmparamsSha256',"$Prepared/installation/vmparams"),@('jfcSha256',"$Prepared/capture/active.jfc"),@('agentSha256',"$Prepared/agent/nf-presentation-agent.jar"))){
  Assert-OrdinaryPath $pair[1]
  if((Get-FileHash -LiteralPath $pair[1] -Algorithm SHA256).Hash-ine$Receipt.($pair[0])){throw 'Frozen fixture provenance changed'}
 }
}
function Assert-FreshCampaignFiles([string]$Installation,$Seed) {
 if($null-ne$Seed){$null=Assert-NativeSeedTarget $binding $Seed.source;return}
 Assert-OrdinaryPath "$Installation/saves"
 if(@(Get-ChildItem -LiteralPath "$Installation/saves" -Force).Count-ne0-or(Test-Path -LiteralPath "$Installation/starsector-core/starsector.log")){throw 'Expected unused private campaign fixture'}
}
function Read-ProtectionState([string]$Prepared,[string]$Source,[bool]$Before,$Seed) {
 $installation="$Prepared/installation"
 if($Before){Assert-FreshCampaignFiles $installation $Seed}
 foreach($path in @($Prepared,$Source,"$Prepared/original-backup","$Source/saves",$installation,"$installation/saves")){Assert-OrdinaryPath $path}
 $snapshotPath="$Prepared/protected-original-snapshot.private.json"
 Assert-OrdinaryPath $snapshotPath
 if((Get-Item -LiteralPath $snapshotPath -Force)-isnot[IO.FileInfo]){throw 'Ordinary protected snapshot file required'}
 $entries=@(Get-Content -LiteralPath $snapshotPath -Raw|ConvertFrom-Json)
 if($entries.Count-ne182){throw 'Protected snapshot census changed'}
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
 foreach($entry in $entries){
  if($entry.relative-isnot[string]-or!$seen.Add($entry.relative)){throw 'Protected snapshot path refused'}
  foreach($root in @($Source,"$Prepared/original-backup")){
   $path=[IO.Path]::GetFullPath((Join-Path $root $entry.relative))
   if(!$path.StartsWith([IO.Path]::GetFullPath($root)+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Protected snapshot path refused'}
   Assert-OrdinaryPath $path
   if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash-ne$entry.sha256){throw 'Protected source or backup changed'}
  }
 }
 $beforeNames=@($entries|Where-Object {$_.relative.Replace('\','/').StartsWith('saves/')}|ForEach-Object {$_.relative}|Sort-Object)
 $all=@(Get-ChildItem -LiteralPath "$Source/saves" -Force -Recurse)
 if(@($all|Where-Object {$_.Attributes-band[IO.FileAttributes]::ReparsePoint}).Count){throw 'Source save reparse entry refused'}
 $current=@($all|Where-Object {!$_.PSIsContainer}|ForEach-Object {[IO.Path]::GetRelativePath($Source,$_.FullName)}|Sort-Object)
 if(($beforeNames-join"`n")-cne($current-join"`n")){throw 'Protected save census changed'}
 $settings=[IO.File]::ReadAllText("$Source/starsector-core/data/config/settings.json")
 foreach($pattern in @('"maxAutosaveSlots"\s*:\s*3\s*,','"autosaveFeatureEnabled"\s*:\s*true','"autosaveEnabledByDefault"\s*:\s*true')){if([regex]::Matches($settings,$pattern).Count-ne1){throw 'Protected autosave retention changed'}}
 Assert-OrdinaryPath "$installation/starsector-core/data/config/settings.json"
 $privateSettings=[IO.File]::ReadAllText("$installation/starsector-core/data/config/settings.json")
 Assert-NativeFixtureSettings $binding $privateSettings
 if($binding.schemaVersion-eq3-and[IO.File]::ReadAllText("$installation/mods/enabled_mods.json")-cne(Read-NativeFixtureRecipe $binding).enabledModsJson){throw 'Generation requires no enabled mods'}
 foreach($key in @('autosaveFeatureEnabled','autosaveEnabledByDefault')){if([regex]::Matches($privateSettings,'"'+$key+'"\s*:\s*false').Count-ne1){throw 'Private autosave must remain disabled'}}
 $processes=@(Get-CimInstance Win32_Process -OperationTimeoutSec 10|Where-Object {$_.ExecutablePath-and($_.ExecutablePath.Replace('\','/').StartsWith(([IO.Path]::GetFullPath($installation).Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase)-or$_.ExecutablePath.Replace('\','/').StartsWith(([IO.Path]::GetFullPath($Source).Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase))})
 if($processes.Count){throw 'Game processes still live at protection boundary'}
 $privateSaves=@(Get-ChildItem -LiteralPath "$installation/saves" -Force)
 if($Before){Assert-FreshCampaignFiles $installation $Seed}
 return [ordered]@{at=Get-Date -Format o;protectedOriginalFiles=$entries.Count;allOriginalAndBackupHashesMatch=$true;protectedSaveFiles=$current.Count;saveCensusUnchanged=$true;originalAutosave=@{enabled=$true;maximumSlots=3};privateAutosaveEnabled=$false;privateSaveEntries=$privateSaves.Count;liveGameProcesses=$processes.Count;originalWrites=$false;restoration='UNNECESSARY_ORIGINALS_UNCHANGED'}
}
if($Phase-eq'validate-runtime'){Read-RuntimeManifest "$prepared/runtime-artifacts.private.json";return}
Assert-ScenarioRoot $EvidenceRoot
if($Phase-eq'admit-probe'){return}
$seed=$null
if($Phase-eq'admit-seed-stage'){
 if([string]::IsNullOrWhiteSpace($StageSeedAuthorityPath)-or[string]::IsNullOrWhiteSpace($StageSeedAuthoritySha256)){throw 'Paired pinned seed staging authority required'}
 foreach($name in @('seed-stage-claim.private.json','seed-stage-receipt.private.json')){if(Test-Path -LiteralPath (Join-Path $prepared $name)){throw 'Seed stage already claimed; preserve partial state'}}
 $stage=Read-NativeSeedStageAuthority $binding $StageSeedAuthorityPath $StageSeedAuthoritySha256
}
if($Phase-eq'admit-launch'){
 Assert-OwnedNodeKillFreshState $EvidenceRoot
 $lease=Read-NativeLease "$EvidenceRoot/native-lane-lease.private.json"
 Assert-OrdinaryPath "$EvidenceRoot/plan.json"
 if((Get-FileHash -LiteralPath "$EvidenceRoot/plan.json" -Algorithm SHA256).Hash-ine$lease.planSha256){throw 'Campaign plan binding changed'}
 $plan=Get-Content "$EvidenceRoot/plan.json" -Raw|ConvertFrom-Json
 $profileReceipt=Read-FrozenReceipt "$prepared/protected-fixture-receipt.private.json" "$prepared/installation"
 Assert-NativeRecordingAuthority $binding $profileReceipt $plan $lease
 if($plan.ownership.driver-isnot[string]-or$plan.ownership.driver-cne$binding.driverRole-or$plan.ownership.onlyGameInputDriver-isnot[bool]-or$plan.ownership.onlyGameInputDriver-ne$true){throw 'Current finite driver plan required'}
 $leaseOptIn=@($lease.PSObject.Properties.Name)-ccontains'ownedNodeKill'
 $planOptIn=@($plan.PSObject.Properties.Name)-ccontains'ownedNodeKill'
 if($leaseOptIn-ne$planOptIn){throw 'Owned-node plan and lease authority must agree'}
 if($leaseOptIn){
  $spec=$plan.ownedNodeKill
  if($spec-isnot[pscustomobject]-or(@($spec.PSObject.Properties.Name|Sort-Object)-join',')-cne'case,caseNonce'-or$spec.case-isnot[string]-or$spec.case-cne'KILL_ONE_OWNED_NODE'-or$spec.caseNonce-isnot[string]-or$spec.caseNonce-cnotmatch'^[a-f0-9]{32}$'-or$spec.caseNonce-cne$lease.ownedNodeKill.caseNonce){throw 'Exact one-shot plan authority required'}
  Assert-OrdinaryPath "$prepared/source-pins.private.json"
  if((Get-FileHash -LiteralPath "$prepared/source-pins.private.json" -Algorithm SHA256).Hash-ine$lease.sourcePinsSha256){throw 'Owned-node source authority changed'}
 }
 $seed=Read-NativeSeedPolicy $binding $plan $lease
}
if($Phase-eq'before-smoke'){
 $planPath=Join-Path $EvidenceRoot 'plan.json';$leasePath=Join-Path $EvidenceRoot 'native-lane-lease.private.json'
 $plan=$null;$lease=$null
 foreach($pair in @(@('plan',$planPath),@('lease',$leasePath))){
  if(Test-Path -LiteralPath $pair[1]){
   Assert-OrdinaryPath $pair[1]
   if((Get-Item -LiteralPath $pair[1]).Length-gt65536){throw 'Bounded current seed authority required'}
   $value=Get-Content -LiteralPath $pair[1] -Raw|ConvertFrom-Json -DateKind String
   if($pair[0]-ceq'plan'){$plan=$value}else{$lease=$value}
  }
 }
 if(($null-ne$plan-and@($plan.PSObject.Properties.Name)-ccontains'seed')-or($null-ne$lease-and@($lease.PSObject.Properties.Name)-ccontains'seed')){
  $lease=Read-NativeLease $leasePath
  if($null-eq$plan-or(Get-FileHash -LiteralPath $planPath).Hash-ine$lease.planSha256-or(Get-FileHash -LiteralPath "$prepared/protected-fixture-receipt.private.json").Hash-ine$lease.fixtureReceiptSha256){throw 'Current seeded plan and fixture binding required'}
  if($plan.ownership.driver-cne$binding.driverRole-or$plan.ownership.onlyGameInputDriver-isnot[bool]-or!$plan.ownership.onlyGameInputDriver){throw 'Current finite seed driver plan required'}
 }
 $seed=Read-NativeSeedPolicy $binding $plan $lease
}
$output="$EvidenceRoot/$Phase-protection.private.json"
if($Phase-ne'admit-launch'-and(Test-Path -LiteralPath $output)){throw 'Refuse replacement of native protection evidence'}
$receipt=Read-FrozenReceipt "$prepared/protected-fixture-receipt.private.json" "$prepared/installation"
if($Phase-eq'admit-launch'-and(Get-FileHash -LiteralPath "$prepared/protected-fixture-receipt.private.json" -Algorithm SHA256).Hash-ine$lease.fixtureReceiptSha256){throw 'Campaign fixture binding changed'}
Assert-FrozenProvenance $receipt $prepared
$record=Read-ProtectionState $prepared $source ($Phase-ne'after-smoke') $seed
if($Phase-eq'after-smoke'-and(Test-Path -LiteralPath "$EvidenceRoot/plan.json")){
 $closedPlan=Get-Content -LiteralPath "$EvidenceRoot/plan.json" -Raw|ConvertFrom-Json -DateKind String
 if($closedPlan.campaignControl.profile-cin@('DISPOSABLE_SEED_CREATION','DISPOSABLE_SEED_CREATION_INITIAL_FOCUS')){$record.sharedPreference=Assert-NativeSeedPreferenceUnchanged $closedPlan $EvidenceRoot}
}
if($Phase-eq'admit-launch'){
 if($lease.phase-ceq'MENU_SMOKE_ONLY'){return [pscustomobject]@{launchGrant=$lease.launchGrant;rootLeaseSha256=(Get-FileHash -LiteralPath "$EvidenceRoot/native-lane-lease.private.json" -Algorithm SHA256).Hash}}
 return
}
if($Phase-eq'admit-seed-stage'){return}
$record.phase=$Phase
$record|ConvertTo-Json -Depth 6|Set-Content $output
Write-Output "Protection boundary $Phase verified: original and backup hashes/census unchanged; no live game."

