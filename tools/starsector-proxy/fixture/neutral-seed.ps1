# Read-only identity inspection. Never creates/rewrites/stages a save or grants native authority.
. (Join-Path $PSScriptRoot 'seed-fixture.ps1')

function Read-NeutralSeedIdentity([string]$Directory,[string]$SaveDirectory,[string]$RecipePath,[string]$RecipeSha256) {
 $inspection=& node (Join-Path $PSScriptRoot 'neutral-recipe-cli.mjs') inspect $RecipePath $RecipeSha256 2>&1
 if($LASTEXITCODE-ne0){throw 'Pinned neutral recipe inspection required before seed identity'}
 $recipe=Read-NativeSeedJson $RecipePath $RecipeSha256 65536
 if($recipe.schemaVersion-ne1-or$recipe.profile-cne'NEUTRAL_ACK32_TOT_NEX_V1'-or$recipe.gameVersion-cne'0.98a-RC8'-or$recipe.protocol-cne'neutral-probe-v1-ack32'-or$recipe.companionVersion-cne'0.1.0'){throw 'Exact neutral seed profile required'}
 if($SaveDirectory-cnotmatch'^save_[A-Za-z0-9_]{1,128}$'){throw 'Neutral seed save identity required'}
 Assert-NativeSeedOrdinary $Directory
 $files=@(Get-ChildItem -LiteralPath $Directory -Force)
 if($files.Count-ne2-or@($files|Where-Object{$_.PSIsContainer-or($_.Attributes-band[IO.FileAttributes]::ReparsePoint)-or$_.Name-cnotin@('campaign.xml','descriptor.xml')}).Count){throw 'Exact two-file neutral seed required'}
 $expected=@{MagicLib='1.5.6';aotd_theory_of_toolbox='1.0.11';ashlib='2.2.3';bmo='2.1.0';lw_lazylib='3.0.0';nexerelin='0.12.2c';near_future_probe='0.1.0'}
 $documents=@{}
 foreach($name in @('campaign.xml','descriptor.xml')){
  $path=Join-Path $Directory $name;Assert-NativeSeedOrdinary $path
  $stream=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
  try{
   if($stream.Length-lt1-or$stream.Length-gt16777216){throw 'Bounded plain neutral seed XML required'}
   $settings=[Xml.XmlReaderSettings]::new();$settings.DtdProcessing=[Xml.DtdProcessing]::Prohibit;$settings.XmlResolver=$null;$settings.MaxCharactersInDocument=16777216
   $reader=[Xml.XmlReader]::Create($stream,$settings);$document=[Xml.XmlDocument]::new();$document.XmlResolver=$null
   try{$document.Load($reader)}finally{$reader.Dispose()}
   $all=@($document.SelectNodes('//*'));if($all.Count-gt50000){throw 'Neutral seed XML node bound'}
   foreach($element in $all){
    if($element.NamespaceURI-cne''){throw 'Neutral seed XML namespace refused'}
    $depth=0;$parent=$element;while($null-ne$parent){$depth++;if($depth-gt64){throw 'Neutral seed XML depth bound'};$parent=$parent.ParentNode}
   }
   $documents[$name]=$document
  }finally{$stream.Dispose()}
 }
 $campaign=$documents['campaign.xml'];$descriptor=$documents['descriptor.xml']
 if($campaign.DocumentElement.Name-cne'CampaignEngine'-or$descriptor.DocumentElement.Name-cne'SaveGameData'){throw 'Exact neutral seed roots required'}
 if((Get-NativeSeedXmlElement $campaign '/CampaignEngine/saveDirName').InnerText-cne$SaveDirectory){throw 'Neutral seed save identity mismatch'}
 $fleet=Get-NativeSeedXmlElement $campaign '/CampaignEngine/playerFleet';$reference=$fleet.GetAttribute('ref')
 if($reference-cnotmatch'^[0-9]{1,20}$'){throw 'Neutral seed fleet reference required'}
 $targets=@($campaign.SelectNodes('//*[@z]')|Where-Object{$_.GetAttribute('z')-ceq$reference})
 if($targets.Count-ne1-or$targets[0].Name-cne'Flt'){throw 'Unique neutral fleet reference required'}
 $character=Get-NativeSeedXmlElement $campaign '/CampaignEngine/characterData'
 if(@($character.ChildNodes|Where-Object NodeType -eq Element).Count-lt1){throw 'Neutral seed character data required'}
 $null=Get-NativeSeedXmlElement $campaign '/CampaignEngine/saveDesc'
 if((Get-NativeSeedXmlElement $descriptor '/SaveGameData/compressed').InnerText-cne'false'-or(Get-NativeSeedXmlElement $descriptor '/SaveGameData/gameVersion').InnerText-cne'0.98a-RC8'){throw 'Neutral seed version/compression mismatch'}
 $index=[Collections.Generic.Dictionary[string,object]]::new([StringComparer]::Ordinal)
 foreach($element in $descriptor.SelectNodes('//*[@z]')){
  $key=$element.GetAttribute('z');if($key-cnotmatch'^[0-9]{1,20}$'-or$index.ContainsKey($key)){throw 'Unique neutral descriptor reference required'}
  $index.Add($key,$element)
 }
 function Resolve-NeutralReference($Node) {
  $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal);$count=0
  while($Node.HasAttribute('ref')){
   $key=$Node.GetAttribute('ref');$count++
   if($Node.Attributes.Count-ne1-or!([string]::IsNullOrWhiteSpace($Node.InnerText))-or@($Node.ChildNodes|Where-Object NodeType -eq Element).Count-ne0-or$key-cnotmatch'^[0-9]{1,20}$'-or!$seen.Add($key)-or$count-gt4-or!$index.ContainsKey($key)){throw 'Closed neutral mod reference required'}
   $Node=$index[$key]
  }
  return $Node
 }
 $mods=Get-NativeSeedXmlElement $descriptor '/SaveGameData/enabledMods';$entries=@($mods.ChildNodes|Where-Object NodeType -eq Element)
 if($entries.Count-ne7){throw 'Exact seven declared neutral mods required'}
 $found=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
 foreach($entry in $entries){
  if($entry.Name-cne'EnabledModData'-or@($entry.ChildNodes|Where-Object NodeType -eq Element).Count-ne1){throw 'Native EnabledModData shape required'}
  $spec=Resolve-NeutralReference (Get-NativeSeedXmlElement $entry 'spec')
  $id=(Get-NativeSeedXmlElement $spec 'id').InnerText
  $version=Resolve-NeutralReference (Get-NativeSeedXmlElement $spec 'versionInfo')
  $major=(Get-NativeSeedXmlElement $version 'major').InnerText;$minor=(Get-NativeSeedXmlElement $version 'minor').InnerText;$patch=(Get-NativeSeedXmlElement $version 'patch').InnerText
  if($major-cnotmatch'^[0-9]{1,3}$'-or$minor-cnotmatch'^[0-9]{1,3}$'-or$patch-cnotmatch'^[0-9A-Za-z]{1,8}$'-or(Get-NativeSeedXmlElement $version 'minorIsRC').InnerText-cne'false'){throw 'Native neutral version shape required'}
  $normalized=$major+'.'+$minor+'.'+$patch
  if(!$expected.ContainsKey($id)-or$found.ContainsKey($id)-or$expected[$id]-cne$normalized){throw 'Neutral declared mod identity/version mismatch'}
  $found.Add($id,$normalized)
 }
 # Metadata inspection cannot prove native creation, source parity or actual loader behavior.
 return [pscustomobject][ordered]@{schemaVersion=1;kind='NEUTRAL_SEED_IDENTITY_ONLY_V1';profile='NEUTRAL_ACK32_TOT_NEX_V1';recipeSha256=$RecipeSha256.ToLowerInvariant();gameVersion='0.98a-RC8';saveDirectory=$SaveDirectory;compressed=$false;declaredMods=@($found.Keys|Sort-Object|ForEach-Object{[pscustomobject]@{id=$_;version=$found[$_]}});nativeCreatedProof=$false;launchAdmitted=$false}
}
