// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,existsSync,readdirSync,renameSync,symlinkSync,unlinkSync} from 'node:fs';
import {resolve} from 'node:path';
import {seedFixture,filePin,write,json,powershell,quote,repository} from './seed-test-support.mjs';

test('stage exactly one generated save, preserve the frozen receipt, and admit its current census',()=>{
 const f=seedFixture();try{
  const frozen=readFileSync(f.frozenPath);
  const stagePlan=readFileSync(resolve(f.target,'seed-stage-plan.private.json')),authority=readFileSync(f.authorityPath);
  const result=f.stage();assert.equal(result.status,0,result.stderr);
  assert.deepEqual(readFileSync(f.frozenPath),frozen);
  assert.ok(existsSync(resolve(f.target,'seed-stage-claim.private.json')));
  const receipt=JSON.parse(readFileSync(resolve(f.target,'seed-stage-receipt.private.json'),'utf8'));
  assert.equal(receipt.kind,'NATIVE99_STAGED_SAVE_RECEIPT_V1');assert.equal(receipt.nativeLaunched,false);
  assert.equal(receipt.saveDirectory,'save_fixture_1');assert.equal(receipt.files.length,2);
  for(const name of ['campaign.xml','descriptor.xml'])assert.deepEqual(filePin(resolve(f.installation,'saves',f.saveDirectory,name)),filePin(resolve(f.sourceSave,name)));
  f.launch();const admitted=f.admit();assert.equal(admitted.status,0,admitted.stderr);
  assert.deepEqual(readFileSync(resolve(f.target,'seed-stage-plan.private.json')),stagePlan);assert.deepEqual(readFileSync(f.authorityPath),authority);
  const beforeSmoke=f.admit('before-smoke');assert.equal(beforeSmoke.status,0,beforeSmoke.stderr);
 }finally{f.close();}
});

test('source protection requires an integer zero rather than boolean false',()=>{
 const f=seedFixture();try{
  const path=resolve(f.evidence,'after-smoke-protection.private.json'),protection=JSON.parse(readFileSync(path,'utf8'));
  protection.liveGameProcesses=false;f.manifest.provenance.protection=json(path,protection);
  Object.assign(f.closeout.protection,{liveGameProcesses:false});f.closeout.evidence.protectionSHA256=f.manifest.provenance.protection.sha256;
  f.manifest.provenance.closeout=json(f.closeoutPath,f.closeout);f.refresh();
  const result=f.stage();assert.notEqual(result.status,0);assert.match(result.stderr,/Bounded seed integer/);
  assert.equal(existsSync(resolve(f.target,'seed-stage-claim.private.json')),false);
 }finally{f.close();}
});

test('a boolean cannot stand in for the pinned seed manifest version',()=>{
 const f=seedFixture();try{
  Object.assign(f.manifest,{schemaVersion:true});f.refresh();
  const result=f.stage();assert.notEqual(result.status,0);assert.match(result.stderr,/Bounded seed integer/);
  assert.equal(existsSync(resolve(f.target,'seed-stage-claim.private.json')),false);
 }finally{f.close();}
});

test('stage authority substitution and changed originals refuse before any claim or copy',()=>{
 const f=seedFixture();try{
  let result=powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/prepare-fixture.ps1'),'-BindingPath',f.bindingPath,'-BindingSha256',f.bindingPin.sha256,'-StageSeedAuthorityPath',f.authorityPath,'-StageSeedAuthoritySha256','A'.repeat(64)]);
  assert.notEqual(result.status,0);assert.match(result.stderr,/Seed JSON digest changed/);
  write(resolve(f.original,'saves/item-0.txt'),'changed original test file');result=f.stage();
  assert.notEqual(result.status,0);assert.match(result.stderr,/Protected source or backup changed/);
  assert.equal(existsSync(resolve(f.target,'seed-stage-claim.private.json')),false);assert.deepEqual(readdirSync(resolve(f.installation,'saves')),[]);
 }finally{f.close();}
});

test('changed source bytes and unknown source files are refused before the stage claim',()=>{
 const f=seedFixture();try{
  write(resolve(f.sourceSave,'campaign.xml'),f.campaign+' ');let result=f.stage();assert.notEqual(result.status,0);assert.match(result.stderr,/Seed file bytes changed/);
  write(resolve(f.sourceSave,'campaign.xml'),f.campaign);write(resolve(f.sourceSave,'extra.txt'),'not in manifest');result=f.stage();
  assert.notEqual(result.status,0);assert.match(result.stderr,/Exact two-file seed census/);assert.equal(existsSync(resolve(f.target,'seed-stage-claim.private.json')),false);
 }finally{f.close();}
});

test('source scenario and file traversal cannot introduce a different seed',()=>{
 const f=seedFixture();try{
  f.manifest.sourceScenario='../outside';f.refresh();let result=f.stage();assert.notEqual(result.status,0);assert.match(result.stderr,/Contained seed source scenario/);
  f.manifest.sourceScenario=f.source.split(/[\\/]/).at(-1)??'';f.manifest.files[0].relative='../campaign.xml';f.refresh();result=f.stage();
  assert.notEqual(result.status,0);assert.match(result.stderr,/Closed seed file paths/);assert.equal(existsSync(resolve(f.target,'seed-stage-claim.private.json')),false);
 }finally{f.close();}
});

test('a source directory junction is refused even when it resolves to the pinned bytes',()=>{
 const f=seedFixture();let linked=false;try{
  const retained=f.sourceSave+'-retained';renameSync(f.sourceSave,retained);symlinkSync(retained,f.sourceSave,'junction');linked=true;
  const result=f.stage();assert.notEqual(result.status,0);assert.match(result.stderr,/Ordinary seed ancestry/);
  assert.equal(existsSync(resolve(f.target,'seed-stage-claim.private.json')),false);
 }finally{if(linked)unlinkSync(f.sourceSave);f.close();}
});

test('a successful stage cannot be replayed or overwrite its receipts',()=>{
 const f=seedFixture();try{
  const first=f.stage();assert.equal(first.status,0,first.stderr);
  const claim=resolve(f.target,'seed-stage-claim.private.json'),receipt=resolve(f.target,'seed-stage-receipt.private.json'),claimBefore=readFileSync(claim),receiptBefore=readFileSync(receipt);
  const result=f.stage();assert.notEqual(result.status,0);assert.match(result.stderr,/already claimed/);
  assert.deepEqual(readFileSync(claim),claimBefore);assert.deepEqual(readFileSync(receipt),receiptBefore);
 }finally{f.close();}
});

test('a filesystem copy failure preserves the claim and partial data and refuses resume',()=>{
 const f=seedFixture();try{
  const frozen=readFileSync(f.frozenPath);
  const command="$ErrorActionPreference='Stop';. "+quote(resolve(repository,'tools/starsector-proxy/fixture/seed-fixture.ps1'))+
   '; $script:realNativeSeedCopy=${function:Copy-NativeSeedFile}; function Copy-NativeSeedFile {param([string]$Source,[string]$Destination,$Expected,[DateTime]$Deadline) if($Expected.relative-ceq\'descriptor.xml\'){throw \'Controlled filesystem copy failure\'}; & $script:realNativeSeedCopy $Source $Destination $Expected $Deadline}; Invoke-NativeSeedStage -BindingPath '+quote(f.bindingPath)+' -BindingSha256 '+quote(f.bindingPin.sha256)+' -AuthorityPath '+quote(f.authorityPath)+' -AuthoritySha256 '+quote(filePin(f.authorityPath).sha256);
  const result=powershell(['-Command',command]);assert.notEqual(result.status,0);assert.match(result.stderr,/Controlled filesystem copy failure/);
  assert.ok(existsSync(resolve(f.target,'seed-stage-claim.private.json')));assert.equal(existsSync(resolve(f.target,'seed-stage-receipt.private.json')),false);
  assert.deepEqual(readdirSync(resolve(f.installation,'saves',f.saveDirectory)),['campaign.xml']);assert.deepEqual(readFileSync(f.frozenPath),frozen);
  const retry=f.stage();assert.notEqual(retry.status,0);assert.match(retry.stderr,/already claimed/);
 }finally{f.close();}
});
