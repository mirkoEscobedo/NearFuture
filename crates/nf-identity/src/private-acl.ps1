param([Parameter(Mandatory=$true)][string]$PrivatePath,[switch]$Initialize)
$ErrorActionPreference = 'Stop'
$phase = 10
try {
    Import-Module -Name (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
    $phase = 11
    $item = Get-Item -LiteralPath $PrivatePath -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { exit 3 }
    $phase = 12
    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
    if ($Initialize) {
        $phase = 13
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
        Set-Acl -LiteralPath $PrivatePath -AclObject $acl
    }
    $phase = 14
    $checked = Get-Acl -LiteralPath $PrivatePath
    $phase = 15
    if (!$checked.AreAccessRulesProtected -or $checked.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { exit 4 }
    $phase = 16
    $rules = @($checked.GetAccessRules($true,$true,[System.Security.Principal.SecurityIdentifier]))
    if ($rules.Count -ne 1 -or $rules[0].IdentityReference.Value -ne $sid.Value -or $rules[0].AccessControlType -ne [System.Security.AccessControl.AccessControlType]::Allow -or $rules[0].FileSystemRights -ne [System.Security.AccessControl.FileSystemRights]::FullControl) { exit 5 }
    exit 0
} catch { exit $phase }
