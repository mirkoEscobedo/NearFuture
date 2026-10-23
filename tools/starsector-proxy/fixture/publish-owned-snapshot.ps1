# Publish a complete current sample without truncating the snapshot held by readers.
# A skipped update retains its old timestamp; existing freshness admission still applies.
function Publish-OwnedSnapshot {
 param([Parameter(Mandatory)][string]$Path,[Parameter(Mandatory)]$Snapshot)
 $destination=[IO.Path]::GetFullPath($Path)
 $temporary=Join-Path ([IO.Path]::GetDirectoryName($destination)) ([IO.Path]::GetFileName($destination)+'.'+[Guid]::NewGuid().ToString('N')+'.next.private.json')
 $bytes=[Text.UTF8Encoding]::new($false).GetBytes(($Snapshot|ConvertTo-Json -Depth 7))
 $created=$false
 try {
  $stream=[IO.File]::Open($temporary,[IO.FileMode]::CreateNew,[IO.FileAccess]::Write,[IO.FileShare]::None)
  $created=$true
  try {$stream.Write($bytes);$stream.Flush($true)} finally {$stream.Dispose()}
  try {
   if([IO.File]::Exists($destination)){[IO.File]::Replace($temporary,$destination,[NullString]::Value)}
   else {[IO.File]::Move($temporary,$destination)}
  }
  catch {
   $cause=$_.Exception
   while($null-ne$cause.InnerException){$cause=$cause.InnerException}
   if($cause-is[IO.IOException]-and($cause.HResult-eq-2147024864-or$cause.HResult-eq-2147024863)){
    return $false # Windows sharing/lock violation only; next existing sample may publish.
   }
   throw
  }
  return $true
 } finally {
  if($created){[IO.File]::Delete($temporary)}
 }
}
