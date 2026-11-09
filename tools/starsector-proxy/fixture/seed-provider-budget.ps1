# A fixed paired data selector; native calls and the original root clock do not change.
function Assert-NativeSeedProviderTimingJson($Value,[byte[]]$Bytes) {
 if($Bytes.Length-lt2-or$Bytes.Length-gt65536){throw 'Bounded provider timing JSON required'}
 $strict=ConvertFrom-ZeroProviderObject ([Text.UTF8Encoding]::new($false,$true).GetString($Bytes))
 # These are the consumed new selector fields. Existing unrelated date/string
 # parsing remains unchanged in the legacy launch reader.
 foreach($key in @('providerTiming','campaignControl','sessionWatchdogMs','historyBaseline')){
  if(($strict.$key|ConvertTo-Json -Depth 20 -Compress)-cne($Value.$key|ConvertTo-Json -Depth 20 -Compress)){throw 'Provider timing JSON changed'}
 }
}
function Get-NativeSeedProviderTimeoutMs($Plan,$Lease) {
 $hasPlan=@($Plan.PSObject.Properties.Name)-ccontains'providerTiming'
 $hasLease=@($Lease.PSObject.Properties.Name)-ccontains'providerTiming'
 if(!$hasPlan-and!$hasLease){return 180000}
 if(!$hasPlan-or!$hasLease){throw 'Closed paired seed provider timing required'}
 foreach($value in @($Plan,$Lease)){
  $timing=$value.providerTiming;$control=$value.campaignControl
  Assert-NativeSeedFields $timing @('schemaVersion','profile','providerTimeoutMs')
  Assert-NativeSeedInteger $timing.schemaVersion 1 1
  Assert-NativeSeedInteger $timing.providerTimeoutMs 480000 480000
  if($timing.profile-isnot[string]-or$timing.profile-cne'INITIAL_FOCUS_PROVIDER_480000'){throw 'Closed paired seed provider timing required'}
  Assert-NativeSeedFields $control @('schemaVersion','profile')
  Assert-NativeSeedInteger $control.schemaVersion 5 5
  if($control.profile-isnot[string]-or$control.profile-cne'DISPOSABLE_SEED_CREATION_INITIAL_FOCUS'){throw 'Initial focus provider timing required'}
  Assert-NativeSeedInteger $value.sessionWatchdogMs 1800000 1800000
  Assert-NativeSeedFields $value.historyBaseline @('overallAttempts','uiAttempts','reservedActions','sentActions')
  foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){
   Assert-NativeSeedInteger $value.historyBaseline.$key 0 9007199254740991
   Assert-NativeSeedInteger $value.historyBaseline.$key $Plan.historyBaseline.$key $Plan.historyBaseline.$key
  }
 }
 return 480000
}
