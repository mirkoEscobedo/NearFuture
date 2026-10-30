# Typed PowerShell admission composed with the fixed current JavaScript custody reader.
# Historical files only: recorded helpers are never executed and clocks are never issued.
function Read-NativePreContinueBudgetHistory($Value,$Ledger,$LedgerPin,$ReceiptPin,$Plan,$Lease) {
 Assert-NativeSeedFields $Value @('schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence')
 Assert-NativeSeedInteger $Value.schemaVersion 1 1
 foreach($key in @('kind','issuer','driver','scenario','sourceWorkspace','sourceHead')){if($Value.$key-isnot[string]){throw 'Scalar pre-Continue budget history authority required'}}
 if($Value.kind-cne'ROOT_CLOSED_PRE_CONTINUE_PROVIDER_BUDGET_FAILED_SEED_ATTEMPT_HISTORY'-or$Value.issuer-cne'/root'-or$Value.driver-cne'/root/recovery_game_driver'-or$Value.allOwnedHandlesTerminalReaped-isnot[bool]-or!$Value.allOwnedHandlesTerminalReaped){throw 'Closed pre-Continue budget failed history required'}
 Assert-NativeSeedFields $Value.actions @('actualGuard','currentRunContext','absoluteMaxReservedActions')
 Assert-NativeSeedFields $Value.actions.actualGuard @('overallAttempts','uiAttempts','reservedActions','sentActions')
 Assert-NativeSeedFields $Value.actions.currentRunContext @('overallAttempts','uiAttempts')
 foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){
  Assert-NativeSeedInteger $Value.actions.actualGuard.$key 0 9007199254740991
  Assert-NativeSeedInteger $Ledger.history.$key $Value.actions.actualGuard.$key $Value.actions.actualGuard.$key
 }
 foreach($key in @('overallAttempts','uiAttempts')){Assert-NativeSeedInteger $Value.actions.currentRunContext.$key ($Ledger.history.$key+1) ($Ledger.history.$key+1)}
 Assert-NativeSeedInteger $Value.actions.absoluteMaxReservedActions ($Ledger.history.reservedActions+1) ($Ledger.history.reservedActions+9)
 foreach($pin in @($LedgerPin,$ReceiptPin)){
  Assert-NativeSeedFields $pin @('path','sha256')
  if($pin.path-isnot[string]-or$pin.sha256-isnot[string]-or$pin.sha256-cnotmatch'^[a-f0-9]{64}$'){throw 'Scalar pre-Continue budget history descriptor required'}
 }
 $null=Read-NativeZeroProviderJson $ReceiptPin.path $ReceiptPin.sha256
 $null=Read-NativeZeroProviderJson $LedgerPin.path $LedgerPin.sha256
 $currentRoot=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../..'))
 $pinsPath=Join-Path ([IO.Path]::GetDirectoryName($Plan.historyAdmission.path)) 'frozen-support-pins.private.json'
 if($Lease.supportPinsPath-cne$pinsPath-or$Lease.supportPinsSha256-isnot[string]){throw 'Current fixed proof source manifest required'}
 $pins=Read-NativeZeroProviderJson $pinsPath $Lease.supportPinsSha256
 Assert-NativeSeedFields $pins @('pins')
 if($pins.pins-isnot[array]-or$pins.pins.Count-gt1024){throw 'Complete bounded current proof pins required'}
 $map=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
 foreach($row in $pins.pins){
  Assert-NativeSeedFields $row @('path','sha256')
  if($row.path-isnot[string]-or$row.sha256-isnot[string]-or$row.sha256-cnotmatch'^[a-fA-F0-9]{64}$'-or$map.ContainsKey($row.path)){throw 'Scalar unique current proof pin required'}
  $map.Add($row.path,$row.sha256)
 }
 Assert-NativeModernSeedPins $pins $null
 $runtime=[IO.Path]::GetFullPath((Join-Path $currentRoot 'tools/computer-use-mcp/node_modules'))
 foreach($item in Get-ChildItem -LiteralPath $runtime -Recurse -Force){
  if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary current proof runtime required'}
  if(!$item.PSIsContainer-and$item.Name-match'\.(?:[cm]?js|json|node|exe|dll|wasm|so|dylib)$'-and!$map.ContainsKey($item.FullName)){throw 'Complete current installed runtime required'}
 }
 $script=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../seed-pre-continue-budget-history.mjs'))
 $queue=[Collections.Generic.Queue[string]]::new();$queue.Enqueue($script)
 $seen=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
 while($queue.Count){
  $path=$queue.Dequeue();if(!$seen.Add($path)){continue}
  if(!$map.ContainsKey($path)){throw 'Current pre-Continue budget proof import missing from pins'}
  if($path.StartsWith($runtime+[IO.Path]::DirectorySeparatorChar,[StringComparison]::Ordinal)){continue}
  $text=[IO.File]::ReadAllText($path)
  foreach($match in [regex]::Matches($text,'(?:import|export)\s+(?:[^;]*?\s+from\s+)?[''"]([^''"]+)[''"]')){
   $specifier=$match.Groups[1].Value
   if($specifier.StartsWith('node:')){continue}
   if(!$specifier.StartsWith('.')){throw 'Explicit current proof import required'}
   $queue.Enqueue([IO.Path]::GetFullPath((Join-Path ([IO.Path]::GetDirectoryName($path)) $specifier)))
  }
 }
 $node=(Get-Command node -CommandType Application -TotalCount 1).Source
 $powershell=(Get-Command pwsh -CommandType Application -TotalCount 1).Source
 $supervisor=Join-Path $currentRoot 'node_modules/workspace-template-win32-x64/bin/workspace-template.exe'
 foreach($path in @($node,$powershell,$supervisor)){if(!$map.ContainsKey([IO.Path]::GetFullPath($path))){throw 'Pinned current proof executable required'}}
 $output=@(& $node $script inspect $ReceiptPin.path $ReceiptPin.sha256 $LedgerPin.path $LedgerPin.sha256 $pinsPath $Lease.supportPinsSha256 $powershell $supervisor)
 if($LASTEXITCODE-ne0){throw 'Shared pre-Continue budget margin custody proof refused'}
 $text=$output-join([char]10);if([Text.Encoding]::UTF8.GetByteCount($text)-gt262144){throw 'Bounded pre-Continue budget proof response required'}
 $proof=ConvertFrom-ZeroProviderObject $text
 Assert-NativeSeedFields $proof @('schemaVersion','kind','receipt','ledger','actions','scope')
 Assert-NativeSeedInteger $proof.schemaVersion 1 1
 if($proof.kind-cne'PRE_CONTINUE_BUDGET_HISTORY_CUSTODY'-or$proof.scope-cne'HISTORICAL_CUSTODY_ONLY'){throw 'Closed historical proof response required'}
 foreach($pair in @(@($proof.receipt,$ReceiptPin),@($proof.ledger,$LedgerPin))){
  Assert-NativeSeedFields $pair[0] @('path','sha256')
  if($pair[0].path-isnot[string]-or$pair[0].sha256-isnot[string]-or$pair[0].path-cne$pair[1].path-or$pair[0].sha256-cne$pair[1].sha256){throw 'Early window focus proof response reference changed'}
 }
 Assert-NativeSeedFields $proof.actions @('actualGuard','currentRunContext','absoluteMaxReservedActions')
 foreach($key in @('overallAttempts','uiAttempts','reservedActions','sentActions')){Assert-NativeSeedInteger $proof.actions.actualGuard.$key $Value.actions.actualGuard.$key $Value.actions.actualGuard.$key}
 foreach($key in @('overallAttempts','uiAttempts')){Assert-NativeSeedInteger $proof.actions.currentRunContext.$key $Value.actions.currentRunContext.$key $Value.actions.currentRunContext.$key}
 Assert-NativeSeedInteger $proof.actions.absoluteMaxReservedActions $Value.actions.absoluteMaxReservedActions $Value.actions.absoluteMaxReservedActions
 return $Value.actions
}
