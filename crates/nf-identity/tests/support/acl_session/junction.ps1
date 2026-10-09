param([Parameter(Mandatory=$true)][string]$PrivatePath, [Parameter(Mandatory=$true)][string]$Target)
$ErrorActionPreference = 'Stop'
try {
    if ([IO.Directory]::Exists($PrivatePath) -or [IO.File]::Exists($PrivatePath)) { exit 2 }
    if (-not [IO.Directory]::Exists($Target)) { exit 2 }
    New-Item -ItemType Junction -Path $PrivatePath -Target $Target | Out-Null
    $item = Get-Item -LiteralPath $PrivatePath -Force
    if (($item.Attributes -band [IO.FileAttributes]::ReparsePoint) -eq 0) { exit 2 }
    exit 0
} catch { exit 2 }
