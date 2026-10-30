# One exact existing Java preference. No key creation, enumeration, export or write.
function Assert-NativeSeedPreferenceFields($Value,[string[]]$Names) {
 if($Value-isnot[pscustomobject]-or(@($Value.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($Names|Sort-Object)-join',')){throw 'Closed seed preference schema required'}
}
function Assert-NativeSeedPreferenceValue($Value) {
 Assert-NativeSeedPreferenceFields $Value @('schemaVersion','kind','view','node','valueName','registryValueName','valueKind','value')
 if(($Value.schemaVersion-isnot[int]-and$Value.schemaVersion-isnot[long])-or$Value.schemaVersion-ne1){throw 'Seed preference version required'}
 foreach($pair in @(@('kind','NATIVE_SEED_SHARED_PREFERENCE_V1'),@('view','Registry64'),@('node','/com/fs/starfarer'),@('valueName','canSkipTutorial'),@('registryValueName','can/Skip/Tutorial'),@('valueKind','String'),@('value','true'))){if($Value.($pair[0])-isnot[string]-or$Value.($pair[0])-cne$pair[1]){throw 'Eligible seed shared preference required'}}
}
function Get-NativeSeedPreferenceWitness($Plan,$Lease,[string]$Scenario) {
 foreach($s in @($Plan.seedCreation,$Lease.seedCreation)){
  Assert-NativeSeedPreferenceFields $s @('mode','preference')
  Assert-NativeSeedPreferenceFields $s.preference @('path','sha256')
  if($s.mode-cne'CREATE_ONE_PRIVATE_DISPOSABLE_SEED'-or$s.preference.path-cne(Join-Path $Scenario 'seed-creation-preference.private.json')-or$s.preference.sha256-isnot[string]-or$s.preference.sha256-cnotmatch'^[a-f0-9]{64}$'){throw 'Fixed seed preference witness required'}
 }
 $pin=$Plan.seedCreation.preference
 if($pin.path-cne$Lease.seedCreation.preference.path-or$pin.sha256-cne$Lease.seedCreation.preference.sha256){throw 'Paired seed preference witness required'}
 $item=Get-Item -LiteralPath $pin.path -Force
 if($item.PSIsContainer-or$item.Length-lt2-or$item.Length-gt16384){throw 'Bounded seed preference witness required'}
 while($null-ne$item){if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary seed preference witness required'};$item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}}
 if((Get-FileHash -LiteralPath $pin.path -Algorithm SHA256).Hash-ine$pin.sha256){throw 'Seed preference witness changed'}
 $text=[Text.UTF8Encoding]::new($false,$true).GetString([IO.File]::ReadAllBytes($pin.path))
 $document=[System.Text.Json.JsonDocument]::Parse($text)
 try{
  if($document.RootElement.ValueKind-ne[System.Text.Json.JsonValueKind]::Object){throw 'Closed seed preference JSON object required'}
  $names=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
  foreach($property in $document.RootElement.EnumerateObject()){if(!$names.Add($property.Name)){throw 'Duplicate seed preference field refused'}}
 }finally{$document.Dispose()}
 $value=$text|ConvertFrom-Json -DateKind String
 Assert-NativeSeedPreferenceValue $value
 return $value
}
function Read-NativeSeedSharedPreference {
 $root=[Microsoft.Win32.RegistryKey]::OpenBaseKey([Microsoft.Win32.RegistryHive]::CurrentUser,[Microsoft.Win32.RegistryView]::Registry64)
 try{
  $key=$root.OpenSubKey('Software\JavaSoft\Prefs\com\fs\starfarer',$false)
  if($null-eq$key){throw 'Existing seed shared preference required'}
  try{
   $value=$key.GetValue('can/Skip/Tutorial',$null,[Microsoft.Win32.RegistryValueOptions]::DoNotExpandEnvironmentNames)
   if($null-eq$value){throw 'Existing seed shared preference required'}
   $record=[pscustomobject]@{schemaVersion=[long]1;kind='NATIVE_SEED_SHARED_PREFERENCE_V1';view='Registry64';node='/com/fs/starfarer';valueName='canSkipTutorial';registryValueName='can/Skip/Tutorial';valueKind=$key.GetValueKind('can/Skip/Tutorial').ToString();value=$value}
   Assert-NativeSeedPreferenceValue $record
   return $record
  }finally{$key.Dispose()}
 }finally{$root.Dispose()}
}
function Assert-NativeSeedPreferenceUnchanged($Plan,[string]$Scenario) {
 $expected=Get-NativeSeedPreferenceWitness $Plan $Plan $Scenario
 $actual=Read-NativeSeedSharedPreference
 foreach($name in $expected.PSObject.Properties.Name){if($actual.$name.GetType()-ne$expected.$name.GetType()-or$actual.$name-cne$expected.$name){throw 'Seed shared preference changed'}}
 return $actual
}
