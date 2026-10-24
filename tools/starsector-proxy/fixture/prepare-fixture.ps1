param([Parameter(Mandatory)][string]$BindingPath,[Parameter(Mandatory)][string]$BindingSha256,[switch]$GuardedResume,[string]$StageSeedAuthorityPath,[string]$StageSeedAuthoritySha256)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'fixture-inputs.ps1')
$binding=Read-NativeFixtureBinding $BindingPath $BindingSha256
if($GuardedResume){throw 'Native99 first-time preparation refuses partial-state resume; preserve failed artifacts'}
if($PSBoundParameters.ContainsKey('StageSeedAuthorityPath')-or$PSBoundParameters.ContainsKey('StageSeedAuthoritySha256')){
 if([string]::IsNullOrWhiteSpace($StageSeedAuthorityPath)-or[string]::IsNullOrWhiteSpace($StageSeedAuthoritySha256)){throw 'Paired pinned seed staging authority required'}
 . (Join-Path $PSScriptRoot 'seed-fixture.ps1')
 Invoke-NativeSeedStage -BindingPath $BindingPath -BindingSha256 $BindingSha256 -AuthorityPath $StageSeedAuthorityPath -AuthoritySha256 $StageSeedAuthoritySha256
 return
}
$sourceRoot=$binding.sourceInstallation

$scenario=$binding.fixtureRoot
$installation=Join-Path $scenario 'installation'
$backup=Join-Path $scenario 'original-backup'
$deadline=[DateTime]::UtcNow.AddMinutes(3)
$admittedFiles=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
$admittedDirectories=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
function Assert-Time { if([DateTime]::UtcNow-ge$deadline){throw 'Fixture preparation exceeded its planned bound'} }
function Assert-OrdinaryPath([string]$path) {
 $item=Get-Item -LiteralPath $path -Force
 while($null-ne$item){
  if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Reparse path refused'}
  $item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}
 }
}
function Assert-PrivateDestination([string]$Path) {
 $resolved=[IO.Path]::GetFullPath($Path)
 $root=[IO.Path]::GetFullPath($scenario)
 if(!$resolved.StartsWith($root+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Destination outside private scenario'}
 $probe=$resolved
 while($probe){
  $item=$null
  try{$item=Get-Item -LiteralPath $probe -Force -ErrorAction Stop}
  catch [System.Management.Automation.ItemNotFoundException]{}
  if($null-ne$item){
   if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Destination reparse ancestry refused'}
   if($probe-cne$resolved-and!$item.PSIsContainer){throw 'Destination ancestor is not a directory'}
  }
  $probe=[IO.Path]::GetDirectoryName($probe)
 }
}
function Assert-FreshPreparationState([string]$Root) {
 if([IO.Path]::GetFullPath($Root)-cne[IO.Path]::GetFullPath($scenario)){throw 'Exact private scenario required'}
 Assert-OrdinaryPath $Root
 $generated=@('installation','original-backup','agent','capture','diagnostic-vault',
  'protected-fixture-receipt.private.json','protected-original-snapshot.private.json','vault-initialization.private.json',
  'seed-stage-claim.private.json','seed-stage-receipt.private.json',
  'controller-start.private.json','controller-terminal.private.json','owned-processes-current.private.json','owned-process-samples.private.json',
  'owned-node-kill-candidate.private.json','kill-one-owned-node.request.private.json','kill-one-owned-node.terminal.private.json',
  'stop-owned-controller','native-lane-lease.private.json','before-smoke-protection.private.json','after-smoke-protection.private.json',
  'launch-supervisor.private.json','ownership-probe-supervisor.private.json','supervision-launch','supervision-ownership-probe')
 foreach($relative in $generated){
  $path=Join-Path $Root $relative
  Assert-PrivateDestination $path
  if(Test-Path -LiteralPath $path){throw 'First-time preparation refuses existing generated state'}
 }
}
function Admit-PrivatePath([string]$path) {
 $resolved=[IO.Path]::GetFullPath($path)
 if(!$resolved.StartsWith([IO.Path]::GetFullPath($scenario)+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Destination outside private scenario'}
 Assert-PrivateDestination $resolved
 [void]$admittedFiles.Add($resolved)
 $parent=[IO.Path]::GetDirectoryName($resolved)
 while($parent-and$parent.StartsWith([IO.Path]::GetFullPath($scenario)+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){
  [void]$admittedDirectories.Add($parent)
  $parent=[IO.Path]::GetDirectoryName($parent)
 }
 return $resolved
}
function Ensure-PrivateDirectory([string]$path){
 $resolved=[IO.Path]::GetFullPath($path)
 if(!$resolved.StartsWith([IO.Path]::GetFullPath($scenario)+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)){throw 'Directory outside private scenario'}
 [void]$admittedDirectories.Add($resolved)
 if($GuardedResume){
  Assert-OrdinaryPath $resolved
  if(!(Get-Item -LiteralPath $resolved -Force).PSIsContainer){throw 'Required partial directory is not a directory'}
 }else{
  Assert-PrivateDestination $resolved
  [void][IO.Directory]::CreateDirectory($resolved)
  Assert-PrivateDestination $resolved
 }
}
function Private-Settings([string]$text) {
 Assert-NativeFixtureSettings $binding $text
 if([regex]::Matches($text,'"maxAutosaveSlots"\s*:\s*3\s*,').Count-ne1){throw 'Original autosave retention shape changed'}
 foreach($key in @('autosaveFeatureEnabled','autosaveEnabledByDefault')){
  $pattern='"'+$key+'"\s*:\s*true'
  if([regex]::Matches($text,$pattern).Count-ne1){throw 'Private autosave setting shape changed'}
  $text=[regex]::Replace($text,$pattern,'"'+$key+'":false')
 }
 return $text
}
function Write-PrivateText([string]$path,[string]$text){
 $resolved=Admit-PrivatePath $path
 if($GuardedResume){
  Assert-OrdinaryPath $resolved
  if([IO.File]::ReadAllText($resolved)-cne$text){throw 'Existing generated private file differs from admitted recipe'}
 }else{
  Assert-PrivateDestination $resolved
  [IO.File]::WriteAllText($resolved,$text)
 }
}
function Copy-PrivateFile([string]$source,[string]$destination) {
 Assert-Time
 $resolved=Admit-PrivatePath $destination
 Assert-OrdinaryPath $source
 if($GuardedResume){
  Assert-OrdinaryPath $resolved
  if($resolved-eq[IO.Path]::GetFullPath("$installation/starsector-core/data/config/settings.json")){
   if([IO.File]::ReadAllText($resolved)-cne(Private-Settings ([IO.File]::ReadAllText($source)))){throw 'Partial private autosave settings mismatch'}
  }elseif((Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash-ne(Get-FileHash -LiteralPath $resolved -Algorithm SHA256).Hash){throw 'Partial copied file hash mismatch'}
  return
 }
 Assert-PrivateDestination ([IO.Path]::GetDirectoryName($resolved))
 [IO.Directory]::CreateDirectory([IO.Path]::GetDirectoryName($resolved))|Out-Null
 Assert-PrivateDestination $resolved
 [IO.File]::Copy($source,$resolved,$false)
}
if(!(Test-Path -LiteralPath (Join-Path $scenario 'plan.json'))){throw 'Finite driver plan required'}
if(Test-Path -LiteralPath "$scenario/protected-fixture-receipt.private.json"){throw 'Refuse replacement of receipted fixture'}
if(!$GuardedResume-and((Test-Path -LiteralPath $installation)-or(Test-Path -LiteralPath $backup))){throw 'Refuse replacement of any existing fixture or backup'}
Assert-OrdinaryPath $sourceRoot
Assert-OrdinaryPath $scenario
Assert-FreshPreparationState $scenario
$running=@(Get-CimInstance Win32_Process -OperationTimeoutSec 10|Where-Object {$_.ExecutablePath-and($_.ExecutablePath.Replace('\','/').StartsWith(([IO.Path]::GetFullPath($installation).Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase)-or$_.ExecutablePath.Replace('\','/').StartsWith(([IO.Path]::GetFullPath($sourceRoot).Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase))})
if($running.Count-ne0){throw 'Game installation has live processes; refuse preparation'}
$runtime=& (Join-Path $PSScriptRoot 'protection-check.ps1') -BindingPath $BindingPath -BindingSha256 $BindingSha256 -Phase validate-runtime
$retainedSnapshot=$null
if($GuardedResume){
 foreach($root in @($installation,$backup,"$scenario/agent","$scenario/capture")){
  Assert-OrdinaryPath $root
  $entries=@(Get-ChildItem -LiteralPath $root -Force -Recurse)
  if($entries.Count-gt20000-or($entries|Where-Object {$_.Attributes-band[IO.FileAttributes]::ReparsePoint})){throw 'Partial fixture inventory refused'}
 }
 if(@(Get-ChildItem -LiteralPath "$installation/saves" -Force).Count-ne0-or(Test-Path -LiteralPath "$installation/starsector-core/starsector.log")){throw 'Partial fixture was used; refuse recovery'}
 $running=@(Get-CimInstance Win32_Process|Where-Object {$_.ExecutablePath-and($_.ExecutablePath.Replace('\','/').StartsWith(([IO.Path]::GetFullPath($installation).Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase)-or$_.ExecutablePath.Replace('\','/').StartsWith(([IO.Path]::GetFullPath($sourceRoot).Replace('\','/').TrimEnd('/')+'/'),[StringComparison]::OrdinalIgnoreCase))})
 if($running.Count-ne0){throw 'Game installation has live processes; refuse preparation'}
 Assert-OrdinaryPath "$scenario/protected-original-snapshot.private.json"
 $retainedSnapshot=@(Get-Content "$scenario/protected-original-snapshot.private.json" -Raw|ConvertFrom-Json)
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
 foreach($entry in $retainedSnapshot){
  $path=[IO.Path]::GetFullPath((Join-Path $sourceRoot $entry.relative))
  if(!$path.StartsWith([IO.Path]::GetFullPath($sourceRoot)+[IO.Path]::DirectorySeparatorChar,[StringComparison]::OrdinalIgnoreCase)-or!$seen.Add($path)){throw 'Invalid or duplicate protected snapshot path'}
  Assert-OrdinaryPath $path
  if((Get-Item -LiteralPath $path).Length-ne$entry.bytes-or(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash-ne$entry.sha256){throw 'Protected original differs from retained snapshot'}
 }
}
$savedFiles=@(Get-ChildItem -LiteralPath "$sourceRoot/saves" -File -Force -Recurse)
if($savedFiles.Count-ne178){throw 'Protected source save census changed; preserve current user data and historical backups'}
$savedEntries=@(Get-ChildItem -LiteralPath "$sourceRoot/saves" -Force -Recurse)
if($savedEntries|Where-Object {$_.Attributes-band[IO.FileAttributes]::ReparsePoint}){throw 'Save reparse point refused'}
if($savedFiles.Count-gt1000-or($savedFiles|Measure-Object Length -Sum).Sum-gt10737418240){throw 'Original save census exceeds planned bound'}
$snapshot=@()
foreach($file in $savedFiles) {
 $relative=[IO.Path]::GetRelativePath($sourceRoot,$file.FullName)
 $hash=(Get-FileHash -LiteralPath $file.FullName -Algorithm SHA256).Hash
 $destination=Join-Path $backup $relative
 Copy-PrivateFile $file.FullName $destination
 if((Get-FileHash -LiteralPath $destination -Algorithm SHA256).Hash-ne$hash){throw 'Save backup hash mismatch'}
 $snapshot+=@{relative=$relative;bytes=$file.Length;sha256=$hash}
}
$configFiles=@('vmparams','mods/enabled_mods.json','starsector-core/data/config/settings.json','starsector-core/compiler_directives.txt')
foreach($relative in $configFiles) {
 $source=Join-Path $sourceRoot $relative
 $hash=(Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
 Copy-PrivateFile $source (Join-Path $backup $relative)
 if((Get-FileHash -LiteralPath (Join-Path $backup $relative) -Algorithm SHA256).Hash-ne$hash){throw 'Configuration backup hash mismatch'}
 $snapshot+=@{relative=$relative;bytes=(Get-Item -LiteralPath $source).Length;sha256=$hash}
}
foreach($tree in @('starsector-core','jre')) {
 $entries=@(Get-ChildItem -LiteralPath (Join-Path $sourceRoot $tree) -Force -Recurse)
 if($entries.Count-gt20000-or($entries|Where-Object {$_.Attributes-band[IO.FileAttributes]::ReparsePoint})){throw 'Installation inventory refused'}
 $files=@($entries|Where-Object {!$_.PSIsContainer-and$_.Name-notlike'*.log*'})
 if(($files|Measure-Object Length -Sum).Sum-gt1073741824){throw 'Private fixture tree exceeds planned byte bound'}
 foreach($file in $files){Copy-PrivateFile $file.FullName (Join-Path $installation ([IO.Path]::GetRelativePath($sourceRoot,$file.FullName)))}
}
Copy-PrivateFile "$sourceRoot/starsector.exe" "$installation/starsector.exe"
foreach($relative in @('saves','screenshots','mods/near_future/jars')){Ensure-PrivateDirectory (Join-Path $installation $relative)}
Ensure-PrivateDirectory "$scenario/agent"
Ensure-PrivateDirectory "$scenario/capture"
foreach($row in $runtime.artifacts){Copy-PrivateFile $row.source (Join-Path $installation $row.relative)}
Copy-PrivateFile $binding.presentationAgentPath "$scenario/agent/nf-presentation-agent.jar"
$mod=@{id='near_future';name='NearFuture native99 diagnostic binding';description='Private NearFuture campaign lifecycle diagnostic fixture.';version='0.1.0';gameVersion='0.98a-RC8';modPlugin='nf.adapter.NearFutureModPlugin';jars=@('jars/adapter-0.1.0.jar','jars/bcprov-jdk18on-1.81.jar','jars/contract-0.1.0.jar','jars/ipc-0.1.0.jar','jars/protobuf-java-4.31.1.jar','jars/wire-0.1.0.jar')}
$modPath=Admit-PrivatePath "$installation/mods/near_future/mod_info.json"
if($GuardedResume){
 Assert-OrdinaryPath $modPath
 $actual=Get-Content -LiteralPath $modPath -Raw|ConvertFrom-Json
 if((@($actual.PSObject.Properties.Name|Sort-Object)-join"`n")-cne(@($mod.Keys|Sort-Object)-join"`n")){throw 'Partial mod metadata fields differ'}
 foreach($key in $mod.Keys){if((@($actual.$key)-join"`0")-cne(@($mod[$key])-join"`0")){throw 'Partial mod metadata differs'}}
}else{
 Assert-PrivateDestination $modPath
 [IO.File]::WriteAllText($modPath,($mod|ConvertTo-Json -Depth 5))
}
Write-PrivateText "$installation/mods/enabled_mods.json" ((Read-NativeFixtureRecipe $binding).enabledModsJson)
$privateSettings="$installation/starsector-core/data/config/settings.json"
$settings=Private-Settings ([IO.File]::ReadAllText("$sourceRoot/starsector-core/data/config/settings.json"))
Write-PrivateText $privateSettings $settings
[xml]$defaultJfc=[IO.File]::ReadAllText("$installation/jre/lib/jfr/default.jfc")
$profile=if($binding.schemaVersion-eq2){$binding.recordingProfile}else{$null}
$jfc=New-NativeRecordingConfiguration $defaultJfc $profile
$jfcPath=Admit-PrivatePath "$scenario/capture/active.jfc"
if($GuardedResume){
 Assert-OrdinaryPath $jfcPath
 [xml]$actualJfc=[IO.File]::ReadAllText($jfcPath)
 if($actualJfc.OuterXml-cne$jfc.OuterXml){throw 'Partial recording configuration differs'}
}else{
 Assert-PrivateDestination $jfcPath
 $jfc.Save($jfcPath)
}
$vmparams=[IO.File]::ReadAllText("$sourceRoot/vmparams").Trim()
foreach($key in @('saves','screenshots','mods','logs')) {
 $pattern='-Dcom\.fs\.starfarer\.settings\.paths\.'+$key+'=\S+'
 if([regex]::Matches($vmparams,$pattern).Count-ne1){throw 'Launcher path shape changed'}
 $destination=if($key-eq'logs'){"$installation/starsector-core"}else{"$installation/$key"}
 $vmparams=[regex]::Replace($vmparams,$pattern,'-Dcom.fs.starfarer.settings.paths.'+$key+'='+$destination)
}
$pin=$binding.presentationClassSha256
if([regex]::Matches($vmparams,'-classpath ').Count-ne1){throw 'Launcher classpath shape changed'}
$profiling="-javaagent:$scenario/agent/nf-presentation-agent.jar=$pin -XX:StartFlightRecording=name=NF99,settings=$scenario/capture/active.jfc,duration=420s,maxsize=64m,dumponexit=true,filename=$scenario/capture/current.jfr "
$vmparams=$vmparams.Replace('-classpath ',$profiling+'-classpath ')
$node=($runtime.artifacts|Where-Object {$_.relative-ceq'diagnostic/nf-ipc-node.exe'})
if([regex]::IsMatch($vmparams,'-Dnf\.ipc\.diagnostic\.')){throw 'Inherited diagnostic configuration refused'}
$diagnostic="-Dnf.ipc.diagnostic.enabled=true -Dnf.ipc.diagnostic.handoffDelayMillis=750 -Dnf.ipc.diagnostic.retainStalePublication=true -Dnf.ipc.diagnostic.executable=$installation/diagnostic/nf-ipc-node.exe -Dnf.ipc.diagnostic.sha256=$($node.sha256.ToLowerInvariant()) -Dnf.ipc.diagnostic.vault=$scenario/diagnostic-vault -Dnf.ipc.diagnostic.saves=$installation/saves -Dnf.ipc.diagnostic.publication=native99_binding -Dnf.ipc.diagnostic.universe=$('11'*16) -Dnf.ipc.diagnostic.history=$('22'*16) -Dnf.ipc.diagnostic.ruleset=$('33'*32) -Dnf.ipc.diagnostic.content=$('44'*32) -Dnf.ipc.diagnostic.account=$('55'*16) -Dnf.ipc.diagnostic.device=$('66'*16) "
$vmparams=$vmparams.Replace('-classpath ',$diagnostic+'-classpath ')
Write-PrivateText "$installation/vmparams" $vmparams
# The maintained CLI opens owner-private vaults during serve-parent; use its existing init command once.
if(Test-Path -LiteralPath "$scenario/diagnostic-vault"){throw 'Refuse an existing diagnostic vault'}
Assert-PrivateDestination "$scenario/diagnostic-vault"
Assert-PrivateDestination "$installation/saves"
Assert-PrivateDestination "$scenario/vault-initialization.private.json"
$initOutput=& "$installation/diagnostic/nf-ipc-node.exe" init "$scenario/diagnostic-vault" "$installation/saves" 2>&1
$initStatus=$LASTEXITCODE
Assert-PrivateDestination "$scenario/vault-initialization.private.json"
[ordered]@{at=Get-Date -Format o;command='diagnostic/nf-ipc-node.exe';args=@('init',"$scenario/diagnostic-vault","$installation/saves");status=$initStatus;output=@($initOutput|ForEach-Object {"$_"});scope='OWNER_PRIVATE_VAULT_INITIALIZATION_ONLY';outerOwnership='REQUIRES_ACTUAL_PREPARATION_WINDOWS_JOB_TERMINAL'}|ConvertTo-Json -Depth 5|Set-Content "$scenario/vault-initialization.private.json"
if($initStatus-ne0){throw 'Existing maintained node vault initialization failed; preserve partial state'}
Assert-Time
foreach($entry in $snapshot){Assert-Time;if((Get-FileHash -LiteralPath (Join-Path $sourceRoot $entry.relative) -Algorithm SHA256).Hash-ne$entry.sha256){throw 'Original protected input changed during preparation'}}
$after=@(Get-ChildItem -LiteralPath "$sourceRoot/saves" -File -Force -Recurse|ForEach-Object{[IO.Path]::GetRelativePath($sourceRoot,$_.FullName)}|Sort-Object)
$before=@($savedFiles|ForEach-Object{[IO.Path]::GetRelativePath($sourceRoot,$_.FullName)}|Sort-Object)
if(($after-join"`n")-cne($before-join"`n")){throw 'Original save census changed during preparation'}
if(@(Get-ChildItem -LiteralPath "$installation/saves" -Force).Count-ne0){throw 'Disposable save root is not empty'}
if($GuardedResume){
 $current=@($snapshot|ForEach-Object {$_.relative+'|'+$_.bytes+'|'+$_.sha256}|Sort-Object)
 $retained=@($retainedSnapshot|ForEach-Object {$_.relative+'|'+$_.bytes+'|'+$_.sha256}|Sort-Object)
 if(($current-join"`n")-cne($retained-join"`n")){throw 'Protected snapshot census differs; refuse overwrite'}
 foreach($root in @($installation,$backup,"$scenario/agent","$scenario/capture")){
  foreach($entry in Get-ChildItem -LiteralPath $root -Force -Recurse){
   $admitted=if($entry.PSIsContainer){$admittedDirectories.Contains($entry.FullName)}else{$admittedFiles.Contains($entry.FullName)}
   if(!$admitted){throw 'Unknown partial fixture entry refused'}
  }
 }
}else{
 Assert-PrivateDestination "$scenario/protected-original-snapshot.private.json"
 $snapshot|ConvertTo-Json -Depth 5|Set-Content -Encoding utf8 "$scenario/protected-original-snapshot.private.json"
}
Assert-Time
$receipt=[ordered]@{schemaVersion=1;at=(Get-Date -Format o);phase='FROZEN_PREPARATION_ONLY';installation=$installation;originalWrites=$false;originalFilesVerified=$snapshot.Count;originalSaveFiles=$savedFiles.Count;backupVerified=$true;originalAutosave=@{featureEnabled=$true;enabledByDefault=$true;maximumSlots=3};privateAutosaveEnabled=$false;privateSaveRoot="$installation/saves";privateSaveRootEmpty=$true;logBaseline=@{path="$installation/starsector-core/starsector.log";existed=$false;bytes=0};classSha256=$pin;agentSha256=(Get-FileHash -LiteralPath "$scenario/agent/nf-presentation-agent.jar" -Algorithm SHA256).Hash;vmparamsSha256=(Get-FileHash -LiteralPath "$installation/vmparams" -Algorithm SHA256).Hash;modInfoSha256=(Get-FileHash -LiteralPath "$installation/mods/near_future/mod_info.json" -Algorithm SHA256).Hash;enabledModsSha256=(Get-FileHash -LiteralPath "$installation/mods/enabled_mods.json" -Algorithm SHA256).Hash;settingsSha256=(Get-FileHash -LiteralPath "$installation/starsector-core/data/config/settings.json" -Algorithm SHA256).Hash;vaultInitializationSha256=(Get-FileHash -LiteralPath "$scenario/vault-initialization.private.json" -Algorithm SHA256).Hash;runtimeManifestSha256=(Get-FileHash -LiteralPath "$scenario/runtime-artifacts.private.json" -Algorithm SHA256).Hash;jfcSha256=(Get-FileHash -LiteralPath "$scenario/capture/active.jfc" -Algorithm SHA256).Hash;nativeLaunched=$false;gameLaunched=$false;nodeVaultInitialized=$true;nativeAcceptance='UNMEASURED'}
if($binding.schemaVersion-eq2){$receipt.recordingProfileSha256=$binding.recordingProfileSha256}
Assert-PrivateDestination "$scenario/protected-fixture-receipt.private.json"
$receipt|ConvertTo-Json -Depth 8|Set-Content -Encoding utf8 "$scenario/protected-fixture-receipt.private.json"
Write-Output 'Private fixture prepared; no launcher or game started.'


