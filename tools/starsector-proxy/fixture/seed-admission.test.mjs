// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,readdirSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {seedFixture,json,filePin,write,powershell,repository} from './seed-test-support.mjs';

test('the default remains an empty fixture and a partial claim cannot masquerade as empty',()=>{
 const f=seedFixture();try{
  f.launch('neither');let result=f.admit();assert.equal(result.status,0,result.stderr);
  json(resolve(f.target,'seed-stage-claim.private.json'),{});result=f.admit();
  assert.notEqual(result.status,0);assert.match(result.stderr,/Seed state requires explicit launch opt-in/);
 }finally{f.close();}
});

test('seeded admission requires matching closed opt-in on both plan and lease',()=>{
 const f=seedFixture();try{
  const staged=f.stage();assert.equal(staged.status,0,staged.stderr);
  for(const which of /** @type {const} */ (['neither','plan','lease'])){f.launch(which);const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Seed state requires explicit|Seed plan and lease authority must agree/);}
  f.launch();const planPath=resolve(f.admission,'plan.json'),leasePath=resolve(f.admission,'native-lane-lease.private.json');
  const plan=JSON.parse(readFileSync(planPath,'utf8')),lease=JSON.parse(readFileSync(leasePath,'utf8'));
  plan.seed.skipProtection=true;lease.seed.skipProtection=true;lease.planSha256=json(planPath,plan).sha256;json(leasePath,lease);
  const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Closed seed schema/);
 }finally{f.close();}
});

test('admission rechecks target bytes and full census rather than trusting the stage receipt',()=>{
 const f=seedFixture();try{
  const staged=f.stage();assert.equal(staged.status,0,staged.stderr);f.launch();
  const campaign=resolve(f.installation,'saves',f.saveDirectory,'campaign.xml');write(campaign,f.campaign+' ');
  let result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Seed file bytes changed/);
  write(campaign,f.campaign);write(resolve(f.installation,'saves',f.saveDirectory,'unlisted.txt'),'not admitted');
  result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Exact two-file seed census/);
 }finally{f.close();}
});

test('seeded before-smoke uses the same current census and refuses a prelaunch log',()=>{
 const f=seedFixture();try{
  const staged=f.stage();assert.equal(staged.status,0,staged.stderr);f.launch();
  write(resolve(f.installation,'starsector-core/starsector.log'),'unexpected existing log');
  const result=f.admit('before-smoke');assert.notEqual(result.status,0);assert.match(result.stderr,/Seeded fixture log must remain absent/);
 }finally{f.close();}
});

test('the original save protections remain mandatory for an otherwise valid seeded fixture',()=>{
 const f=seedFixture();try{
  const staged=f.stage();assert.equal(staged.status,0,staged.stderr);f.launch();
  write(resolve(f.original,'saves/item-0.txt'),'changed original test file');
  const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Protected source or backup changed/);
 }finally{f.close();}
});

test('an immutable staging receipt is hash-bound and cannot be substituted at admission',()=>{
 const f=seedFixture();try{
  const staged=f.stage();assert.equal(staged.status,0,staged.stderr);f.launch();
  const path=resolve(f.target,'seed-stage-receipt.private.json'),before=filePin(path);
  write(path,readFileSync(path,'utf8')+' ');assert.notEqual(filePin(path).sha256,before.sha256);
  const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Seed JSON digest changed/);
  assert.equal(readdirSync(resolve(f.installation,'saves')).length,1);
 }finally{f.close();}
});

test('both fresh gates reject changed seed source bytes after successful staging',()=>{
 const f=seedFixture();try{
  const staged=f.stage();assert.equal(staged.status,0,staged.stderr);f.launch();write(resolve(f.sourceSave,'campaign.xml'),f.campaign+' ');
  for(const phase of ['admit-launch','before-smoke']){const result=f.admit(phase);assert.notEqual(result.status,0);assert.match(result.stderr,/Seed file bytes changed/);}
  assert.equal(existsSync(resolve(f.admission,'before-smoke-protection.private.json')),false);
 }finally{f.close();}
});

test('the public runner and controller reach seeded admission before their launch effects',()=>{
 const f=seedFixture();try{
  const staged=f.stage();assert.equal(staged.status,0,staged.stderr);f.launch();
  // Runner stops after its real admission, before supervisor creation; no game actor exists in this generated fixture.
  const sentinel=resolve(f.admission,'launch-supervisor.private.json');write(sentinel,'controlled existing evidence');
  let result=powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/run-native.ps1'),'-BindingPath',f.bindingPath,'-BindingSha256',f.bindingPin.sha256,'-EvidenceRoot',f.admission,'-Phase','launch']);
  assert.notEqual(result.status,0);assert.match(result.stderr,/Preserve existing native evidence/);assert.equal(existsSync(resolve(f.admission,'supervision-launch')),false);
  // Controller inherits the test's Windows Job, admits the seed, then refuses the deliberately absent JRE before Start-Process.
  const deadline=new Date(Date.now()+470000).toISOString().replace(/Z$/, '0000Z');
  result=powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/native-controller.ps1'),'-BindingPath',f.bindingPath,'-BindingSha256',f.bindingPin.sha256,'-EvidenceRoot',f.admission,'-OuterDeadlineUtc',deadline]);
  assert.notEqual(result.status,0);assert.match(result.stderr,/Could not find a part of the path/);assert.match(result.stderr,/java\.exe/);
  assert.equal(existsSync(resolve(f.admission,'controller-start.private.json')),false);assert.equal(existsSync(resolve(f.installation,'starsector-core/starsector.log')),false);
 }finally{f.close();}
});
