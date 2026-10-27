// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {fileURLToPath} from 'node:url';
import {powershell,quote} from './seed-test-support.mjs';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {claimSeededLauncher} from './campaign-input-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

const root=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
test('the real controller deadline never extends the original outer deadline after slow admission',()=>{
 const script=String.raw`$ErrorActionPreference='Stop'
$tokens=$null;$errors=$null
$tree=[Management.Automation.Language.Parser]::ParseFile(${quote(resolve(root,'tools/starsector-proxy/fixture/native-controller.ps1'))},[ref]$tokens,[ref]$errors)
if(@($errors).Count){throw 'Actual controller must parse'}
$values=@($tree.FindAll({param($n) $n-is[Management.Automation.Language.HashtableAst]},$true)|ForEach-Object{$_.KeyValuePairs}|Where-Object{$_.Item1.Value-ceq'ControllerDeadlineUtc'})
if($values.Count-ne1){throw 'Exactly one published controller deadline required'}
$setup=@($tree.EndBlock.Statements|Where-Object{($_-is[Management.Automation.Language.AssignmentStatementAst]-and$_.Left.Extent.Text-ceq'$controllerDeadlineUtc')-or($_-is[Management.Automation.Language.IfStatementAst]-and$_.Extent.Text.StartsWith('if($controllerDeadlineUtc-gt$conservativeOuterDeadline)'))})
$cases=@(
 @('2026-10-27T07:50:59.2838289Z','2026-10-27T07:58:48.5837603Z',480000,'2026-10-27T07:58:48.5837603Z'),
 @('2026-10-27T07:50:50.0000000Z','2026-10-27T07:58:48.0000000Z',480000,'2026-10-27T07:58:40.0000000Z'),
 @('2026-10-27T07:50:58.0000000Z','2026-10-27T07:58:48.0000000Z',480000,'2026-10-27T07:58:48.0000000Z'),
 @('2026-10-27T07:50:58.0000001Z','2026-10-27T07:58:48.0000000Z',480000,'2026-10-27T07:58:48.0000000Z'),
 @('2026-10-27T07:50:59.2838289Z','2026-10-27T09:50:48.5837603Z',7200000,'2026-10-27T09:50:48.5837603Z')
)
$results=@(foreach($case in $cases){
 $script:started=[DateTime]::Parse($case[0]).ToUniversalTime();$conservativeOuterDeadline=[DateTime]::Parse($case[1]).ToUniversalTime();$SessionWatchdogMs=[long]$case[2]
 foreach($statement in $setup){. ([scriptblock]::Create($statement.Extent.Text))}
 $actual=. ([scriptblock]::Create($values[0].Item2.Extent.Text))
 $expected=[DateTime]::Parse($case[3]).ToUniversalTime()
 if($actual-ne$expected-or$actual-gt$conservativeOuterDeadline-or$actual-gt$script:started.AddMilliseconds($SessionWatchdogMs-10000)){throw ('Controller extended original deadline: '+$actual.ToString('o'))}
 $actual.ToString('o')
})
$results|ConvertTo-Json -Compress`;
 const result=powershell(['-Command',script]);
 assert.equal(result.error,undefined);assert.equal(result.status,0,result.stderr);
 assert.equal(JSON.parse(result.stdout).length,5);
});

test('the real delayed-start producer composes with public launch admission without relaxing forged-deadline refusal',()=>{
 const f=campaignLaunchFixture();
 try{
  const anchor=Date.now()-20000,outer=new Date(anchor+480000).toISOString(),started=new Date(anchor+10700).toISOString();
  const script=String.raw`$ErrorActionPreference='Stop'
$tokens=$null;$errors=$null
$tree=[Management.Automation.Language.Parser]::ParseFile(${quote(resolve(root,'tools/starsector-proxy/fixture/native-controller.ps1'))},[ref]$tokens,[ref]$errors)
if(@($errors).Count){throw 'Actual controller must parse'}
$script:started=[DateTime]::Parse(${quote(started)}).ToUniversalTime();$conservativeOuterDeadline=[DateTime]::Parse(${quote(outer)}).ToUniversalTime();$SessionWatchdogMs=480000
$setup=@($tree.EndBlock.Statements|Where-Object{($_-is[Management.Automation.Language.AssignmentStatementAst]-and$_.Left.Extent.Text-ceq'$controllerDeadlineUtc')-or($_-is[Management.Automation.Language.IfStatementAst]-and$_.Extent.Text.StartsWith('if($controllerDeadlineUtc-gt$conservativeOuterDeadline)'))})
foreach($statement in $setup){. ([scriptblock]::Create($statement.Extent.Text))}
$value=@($tree.FindAll({param($n) $n-is[Management.Automation.Language.HashtableAst]},$true)|ForEach-Object{$_.KeyValuePairs}|Where-Object{$_.Item1.Value-ceq'ControllerDeadlineUtc'})
if($value.Count-ne1){throw 'One published deadline required'}
$actual=. ([scriptblock]::Create($value[0].Item2.Extent.Text));$actual.ToString('o')`;
  const produced=powershell(['-Command',script]);assert.equal(produced.status,0,produced.stderr);
  assert.equal(Date.parse(produced.stdout.trim()),anchor+480000);
  Object.assign(f.campaign.rootClock,{startMs:anchor,deadlineMs:anchor+480000,callerAdmissionDeadlineMs:anchor+420000,
   originalCallerAdmissionDeadlineMs:anchor+420000,effectiveDeadlineMs:anchor+420000});
  f.campaign.rootCallRecord.value.startMs=anchor;
  f.plan.callerAdmissionDeadlineMs=f.launchLease.callerAdmissionDeadlineMs=anchor+420000;
  Object.assign(f.controller,{controllerCreationTime:new Date(anchor+100).toISOString(),at:started,
   conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:produced.stdout.trim()});
  Object.assign(f.campaign.producerWitness,{controllerCreationTime:f.controller.controllerCreationTime,
   conservativeOuterDeadlineUtc:outer,controllerDeadlineUtc:produced.stdout.trim(),effectiveDeadlineMs:anchor+420000});
  f.refreshLaunch();let first=claimSeededLauncher(f,'produced-deadline-B1',false);
  assert.deepEqual(readBatchAuthorization(first.source,{root:f.root,runID:first.runID}).binding.ownedPids,[101]);
  f.controller.controllerDeadlineUtc=new Date(anchor+481000).toISOString();f.campaign.producerWitness.controllerDeadlineUtc=f.controller.controllerDeadlineUtc;
  f.refreshLaunch();first=claimSeededLauncher(f,'forged-deadline-B1',false);
  assert.throws(()=>readBatchAuthorization(first.source,{root:f.root,runID:first.runID}),/actual controller/);
 }finally{f.close();}
});
