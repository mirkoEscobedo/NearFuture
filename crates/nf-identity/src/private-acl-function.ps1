function Test-PrivateAccess([string]$PrivatePath, [bool]$Initialize) {
    if ($Initialize) {
        Import-Module -Name (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
    }
    $item = Get-Item -LiteralPath $PrivatePath -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'private access' }
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
        Set-Acl -LiteralPath $PrivatePath -AclObject $acl
    }
    if ($item.PSIsContainer) {
        $checked = [System.IO.Directory]::GetAccessControl($PrivatePath)
    } else {
        $checked = [System.IO.File]::GetAccessControl($PrivatePath)
    }
    if (!$checked.AreAccessRulesProtected -or $checked.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { throw 'private access' }
    $rules = @($checked.GetAccessRules($true,$true,[System.Security.Principal.SecurityIdentifier]))
    if ($rules.Count -ne 1 -or $rules[0].IdentityReference.Value -ne $sid.Value -or $rules[0].AccessControlType -ne [System.Security.AccessControl.AccessControlType]::Allow -or $rules[0].FileSystemRights -ne [System.Security.AccessControl.FileSystemRights]::FullControl) { throw 'private access' }
}
