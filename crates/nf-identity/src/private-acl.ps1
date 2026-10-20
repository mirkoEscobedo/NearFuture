param([Parameter(Mandatory=$true)][string]$PrivatePath,[switch]$Initialize)
$ErrorActionPreference = 'Stop'
try {
    # Use the CLR directly: loading PowerShell providers/modules spends the helper's fixed budget.
    $attributes = [System.IO.File]::GetAttributes($PrivatePath)
    if (($attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { exit 2 }
    $isDirectory = ($attributes -band [System.IO.FileAttributes]::Directory) -ne 0
    if ($isDirectory) {
        $item = [System.IO.DirectoryInfo]::new($PrivatePath)
    } else {
        $item = [System.IO.FileInfo]::new($PrivatePath)
    }
    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
    if ($Initialize) {
        if ($isDirectory) {
            $acl = [System.Security.AccessControl.DirectorySecurity]::new()
            $inheritance = [System.Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit'
        } else {
            $acl = [System.Security.AccessControl.FileSecurity]::new()
            $inheritance = [System.Security.AccessControl.InheritanceFlags]::None
        }
        $acl.SetOwner($sid)
        $acl.SetAccessRuleProtection($true, $false)
        $rule = [System.Security.AccessControl.FileSystemAccessRule]::new($sid,[System.Security.AccessControl.FileSystemRights]::FullControl,$inheritance,[System.Security.AccessControl.PropagationFlags]::None,[System.Security.AccessControl.AccessControlType]::Allow)
        $acl.AddAccessRule($rule)
        $item.SetAccessControl($acl)
    }
    $checked = $item.GetAccessControl()
    if (!$checked.AreAccessRulesProtected -or $checked.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { exit 2 }
    $rules = @($checked.GetAccessRules($true,$true,[System.Security.Principal.SecurityIdentifier]))
    if ($rules.Count -ne 1 -or $rules[0].IdentityReference.Value -ne $sid.Value -or $rules[0].AccessControlType -ne [System.Security.AccessControl.AccessControlType]::Allow -or $rules[0].FileSystemRights -ne [System.Security.AccessControl.FileSystemRights]::FullControl) { exit 2 }
    exit 0
} catch { exit 2 }
