function Read-NativeMenuRecord([string]$Path,[long]$Limit=262144) {
 $item=Get-Item -LiteralPath $Path -Force
 if($item.PSIsContainer-or$item.Length-lt2-or$item.Length-gt$Limit){throw 'Bounded menu authority file required'}
 while($null-ne$item){if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary menu authority ancestry required'};$item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}}
 return (Get-Content -LiteralPath $Path -Raw|ConvertFrom-Json -DateKind String)
}
function Assert-NativeMenuClosed($Value,[string[]]$Names) {
 if($Value-isnot[pscustomobject]-or(@($Value.PSObject.Properties.Name|Sort-Object)-join',')-cne(@($Names|Sort-Object)-join',')){throw 'Closed menu authority schema required'}
}
function Read-NativeMenuDescriptor($Descriptor,[string]$Expected) {
 Assert-NativeMenuClosed $Descriptor @('path','sha256')
 if($Descriptor.path-isnot[string]-or![IO.Path]::IsPathFullyQualified($Descriptor.path)-or[IO.Path]::GetFullPath($Descriptor.path)-cne$Expected-or$Descriptor.sha256-isnot[string]-or$Descriptor.sha256-cnotmatch'^[a-fA-F0-9]{64}$'){throw 'Exact menu descriptor required'}
 $value=Read-NativeMenuRecord $Expected 16384
 if((Get-FileHash -LiteralPath $Expected -Algorithm SHA256).Hash-ine$Descriptor.sha256){throw 'Menu descriptor digest changed'}
 return $value
}
function Read-NativeMenuLaunchLease([string]$Path,$Binding,[string]$BindingPath,[string]$BindingSha256) {
 $scenario=$Binding.fixtureRoot
 if([IO.Path]::GetDirectoryName([IO.Path]::GetFullPath($Path))-cne$scenario){throw 'Menu launch must use bound scenario'}
 $lease=Read-NativeMenuRecord $Path
 $planPath=Join-Path $scenario 'plan.json';$plan=Read-NativeMenuRecord $planPath
 if($lease.schemaVersion-isnot[int]-and$lease.schemaVersion-isnot[long]){throw 'Menu lease schema required'}
 if($lease.schemaVersion-ne1-or$lease.scenarioId-cne$Binding.fixtureId-or$lease.authorized-isnot[bool]-or!$lease.authorized-or$lease.phase-cne'MENU_SMOKE_ONLY'-or$lease.issuer-cne'/root'-or$lease.driver-cne'/root/recovery_game_driver'-or$lease.driver-cne$Binding.driverRole){throw 'Complete V1 menu scope required'}
 foreach($row in @(@($lease,'ownedGameHardTimeoutMs',480000),@($lease,'maxActions',8),@($lease,'maxProxyInvocations',3),@($plan.bounds,'ownedGameHardTimeoutMs',480000),@($plan.bounds,'proxyTimeoutMs',180000),@($plan.bounds,'maxScenarioActionsIncludingActivations',8),@($plan.bounds,'maxProxyInvocations',3))){
  $number=$row[0].($row[1]);if(($number-isnot[int]-and$number-isnot[long])-or$number-ne$row[2]){throw 'Fixed menu bounds required'}
 }
 if($lease.campaignLoadSaveAllowed-isnot[bool]-or$lease.campaignLoadSaveAllowed-or$plan.scenarioId-cne$Binding.fixtureId-or$plan.paths.scenario-cne$scenario-or$plan.ownership.driver-cne$Binding.driverRole-or$plan.ownership.onlyGameInputDriver-isnot[bool]-or!$plan.ownership.onlyGameInputDriver){throw 'Exact menu ownership required'}
 if($lease.planSha256-isnot[string]-or(Get-FileHash -LiteralPath $planPath -Algorithm SHA256).Hash-ine$lease.planSha256){throw 'Frozen menu plan changed'}
 if($plan.batches-isnot[array]-or$plan.batches.Count-lt1-or$plan.batches.Count-gt3){throw 'Closed menu batch scope required'}
 $batchIds=[Collections.Generic.HashSet[string]]::new([StringComparer]::Ordinal)
 foreach($batch in $plan.batches){if($batch.id-isnot[string]-or$batch.id-cnotin@('B1','B2','B3')-or!$batchIds.Add($batch.id)){throw 'Closed menu batch scope required'}}
 if(!$batchIds.Contains('B1')){throw 'Explicit menu launcher batch required'}
 foreach($value in @($plan,$lease)){
  foreach($key in @('seed','ownedNodeKill','recordingProfile','recordingProfileSha256','campaign','e1MenuReceipt')){if(@($value.PSObject.Properties.Name)-contains$key){throw 'Menu campaign extensions refused'}}
  if(@($value.PSObject.Properties.Name)-contains'sessionWatchdogMs'){$n=$value.sessionWatchdogMs;if(($n-isnot[int]-and$n-isnot[long])-or$n-ne480000){throw 'Fixed menu watchdog required'}}
 }
 $grantPath=Join-Path $scenario 'menu-launch-grant.private.json'
 $grant=Read-NativeMenuDescriptor $plan.launchGrant $grantPath
 $null=Read-NativeMenuDescriptor $lease.launchGrant $grantPath
 if($plan.launchGrant.path-cne$lease.launchGrant.path-or$plan.launchGrant.sha256-cne$lease.launchGrant.sha256){throw 'Plan and lease grant differ'}
 Assert-NativeMenuClosed $grant @('schemaVersion','kind','phase','authorized','issuer','driver','scenarioId','fixtureBinding','fixtureReceiptSha256','supportPinsSha256','ownedGameHardTimeoutMs','campaignLoadSaveAllowed')
 if(($grant.schemaVersion-isnot[int]-and$grant.schemaVersion-isnot[long])-or$grant.schemaVersion-ne1-or$grant.kind-cne'FINITE_MENU_LAUNCH_GRANT_V1'-or$grant.phase-cne'CAMPAIGN_BINDING_ONLY'-or$grant.authorized-isnot[bool]-or!$grant.authorized-or$grant.issuer-cne'/root'-or$grant.driver-cne$lease.driver-or$grant.scenarioId-cne$Binding.fixtureId-or($grant.ownedGameHardTimeoutMs-isnot[int]-and$grant.ownedGameHardTimeoutMs-isnot[long])-or$grant.ownedGameHardTimeoutMs-ne480000-or$grant.campaignLoadSaveAllowed-isnot[bool]-or$grant.campaignLoadSaveAllowed){throw 'Closed menu launch-only grant required'}
 $expectedBinding=Join-Path $scenario 'fixture-binding.private.json'
 $null=Read-NativeMenuDescriptor $grant.fixtureBinding $expectedBinding
 $null=Read-NativeMenuDescriptor $plan.fixtureBinding $expectedBinding
 if($Binding.schemaVersion-ne1-or$grant.fixtureBinding.path-cne[IO.Path]::GetFullPath($BindingPath)-or$grant.fixtureBinding.sha256-ine$BindingSha256-or$plan.fixtureBinding.path-cne$grant.fixtureBinding.path-or$plan.fixtureBinding.sha256-cne$grant.fixtureBinding.sha256){throw 'Exact legacy menu fixture binding required'}
 $fixtureHash=(Get-FileHash -LiteralPath (Join-Path $scenario 'protected-fixture-receipt.private.json') -Algorithm SHA256).Hash
 if($lease.fixtureReceiptSha256-isnot[string]-or$lease.fixtureReceiptSha256-ine$fixtureHash-or$grant.fixtureReceiptSha256-isnot[string]-or$grant.fixtureReceiptSha256-ine$fixtureHash){throw 'Menu fixture receipt changed'}
 $pinsPath=Join-Path $scenario 'frozen-support-pins.private.json'
 $pins=Read-NativeMenuRecord $pinsPath
 $pinsHash=(Get-FileHash -LiteralPath $pinsPath -Algorithm SHA256).Hash
 if($lease.supportPinsPath-cne$pinsPath-or$lease.supportPinsSha256-isnot[string]-or$lease.supportPinsSha256-ine$pinsHash-or$grant.supportPinsSha256-isnot[string]-or$grant.supportPinsSha256-ine$pinsHash){throw 'Menu support manifest changed'}
 Assert-NativeMenuClosed $pins @('pins')
 if($pins.pins-isnot[array]-or$pins.pins.Count-gt1024){throw 'Bounded menu source pins required'}
 $seen=[Collections.Generic.Dictionary[string,string]]::new([StringComparer]::Ordinal)
 foreach($pin in $pins.pins){
  Assert-NativeMenuClosed $pin @('path','sha256')
  if($pin.path-isnot[string]-or![IO.Path]::IsPathFullyQualified($pin.path)-or$pin.sha256-isnot[string]-or$pin.sha256-cnotmatch'^[a-fA-F0-9]{64}$'){throw 'Exact menu source pin required'}
  $file=[IO.Path]::GetFullPath($pin.path);$suffix=[IO.Path]::GetRelativePath($Binding.repositoryRoot,$file)
  if($suffix-ceq'.'-or($suffix-split'[\\/]')[0]-in@('.tmp','.git')-or$seen.ContainsKey($file)){throw 'Private authority or duplicate menu source pin refused'}
  $seen.Add($file,$pin.sha256)
 }
 $required=@('menu-launch-grant.mjs','batch-authorization.mjs','fixture-binding.mjs','guarded-http-ownership.ps1')|ForEach-Object {'tools/starsector-proxy/'+$_}
 $required+=@('menu-launch.ps1','native-controller.ps1','protection-check.ps1','fixture-inputs.ps1','seed-fixture.ps1','publish-owned-snapshot.ps1','session-watchdog.ps1')|ForEach-Object {'tools/starsector-proxy/fixture/'+$_}
 foreach($name in $required){
  $file=[IO.Path]::GetFullPath((Join-Path $Binding.repositoryRoot $name))
  if(!$seen.ContainsKey($file)){throw 'Current menu launch tooling pins required'}
  $null=Read-NativeMenuSource $file
  if((Get-FileHash -LiteralPath $file -Algorithm SHA256).Hash-ine$seen[$file]){throw 'Menu source bytes changed'}
 }
 return $lease
}
function Read-NativeMenuSource([string]$Path) {
 $item=Get-Item -LiteralPath $Path -Force
 if($item.PSIsContainer-or$item.Length-lt2-or$item.Length-gt1048576){throw 'Bounded menu source file required'}
 while($null-ne$item){if($item.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary menu source ancestry required'};$item=if($item.PSIsContainer){$item.Parent}else{$item.Directory}}
}
