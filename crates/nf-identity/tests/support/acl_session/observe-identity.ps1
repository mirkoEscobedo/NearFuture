param([Parameter(Mandatory=$true)][uint32]$KnownProcess,[Parameter(Mandatory=$true)][long]$CreationTicks,[switch]$ExpectExited)
$ErrorActionPreference='Stop'
try{
    try{$process=[Diagnostics.Process]::GetProcessById($KnownProcess)}catch{
        $lookupFailure=$_.Exception
        while($null -ne $lookupFailure.InnerException){$lookupFailure=$lookupFailure.InnerException}
        if($ExpectExited -and $lookupFailure -is [ArgumentException]){exit 0};exit 2
    }
    $same=$process.StartTime.ToUniversalTime().Ticks -eq $CreationTicks
    if($ExpectExited){if(-not $same -or $process.HasExited){exit 0};exit 2}
    if($same -and -not $process.HasExited){exit 0};exit 2
}catch{exit 2}
