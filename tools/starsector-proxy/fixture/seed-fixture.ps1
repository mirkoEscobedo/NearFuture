# Exactly one generated disposable save. This module never loads a game or imports original saves.
. (Join-Path $PSScriptRoot 'fixture-inputs.ps1')
. (Join-Path $PSScriptRoot 'seed-creation-history.ps1')
. (Join-Path $PSScriptRoot 'seed-preference.ps1')
. (Join-Path $PSScriptRoot 'seed-provenance-v2.ps1')

function Assert-NativeSeedFields($Value,[string[]]$Names) {
 if($Value-isnot[pscustomobject]-or(@($Value.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($Names|Sort-Object)-join',')){throw 'Closed seed schema required'}
}
function Assert-NativeSeedHash($Value) {
 if($Value-isnot[string]-or$Value-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Seed digest required'}
}
function Assert-NativeSeedInteger($Value,[long]$Minimum,[long]$Maximum) {
 if(($Value-isnot[int]-and$Value-isnot[long])-or$Value-lt$Minimum-or$Value-gt$Maximum){throw 'Bounded seed integer required'}
}
function Assert-NativeSeedOrdinary([string]$Path) {
 $item=Get-Item -LiteralPath $Path -Force
 while($null-ne$item){
  if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary seed ancestry required'}
  $item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}
 }
}
function Test-NativeSeedSamePath([string]$Left,[string]$Right) {
 return [IO.Path]::GetFullPath($Left)-ieq[IO.Path]::GetFullPath($Right)
}
function Read-NativeSeedJson([string]$Path,[string]$Hash,[long]$Maximum=16384) {
 Assert-NativeSeedHash $Hash
 Assert-NativeSeedOrdinary $Path
 $file=Get-Item -LiteralPath $Path -Force
 if($file.PSIsContainer-or$file.Length-lt2-or$file.Length-gt$Maximum){throw 'Bounded seed JSON file required'}
 $bytes=[IO.File]::ReadAllBytes($Path)
 if($bytes.Length-ne$file.Length-or[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))-ine$Hash){throw 'Seed JSON digest changed'}
 return [Text.Encoding]::UTF8.GetString($bytes)|ConvertFrom-Json -DateKind String
}
function Get-NativeSeedXmlElement($Document,[string]$XPath) {
 $nodes=$Document.SelectNodes($XPath)
 if($nodes.Count-ne1){throw 'Unique seed XML field required'}
 return $nodes[0]
}
function Read-NativeSeedXml([string]$Directory,[string]$SaveDirectory,[string]$GameVersion) {
 $documents=@{}
 foreach($name in @('campaign.xml','descriptor.xml')){
  $path=Join-Path $Directory $name
  Assert-NativeSeedOrdinary $path
  $stream=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  try{
   if($stream.Length-lt1-or$stream.Length-gt16777216){throw 'Bounded plain seed XML required'}
   $settings=[Xml.XmlReaderSettings]::new()
   $settings.DtdProcessing=[Xml.DtdProcessing]::Prohibit
   $settings.XmlResolver=$null
   $settings.MaxCharactersInDocument=16777216
   $reader=[Xml.XmlReader]::Create($stream,$settings)
   $document=[Xml.XmlDocument]::new()
   $document.XmlResolver=$null
   try{$document.Load($reader)}finally{$reader.Dispose()}
   $documents[$name]=$document
  }finally{$stream.Dispose()}
 }
 $campaign=$documents['campaign.xml'];$descriptor=$documents['descriptor.xml']
 if($campaign.DocumentElement.Name-cne'CampaignEngine'-or$campaign.DocumentElement.NamespaceURI-cne''-or$descriptor.DocumentElement.Name-cne'SaveGameData'-or$descriptor.DocumentElement.NamespaceURI-cne''){throw 'Exact seed XML roots required'}
 if((Get-NativeSeedXmlElement $campaign '/CampaignEngine/saveDirName').InnerText-cne$SaveDirectory){throw 'Seed save identity mismatch'}
 $fleet=Get-NativeSeedXmlElement $campaign '/CampaignEngine/playerFleet'
 $reference=$fleet.GetAttribute('ref')
 if($reference-cnotmatch'^[0-9]{1,20}$'){throw 'Unique seed fleet reference required'}
 $targets=@($campaign.SelectNodes('//*[@z]')|Where-Object{$_.GetAttribute('z')-ceq$reference})
 if($targets.Count-ne1-or$targets[0].Name-cne'Flt'){throw 'Unique seed fleet reference required'}
 $character=Get-NativeSeedXmlElement $campaign '/CampaignEngine/characterData'
 if(@($character.ChildNodes|Where-Object{$_.NodeType-eq[Xml.XmlNodeType]::Element}).Count-lt1){throw 'Seed character data required'}
 $null=Get-NativeSeedXmlElement $campaign '/CampaignEngine/saveDesc' # Empty is the actual serialized form.
 $compressed=Get-NativeSeedXmlElement $descriptor '/SaveGameData/compressed'
 $version=Get-NativeSeedXmlElement $descriptor '/SaveGameData/gameVersion'
 $mods=Get-NativeSeedXmlElement $descriptor '/SaveGameData/enabledMods'
 if($compressed.InnerText-cne'false'-or$version.InnerText-cne$GameVersion-or@($mods.ChildNodes|Where-Object{$_.NodeType-eq[Xml.XmlNodeType]::Element}).Count-ne0-or!([string]::IsNullOrWhiteSpace($mods.InnerText))){throw 'Seed descriptor format mismatch'}
 return [pscustomobject][ordered]@{campaignRoot='CampaignEngine';descriptorRoot='SaveGameData';saveDirectory=$SaveDirectory;gameVersion=$GameVersion;compressed=$false;enabledMods=0;playerFleetResolved=$true}
}
function Get-NativeSeedFiles([string]$Directory,$Expected) {
 Assert-NativeSeedOrdinary $Directory
 if(!(Get-Item -LiteralPath $Directory -Force).PSIsContainer){throw 'Seed save directory required'}
 $entries=@(Get-ChildItem -LiteralPath $Directory -Force -Recurse)
 if($entries.Count-ne2-or@($entries|Where-Object{$_.PSIsContainer-or($_.Attributes-band[IO.FileAttributes]::ReparsePoint)}).Count){throw 'Exact two-file seed census required'}
 $result=@(foreach($row in $Expected){
  $path=Join-Path $Directory $row.relative
  Assert-NativeSeedOrdinary $path
  $stream=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  try{
   $hash=[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($stream))
   if($stream.Length-ne$row.bytes-or$hash-ine$row.sha256){throw 'Seed file bytes changed'}
   [pscustomobject][ordered]@{relative=$row.relative;bytes=$stream.Length;sha256=$hash}
  }finally{$stream.Dispose()}
 })
 return $result
}
function Read-NativeSeedProvenance($Binding,$Manifest,[string]$Source,[string]$Evidence) {
 $paths=@{fixtureReceipt=Join-Path $Source 'protected-fixture-receipt.private.json';rootCall=Join-Path $Evidence 'root-call.actual.private.json';closeout=Join-Path $Evidence ('driver-closeout.actual'+$Manifest.sourceRun+'.private.json');protection=Join-Path $Evidence 'after-smoke-protection.private.json';launch=Join-Path $Evidence 'launch-supervisor.private.json'}
 $records=@{}
 foreach($key in @('fixtureReceipt','rootCall','closeout','protection','launch')){
  $pin=$Manifest.provenance.$key
  Assert-NativeSeedFields $pin @('bytes','sha256')
  Assert-NativeSeedInteger $pin.bytes 2 16384
  if((Get-Item -LiteralPath $paths[$key] -Force).Length-ne$pin.bytes){throw 'Seed provenance size changed'}
  $records[$key]=Read-NativeSeedJson $paths[$key] $pin.sha256
 }
 $f=$records.fixtureReceipt;$p=$records.protection;$c=$records.closeout;$l=$records.launch;$r=$records.rootCall
 Assert-NativeSeedInteger $f.schemaVersion 1 1
 Assert-NativeSeedInteger $f.originalFilesVerified 182 182
 Assert-NativeSeedInteger $f.originalSaveFiles 178 178
 Assert-NativeSeedInteger $f.logBaseline.bytes 0 0
 Assert-NativeSeedInteger $p.protectedOriginalFiles 182 182
 Assert-NativeSeedInteger $p.protectedSaveFiles 178 178
 Assert-NativeSeedInteger $p.liveGameProcesses 0 0
 Assert-NativeSeedInteger $p.privateSaveEntries 1 1000000
 Assert-NativeSeedInteger $p.originalAutosave.maximumSlots 3 3
 Assert-NativeSeedInteger $l.schemaVersion 1 1
 Assert-NativeSeedInteger $c.schemaVersion 1 1
 Assert-NativeSeedInteger $c.history.overall $Manifest.sourceRun $Manifest.sourceRun
 Assert-NativeSeedInteger $c.history.rootSessionId 1 2147483647
 Assert-NativeSeedInteger $c.cleanup.status 0 1
 if($f.schemaVersion-ne1-or$f.phase-cne'FROZEN_PREPARATION_ONLY'-or$f.privateSaveRootEmpty-isnot[bool]-or!$f.privateSaveRootEmpty-or$f.nativeLaunched-isnot[bool]-or$f.nativeLaunched-or$f.gameLaunched-isnot[bool]-or$f.gameLaunched-or$f.originalWrites-isnot[bool]-or$f.originalWrites-or$f.backupVerified-isnot[bool]-or!$f.backupVerified-or$f.originalFilesVerified-ne182-or$f.originalSaveFiles-ne178-or$f.logBaseline.existed-isnot[bool]-or$f.logBaseline.existed-or$f.logBaseline.bytes-ne0){throw 'Generated private seed preparation provenance required'}
 if(!(Test-NativeSeedSamePath $f.installation (Join-Path $Source 'installation'))-or!(Test-NativeSeedSamePath $f.privateSaveRoot (Join-Path $Source 'installation/saves'))){throw 'Seed source fixture identity mismatch'}
 # Source cleanup was forced: a FAIL verdict is truthful provenance, not a reason to invent PASS.
 if($p.phase-cne'after-smoke'-or$p.protectedOriginalFiles-ne182-or$p.protectedSaveFiles-ne178-or$p.allOriginalAndBackupHashesMatch-isnot[bool]-or!$p.allOriginalAndBackupHashesMatch-or$p.saveCensusUnchanged-isnot[bool]-or!$p.saveCensusUnchanged-or$p.liveGameProcesses-ne0-or$p.originalWrites-isnot[bool]-or$p.originalWrites-or$p.privateAutosaveEnabled-isnot[bool]-or$p.privateAutosaveEnabled-or$p.originalAutosave.enabled-isnot[bool]-or!$p.originalAutosave.enabled-or$p.originalAutosave.maximumSlots-ne3){throw 'Closed seed source protection required'}
 if($l.schemaVersion-ne1-or$l.command-cne'verify'-or$l.ok-isnot[bool]-or!$l.ok-or@($l.result.steps).Count-ne1-or$l.result.verdict-cnotin@('PASS','FAIL')){throw 'Actual seed source supervisor required'}
 $step=$l.result.steps[0]
 Assert-NativeSeedInteger $step.status 0 1
 if($step.timedOut-isnot[bool]-or$step.timedOut-or$step.processOwnership-cne'windows-job-object'-or$step.status-cnotin@(0,1)-or(($step.status-eq0)-ne($l.result.verdict-ceq'PASS'))){throw 'Reaped seed source supervisor required'}
 if($c.schemaVersion-ne1-or$c.kind-cne('ACTUAL'+$Manifest.sourceRun+'_DRIVER_FINAL_CLOSEOUT')-or$c.scenarioId-cne$Manifest.sourceScenario-or$c.status-isnot[string]-or!$c.status.StartsWith('CLOSED_', [StringComparison]::Ordinal)-or$c.history.overall-ne$Manifest.sourceRun-or$c.cleanup.verdict-cne$l.result.verdict-or$c.cleanup.status-ne$step.status-or$c.cleanup.timedOut-isnot[bool]-or$c.cleanup.timedOut-or$c.cleanup.processOwnership-cne'windows-job-object'-or$c.cleanup.zeroLiveGameProcesses-isnot[bool]-or!$c.cleanup.zeroLiveGameProcesses){throw 'Seed source closeout identity mismatch'}
 Assert-NativeSeedFields $r @('key','value')
 Assert-NativeSeedFields $r.value @('startMs','args','raw')
 Assert-NativeSeedInteger $r.value.startMs 1 9007199254740991
 Assert-NativeSeedInteger $r.value.raw.session_id 1 2147483647
 if($r.key-cne('native'+$Manifest.sourceRun+'ActualRootCall')-or$r.value.args-isnot[pscustomobject]-or$c.history.rootSessionId-ne$r.value.raw.session_id){throw 'Actual seed source root wrapper required'}
 foreach($key in @('protectedOriginalFiles','protectedSaveFiles','liveGameProcesses','privateSaveEntries','allOriginalAndBackupHashesMatch','saveCensusUnchanged','originalWrites','privateAutosaveEnabled')){if($c.protection.$key.GetType()-ne$p.$key.GetType()-or$c.protection.$key-ne$p.$key){throw 'Seed source closeout protection mismatch'}}
 foreach($pair in @(@('launchSupervisor','launch'),@('protection','protection'))){
  if(!(Test-NativeSeedSamePath $c.evidence.($pair[0]) $paths[$pair[1]])-or$c.evidence.($pair[0]+'SHA256')-ine$Manifest.provenance.($pair[1]).sha256){throw 'Seed source receipt linkage mismatch'}
 }
 if(!(Test-NativeSeedSamePath $c.evidence.fixtureReceipt $paths.fixtureReceipt)){throw 'Seed source frozen receipt linkage mismatch'}
}
function Read-NativeSeedSource($Binding,[string]$ManifestSha256,$CurrentPins=$null,[string]$CurrentPinsSha256='') {
 $path=Join-Path $Binding.fixtureRoot 'seed-manifest.private.json'
 $manifest=Read-NativeSeedJson $path $ManifestSha256
 if(($manifest.schemaVersion-is[int]-or$manifest.schemaVersion-is[long])-and$manifest.schemaVersion-eq2){return (Read-NativeModernSeedSource $Binding $manifest $ManifestSha256 $CurrentPins $CurrentPinsSha256)}
 Assert-NativeSeedFields $manifest @('schemaVersion','kind','sourceScenario','sourceEvidenceScenario','sourceRun','saveDirectory','gameVersion','files','provenance')
 Assert-NativeSeedInteger $manifest.schemaVersion 1 1
 if($manifest.schemaVersion-ne1-or$manifest.kind-cne'NATIVE99_DISPOSABLE_SEED_MANIFEST_V1'){throw 'Seed manifest version required'}
 Assert-NativeSeedInteger $manifest.sourceRun 1 1000000
 foreach($key in @('sourceScenario','sourceEvidenceScenario')){if($manifest.$key-isnot[string]-or$manifest.$key-cnotmatch'^[A-Za-z0-9][A-Za-z0-9_-]{1,127}$'){throw 'Contained seed source scenario required'}}
 if($manifest.saveDirectory-isnot[string]-or$manifest.saveDirectory-cnotmatch'^save_[A-Za-z0-9_]{1,128}$'-or$manifest.gameVersion-isnot[string]-or$manifest.gameVersion.Length-lt1-or$manifest.gameVersion.Length-gt64){throw 'Seed save identity required'}
 Assert-NativeSeedFields $manifest.provenance @('fixtureReceipt','rootCall','closeout','protection','launch')
 $files=@($manifest.files)
 if($files.Count-ne2){throw 'Exact two-file seed manifest required'}
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
 $total=0L
 foreach($row in $files){
  Assert-NativeSeedFields $row @('relative','bytes','sha256')
  if($row.relative-isnot[string]-or$row.relative-cnotin@('campaign.xml','descriptor.xml')-or!$seen.Add($row.relative)){throw 'Closed seed file paths required'}
  Assert-NativeSeedInteger $row.bytes 1 16777216
  Assert-NativeSeedHash $row.sha256
  $total+=$row.bytes
 }
 if($total-gt16777216){throw 'Bounded seed file census required'}
 $source=Join-Path $Binding.scenariosRoot $manifest.sourceScenario
 $evidence=Join-Path $Binding.scenariosRoot $manifest.sourceEvidenceScenario
 foreach($root in @($source,$evidence)){
  if((Test-NativeSeedSamePath $root $Binding.fixtureRoot)-or[IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($root))-ine[IO.Path]::GetFullPath($Binding.scenariosRoot)){throw 'Distinct contained seed source required'}
  Assert-NativeSeedOrdinary $root
  if(!(Get-Item -LiteralPath $root -Force).PSIsContainer){throw 'Seed source directory required'}
 }
 Read-NativeSeedProvenance $Binding $manifest $source $evidence
 $save=Join-Path (Join-Path $source 'installation/saves') $manifest.saveDirectory
 $actual=@(Get-NativeSeedFiles $save $files)
 $xml=Read-NativeSeedXml $save $manifest.saveDirectory $manifest.gameVersion
 $modPath=Join-Path $Binding.fixtureRoot 'installation/mods/near_future/mod_info.json'
 Assert-NativeSeedOrdinary $modPath
 $mod=Get-Content -LiteralPath $modPath -Raw|ConvertFrom-Json -DateKind String
 if($mod.gameVersion-cne$manifest.gameVersion){throw 'Seed target game version mismatch'}
 return [pscustomobject]@{manifest=$manifest;manifestSha256=$ManifestSha256.ToUpperInvariant();sourceSave=$save;files=$actual;xml=$xml}
}
function Read-NativeSeedStageAuthority($Binding,[string]$AuthorityPath,[string]$AuthoritySha256) {
 $expected=Join-Path $Binding.fixtureRoot 'seed-stage-authority.private.json'
 if(![IO.Path]::IsPathFullyQualified($AuthorityPath)-or!(Test-NativeSeedSamePath $AuthorityPath $expected)){throw 'Fixed seed stage authority path required'}
 $authority=Read-NativeSeedJson $expected $AuthoritySha256 8192
 Assert-NativeSeedFields $authority @('schemaVersion','kind','authorized','scenarioId','planSha256','fixtureReceiptSha256','sourcePinsSha256','manifestSha256')
 Assert-NativeSeedInteger $authority.schemaVersion 1 1
 if($authority.kind-cne'NATIVE99_SEED_STAGE_AUTHORITY_V1'-or$authority.authorized-isnot[bool]-or!$authority.authorized-or$authority.scenarioId-cne$Binding.fixtureId){throw 'Explicit seed staging authority required'}
 foreach($key in @('planSha256','fixtureReceiptSha256','sourcePinsSha256','manifestSha256')){Assert-NativeSeedHash $authority.$key}
 $plan=Read-NativeSeedJson (Join-Path $Binding.fixtureRoot 'seed-stage-plan.private.json') $authority.planSha256 65536
 Assert-NativeSeedFields $plan.seedStage @('mode','manifestSha256')
 if($plan.seedStage.mode-cne'STAGE_ONE_PRIVATE_DISPOSABLE_SEED'-or$plan.seedStage.manifestSha256-ine$authority.manifestSha256){throw 'Seed staging plan agreement required'}
 $null=Read-NativeSeedJson (Join-Path $Binding.fixtureRoot 'protected-fixture-receipt.private.json') $authority.fixtureReceiptSha256
 $manifestHeader=Read-NativeSeedJson (Join-Path $Binding.fixtureRoot 'seed-manifest.private.json') $authority.manifestSha256
 $modernManifest=($manifestHeader.schemaVersion-is[int]-or$manifestHeader.schemaVersion-is[long])-and$manifestHeader.schemaVersion-eq2
 $pinLimit=if($modernManifest){262144}else{131072}
 $pins=Read-NativeSeedJson (Join-Path $Binding.fixtureRoot 'source-pins.private.json') $authority.sourcePinsSha256 $pinLimit
 if($modernManifest){Assert-NativeModernSeedPins $pins $Binding}
 foreach($name in @('fixture-inputs.ps1','seed-fixture.ps1','prepare-fixture.ps1','protection-check.ps1')){
  $path=Join-Path $PSScriptRoot $name
  $rows=@($pins.pins|Where-Object{$_.path-is[string]-and(Test-NativeSeedSamePath $_.path $path)})
  if($rows.Count-ne1-or$rows[0].sha256-isnot[string]-or(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash-ine$rows[0].sha256){throw 'Seed tooling source pins required'}
 }
 $source=Read-NativeSeedSource $Binding $authority.manifestSha256 $pins $authority.sourcePinsSha256
 return [pscustomobject]@{authority=$authority;authoritySha256=$AuthoritySha256.ToUpperInvariant();source=$source}
}
function Write-NativeSeedCreateNewJson([string]$Path,$Value) {
 Assert-NativeSeedOrdinary ([IO.Path]::GetDirectoryName($Path))
 $bytes=[Text.UTF8Encoding]::new($false).GetBytes(($Value|ConvertTo-Json -Depth 16 -Compress)+"`n")
 $stream=[IO.File]::Open($Path,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
 try{$stream.Write($bytes,0,$bytes.Length);$stream.Flush($true)}finally{$stream.Dispose()}
 return (Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash
}
function Copy-NativeSeedFile([string]$Source,[string]$Destination,$Expected,[DateTime]$Deadline) {
 Assert-NativeSeedOrdinary $Source
 Assert-NativeSeedOrdinary ([IO.Path]::GetDirectoryName($Destination))
 $sourceStream=[IO.File]::Open($Source,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
 try{
  if($sourceStream.Length-ne$Expected.bytes-or[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($sourceStream))-ine$Expected.sha256){throw 'Seed copy source changed'}
  $sourceStream.Position=0
  $destinationStream=[IO.File]::Open($Destination,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  try{
   $buffer=[byte[]]::new(65536)
   while(($count=$sourceStream.Read($buffer,0,$buffer.Length))-gt0){
    if([DateTime]::UtcNow-ge$Deadline){throw 'Seed staging deadline exceeded'}
    $destinationStream.Write($buffer,0,$count)
   }
   $destinationStream.Flush($true)
  }finally{$destinationStream.Dispose()}
  $sourceStream.Position=0
  if($sourceStream.Length-ne$Expected.bytes-or[Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($sourceStream))-ine$Expected.sha256){throw 'Seed copy source changed'}
 }finally{$sourceStream.Dispose()}
 Assert-NativeSeedOrdinary $Destination
 if((Get-Item -LiteralPath $Destination -Force).Length-ne$Expected.bytes-or(Get-FileHash -LiteralPath $Destination -Algorithm SHA256).Hash-ine$Expected.sha256){throw 'Seed copy readback changed'}
}
function Assert-NativeSeedTarget($Binding,$Source) {
 $saveRoot=Join-Path $Binding.fixtureRoot 'installation/saves'
 Assert-NativeSeedOrdinary $saveRoot
 $entries=@(Get-ChildItem -LiteralPath $saveRoot -Force)
 if($entries.Count-ne1-or!$entries[0].PSIsContainer-or$entries[0].Name-cne$Source.manifest.saveDirectory){throw 'Exactly one staged seed directory required'}
 $destination=Join-Path $saveRoot $Source.manifest.saveDirectory
 $files=@(Get-NativeSeedFiles $destination $Source.files)
 $xml=Read-NativeSeedXml $destination $Source.manifest.saveDirectory $Source.manifest.gameVersion
 if(Test-Path -LiteralPath (Join-Path $Binding.fixtureRoot 'installation/starsector-core/starsector.log')){throw 'Seeded fixture log must remain absent'}
 return [pscustomobject]@{files=$files;xml=$xml}
}
function Invoke-NativeSeedStage([string]$BindingPath,[string]$BindingSha256,[string]$AuthorityPath,[string]$AuthoritySha256) {
 $binding=Read-NativeFixtureBinding $BindingPath $BindingSha256
 $deadline=[DateTime]::UtcNow.AddMinutes(3)
 $grant=Read-NativeSeedStageAuthority $binding $AuthorityPath $AuthoritySha256
 $claimPath=Join-Path $binding.fixtureRoot 'seed-stage-claim.private.json'
 $receiptPath=Join-Path $binding.fixtureRoot 'seed-stage-receipt.private.json'
 if((Test-Path -LiteralPath $claimPath)-or(Test-Path -LiteralPath $receiptPath)){throw 'Seed stage already claimed; preserve partial state'}
 # Read-only full frozen/original/no-live/empty/log validation precedes the first effect.
 & (Join-Path $PSScriptRoot 'protection-check.ps1') -BindingPath $BindingPath -BindingSha256 $BindingSha256 -Phase admit-seed-stage -EvidenceRoot $binding.fixtureRoot -StageSeedAuthorityPath $AuthorityPath -StageSeedAuthoritySha256 $AuthoritySha256
 if([DateTime]::UtcNow-ge$deadline){throw 'Seed staging deadline exceeded'}
 $claim=[ordered]@{schemaVersion=1;kind='NATIVE99_SEED_STAGE_CLAIM_V1';scenarioId=$binding.fixtureId;authoritySha256=$grant.authoritySha256;manifestSha256=$grant.source.manifestSha256;fixtureReceiptSha256=$grant.authority.fixtureReceiptSha256;sourcePinsSha256=$grant.authority.sourcePinsSha256;atUtc=[DateTime]::UtcNow.ToString('o')}
 $claimHash=Write-NativeSeedCreateNewJson $claimPath $claim
 $saveRoot=Join-Path $binding.fixtureRoot 'installation/saves'
 Assert-NativeSeedOrdinary $saveRoot
 if(@(Get-ChildItem -LiteralPath $saveRoot -Force).Count-ne0){throw 'Seed stage destination must remain empty'}
 $destination=Join-Path $saveRoot $grant.source.manifest.saveDirectory
 if(Test-Path -LiteralPath $destination){throw 'Seed destination already exists'}
 [void][IO.Directory]::CreateDirectory($destination)
 Assert-NativeSeedOrdinary $destination
 foreach($row in $grant.source.files){Copy-NativeSeedFile (Join-Path $grant.source.sourceSave $row.relative) (Join-Path $destination $row.relative) $row $deadline}
 $null=Get-NativeSeedFiles $grant.source.sourceSave $grant.source.files
 $actual=Assert-NativeSeedTarget $binding $grant.source
 if([DateTime]::UtcNow-ge$deadline){throw 'Seed staging deadline exceeded'}
 $receipt=[ordered]@{schemaVersion=1;kind='NATIVE99_STAGED_SAVE_RECEIPT_V1';scenarioId=$binding.fixtureId;authoritySha256=$grant.authoritySha256;claimSha256=$claimHash;manifestSha256=$grant.source.manifestSha256;fixtureReceiptSha256=$grant.authority.fixtureReceiptSha256;sourcePinsSha256=$grant.authority.sourcePinsSha256;saveDirectory=$grant.source.manifest.saveDirectory;files=$actual.files;xml=$actual.xml;sourceUnchanged=$true;logAbsent=$true;nativeLaunched=$false;gameLaunched=$false;atUtc=[DateTime]::UtcNow.ToString('o')}
 $receiptHash=Write-NativeSeedCreateNewJson $receiptPath $receipt
 return [pscustomobject]@{schemaVersion=1;kind='NATIVE99_SEED_STAGE_COMPLETED_V1';receiptPath=$receiptPath;receiptSha256=$receiptHash;nativeAuthority=$false}
}
function Assert-NativeCampaignControl($Binding,$Plan,$Lease) {
 $planControl=$null-ne$Plan-and@($Plan.PSObject.Properties.Name)-ccontains'campaignControl'
 $leaseControl=$null-ne$Lease-and@($Lease.PSObject.Properties.Name)-ccontains'campaignControl'
 $creation=$Plan.campaignControl.profile-ceq'DISPOSABLE_SEED_CREATION'
 if($Binding.schemaVersion-eq3-and($null-ne$Plan-or$null-ne$Lease)-and(!$planControl-or!$leaseControl-or$Plan.campaignControl.profile-cnotin@('GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE','DISPOSABLE_SEED_CREATION')-or$Lease.campaignControl.profile-cne$Plan.campaignControl.profile)){throw 'Generation fixture requires its explicit campaign control'}
 if($planControl-ne$leaseControl){throw 'Campaign control plan and lease must agree'}
 if(!$planControl){return}
 $generation=$Plan.campaignControl.profile-ceq'GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE'
 if(($generation-or$creation)-and$Binding.schemaVersion-ne3){throw 'Generation campaign control requires its fixed fixture'}
 $inspection=$Plan.campaignControl.profile-ceq'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE'
 foreach($control in @($Plan.campaignControl,$Lease.campaignControl)){
  Assert-NativeSeedFields $control @('schemaVersion','profile')
  $version=if($creation){4}elseif($generation){3}elseif($inspection){2}else{1};$profile=if($creation){'DISPOSABLE_SEED_CREATION'}elseif($generation){'GENERATED_NAME_FIRST_CONTINUE_INSPECTION_NO_SAVE'}elseif($inspection){'NEW_GAME_FIRST_PAGE_INSPECTION_NO_SAVE'}else{'DISPOSABLE_SEED_ENTRY_EXIT_NO_SAVE'}
  Assert-NativeSeedInteger $control.schemaVersion $version $version
  if($control.profile-isnot[string]-or$control.profile-cne$profile){throw 'Closed campaign control profile required'}
 }
 if($Plan.scenarioId-cne$Binding.fixtureId-or$Lease.scenarioId-cne$Binding.fixtureId-or$Lease.phase-cne'CAMPAIGN_BINDING_ONLY'-or$Lease.schemaVersion-ne1-or$Lease.authorized-isnot[bool]-or!$Lease.authorized-or$Lease.issuer-cne'/root'-or$Lease.driver-cne'/root/recovery_game_driver'-or$Binding.driverRole-cne$Lease.driver){throw 'Closed campaign control launch authority required'}
 if($Plan.ownership.driver-cne$Lease.driver-or$Plan.ownership.onlyGameInputDriver-isnot[bool]-or!$Plan.ownership.onlyGameInputDriver-or[IO.Path]::GetFullPath($Plan.paths.scenario)-cne[IO.Path]::GetFullPath($Binding.fixtureRoot)){throw 'Campaign control driver and scenario required'}
 foreach($pair in @(@('maxScenarioActionsIncludingActivations',8),@('maxProxyInvocations',3),@('proxyTimeoutMs',180000),@('ownedGameHardTimeoutMs',480000))){Assert-NativeSeedInteger $Plan.bounds.($pair[0]) $pair[1] $pair[1]}
 foreach($pair in @(@('maxActions',8),@('maxProxyInvocations',3),@('ownedGameHardTimeoutMs',480000))){Assert-NativeSeedInteger $Lease.($pair[0]) $pair[1] $pair[1]}
 if($Lease.campaignLoadSaveAllowed-isnot[bool]-or$Lease.campaignLoadSaveAllowed){throw 'Campaign control refuses general save and load authority'}
 $batches=@($Plan.batches)
 $batchCount=if($inspection){2}else{3};$batchNames=if($inspection){'B1,E1'}else{'B1,E1,E2'}
 if($batches.Count-ne$batchCount-or(@($batches.id|Sort-Object -Unique)-join',')-cne$batchNames){throw 'Campaign control requires exact profile batches'}
 Assert-NativeSeedFields $Plan.historyBaseline @('overallAttempts','uiAttempts','reservedActions','sentActions')
 foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){Assert-NativeSeedInteger $Plan.historyBaseline.$key 0 9007199254740991}
 if($Plan.historyBaseline.sentActions-gt$Plan.historyBaseline.reservedActions){throw 'Campaign control original action history required'}
 Assert-NativeSeedInteger $Plan.callerAdmissionDeadlineMs 1 9007199254740991
 Assert-NativeSeedInteger $Lease.callerAdmissionDeadlineMs $Plan.callerAdmissionDeadlineMs $Plan.callerAdmissionDeadlineMs
 $remaining=$Plan.callerAdmissionDeadlineMs-[DateTimeOffset]::UtcNow.ToUnixTimeMilliseconds()
 if($remaining-lt150000-or$remaining-gt480000){throw 'Campaign control original caller admission deadline required'}
 $planWatchdog=@($Plan.PSObject.Properties.Name)-ccontains'sessionWatchdogMs'
 $leaseWatchdog=@($Lease.PSObject.Properties.Name)-ccontains'sessionWatchdogMs'
 if($planWatchdog-ne$leaseWatchdog){throw 'Campaign control watchdog must be paired'}
 if($planWatchdog){Assert-NativeSeedInteger $Plan.sessionWatchdogMs 480000 480000;Assert-NativeSeedInteger $Lease.sessionWatchdogMs 480000 480000}
 foreach($authority in @($Plan,$Lease)){
  foreach($key in @('ownedNodeKill','campaign','e1MenuReceipt')){if(@($authority.PSObject.Properties.Name)-ccontains$key){throw 'Campaign control refuses unrelated phase authority'}}
  $hasSeed=@($authority.PSObject.Properties.Name)-ccontains'seed'
  if((($inspection-or$generation-or$creation)-and$hasSeed)-or(!$inspection-and!$generation-and!$creation-and!$hasSeed)){throw 'Campaign control exact profile seed authority required'}
 }
 if($inspection-or$generation-or$creation){
  foreach($name in @('seed-manifest.private.json','seed-stage-plan.private.json','seed-stage-authority.private.json','seed-stage-claim.private.json','seed-stage-receipt.private.json')){if(Test-Path -LiteralPath (Join-Path $Binding.fixtureRoot $name)){throw 'First-page inspection refuses seed metadata'}}
  $saves=Join-Path $Binding.fixtureRoot 'installation/saves';Assert-NativeSeedOrdinary $saves
  if(!(Get-Item -LiteralPath $saves).PSIsContainer-or@(Get-ChildItem -LiteralPath $saves -Force).Count-ne0){throw 'Empty first-page inspection saves required'}
 }
 $supportPath=Join-Path $Binding.fixtureRoot 'frozen-support-pins.private.json'
 if([IO.Path]::GetFullPath($Lease.supportPinsPath)-cne[IO.Path]::GetFullPath($supportPath)){throw 'Campaign control exact support manifest required'}
 $support=Read-NativeSeedJson $supportPath $Lease.supportPinsSha256 262144
 Assert-NativeSeedFields $support @('pins')
 if($creation){Assert-NativeSeedCreationHistory $Plan $Lease $Binding.fixtureRoot}
 $sources=@('guarded-http-ownership.ps1','campaign-authority.mjs','batch-authorization.mjs','batch-session.mjs','finite-consent.mjs','fixture-binding.mjs','direct-frames.mjs','campaign-proof.mjs','generation-proof.mjs','seed-creation-history.mjs','seed-zero-provider-attempt.mjs','seed-zero-provider-terminal.mjs','seed-wizard-proof.mjs','seed-creation-output.mjs','seed-creation-custody.mjs')+@('native-controller.ps1','protection-check.ps1','fixture-inputs.ps1','seed-fixture.ps1','seed-creation-history.ps1','seed-zero-provider-attempt.ps1','seed-zero-provider-terminal.ps1','seed-preference.ps1','seed-provenance-v2.ps1','publish-owned-snapshot.ps1','session-watchdog.ps1','menu-launch.ps1'|ForEach-Object {'fixture/'+$_})
 foreach($relative in $sources){
  $path=[IO.Path]::GetFullPath((Join-Path $Binding.repositoryRoot ('tools/starsector-proxy/'+$relative)))
  $pins=@($support.pins|Where-Object {$_.path-is[string]-and[IO.Path]::GetFullPath($_.path)-ceq$path})
  if($pins.Count-ne1){throw 'Campaign control current tooling pin required'}
  Assert-NativeSeedOrdinary $path
  if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash-ine$pins[0].sha256){throw 'Campaign control tooling source changed'}
 }
}
function Read-NativeSeedPolicy($Binding,$Plan,$Lease) {
 Assert-NativeCampaignControl $Binding $Plan $Lease
 $planOptIn=$null-ne$Plan-and@($Plan.PSObject.Properties.Name)-ccontains'seed'
 $leaseOptIn=$null-ne$Lease-and@($Lease.PSObject.Properties.Name)-ccontains'seed'
 if($planOptIn-ne$leaseOptIn){throw 'Seed plan and lease authority must agree'}
 if(!$planOptIn){
  foreach($name in @('seed-stage-claim.private.json','seed-stage-receipt.private.json')){if(Test-Path -LiteralPath (Join-Path $Binding.fixtureRoot $name)){throw 'Seed state requires explicit launch opt-in'}}
  return $null
 }
 foreach($spec in @($Plan.seed,$Lease.seed)){
  Assert-NativeSeedFields $spec @('mode','manifestSha256','stagingReceiptSha256')
  if($spec.mode-cne'LOAD_ONE_PRIVATE_DISPOSABLE_SEED'){throw 'Closed seeded load mode required'}
  Assert-NativeSeedHash $spec.manifestSha256
  Assert-NativeSeedHash $spec.stagingReceiptSha256
 }
 if($Plan.seed.manifestSha256-ine$Lease.seed.manifestSha256-or$Plan.seed.stagingReceiptSha256-ine$Lease.seed.stagingReceiptSha256){throw 'Seed plan and lease authority must agree'}
 $receipt=Read-NativeSeedJson (Join-Path $Binding.fixtureRoot 'seed-stage-receipt.private.json') $Plan.seed.stagingReceiptSha256
 Assert-NativeSeedFields $receipt @('schemaVersion','kind','scenarioId','authoritySha256','claimSha256','manifestSha256','fixtureReceiptSha256','sourcePinsSha256','saveDirectory','files','xml','sourceUnchanged','logAbsent','nativeLaunched','gameLaunched','atUtc')
 Assert-NativeSeedInteger $receipt.schemaVersion 1 1
 if($receipt.kind-cne'NATIVE99_STAGED_SAVE_RECEIPT_V1'-or$receipt.scenarioId-cne$Binding.fixtureId-or$receipt.manifestSha256-ine$Plan.seed.manifestSha256){throw 'Seed staging receipt identity mismatch'}
 foreach($key in @('sourceUnchanged','logAbsent')){if($receipt.$key-isnot[bool]-or!$receipt.$key){throw 'Successful seed staging receipt required'}}
 foreach($key in @('nativeLaunched','gameLaunched')){if($receipt.$key-isnot[bool]-or$receipt.$key){throw 'Prelaunch seed staging receipt required'}}
 $grant=Read-NativeSeedStageAuthority $Binding (Join-Path $Binding.fixtureRoot 'seed-stage-authority.private.json') $receipt.authoritySha256
 if($grant.source.manifestSha256-ine$receipt.manifestSha256-or$grant.authority.fixtureReceiptSha256-ine$receipt.fixtureReceiptSha256-or$grant.authority.sourcePinsSha256-ine$receipt.sourcePinsSha256-or$receipt.saveDirectory-cne$grant.source.manifest.saveDirectory){throw 'Seed staging authority linkage mismatch'}
 $claim=Read-NativeSeedJson (Join-Path $Binding.fixtureRoot 'seed-stage-claim.private.json') $receipt.claimSha256 8192
 Assert-NativeSeedFields $claim @('schemaVersion','kind','scenarioId','authoritySha256','manifestSha256','fixtureReceiptSha256','sourcePinsSha256','atUtc')
 Assert-NativeSeedInteger $claim.schemaVersion 1 1
 if($claim.kind-cne'NATIVE99_SEED_STAGE_CLAIM_V1'-or$claim.scenarioId-cne$Binding.fixtureId){throw 'Seed staging claim identity mismatch'}
 foreach($key in @('authoritySha256','manifestSha256','fixtureReceiptSha256','sourcePinsSha256')){if($claim.$key-ine$receipt.$key){throw 'Seed staging claim linkage mismatch'}}
 $actual=Assert-NativeSeedTarget $Binding $grant.source
 # Receipt file rows are independently compared to current files, not trusted as a census.
 $rows=@($receipt.files)
 if($rows.Count-ne2){throw 'Seed receipt file census mismatch'}
 foreach($row in $rows){
  Assert-NativeSeedFields $row @('relative','bytes','sha256')
  if($row.relative-isnot[string]-or$row.relative-cnotin@('campaign.xml','descriptor.xml')){throw 'Closed seed receipt file paths required'}
  Assert-NativeSeedInteger $row.bytes 1 16777216
  Assert-NativeSeedHash $row.sha256
  $match=@($actual.files|Where-Object{$_.relative-ceq$row.relative-and$_.bytes-eq$row.bytes-and$_.sha256-ieq$row.sha256})
  if($match.Count-ne1){throw 'Seed receipt file census mismatch'}
 }
 if(@($rows.relative|Sort-Object -Unique).Count-ne2){throw 'Seed receipt file census mismatch'}
 Assert-NativeSeedFields $receipt.xml @('campaignRoot','descriptorRoot','saveDirectory','gameVersion','compressed','enabledMods','playerFleetResolved')
 Assert-NativeSeedInteger $receipt.xml.enabledMods 0 0
 foreach($key in $actual.xml.PSObject.Properties.Name){
  if($key-ceq'enabledMods'){continue}
  if($receipt.xml.$key.GetType()-ne$actual.xml.$key.GetType()-or$receipt.xml.$key-cne$actual.xml.$key){throw 'Seed receipt XML summary mismatch'}
 }
 return [pscustomobject]@{saveDirectory=$grant.source.manifest.saveDirectory;files=$actual.files;manifestSha256=$receipt.manifestSha256;stagingReceiptSha256=$Plan.seed.stagingReceiptSha256;source=$grant.source}
}
