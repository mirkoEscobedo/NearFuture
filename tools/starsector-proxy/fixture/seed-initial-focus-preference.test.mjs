// @ts-check
// Execute only the two real selector ASTs with an injected preference reader.
// The controller, protection program and registry are never executed.
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {repository,powershell,quote} from './seed-test-support.mjs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';

for(const file of ['native-controller.ps1','protection-check.ps1'])test(file+' retains the read-only preference guard for both creation profiles',()=>{
    const f=seedCreationFixture();
    try{
    const script=String.raw`$ErrorActionPreference='Stop'
$tokens=$null;$errors=$null
$ast=[Management.Automation.Language.Parser]::ParseFile(${quote(resolve(repository,'tools/starsector-proxy/fixture',file))},[ref]$tokens,[ref]$errors)
if(@($errors).Count){throw 'Maintained source must parse'}
$commands=@($ast.FindAll({param($n)$n-is[Management.Automation.Language.CommandAst]-and$n.GetCommandName()-ceq'Assert-NativeSeedPreferenceUnchanged'},$true))
if($commands.Count-ne1){throw 'Exactly one live preference guard required'}
$statement=$commands[0].Parent
while($null-ne$statement-and$statement-isnot[Management.Automation.Language.IfStatementAst]){$statement=$statement.Parent}
if($null-eq$statement){throw 'Preference selector boundary required'}
. ${quote(resolve(repository,'tools/starsector-proxy/fixture/seed-preference.ps1'))}
$base=Get-Content -LiteralPath ${quote(f.planPath)} -Raw|ConvertFrom-Json -DateKind String
$script:expected=Get-Content -LiteralPath ${quote(f.preferencePath)} -Raw|ConvertFrom-Json -DateKind String
function Read-NativeSeedSharedPreference{
 $script:called++;$v=$script:expected|ConvertTo-Json -Depth 10|ConvertFrom-Json -DateKind String;$v.value=$script:actual;return $v
}
$results=@(foreach($actual in @('true','false')){foreach($case in @(@('DISPOSABLE_SEED_CREATION',4),@('DISPOSABLE_SEED_CREATION_INITIAL_FOCUS',5),@('DISPOSABLE_SEED_CREATION_INITIAL_FOCUS',4),@('UNKNOWN',5))){
 $seedCreationPlan=$closedPlan=[pscustomobject]@{campaignControl=[pscustomobject]@{profile=$case[0];schemaVersion=$case[1]};seedCreation=$base.seedCreation}
 $EvidenceRoot=${quote(f.scenario)};$record=@{};$script:called=0;$script:actual=$actual;$rejected=$false
 try{. ([scriptblock]::Create($statement.Extent.Text))}catch{if($_.Exception.Message-cne'Seed shared preference changed'){throw};$rejected=$true}
 if($record.ContainsKey('sharedPreference')-and$record.sharedPreference.value-cne'true'){throw 'Eligible unchanged value must be recorded'}
 [pscustomobject]@{calls=$script:called;rejected=$rejected;recorded=$record.ContainsKey('sharedPreference')}
}})
$results|ConvertTo-Json -Compress`;
    const result=powershell(['-Command',script]);assert.equal(result.error,undefined);assert.equal(result.signal,null);assert.equal(result.status,0,result.stderr);
    const rows=JSON.parse(result.stdout);
    assert.deepEqual(rows.map(/** @param {any} row */row=>row.calls),[1,1,1,0,1,1,1,0]);
    assert.deepEqual(rows.map(/** @param {any} row */row=>row.rejected),[false,false,false,false,true,true,true,false]);
    assert.deepEqual(rows.map(/** @param {any} row */row=>row.recorded),file==='protection-check.ps1'?[true,true,true,false,false,false,false,false]:Array(8).fill(false));
    }finally{f.close();}
});
