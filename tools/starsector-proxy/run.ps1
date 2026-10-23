[CmdletBinding()]
param(
    [ValidateSet('starsector-observer', 'starsector-proxy')]
    [string]$Agent = 'starsector-observer',
    [string]$Prompt = 'Observe the current Starsector window only and stop.',
    [string]$OpenCodeExe,
    [string]$Model = 'opencode-go/glm-5.3-flash',
    [switch]$DisableThinking,
    [switch]$DisableParallelToolCalls,
    [switch]$CheckMcpOnly,
    [string]$ExportSession,
    [switch]$WaitForReport,
    [string]$BatchAuthorization,
    [ValidateRange(1000, 300000)]
    [int]$TimeoutMs = 180000
)
$ErrorActionPreference = 'Stop'
if ($Model -cnotmatch '^[A-Za-z0-9][A-Za-z0-9._-]*/[A-Za-z0-9][A-Za-z0-9._:/#-]*$') { throw 'Provide a provider/model identifier' }
if (($DisableThinking -or $DisableParallelToolCalls) -and ($Model -cne 'unsloth-studio/unsloth/Qwen3.8-27B-GGUF:UD-Q4_K_XL' -or $ExportSession)) { throw 'Inference control requires a new run of the qualified local model' }
if ($CheckMcpOnly -and $ExportSession) { throw 'MCP readiness cannot export a historical session' }
$proxyRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..\..'))
if ($BatchAuthorization -and ($Agent -cne 'starsector-proxy' -or $ExportSession -or $CheckMcpOnly)) { throw 'Batch authorization requires a new input-agent run' }
if ($Agent -ceq 'starsector-proxy' -and !$ExportSession -and !$CheckMcpOnly -and (!$BatchAuthorization -or !$WaitForReport)) { throw 'Input agent requires an explicit current batch authorization and full report' }
if ($BatchAuthorization) {
    $batchPath = [IO.Path]::GetFullPath($BatchAuthorization)
    $batchPrefix = [IO.Path]::GetFullPath((Join-Path $proxyRoot '.tmp/starsector-proxy/scenarios')) + [IO.Path]::DirectorySeparatorChar
    if (!$batchPath.StartsWith($batchPrefix, [StringComparison]::OrdinalIgnoreCase)) { throw 'Batch authorization must be inside the private scenario root' }
    $batchItem = Get-Item -LiteralPath $batchPath -Force
    if ($batchItem.PSIsContainer -or $batchItem.Length -gt 262144) { throw 'Bounded ordinary batch file required' }
    $batchAncestor = $batchItem
    while ($null -ne $batchAncestor) {
        if ($batchAncestor.Attributes -band [IO.FileAttributes]::ReparsePoint) { throw 'Batch paths must be ordinary, without reparse links' }
        $batchAncestor = if ($batchAncestor.PSIsContainer) { $batchAncestor.Parent } else { $batchAncestor.Directory }
    }
    $batchDocument = Get-Content -LiteralPath $batchPath -Raw | ConvertFrom-Json -AsHashtable
    $batchKeys = @('batch','leasePath','leaseSha256','ownedReceiptPath','ownedReceiptSha256','planPath','planSha256','scenario','schemaVersion')
    if ($batchDocument -isnot [Collections.IDictionary] -or (($batchDocument.Keys | Sort-Object) -join ',') -cne ($batchKeys -join ',') -or
        ($batchDocument.schemaVersion -isnot [int] -and $batchDocument.schemaVersion -isnot [long]) -or $batchDocument.schemaVersion -ne 1 -or
        $batchDocument.batch -isnot [string] -or $batchDocument.batch -cnotin @('B1','B2','B3')) { throw 'Closed caller batch authorization required' }
    $BatchAuthorization = $batchPath
}
if (!$OpenCodeExe) {
    $proxyCommand = Get-Command opencode -ErrorAction Stop
    $OpenCodeExe = Join-Path (Split-Path $proxyCommand.Source -Parent) 'node_modules\opencode-ai\bin\opencode.exe'
}
if (!(Test-Path -LiteralPath $OpenCodeExe -PathType Leaf)) { throw 'Provide -OpenCodeExe pointing to the existing OpenCode Windows executable.' }
$proxySupervisor = Join-Path $proxyRoot 'node_modules\workspace-template-win32-x64\bin\workspace-template.exe'
if (!(Test-Path -LiteralPath $proxySupervisor -PathType Leaf)) { throw 'The repository native supervisor must be installed.' }
$proxyRunID = (Get-Date -Format 'yyyyMMdd-HHmmss') + '-' + [Guid]::NewGuid().ToString('N').Substring(0, 8)
$proxyVerification = Join-Path $proxyRoot ".tmp\starsector-proxy\supervision-$proxyRunID"
New-Item -ItemType Directory -Force -Path "$proxyVerification\.agentic" | Out-Null
$proxyProject = Get-Content -Raw -LiteralPath (Join-Path $proxyRoot '.agentic\project.json') | ConvertFrom-Json -AsHashtable
$proxyArgs = @((Join-Path $PSScriptRoot 'run.mjs'), '--opencode', $OpenCodeExe, '--run-id', $proxyRunID, '--agent', $Agent, '--model', $Model, '--prompt', $Prompt)
if ($ExportSession) { $proxyArgs += @('--session-export', $ExportSession) }
if ($WaitForReport) { $proxyArgs += '--wait-for-report' }
if ($DisableThinking) { $proxyArgs += '--disable-thinking' }
if ($DisableParallelToolCalls) { $proxyArgs += '--disable-parallel-tool-calls' }
if ($CheckMcpOnly) { $proxyArgs += '--check-mcp-only' }
if ($BatchAuthorization) { $proxyArgs += @('--batch-authorization', $BatchAuthorization) }
$proxyProject.overrides = @{ workspace = @{ commands = @{ 'node:.' = @(@{ program = (Get-Command node.exe -CommandType Application -TotalCount 1).Source; args = $proxyArgs; cwd = '.' }) } } }
[IO.File]::WriteAllText("$proxyVerification\.agentic\project.json", ($proxyProject | ConvertTo-Json -Depth 30), [Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText("$proxyVerification\package.json", '{"name":"near-future-starsector-proxy-verification","version":"1.0.0","private":true}', [Text.UTF8Encoding]::new($false))
$proxyResult = & $proxySupervisor verify $proxyVerification --scope module --module 'node:.' --concurrency 1 --timeout $TimeoutMs
$proxyExit = $LASTEXITCODE
[IO.File]::WriteAllText("$proxyVerification\verify.raw.json", ($proxyResult -join [Environment]::NewLine), [Text.UTF8Encoding]::new($false))
$proxyParsed = ($proxyResult -join [Environment]::NewLine) | ConvertFrom-Json
$proxyParsed.result.steps | ForEach-Object {
    [pscustomobject]@{ status = $_.status; timedOut = $_.timedOut; processOwnership = $_.processOwnership; stdout = $_.stdout; stderr = $_.stderr }
} | ConvertTo-Json -Depth 8
Write-Output "Supervisor evidence: $proxyVerification\verify.raw.json"
exit $proxyExit
