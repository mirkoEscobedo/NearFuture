// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,rmSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {resolve,relative,isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';

const repository=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
const scenarios=resolve(repository,'.tmp/starsector-proxy/scenarios');
const powershell=process.env.NF_TEST_POWERSHELL ?? '';
if (!powershell || !isAbsolute(powershell)) throw new Error('Explicit absolute NF_TEST_POWERSHELL required for Windows fixture checks');
const quote=(/** @type {string} */ value)=>"'"+value.replaceAll("'","''")+"'";
const hash=(/** @type {string} */ value)=>createHash('sha256').update(value).digest('hex');
function fixture() {
 mkdirSync(scenarios,{recursive:true});
 const root=mkdtempSync(resolve(scenarios,'fixture-binding-test-'));
 for(const name of ['source','runtime'])mkdirSync(resolve(root,name));
 const agent=resolve(root,'test-agent.jar');writeFileSync(agent,'test-only agent input');
 const binding={schemaVersion:1,fixtureRoot:root,sourceInstallation:resolve(root,'source'),
 qualifiedRuntimeRoot:resolve(root,'runtime'),candidateHead:'1234567890123456789012345678901234567890',
 runtimeManifestSha256:'a'.repeat(64),presentationAgentPath:agent,presentationAgentSha256:hash('test-only agent input'),
 presentationClassSha256:'b'.repeat(64),driverRole:'/test/driver'};
 const path=resolve(root,'fixture-binding.private.json');
 const text=JSON.stringify(binding);writeFileSync(path,text);
 return {root,path,binding,digest:hash(text),close(){
  const suffix=relative(scenarios,root);assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix));
  rmSync(root,{recursive:true});
 }};
}
function read(/** @type {ReturnType<typeof fixture>} */ f,/** @type {string} */ digest=f.digest) {
 const source=resolve(repository,'tools/starsector-proxy/fixture/fixture-inputs.ps1');
 return spawnSync(powershell,['-NoProfile','-NonInteractive','-Command',
 "$ErrorActionPreference='Stop'; . "+quote(source)+"; $b=Read-NativeFixtureBinding "+quote(f.path)+" "+quote(digest)+"; $b|ConvertTo-Json -Depth 6"],
 {cwd:repository,encoding:'utf8',timeout:15000});
}
test('a pinned private binding selects reusable fixture inputs without machine paths in tooling',()=>{
 const f=fixture();
 try {const result=read(f);assert.equal(result.status,0,result.stderr);
 const binding=JSON.parse(result.stdout);
 assert.equal(binding.fixtureRoot,f.root);
 assert.equal(binding.sourceInstallation,f.binding.sourceInstallation);
 assert.equal(binding.fixtureId,f.root.split(/[\\/]/).at(-1));
 assert.equal(binding.repositoryRoot,repository);
 }finally{f.close();}
});

test('changed binding bytes are refused before any native fixture effect',()=>{
 const f=fixture();try {
 const result=read(f,'f'.repeat(64));assert.notEqual(result.status,0);assert.match(result.stderr,/binding hash or size mismatch/);
 }finally{f.close();}
});
test('a correctly hashed binding cannot redirect the fixture outside private scenarios',()=>{
 const f=fixture();try {
 f.binding.fixtureRoot=repository;const text=JSON.stringify(f.binding);writeFileSync(f.path,text);
 const result=read(f,hash(text));assert.notEqual(result.status,0);assert.match(result.stderr,/containment required/);
 }finally{f.close();}
});
test('a changed presentation agent is refused without modifying the binding',()=>{
 const f=fixture();try {
 writeFileSync(f.binding.presentationAgentPath,'changed input');
 const result=read(f);assert.notEqual(result.status,0);assert.match(result.stderr,/agent binding changed/);
 }finally{f.close();}
});
test('unknown binding fields cannot introduce additional authority',()=>{
 const f=fixture();try {
 const text=JSON.stringify({...f.binding,skipProtection:true});writeFileSync(f.path,text);
 const result=read(f,hash(text));assert.notEqual(result.status,0);assert.match(result.stderr,/Closed fixture input schema/);
 }finally{f.close();}
});


test('runtime validation uses the admitted manifest digest before trusting manifest rows',()=>{
 const f=fixture();try {
 writeFileSync(resolve(f.root,'runtime-artifacts.private.json'),'{}');
 const result=spawnSync(powershell,['-NoProfile','-NonInteractive','-File',
 resolve(repository,'tools/starsector-proxy/fixture/protection-check.ps1'),
 '-Phase','validate-runtime','-BindingPath',f.path,'-BindingSha256',f.digest],
 {cwd:repository,encoding:'utf8',timeout:15000});
 assert.notEqual(result.status,0);assert.match(result.stderr,/Qualified runtime manifest binding changed/);
 }finally{f.close();}
});


test('native Windows source paths still refuse an already running source installation',()=>{
 const f=fixture();try {
 const separator=Math.max(powershell.lastIndexOf('/'),powershell.lastIndexOf('\\'));
 f.binding.sourceInstallation=powershell.slice(0,separator).replaceAll('/','\\');
 const text=JSON.stringify(f.binding);writeFileSync(f.path,text);
 writeFileSync(resolve(f.root,'plan.json'),'{}');
 const result=spawnSync(powershell,['-NoProfile','-NonInteractive','-File',
 resolve(repository,'tools/starsector-proxy/fixture/prepare-fixture.ps1'),
 '-BindingPath',f.path,'-BindingSha256',hash(text)],
 {cwd:repository,encoding:'utf8',timeout:15000});
 assert.notEqual(result.status,0);assert.match(result.stderr,/Game installation has live processes; refuse preparation/);
 }finally{f.close();}
});

