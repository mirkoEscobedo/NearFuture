param([Parameter(Mandatory=$true)][string]$PrivatePath,[Parameter(Mandatory=$true)][string]$DescriptorPath,[switch]$Compare)
$ErrorActionPreference = 'Stop'
try {
    Import-Module -Name (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
    $item = Get-Item -LiteralPath $PrivatePath -Force
    if (($item.Attributes -band [System.IO.FileAttributes]::ReparsePoint) -ne 0) { throw 'real ordinary fixture required' }
    $sid = [System.Security.Principal.WindowsIdentity]::GetCurrent().User
    $acl = Get-Acl -LiteralPath $PrivatePath
    if (!$acl.AreAccessRulesProtected -or $acl.GetOwner([System.Security.Principal.SecurityIdentifier]).Value -ne $sid.Value) { throw 'independent owner/protection oracle' }
    $rules = @($acl.GetAccessRules($true,$true,[System.Security.Principal.SecurityIdentifier]))
    if ($rules.Count -ne 1 -or $rules[0].IdentityReference.Value -ne $sid.Value -or $rules[0].AccessControlType -ne [System.Security.AccessControl.AccessControlType]::Allow -or $rules[0].FileSystemRights -ne [System.Security.AccessControl.FileSystemRights]::FullControl -or $rules[0].IsInherited) { throw 'independent ACE oracle' }
    $inheritance = [System.Security.AccessControl.InheritanceFlags]::None
    if ($item.PSIsContainer) { $inheritance = [System.Security.AccessControl.InheritanceFlags]'ContainerInherit, ObjectInherit' }
    if ($rules[0].InheritanceFlags -ne $inheritance -or $rules[0].PropagationFlags -ne [System.Security.AccessControl.PropagationFlags]::None) { throw 'independent inheritance oracle' }
    $descriptor = $acl.GetSecurityDescriptorBinaryForm()
    if ($Compare) {
        $before = [System.IO.File]::ReadAllBytes($DescriptorPath)
        if ([Convert]::ToBase64String($descriptor) -ne [Convert]::ToBase64String($before)) { throw 'validation changed original binary descriptor' }
    } else {
        [System.IO.File]::WriteAllBytes($DescriptorPath, $descriptor)
    }
    exit 0
} catch { exit 2 }
