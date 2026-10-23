# Automation safety watchdog only; product timing and per-call limits are separate.
function Assert-NativeSessionWatchdog([long]$Requested,$Plan,$Lease) {
 if($Requested-lt480000-or$Requested-gt43200000){throw 'Session watchdog must be between 480000 and 43200000 milliseconds'}
 $hasPlan=$null-ne$Plan-and@($Plan.PSObject.Properties.Name)-ccontains'sessionWatchdogMs'
 $hasLease=$null-ne$Lease-and@($Lease.PSObject.Properties.Name)-ccontains'sessionWatchdogMs'
 if(!$hasPlan-and!$hasLease-and$Requested-eq480000){return $Requested}
 if(!$hasPlan-or!$hasLease){throw 'Explicit matching session watchdog plan and lease required'}
 foreach($value in @($Plan.sessionWatchdogMs,$Lease.sessionWatchdogMs)){
  if(($value-isnot[int]-and$value-isnot[long])-or$value-ne$Requested){throw 'Explicit matching session watchdog plan and lease required'}
 }
 return $Requested
}
function Read-NativeSessionWatchdog([string]$EvidenceRoot,[long]$Requested) {
 $records=@{}
 foreach($name in @('plan.json','native-lane-lease.private.json')){
  $path=Join-Path $EvidenceRoot $name
  $item=Get-Item -LiteralPath $path -Force
  if($item.PSIsContainer-or$item.Length-lt2-or$item.Length-gt65536){throw 'Bounded session watchdog authority required'}
  $ancestor=$item
  while($null-ne$ancestor){
   if($ancestor.Attributes-band[IO.FileAttributes]::ReparsePoint){throw 'Ordinary session watchdog authority required'}
   $ancestor=if($ancestor.PSIsContainer){$ancestor.Parent}else{$ancestor.Directory}
  }
  $records[$name]=Get-Content -LiteralPath $path -Raw|ConvertFrom-Json -DateKind String
 }
 # Existing protection-check separately verifies the lease identity and exact plan hash.
 return Assert-NativeSessionWatchdog $Requested $records['plan.json'] $records['native-lane-lease.private.json']
}
