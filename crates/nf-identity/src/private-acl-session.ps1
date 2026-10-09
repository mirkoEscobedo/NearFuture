$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
try {
    . (Join-Path $PSScriptRoot 'private-acl-function.ps1')
    $reader = [System.IO.StreamReader]::new([Console]::OpenStandardInput(), [System.Text.UTF8Encoding]::new($false,$true), $false, 128, $false)
    function Read-BoundedLine([int]$Limit, [bool]$EndAllowed) {
        $buffer = [System.Text.StringBuilder]::new($Limit)
        while ($true) {
            $value = $reader.Read()
            if ($value -lt 0) {
                if ($EndAllowed -and $buffer.Length -eq 0) { return $null }
                throw 'input'
            }
            if ($value -eq 10) { return $buffer.ToString() }
            if ($value -eq 0 -or $value -eq 13 -or $buffer.Length -ge $Limit) { throw 'input' }
            $null = $buffer.Append([char]$value)
        }
    }
    $sequence = 0
    while ($true) {
        $header = Read-BoundedLine 8 $true
        if ($null -eq $header) { throw 'input' }
        if ($header -ceq 'R 2' -or $header -ceq 'A 6') {
            if (($header -ceq 'R 2' -and $sequence -ne 2) -or ($header -ceq 'A 6' -and $sequence -ne 6)) { throw 'input' }
            if ($reader.Read() -ne -1) { throw 'input' }
            [Console]::Out.Write("DONE`n")
            [Console]::Out.Flush()
            exit 0
        }
        if ($header -cnotmatch '^([VI]) ([1-6]) ([1-9]|[1-5][0-9]|6[0-5])$') { throw 'input' }
        $operation = $Matches[1]
        $number = [int]$Matches[2]
        $count = [int]$Matches[3]
        if ($number -ne $sequence + 1 -or (($operation -ceq 'I') -ne ($number -eq 4))) { throw 'input' }
        if ((@(2,3,4,6) -contains $number) -and $count -ne 1) { throw 'input' }
        $total = $header.Length + 1
        $paths = [System.Collections.Generic.List[string]]::new()
        $unique = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)
        for ($index = 0; $index -lt $count; $index++) {
            $path = Read-BoundedLine 4096 $false
            $bytes = [System.Text.Encoding]::UTF8.GetByteCount($path)
            if ($bytes -lt 1 -or $bytes -gt 4096 -or !$unique.Add($path)) { throw 'input' }
            $total += $bytes + 1
            if ($total -gt 272384) { throw 'input' }
            $paths.Add($path)
        }
        foreach ($path in $paths) { Test-PrivateAccess $path ($operation -ceq 'I') }
        $sequence = $number
        [Console]::Out.Write("ACK $sequence`n")
        [Console]::Out.Flush()
    }
} catch { exit 2 }
