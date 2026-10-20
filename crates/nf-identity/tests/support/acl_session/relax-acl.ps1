param([Parameter(Mandatory=$true)][string]$PrivatePath)
$ErrorActionPreference = 'Stop'
$stage = 19
try {
    Import-Module -Name (Join-Path $PSHOME 'Modules/Microsoft.PowerShell.Security/Microsoft.PowerShell.Security.psd1') -ErrorAction Stop
    $stage = 20
    $acl = Get-Acl -LiteralPath $PrivatePath
    $ownerGroup = [Security.AccessControl.AccessControlSections]::Owner -bor [Security.AccessControl.AccessControlSections]::Group
    $ownerGroupBefore = $acl.GetSecurityDescriptorSddlForm($ownerGroup)
    $protectedBefore = $acl.AreAccessRulesProtected
    $stage = 21
    $sid = [Security.Principal.SecurityIdentifier]::new('S-1-1-0')
    $rule = [Security.AccessControl.FileSystemAccessRule]::new($sid, [Security.AccessControl.FileSystemRights]::Read, [Security.AccessControl.AccessControlType]::Allow)
    $stage = 22
    $acl.AddAccessRule($rule)
    $accessExpected = $acl.GetSecurityDescriptorSddlForm([Security.AccessControl.AccessControlSections]::Access)
    $stage = 23
    if ($acl -is [Security.AccessControl.DirectorySecurity]) {
        [IO.Directory]::SetAccessControl($PrivatePath, $acl)
    } elseif ($acl -is [Security.AccessControl.FileSecurity]) {
        [IO.File]::SetAccessControl($PrivatePath, $acl)
    } else { exit 23 }
    $stage = 24
    $actual = Get-Acl -LiteralPath $PrivatePath
    $stage = 25
    $found = @($actual.GetAccessRules($true, $true, [Security.Principal.SecurityIdentifier]) | Where-Object { $_.IdentityReference.Value -eq 'S-1-1-0' })
    $stage = 26
    if ($found.Count -ne 1 -or
        $actual.GetSecurityDescriptorSddlForm($ownerGroup) -cne $ownerGroupBefore -or
        $actual.AreAccessRulesProtected -ne $protectedBefore -or
        $actual.GetSecurityDescriptorSddlForm([Security.AccessControl.AccessControlSections]::Access) -cne $accessExpected) { exit 26 }
    exit 0
} catch {
    if ($stage -ne 23) { exit $stage }
    $category = $_.CategoryInfo.Category
    if ($_.Exception -is [System.Management.Automation.CommandNotFoundException]) { exit 30 }
    if ($category -eq [System.Management.Automation.ErrorCategory]::PermissionDenied -or
        $category -eq [System.Management.Automation.ErrorCategory]::SecurityError -or
        $_.Exception -is [System.UnauthorizedAccessException] -or
        $_.Exception -is [System.Security.SecurityException]) { exit 31 }
    if ($category -eq [System.Management.Automation.ErrorCategory]::InvalidArgument -or
        $_.Exception -is [System.ArgumentException]) { exit 32 }
    exit 33
}
