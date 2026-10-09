# Local owned diagnostic wrapper; prior opaque-stderr failure retained. Not yet executed.
$ErrorActionPreference='Stop'
$taskPhase='SourcePin'
try {
$taskPhase='PlatformPin'
if($PSVersionTable.PSVersion.Major -ne 5 -or $PSHOME.Replace([char]92,[char]47) -ine 'C:/Windows/System32/WindowsPowerShell/v1.0'){throw 'PINNED_WINDOWS_PS5_HOME'}
# Hosted image bytes differ from the local machine. Require the same WindowsPS5 module paths and command roles.
$taskPhase='PlatformImport'
Import-Module -Name 'C:/Windows/System32/WindowsPowerShell/v1.0/Modules/Microsoft.PowerShell.Management/Microsoft.PowerShell.Management.psd1' -ErrorAction Stop
Import-Module -Name 'C:/Windows/System32/WindowsPowerShell/v1.0/Modules/Microsoft.PowerShell.Utility/Microsoft.PowerShell.Utility.psd1' -ErrorAction Stop
$taskPhase='WrapperCommands'
foreach($taskSpec in @(@('Get-FileHash','Microsoft.PowerShell.Utility'),@('Test-Path','Microsoft.PowerShell.Management'),@('Get-Item','Microsoft.PowerShell.Management'),@('Set-PSBreakpoint','Microsoft.PowerShell.Utility'),@('Remove-PSBreakpoint','Microsoft.PowerShell.Utility'))){
    $taskCommand=Get-Command -Name $taskSpec[0] -CommandType Cmdlet,Function -ErrorAction Stop
    if($taskCommand.ModuleName -cne $taskSpec[1]){throw 'WRAPPER_COMMAND_MODULE'}
}
$taskRoot=[IO.Path]::GetFullPath([IO.Path]::Combine($PSScriptRoot,'../..'))
$taskScript=[IO.Path]::Combine($taskRoot,'crates/nf-identity/src/private-acl.ps1')
$taskPrivate=[IO.Path]::Combine($taskRoot,'.tmp/private-acl-observation/fixture')
$taskPhase='SourcePin'
$taskSourceText=[Text.UTF8Encoding]::new($false,$true).GetString([IO.File]::ReadAllBytes($taskScript)).Replace("`r`n","`n")
$taskHasher=[Security.Cryptography.SHA256]::Create()
try { $taskSourceHash=[BitConverter]::ToString($taskHasher.ComputeHash([Text.UTF8Encoding]::new($false).GetBytes($taskSourceText))).Replace('-','').ToLowerInvariant() } finally { $taskHasher.Dispose() }
if($taskSourceHash -cne '278da1aa88d8c0082c02ba8d8279953a1e67158bffea131c1e9c16f1d6fcfd92'){throw 'EXACT_CANONICAL_SCRIPT_PIN'}
$taskPhase='FixtureExists'
if(-not (Test-Path -LiteralPath $taskPrivate -PathType Container)){throw 'PRECREATED_OWNED_FIXTURE_REQUIRED'}
$taskPhase='FixtureMetadata'
$taskLeaf=Get-Item -LiteralPath $taskPrivate -Force
if(($taskLeaf.Attributes -band [IO.FileAttributes]::ReparsePoint) -ne 0){throw 'OWNED_NO_REPARSE'}
[Console]::Out.WriteLine('NF_ACL_DIAG|DEBUGGER_SETUP_BEGIN')
$taskPhase='DebuggerSetup'
$taskBreakpoints=@()
try {
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 5 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|IMPORT_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 6 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|IMPORT_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 7 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|GETITEM_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 8 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|GETITEM_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 10 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|SID_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 11 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|SID_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 14 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|DIRECTORY_ACL_NEW'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 20 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|SETOWNER_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 21 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|SETOWNER_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 24 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|SETACL_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 26 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|SETACL_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 27 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|GETACL_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 28 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|GETACL_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 29 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|OWNER_CHECK_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 30 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|OWNER_CHECK_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 31 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|RULES_BEGIN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 32 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|RULES_RETURN'); continue }
    $taskBreakpoints+=Set-PSBreakpoint -Script $taskScript -Line 33 -Action { [Console]::Out.WriteLine('NF_ACL_DIAG|ACCEPT_EXIT'); continue }
    [Console]::Out.WriteLine('NF_ACL_DIAG|DEBUGGER_SETUP_READY')
    $taskPhase='HelperInvocation'
    & $taskScript -PrivatePath $taskPrivate -Initialize
    $taskExit=$LASTEXITCODE
} finally {
    $taskBreakpoints|Remove-PSBreakpoint
}


    exit $taskExit
} catch {
    $taskType=$_.Exception.GetType().FullName
    $taskId=$_.FullyQualifiedErrorId
    if($taskType.Length -gt 128 -or $taskType -notmatch '^[A-Za-z0-9_.]+$'){ $taskType='RedactedType' }
    if($taskId.Length -gt 128 -or $taskId -notmatch '^[A-Za-z0-9_.,-]+$'){ $taskId='RedactedErrorId' }
    [Console]::Out.WriteLine('NF_ACL_ERROR|'+$taskPhase+'|'+$taskType+'|'+$taskId)
    exit 90
}
