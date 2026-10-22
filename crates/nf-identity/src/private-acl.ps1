param([Parameter(Mandatory=$true)][string]$PrivatePath,[switch]$Initialize,[string]$PhasePath = '',[string]$HealthPath = '')
$script:PhaseSequence = 0
$script:PhaseBytes = 0
$script:FirstPhaseAttempted = $false
$script:FirstPhasePublished = $false
$script:HealthSequence = 0
$script:HealthBytes = 0
function Write-PrivateAclHealth([string]$Health) {
    $stream = $null
    try {
        if ([string]::IsNullOrEmpty($HealthPath)) { return $false }
        if ([string]::IsNullOrEmpty($PhasePath) -or ![IO.File]::Exists($PhasePath) -or
            [IO.Path]::GetFileName($PhasePath) -cnotmatch '^\.nf-acl-phase-[a-f0-9]{32}\.tmp$') { return $false }
        if ([string]::IsNullOrEmpty($HealthPath) -or $HealthPath -ieq $PhasePath -or
            [IO.Path]::GetFileName($HealthPath) -cnotmatch '^\.nf-acl-phase-[a-f0-9]{32}\.tmp$') { return $false }
        if (!(($script:HealthSequence -eq 0 -and $Health -ceq 'BODY') -or
            ($script:HealthSequence -eq 1 -and @('FIRST_PUBLISHED','FIRST_FAILED') -ccontains $Health))) { return $false }
        $next = $script:HealthSequence + 1
        $bytes = [Text.Encoding]::ASCII.GetBytes(('NFACLH1|' + $next + '|' + $Health + [char]10))
        if ($script:HealthBytes + $bytes.Length -gt 64) { return $false }
        $stream = [IO.File]::Open($HealthPath,[IO.FileMode]::Open,[IO.FileAccess]::Write,[IO.FileShare]::ReadWrite)
        if ($stream.Length -ne $script:HealthBytes) { return $false }
        $null = $stream.Seek(0,[IO.SeekOrigin]::End)
        $stream.Write($bytes,0,$bytes.Length)
        $stream.Flush()
        $script:HealthSequence = $next
        $script:HealthBytes += $bytes.Length
        return $true
    } catch {
        return $false # This independent observation never changes the ACL result.
    } finally {
        if ($null -ne $stream) { try { $stream.Dispose() } catch {} }
    }
}
function Write-PrivateAclPhase([string]$Phase) {
    $stream = $null
    try {
        if ([string]::IsNullOrEmpty($PhasePath) -or $script:PhaseSequence -ge 8) { return }
        if (@('SCRIPT_BODY_ENTERED','SECURITY_MODULE_READY','PRE_SET_ACL','POST_SET_ACL','PRE_GET_ACL','POST_GET_ACL','POLICY_VERIFIED','REFUSED') -cnotcontains $Phase) { return }
        if ([IO.Path]::GetFileName($PhasePath) -cnotmatch '^\.nf-acl-phase-[a-f0-9]{32}\.tmp$') { return }
        $next = $script:PhaseSequence + 1
        $bytes = [Text.Encoding]::ASCII.GetBytes(('NFACL1|' + $next + '|' + $Phase + [char]10))
        if ($script:PhaseBytes + $bytes.Length -gt 512) { return }
        if ($Phase -ceq 'SCRIPT_BODY_ENTERED' -and $next -eq 1) { $script:FirstPhaseAttempted = $true }
        $stream = [IO.File]::Open($PhasePath,[IO.FileMode]::Open,[IO.FileAccess]::Write,[IO.FileShare]::ReadWrite)
        if ($stream.Length -ne $script:PhaseBytes) { return }
        $null = $stream.Seek(0,[IO.SeekOrigin]::End)
        $stream.Write($bytes,0,$bytes.Length)
        $stream.Flush()
        $script:PhaseSequence = $next
        $script:PhaseBytes += $bytes.Length
        if ($Phase -ceq 'SCRIPT_BODY_ENTERED' -and $next -eq 1) { $script:FirstPhasePublished = $true }
    } catch {
        # Observation failure never changes the ACL result.
    } finally {
        if ($null -ne $stream) { try { $stream.Dispose() } catch {} }
    }
}
$bodyPublished = Write-PrivateAclHealth 'BODY'
Write-PrivateAclPhase 'SCRIPT_BODY_ENTERED'
if ($bodyPublished -and $script:FirstPhaseAttempted) {
    $outcome = if ($script:FirstPhasePublished) { 'FIRST_PUBLISHED' } else { 'FIRST_FAILED' }
    $null = Write-PrivateAclHealth $outcome
}
$ErrorActionPreference = 'Stop'
try {
    Import-Module -Name (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
    Write-PrivateAclPhase 'SECURITY_MODULE_READY'
    $item = Get-Item -LiteralPath $PrivatePath -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { Write-PrivateAclPhase 'REFUSED'; exit 2 }
    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
    if ($Initialize) {
        if ($item.PSIsContainer) {
            $acl = New-Object System.Security.AccessControl.DirectorySecurity
            $inheritance = [System.Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
        } else {
            $acl = New-Object System.Security.AccessControl.FileSecurity
            $inheritance = [System.Security.AccessControl.InheritanceFlags]::None
        }
        $acl.SetOwner($sid)
        $acl.SetAccessRuleProtection($true, $false)
        $rule = New-Object System.Security.AccessControl.FileSystemAccessRule($sid,[System.Security.AccessControl.FileSystemRights]::FullControl,$inheritance,[System.Security.AccessControl.PropagationFlags]::None,[System.Security.AccessControl.AccessControlType]::Allow)
        $acl.AddAccessRule($rule)
        Write-PrivateAclPhase 'PRE_SET_ACL'
        Set-Acl -LiteralPath $PrivatePath -AclObject $acl
        Write-PrivateAclPhase 'POST_SET_ACL'
    }
    Write-PrivateAclPhase 'PRE_GET_ACL'
    $checked = Get-Acl -LiteralPath $PrivatePath
    Write-PrivateAclPhase 'POST_GET_ACL'
    if (!$checked.AreAccessRulesProtected -or $checked.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { Write-PrivateAclPhase 'REFUSED'; exit 2 }
    $rules = @($checked.GetAccessRules($true,$true,[System.Security.Principal.SecurityIdentifier]))
    if ($rules.Count -ne 1 -or $rules[0].IdentityReference.Value -ne $sid.Value -or $rules[0].AccessControlType -ne [System.Security.AccessControl.AccessControlType]::Allow -or $rules[0].FileSystemRights -ne [System.Security.AccessControl.FileSystemRights]::FullControl) { Write-PrivateAclPhase 'REFUSED'; exit 2 }
    Write-PrivateAclPhase 'POLICY_VERIFIED'
    exit 0
} catch { Write-PrivateAclPhase 'REFUSED'; exit 2 }
