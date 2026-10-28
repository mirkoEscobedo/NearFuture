# New explicit seed-creation authority. Legacy generation and seed provenance remain separate.
. (Join-Path $PSScriptRoot 'seed-failed-attempt.ps1')
. (Join-Path $PSScriptRoot 'seed-zero-provider-attempt.ps1')
. (Join-Path $PSScriptRoot 'seed-pre-start-margin-attempt.ps1')
function Assert-NativeSeedCreationHistory($Plan,$Lease,[string]$Scenario) {
 $null=Get-NativeSeedPreferenceWitness $Plan $Lease $Scenario
 Assert-NativeSeedFields $Plan.historyAdmission @('path','sha256')
 Assert-NativeSeedFields $Lease.historyAdmission @('path','sha256')
 $reference=$Plan.historyAdmission
 $path=Join-Path $Scenario 'root-history-baseline.admitted.private.json'
 if($reference.path-cne$path-or$Lease.historyAdmission.path-cne$reference.path-or$Lease.historyAdmission.sha256-cne$reference.sha256){throw 'Fixed seed-creation history admission required'}
 $grant=Read-NativeModernSeedJson $path $reference.sha256 262144
 Assert-NativeSeedFields $grant @('schemaVersion','kind','issuer','driver','historyBaseline','maxReservedActions','previous')
 Assert-NativeSeedInteger $grant.schemaVersion 2 2
 if($grant.kind-cne'ROOT_ADMITTED_SEED_CREATION_HISTORY'-or$grant.issuer-cne'/root'-or$grant.driver-cne'/root/recovery_game_driver'){throw 'Explicit seed-creation extension required'}
 Assert-NativeSeedFields $grant.historyBaseline @('overallAttempts','uiAttempts','reservedActions','sentActions')
 foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){Assert-NativeSeedInteger $grant.historyBaseline.$key $Plan.historyBaseline.$key $Plan.historyBaseline.$key}
 $limit=$Plan.absoluteMaxReservedActions
 Assert-NativeSeedInteger $limit ($Plan.historyBaseline.reservedActions+1) ($Plan.historyBaseline.reservedActions+8)
 Assert-NativeSeedInteger $Lease.absoluteMaxReservedActions $limit $limit
 Assert-NativeSeedInteger $grant.maxReservedActions $limit $limit
 Assert-NativeSeedFields $grant.previous @('result','ledger')
 $prior=@{}
 foreach($key in @('result','ledger')){
  $pin=$grant.previous.$key;Assert-NativeSeedFields $pin @('path','sha256')
  if($pin.path-isnot[string]-or![IO.Path]::IsPathFullyQualified($pin.path)){throw 'Absolute previous seed history required'}
  $prior[$key]=Read-NativeSeedJson $pin.path $pin.sha256 262144
 }
 $r=$prior.result;$l=$prior.ledger
 if($r.kind-ceq'ROOT_CLOSED_PRE_START_INPUT_MARGIN_FAILED_SEED_ATTEMPT_HISTORY'){
  foreach($pin in @($Plan.historyAdmission,$Lease.historyAdmission,$grant.previous.result,$grant.previous.ledger)){
   Assert-NativeSeedFields $pin @('path','sha256')
   if($pin.path-isnot[string]-or$pin.sha256-isnot[string]-or$pin.sha256-cnotmatch'^[a-f0-9]{64}$'){throw 'Scalar pre-Start history descriptor required'}
  }
  foreach($key in @('kind','issuer','driver')){if($grant.$key-isnot[string]){throw 'Scalar pre-Start grant authority required'}}
  $null=Read-NativeZeroProviderJson $path $reference.sha256
  $r=Read-NativeZeroProviderJson $grant.previous.result.path $grant.previous.result.sha256
  $l=Read-NativeZeroProviderJson $grant.previous.ledger.path $grant.previous.ledger.sha256
  $actions=Read-NativePreStartMarginFailedSeedAttempt $r $l $grant.previous.ledger $grant.previous.result $Plan $Lease
  $r=[pscustomobject]@{schemaVersion=1;kind=$r.kind;allOwnedHandlesTerminalReaped=$true;actions=$actions}
 }
 if($r.kind-ceq'ROOT_CLOSED_PRE_INPUT_CALLER_DATE_FAILED_SEED_ATTEMPT_HISTORY'){
  foreach($pin in @($Plan.historyAdmission,$Lease.historyAdmission,$grant.previous.result,$grant.previous.ledger)){
   Assert-NativeSeedFields $pin @('path','sha256');if($pin.path-isnot[string]-or$pin.sha256-isnot[string]-or$pin.sha256-cnotmatch'^[a-f0-9]{64}$'){throw 'Scalar caller-date history descriptor required'}
  }
  foreach($key in @('kind','issuer','driver')){if($grant.$key-isnot[string]){throw 'Scalar caller-date grant authority required'}}
  $null=Read-NativeZeroProviderJson $path $reference.sha256
  $r=Read-NativeZeroProviderJson $grant.previous.result.path $grant.previous.result.sha256
  $l=Read-NativeZeroProviderJson $grant.previous.ledger.path $grant.previous.ledger.sha256
  $actions=Read-NativeCallerDateFailedSeedAttempt $r $l $grant.previous.ledger
  $r=[pscustomobject]@{schemaVersion=1;kind=$r.kind;allOwnedHandlesTerminalReaped=$true;actions=$actions}
 }
 if($r.kind-ceq'ROOT_CLOSED_ZERO_PROVIDER_CONTROLLER_CLOCK_FAILED_SEED_ATTEMPT_HISTORY'){
  foreach($pin in @($Plan.historyAdmission,$Lease.historyAdmission,$grant.previous.result,$grant.previous.ledger)){
   Assert-NativeSeedFields $pin @('path','sha256')
   if($pin.path-isnot[string]-or$pin.sha256-isnot[string]-or$pin.sha256-cnotmatch'^[a-f0-9]{64}$'){throw 'Scalar zero-provider history descriptor required'}
  }
  foreach($key in @('kind','issuer','driver')){if($grant.$key-isnot[string]){throw 'Scalar zero-provider grant authority required'}}
  $null=Read-NativeZeroProviderJson $path $reference.sha256
  $r=Read-NativeZeroProviderJson $grant.previous.result.path $grant.previous.result.sha256
  $actions=Read-NativeZeroProviderFailedSeedAttempt $r $l $grant.previous.ledger
  $r=[pscustomobject]@{schemaVersion=1;kind=$r.kind;allOwnedHandlesTerminalReaped=$true;actions=$actions}
 }
 if($r.kind-ceq'ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT'){
  Assert-NativeSeedFields $r @('schemaVersion','kind','scenarioId','toolingHead','rootSessionId','allOwnedHandlesTerminalReaped','actions','cleanup','protection','files','evidence')
  Assert-NativeSeedFields $r.actions @('inheritedBaseline','actualGuard','currentRunContext','absoluteMaxReservedActions','clicks','activations','providers')
  Assert-NativeSeedInteger $r.actions.clicks 8 8;Assert-NativeSeedInteger $r.actions.activations 0 0;Assert-NativeSeedInteger $r.actions.providers 3 3
 }
 if($r.kind-cin@('ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY')){
  $r=Read-NativeModernSeedJson $grant.previous.result.path $grant.previous.result.sha256 262144
  $actions=if($r.kind-ceq'ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY'){Read-NativeCaptureLinkageFailedSeedAttempt $r $l $grant.previous.ledger}elseif($r.kind-ceq'ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY'){Read-NativeFailedSeedAttempt $r $l $grant.previous.ledger}elseif($r.kind-ceq'ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY'){Read-NativeWrongSceneFailedSeedAttempt $r $l $grant.previous.ledger}elseif($r.kind-ceq'ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY'){Read-NativeCaptureStopFailedSeedAttempt $r $l $grant.previous.ledger}else{Read-NativeKnownPlayFailedSeedAttempt $r $l $grant.previous.ledger}
  $r=[pscustomobject]@{schemaVersion=1;kind=$r.kind;allOwnedHandlesTerminalReaped=$true;actions=$actions}
 }
 if($r.schemaVersion-ne1-or$r.kind-cnotin@('ACTUAL_SCHEMA5_COMMON_PREFERENCE_PREFIX_DRIVER_CLOSEOUT','ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT','ROOT_CLOSED_NO_INPUT_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_KNOWN_PLAY_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_CAPTURE_STOP_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_NEW_GAME_WRONG_SCENE_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_PRE_INPUT_CAPTURE_LINKAGE_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_ZERO_PROVIDER_CONTROLLER_CLOCK_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_PRE_INPUT_CALLER_DATE_FAILED_SEED_ATTEMPT_HISTORY','ROOT_CLOSED_PRE_START_INPUT_MARGIN_FAILED_SEED_ATTEMPT_HISTORY')-or$r.allOwnedHandlesTerminalReaped-isnot[bool]-or!$r.allOwnedHandlesTerminalReaped-or$l.schemaVersion-ne2-or$l.kind-cne'STARSECTOR_FINITE_LAUNCH_HISTORY'){throw 'Completed previous seed history required'}
 foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){Assert-NativeSeedInteger $r.actions.actualGuard.$key $l.history.$key $l.history.$key}
 foreach($key in @('overallAttempts','uiAttempts')){Assert-NativeSeedInteger $r.actions.currentRunContext.$key $Plan.historyBaseline.$key $Plan.historyBaseline.$key;Assert-NativeSeedInteger ($l.history.$key+1) $Plan.historyBaseline.$key $Plan.historyBaseline.$key}
 foreach($key in @('reservedActions','sentActions')){Assert-NativeSeedInteger $l.history.$key $Plan.historyBaseline.$key $Plan.historyBaseline.$key}
 Assert-NativeSeedInteger $r.actions.absoluteMaxReservedActions $Plan.historyBaseline.reservedActions 9007199254740991
}
