// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,rmSync} from 'node:fs';
import {spawnSync} from 'node:child_process';
import {resolve,relative,isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';
const repository=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
const parent=resolve(repository,'.tmp/starsector-proxy');
const powershell=process.env.NF_TEST_POWERSHELL ?? '';
if (!powershell || !isAbsolute(powershell)) throw new Error('Explicit absolute NF_TEST_POWERSHELL required for Windows snapshot checks');
const quote=(/** @type {string} */ value)=>"'"+value.replaceAll("'","''")+"'";
function run(/** @type {string} */ body) {
 const root=mkdtempSync(resolve(parent,'owned-snapshot-test-'));
 try {
  const command="$ErrorActionPreference='Stop';. "+quote(resolve(repository,'tools/starsector-proxy/fixture/publish-owned-snapshot.ps1'))+";$root="+quote(root)+";"+body;
  return spawnSync(powershell,['-NoProfile','-NonInteractive','-Command',command],{cwd:repository,encoding:'utf8',timeout:15000});
 } finally {
  const suffix=relative(parent,root);assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix));
  rmSync(root,{recursive:true});
 }
}
test('a current snapshot reader cannot terminate publication or expose a partial replacement',()=>{
 const result=run(`
 $path=Join-Path $root 'owned-processes-current.private.json'
 $first=[ordered]@{generation=1;processes=@(@{pid=123;label='original'})}
 if(!(Publish-OwnedSnapshot -Path $path -Snapshot $first)){throw 'Initial publication failed'}
 $original=[IO.File]::ReadAllBytes($path)
 $reader=[IO.File]::Open($path,[IO.FileMode]::Open,[IO.FileAccess]::Read,[IO.FileShare]::Read)
 try {
  $next=[ordered]@{generation=2;processes=@(@{pid=456;label=('α'*8192)})}
  if(Publish-OwnedSnapshot -Path $path -Snapshot $next){throw 'Read lock must retain prior snapshot'}
  if([Convert]::ToBase64String([IO.File]::ReadAllBytes($path))-cne[Convert]::ToBase64String($original)){throw 'Locked snapshot was modified'}
 } finally {$reader.Dispose()}
 if(!(Publish-OwnedSnapshot -Path $path -Snapshot $next)){throw 'Unlocked publication failed'}
 $actual=[IO.File]::ReadAllText($path)|ConvertFrom-Json
 if($actual.generation-ne2-or$actual.processes[0].pid-ne456-or$actual.processes[0].label.Length-ne8192){throw 'Published snapshot incomplete'}
 if(@(Get-ChildItem -LiteralPath $root -Force).Count-ne1){throw 'Publication left temporary artifacts'}
 'LOCKED_OLD_INTACT_THEN_COMPLETE_NEW'
 `);
 assert.equal(result.status,0,result.stderr);
 assert.match(result.stdout,/LOCKED_OLD_INTACT_THEN_COMPLETE_NEW/);
});

test('other publication failures remain fatal and remove only this call temporary file',()=>{
 const result=run(`
 $path=Join-Path $root 'owned-processes-current.private.json'
 [void][IO.Directory]::CreateDirectory($path)
 $refused=$false
 try {$null=Publish-OwnedSnapshot -Path $path -Snapshot @{generation=1}} catch {$refused=$true}
 if(!$refused){throw 'Non-sharing failure was hidden'}
 if(!(Test-Path -LiteralPath $path -PathType Container)){throw 'Existing destination modified'}
 if(@(Get-ChildItem -LiteralPath $root -Force).Count-ne1){throw 'Failure left temporary artifacts'}
 'OTHER_ERRORS_REMAIN_FATAL'
 `);
 assert.equal(result.status,0,result.stderr);
 assert.match(result.stdout,/OTHER_ERRORS_REMAIN_FATAL/);
});
