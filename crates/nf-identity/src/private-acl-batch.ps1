$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$phase = 20
try {
    $reader = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), [System.Text.UTF8Encoding]::new($false,$true), $false, 128, $false)
    function Read-BoundedLine([int]$Limit) {
        $buffer = [System.Text.StringBuilder]::new($Limit)
        while ($true) {
            $value = $reader.Read()
            if ($value -lt 0) { throw 'input' }
            if ($value -eq 10) { return $buffer.ToString() }
            if ($value -eq 0 -or $value -eq 13 -or $buffer.Length -ge $Limit) { throw 'input' }
            $null = $buffer.Append([char]$value)
        }
    }
    $countLine = Read-BoundedLine 2
    if ($countLine -cnotmatch '^(?:[1-9]|[1-5][0-9]|6[0-5])$') { exit 21 }
    $count = [int]$countLine
    $total = $countLine.Length + 1
    $paths = [System.Collections.Generic.List[string]]::new()
    $unique = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
    for ($index = 0; $index -lt $count; $index++) {
        $path = Read-BoundedLine 4096
        $bytes = [System.Text.Encoding]::UTF8.GetByteCount($path)
        if ($bytes -lt 1 -or $bytes -gt 4096 -or !$unique.Add($path)) { exit 22 }
        $total += $bytes + 1
        if ($total -gt 272384) { exit 22 }
        $paths.Add($path)
    }
    if ($reader.Read() -ne -1) { exit 23 }
    $phase = 24
    $helper = Join-Path $PSScriptRoot 'private-acl.ps1'
    foreach ($path in $paths) {
        $LASTEXITCODE = -1
        & $helper -PrivatePath $path
        if ($LASTEXITCODE -ne 0) {
            if ($LASTEXITCODE -in @(2,3,4,5,10,11,12,13,14,15,16)) { exit $LASTEXITCODE }
            exit 2
        }
    }
    exit 0
} catch { exit $phase }