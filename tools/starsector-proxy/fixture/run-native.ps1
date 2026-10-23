param([Parameter(Mandatory)][string]$BindingPath,[Parameter(Mandatory)][string]$BindingSha256,[string]$Phase,[string]$EvidenceRoot,[ValidateRange(480000,43200000)][long]$SessionWatchdogMs=480000)
$ErrorActionPreference='Stop'
. (Join-Path $PSScriptRoot 'fixture-inputs.ps1')
. (Join-Path $PSScriptRoot 'session-watchdog.ps1')
$binding=Read-NativeFixtureBinding $BindingPath $BindingSha256
if([string]::IsNullOrWhiteSpace($EvidenceRoot)){$EvidenceRoot=$binding.fixtureRoot}
if($Phase-cnotin@('ownership-probe','launch')){throw 'Explicit native phase required'}
$root=$binding.repositoryRoot
$evidence=$EvidenceRoot
if($Phase-eq'launch'){$SessionWatchdogMs=Read-NativeSessionWatchdog $evidence $SessionWatchdogMs}
$admission=if($Phase-eq'launch'){'admit-launch'}else{'admit-probe'}
& (Join-Path $PSScriptRoot 'protection-check.ps1') -BindingPath $BindingPath -BindingSha256 $BindingSha256 -Phase $admission -EvidenceRoot $evidence
$output="$evidence/$Phase-supervisor.private.json"
if(Test-Path -LiteralPath $output){throw 'Preserve existing native evidence; no unnamed retry'}
$verification="$evidence/supervision-$Phase"
if(Test-Path -LiteralPath $verification){throw 'Refuse replacement of native verification root'}
$timeout=if($Phase-eq'ownership-probe'){60000}else{$SessionWatchdogMs}
# Earlier than owned project preparation and verifier startup; never an invented Job birth.
$outerDeadlineUtc=[DateTime]::UtcNow.AddMilliseconds($timeout).ToString('o')
[void][IO.Directory]::CreateDirectory("$verification/.agentic")
$project=Get-Content "$root/.agentic/project.json" -Raw|ConvertFrom-Json -AsHashtable
$args=@('-NoProfile','-NonInteractive','-File',(Join-Path $PSScriptRoot 'native-controller.ps1'),'-EvidenceRoot',$evidence,'-BindingPath',$BindingPath,'-BindingSha256',$BindingSha256)
if($Phase-eq'ownership-probe'){$args+='-OwnershipProbeOnly'}else{$args+=@('-OuterDeadlineUtc',$outerDeadlineUtc,'-SessionWatchdogMs',[string]$SessionWatchdogMs)}
$project.overrides=@{workspace=@{commands=@{'node:.'=@(@{program=(Get-Command pwsh -CommandType Application -TotalCount 1).Source;args=$args;cwd='.'})}}}
[IO.File]::WriteAllText("$verification/.agentic/project.json",($project|ConvertTo-Json -Depth 30),[Text.UTF8Encoding]::new($false))
[IO.File]::WriteAllText("$verification/package.json",'{"name":"near-future-recovery-native-owned-launch","version":"1.0.0","private":true}',[Text.UTF8Encoding]::new($false))
& "$root/node_modules/workspace-template-win32-x64/bin/workspace-template.exe" verify $verification --scope module --module 'node:.' --concurrency 1 --timeout $timeout|Set-Content -Encoding utf8 $output
$code=$LASTEXITCODE
$result=Get-Content $output -Raw|ConvertFrom-Json
$result.result|Select-Object verdict,blocked,@{Name='steps';Expression={$_.steps|Select-Object status,timedOut,processOwnership,stdout,stderr}}|ConvertTo-Json -Depth 7
exit $code
