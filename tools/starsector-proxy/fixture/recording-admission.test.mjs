// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {recordingAdmissionFixture} from './recording-admission-test-support.mjs';
import {json,write,filePin,powershell,repository} from './seed-test-support.mjs';

test('real v2 protection admission accepts the pinned profile while retaining receipt schema1',()=>{
 const f=recordingAdmissionFixture();try{
  const result=f.admit();assert.equal(result.status,0,result.stderr);
  assert.equal(JSON.parse(readFileSync(f.frozenPath,'utf8')).schemaVersion,1);
  assert.equal(existsSync(resolve(f.admission,'controller-start.private.json')),false);
  assert.equal(existsSync(resolve(f.admission,'supervision-launch')),false);
 }finally{f.close();}
});

for(const kind of ['jfc','flags'])test('real admission independently refuses a rehashed wrong '+kind+' recipe',()=>{
 const f=recordingAdmissionFixture();try{
  const before=f.admit();assert.equal(before.status,0,before.stderr);
  const path=kind==='jfc'?resolve(f.target,'capture/active.jfc'):resolve(f.installation,'vmparams');
  write(path,readFileSync(path,'utf8').replace(kind==='jfc'?'<setting name="threshold">0 ms</setting>':'duration=420s',kind==='jfc'?'<setting name="threshold">1 ms</setting>':'duration=421s'));
  f.refreshGeneratedPins();
  const result=f.admit();assert.notEqual(result.status,0);
  assert.match(result.stderr,kind==='jfc'?/Expected complete recording configuration/:/Fixed private recording flag/);
  assert.equal(existsSync(resolve(f.admission,'controller-start.private.json')),false);
 }finally{f.close();}
});

for(const owner of ['receipt','plan','lease'])test('real v2 admission requires '+owner+' profile agreement',()=>{
 const f=recordingAdmissionFixture();try{
  const path=owner==='receipt'?f.frozenPath:resolve(f.admission,owner==='plan'?'plan.json':'native-lane-lease.private.json');
  const value=JSON.parse(readFileSync(path,'utf8'));value.recordingProfileSha256='F'.repeat(64);json(path,value);
  const leasePath=resolve(f.admission,'native-lane-lease.private.json'),lease=JSON.parse(readFileSync(leasePath,'utf8'));
  lease.fixtureReceiptSha256=filePin(f.frozenPath).sha256;lease.planSha256=filePin(resolve(f.admission,'plan.json')).sha256;json(leasePath,lease);
  const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/receipt plan and lease must agree/);
 }finally{f.close();}
});

for(const owner of ['receipt','plan','lease'])test('real v1 admission refuses '+owner+' profile authority without changing valid legacy behavior',()=>{
 const f=recordingAdmissionFixture(1);try{
  const valid=f.admit();assert.equal(valid.status,0,valid.stderr);
  const path=owner==='receipt'?f.frozenPath:resolve(f.admission,owner==='plan'?'plan.json':'native-lane-lease.private.json');
  const value=JSON.parse(readFileSync(path,'utf8'));value.recordingProfileSha256=null;json(path,value);
  const leasePath=resolve(f.admission,'native-lane-lease.private.json'),lease=JSON.parse(readFileSync(leasePath,'utf8'));
  lease.fixtureReceiptSha256=filePin(f.frozenPath).sha256;lease.planSha256=filePin(resolve(f.admission,'plan.json')).sha256;json(leasePath,lease);
  const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Legacy fixture cannot carry/);
 }finally{f.close();}
});

test('the public runner stops on wrong v2 recording flags before any owned verifier is created',()=>{
 const f=recordingAdmissionFixture();try{
  const vmparams=resolve(f.installation,'vmparams');write(vmparams,readFileSync(vmparams,'utf8').replace('maxsize=64m','maxsize=65m'));f.refreshGeneratedPins();
  const result=powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/run-native.ps1'),'-BindingPath',f.bindingPath,'-BindingSha256',f.bindingPin.sha256,'-EvidenceRoot',f.admission,'-Phase','launch']);
  assert.notEqual(result.status,0);assert.match(result.stderr,/Fixed private recording flag/);
  for(const name of ['launch-supervisor.private.json','supervision-launch','controller-start.private.json'])assert.equal(existsSync(resolve(f.admission,name)),false,name);
 }finally{f.close();}
});

for(const shape of ['quoted-second','quoted-only','malformed-prefix'])test('real v2 admission refuses '+shape+' recording syntax after receipt rehash',()=>{
 const f=recordingAdmissionFixture();try{
  const path=resolve(f.installation,'vmparams'),base=readFileSync(path,'utf8');
  const flag=base.match(/-XX:StartFlightRecording=\S+/)?.[0];assert.ok(flag);
  const changed=shape==='quoted-second'?base+' "-XX:StartFlightRecording=name=Other,duration=999s,filename=C:/foreign.jfr"':
   base.replace(flag,shape==='quoted-only'?'"'+flag+'"':'prefix'+flag);
  write(path,changed);f.refreshGeneratedPins();
  const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/Fixed private recording flag/);
  assert.equal(existsSync(resolve(f.admission,'controller-start.private.json')),false);
 }finally{f.close();}
});

