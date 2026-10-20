param([Parameter(Mandatory=$true)][ValidateSet('WrongAck','EarlyExit','Nonzero','Stderr','Trailing','Oversize','FullPipes','Silent','Descendant','Grandchild')][string]$Mode)
$ErrorActionPreference='Stop'
try {
    if($Mode -eq 'Grandchild'){while($true){Start-Sleep -Milliseconds 100}}
    if($Mode -in @('Descendant','FullPipes')){
        $info=[Diagnostics.ProcessStartInfo]::new('C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe');$info.UseShellExecute=$false;$info.CreateNoWindow=$true
        $info.Arguments='-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "'+$PSCommandPath+'" -Mode Grandchild'
        $grand=[Diagnostics.Process]::Start($info);$self=[Diagnostics.Process]::GetCurrentProcess()
        [Console]::Out.Write("READY $PID $($self.StartTime.ToUniversalTime().Ticks) $($grand.Id) $($grand.StartTime.ToUniversalTime().Ticks)`n");[Console]::Out.Flush()
        if([Console]::In.ReadLine() -ne 'GO'){exit 2}
        if($Mode -eq 'FullPipes'){$bytes=[byte[]]::new(65536);[Console]::OpenStandardOutput().Write($bytes,0,$bytes.Length);[Console]::OpenStandardError().Write($bytes,0,$bytes.Length)}
        while($true){Start-Sleep -Milliseconds 100}
    }
    if([Console]::In.ReadLine() -ne 'GO'){exit 2}
    switch($Mode){
        'WrongAck'{[Console]::Out.Write("ACK 9`n");exit 0}
        'EarlyExit'{exit 0}
        'Nonzero'{exit 2}
        'Stderr'{[Console]::Error.Write('public-fixture-error');[Console]::Error.Flush();[Console]::Out.Write("ACK 1`n");[Console]::Out.Flush();while([Console]::In.ReadLine() -ne $null){};[Console]::Out.Write("DONE`n");exit 0}
        'Trailing'{[Console]::Out.Write("ACK 1`n");[Console]::Out.Flush();if([Console]::In.ReadLine() -ne 'R 2' -or [Console]::In.ReadLine() -ne $null){exit 2};[Console]::Out.Write("DONE`nEXTRA`n");exit 0}
        'Oversize'{[Console]::Out.Write(('x'*65));[Console]::Out.Flush();exit 0}
        'Silent'{while($true){Start-Sleep -Milliseconds 100}}
        default{exit 2}
    }
}catch{exit 2}
