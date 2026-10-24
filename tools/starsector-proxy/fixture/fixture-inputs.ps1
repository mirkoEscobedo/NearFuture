function Read-NativeFixtureBinding([string]$Path,[string]$ExpectedSha256) {
 $ErrorActionPreference='Stop'
 $repositoryRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..')).TrimEnd([IO.Path]::DirectorySeparatorChar)
 $scenariosRoot=Join-Path $repositoryRoot '.tmp/starsector-proxy/scenarios'
 function Require-Ordinary([string]$Value) {
  $item=Get-Item -LiteralPath $Value -Force
  while($null-ne$item){if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary fixture input required'};$item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}}
 }
 if($ExpectedSha256-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Pinned fixture binding required'}
 Require-Ordinary $Path
 $file=Get-Item -LiteralPath $Path
 if($file.PSIsContainer-or$file.Length-lt2-or$file.Length-gt16384-or(Get-FileHash -LiteralPath $Path -Algorithm SHA256).Hash-ine$ExpectedSha256){throw 'Fixture binding hash or size mismatch'}
 $binding=Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json -DateKind String
 $version=$binding.schemaVersion
 if(($version-isnot[int]-and$version-isnot[long])-or$version-cnotin@(1,2,3)){throw 'Fixture input version required'}
 if($version-eq3){
  $document=[Text.Json.JsonDocument]::Parse([IO.File]::ReadAllText($Path))
  try{
   $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
   foreach($property in $document.RootElement.EnumerateObject()){if(!$seen.Add($property.Name)){throw 'Duplicate generation binding field refused'}}
  }finally{$document.Dispose()}
 }
 $names=@('schemaVersion','fixtureRoot','sourceInstallation','qualifiedRuntimeRoot','candidateHead','runtimeManifestSha256','presentationAgentPath','presentationAgentSha256','presentationClassSha256','driverRole')
 if($version-eq2){$names+='recordingProfileSha256'}
 if($version-eq3){$names+='fixtureProfile'}
 if((@($binding.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($names|Sort-Object)-join',')){throw 'Closed fixture input schema required'}
 if($version-eq3-and($binding.fixtureProfile-isnot[string]-or$binding.fixtureProfile-cne'VANILLA_UNCOMPRESSED_GENERATION')){throw 'Closed vanilla generation fixture profile required'}
 foreach($key in @('fixtureRoot','sourceInstallation','qualifiedRuntimeRoot','presentationAgentPath')){
  if($binding.$key-isnot[string]-or![IO.Path]::IsPathFullyQualified($binding.$key)){throw 'Absolute fixture input path required'}
  Require-Ordinary $binding.$key
 }
 $fixture=[IO.Path]::GetFullPath($binding.fixtureRoot)
 if([IO.Path]::GetDirectoryName($fixture)-ine[IO.Path]::GetFullPath($scenariosRoot)-or[IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($Path))-ine$fixture){throw 'Private scenario binding containment required'}
 if($binding.candidateHead-isnot[string]-or$binding.candidateHead-cnotmatch'^[a-f0-9]{40}$'){throw 'Qualified candidate head required'}
 foreach($key in @('runtimeManifestSha256','presentationAgentSha256','presentationClassSha256')){if($binding.$key-isnot[string]-or$binding.$key-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Qualified artifact hash required'}}
 if($binding.driverRole-isnot[string]-or[string]::IsNullOrWhiteSpace($binding.driverRole)-or$binding.driverRole.Length-gt128){throw 'Explicit finite driver role required'}
 foreach($key in @('fixtureRoot','sourceInstallation','qualifiedRuntimeRoot')){if(!(Get-Item -LiteralPath $binding.$key).PSIsContainer){throw 'Fixture input directory required'}}
 $agent=Get-Item -LiteralPath $binding.presentationAgentPath
 if($agent.PSIsContainer-or$agent.Length-lt1-or$agent.Length-gt67108864-or(Get-FileHash -LiteralPath $binding.presentationAgentPath -Algorithm SHA256).Hash-ine$binding.presentationAgentSha256){throw 'Presentation agent binding changed'}
 $binding|Add-Member -NotePropertyName repositoryRoot -NotePropertyValue $repositoryRoot
 $binding|Add-Member -NotePropertyName scenariosRoot -NotePropertyValue $scenariosRoot
 $binding|Add-Member -NotePropertyName fixtureId -NotePropertyValue ([IO.Path]::GetFileName($fixture))
 if($version-eq2){$binding|Add-Member -NotePropertyName recordingProfile -NotePropertyValue (Read-NativeRecordingProfile $binding)}
 return $binding
}

function Read-NativeFixtureRecipe($Binding) {
 $version=$Binding.schemaVersion
 if(($version-isnot[int]-and$version-isnot[long])-or$version-cnotin@(1,2,3)){throw 'Fixture recipe version required'}
 if($version-eq3){
  if($Binding.fixtureProfile-cne'VANILLA_UNCOMPRESSED_GENERATION'){throw 'Closed vanilla generation fixture profile required'}
  return [pscustomobject]@{enabledModsJson='{"enabledMods":[]}';requireUncompressed=$true}
 }
 return [pscustomobject]@{enabledModsJson='{"enabledMods":["near_future"]}';requireUncompressed=$false}
}

function Assert-NativeFixtureSettings($Binding,[string]$Text) {
 if(!(Read-NativeFixtureRecipe $Binding).requireUncompressed){return}
 # Preserve strings while removing the comments accepted by the game's settings file.
 $clean=[regex]::Replace($Text,'"(?:\\.|[^"\\])*"|//[^\r\n]*|/\*[\s\S]*?\*/|#[^\r\n]*',[Text.RegularExpressions.MatchEvaluator]{
  param($match)
  if($match.Value.StartsWith('"')){return $match.Value}
  return ''
 })
 $options=[Text.Json.JsonDocumentOptions]::new();$options.AllowTrailingCommas=$true
 try{$document=[Text.Json.JsonDocument]::Parse($clean,$options)}catch{throw 'Generation requires uncompressed save data'}
 try{
  if($document.RootElement.ValueKind-ne[Text.Json.JsonValueKind]::Object){throw 'Generation requires uncompressed save data'}
  $fields=@($document.RootElement.EnumerateObject()|Where-Object {$_.Name-ieq'compressSaveGameData'})
  if($fields.Count-ne1-or$fields[0].Name-cne'compressSaveGameData'-or$fields[0].Value.ValueKind-ne[Text.Json.JsonValueKind]::False){throw 'Generation requires uncompressed save data'}
 }finally{$document.Dispose()}
}


function Assert-NativeRecordingOrdinary([string]$Path) {
 $item=Get-Item -LiteralPath $Path -Force
 while($null-ne$item){if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary recording profile input required'};$item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}}
 if((Get-Item -LiteralPath $Path).PSIsContainer){throw 'Recording profile file required'}
}
function Assert-NativeRecordingProfileValue($Profile) {
 $names=@('schemaVersion','kind','id','baseDefaultJfcSHA256','gameJvmReleaseSHA256','enabledEvents','stackTrace','classLoadThresholdMs','recordingDurationSeconds','recordingMaxSizeMiB')
 if($Profile-isnot[pscustomobject]-or(@($Profile.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($names|Sort-Object)-join',')){throw 'Closed recording profile required'}
 foreach($pair in @(@('schemaVersion',1),@('classLoadThresholdMs',0),@('recordingDurationSeconds',420),@('recordingMaxSizeMiB',64))){
  $value=$Profile.($pair[0]);if(($value-isnot[int]-and$value-isnot[long])-or$value-ne$pair[1]){throw 'Fixed recording profile bounds required'}
 }
 if($Profile.kind-isnot[string]-or$Profile.id-isnot[string]-or$Profile.kind-cne'NATIVE99_JFR_STARTUP_PROFILE_V1'-or$Profile.id-cne'CLASSLOAD_EXCEPTIONS_V1'-or$Profile.stackTrace-isnot[bool]-or!$Profile.stackTrace){throw 'Fixed startup recording profile required'}
 foreach($key in @('baseDefaultJfcSHA256','gameJvmReleaseSHA256')){if($Profile.$key-isnot[string]-or$Profile.$key-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Recording runtime digest required'}}
 $events=@('jdk.ClassLoad','jdk.JavaExceptionThrow','jdk.JavaErrorThrow')
 if($Profile.enabledEvents-isnot[array]-or$Profile.enabledEvents.Count-ne3){throw 'Exactly three recording events required'}
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
 foreach($name in $Profile.enabledEvents){if($name-isnot[string]-or$name-cnotin$events-or!$seen.Add($name)){throw 'Exactly three unique recording events required'}}
}
function Read-NativeRecordingProfile($Binding) {
 if($Binding.schemaVersion-in@(1,3)){return $null}
 if($Binding.recordingProfileSha256-isnot[string]-or$Binding.recordingProfileSha256-cnotmatch'^[A-Fa-f0-9]{64}$'){throw 'Pinned recording profile required'}
 $path=Join-Path $Binding.fixtureRoot 'recording-profile.private.json'
 Assert-NativeRecordingOrdinary $path
 $file=Get-Item -LiteralPath $path
 if($file.Length-lt2-or$file.Length-gt16384-or(Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash-ine$Binding.recordingProfileSha256){throw 'Recording profile hash or size mismatch'}
 $text=[IO.File]::ReadAllText($path)
 $document=[Text.Json.JsonDocument]::Parse($text)
 try {
  if($document.RootElement.ValueKind-ne[Text.Json.JsonValueKind]::Object){throw 'Recording profile object required'}
  $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::OrdinalIgnoreCase)
  foreach($property in $document.RootElement.EnumerateObject()){if(!$seen.Add($property.Name)){throw 'Duplicate recording profile field refused'}}
 }finally{$document.Dispose()}
 $profile=$text|ConvertFrom-Json -DateKind String
 Assert-NativeRecordingProfileValue $profile
 foreach($row in @(@('jre/lib/jfr/default.jfc','baseDefaultJfcSHA256'),@('jre/release','gameJvmReleaseSHA256'))){
  $runtimePath=Join-Path $Binding.sourceInstallation $row[0]
  Assert-NativeRecordingOrdinary $runtimePath
  if((Get-FileHash -LiteralPath $runtimePath -Algorithm SHA256).Hash-ine$profile.($row[1])){throw 'Recording source JVM or base JFC changed'}
 }
 return $profile
}
function New-NativeRecordingConfiguration([xml]$DefaultJfc,$Profile=$null) {
 [xml]$jfc=$DefaultJfc.CloneNode($true)
 $jfc.configuration.SetAttribute('label','NF99 disposable campaign lifecycle')
 $jfc.configuration.SetAttribute('description','Private read-only lifecycle binding; no performance or full campaign acceptance.')
 foreach($event in $jfc.configuration.event){
  foreach($setting in $event.setting){$setting.RemoveAttribute('control');if($setting.name-eq'enabled'){$setting.InnerText='false'}}
  if($event.name-in@('jdk.JVMInformation','jdk.ActiveRecording','jdk.ActiveSetting','jdk.DataLoss')){
   $event.SelectSingleNode("setting[@name='enabled']").InnerText='true'
  }
 }
 foreach($name in @('nf.PresentInterval','nf.FrameProbeRejected','nf.ProbeConfiguration','nf.LocalLifecycle','nf.DiagnosticHandoffDelay','nf.DiagnosticStaleMailbox')){
  $event=$jfc.CreateElement('event');$event.SetAttribute('name',$name)
  foreach($entry in @(@('enabled','true'),@('stackTrace','false'))){$setting=$jfc.CreateElement('setting');$setting.SetAttribute('name',$entry[0]);$setting.InnerText=$entry[1];$event.AppendChild($setting)|Out-Null}
  if($name-ceq'nf.DiagnosticHandoffDelay'){
   $setting=$jfc.CreateElement('setting');$setting.SetAttribute('name','threshold');$setting.InnerText='0 ms';$event.AppendChild($setting)|Out-Null
  }
  $jfc.configuration.AppendChild($event)|Out-Null
 }
 if($null-ne$Profile){
  Assert-NativeRecordingProfileValue $Profile
  foreach($name in @('jdk.ClassLoad','jdk.JavaExceptionThrow','jdk.JavaErrorThrow')){
   $events=@($jfc.configuration.event|Where-Object {$_.name-ceq$name})
   if($events.Count-ne1){throw 'Unique existing recording event required'}
   $enabled=@($events[0].setting|Where-Object {$_.name-ceq'enabled'})
   $stack=@($events[0].setting|Where-Object {$_.name-ceq'stackTrace'})
   if($enabled.Count-ne1-or$enabled[0].InnerText-cne'false'-or$stack.Count-ne1-or$stack[0].InnerText-cne'true'){throw 'Existing recording event recipe changed'}
   if($name-ceq'jdk.ClassLoad'){
    $threshold=@($events[0].setting|Where-Object {$_.name-ceq'threshold'})
    if($threshold.Count-ne1-or$threshold[0].InnerText-cne'0 ms'){throw 'Existing class load threshold changed'}
   }
   $enabled[0].InnerText='true'
  }
 }
 return ,$jfc
}
function Assert-NativeRecordingRecipe($Binding,$Receipt) {
 if($Binding.schemaVersion-in@(1,3)){return}
 $profile=Read-NativeRecordingProfile $Binding
 if($Receipt.recordingProfileSha256-isnot[string]-or$Receipt.recordingProfileSha256-ine$Binding.recordingProfileSha256){throw 'Frozen recording profile agreement required'}
 foreach($row in @(@('jre/lib/jfr/default.jfc','baseDefaultJfcSHA256'),@('jre/release','gameJvmReleaseSHA256'))){
  $path=Join-Path (Join-Path $Binding.fixtureRoot 'installation') $row[0]
  Assert-NativeRecordingOrdinary $path
  if((Get-FileHash -LiteralPath $path -Algorithm SHA256).Hash-ine$profile.($row[1])){throw 'Staged recording JVM or base JFC changed'}
 }
 $jfcPath=Join-Path $Binding.fixtureRoot 'capture/active.jfc'
 $vmparamsPath=Join-Path $Binding.fixtureRoot 'installation/vmparams'
 foreach($path in @($jfcPath,$vmparamsPath)){
  Assert-NativeRecordingOrdinary $path
  if((Get-Item -LiteralPath $path).Length-gt1048576){throw 'Bounded recording recipe required'}
 }
 [xml]$baseline=[IO.File]::ReadAllText((Join-Path $Binding.sourceInstallation 'jre/lib/jfr/default.jfc'))
 $expected=New-NativeRecordingConfiguration $baseline $profile
 [xml]$actual=[IO.File]::ReadAllText($jfcPath)
 if($actual.OuterXml-cne$expected.OuterXml){throw 'Expected complete recording configuration required'}
 $scenario=$Binding.fixtureRoot
 $expectedFlag="-XX:StartFlightRecording=name=NF99,settings=$scenario/capture/active.jfc,duration=420s,maxsize=64m,dumponexit=true,filename=$scenario/capture/current.jfr"
 $vmparams=[IO.File]::ReadAllText($vmparamsPath)
 $occurrences=[regex]::Matches($vmparams,'-XX:StartFlightRecording')
 $flags=[regex]::Matches($vmparams,'(?<!\S)-XX:StartFlightRecording(?:=\S+)?(?=\s|$)')
 if($occurrences.Count-ne1-or$flags.Count-ne1-or$flags[0].Value-cne$expectedFlag){throw 'Fixed private recording flag required'}
}
function Assert-NativeRecordingAuthority($Binding,$Receipt,$Plan,$Lease) {
 if($Binding.schemaVersion-in@(1,3)){
  foreach($value in @($Receipt,$Plan,$Lease)){
   if($null-ne$value-and@($value.PSObject.Properties.Name)-contains'recordingProfileSha256'){throw 'Legacy fixture cannot carry recording profile authority'}
  }
  return
 }
 foreach($value in @($Receipt,$Plan,$Lease)){
  if($null-eq$value-or$value.recordingProfileSha256-isnot[string]-or$value.recordingProfileSha256-ine$Binding.recordingProfileSha256){throw 'Recording binding receipt plan and lease must agree'}
 }
}

