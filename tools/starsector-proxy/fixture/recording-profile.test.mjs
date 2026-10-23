// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,unlinkSync,mkdirSync,symlinkSync,lstatSync} from 'node:fs';
import {resolve,relative,isAbsolute} from 'node:path';
import {recordingFixture,targets,goldenV1,hash,quote} from './recording-profile-test-support.mjs';

// Compatibility check after helper extraction, not a manufactured pre-change RED.
test('legacy no-profile recording retains the complete fixed lifecycle recipe',()=>{
 const f=recordingFixture(1);try{
  const admitted=f.invoke();assert.equal(admitted.status,0,admitted.stderr);
  assert.equal(Object.hasOwn(JSON.parse(admitted.stdout),'recordingProfile'),false);
  const result=f.recipe();assert.equal(result.status,0,result.stderr);
  assert.equal(JSON.parse(result.stdout).xml,goldenV1);
 }finally{f.close();}
});

// First behavior RED: old public binding boundary rejects this supported v2 profile.
test('a separately pinned startup profile enables exactly three existing events',()=>{
 const f=recordingFixture();try{
  const result=f.recipe();assert.equal(result.status,0,result.stderr);
  const value=JSON.parse(result.stdout);
  assert.deepEqual(value.enabled,['jdk.JVMInformation','jdk.ActiveRecording','jdk.ActiveSetting','jdk.DataLoss',...targets,
   'nf.PresentInterval','nf.FrameProbeRejected','nf.ProbeConfiguration','nf.LocalLifecycle','nf.DiagnosticHandoffDelay','nf.DiagnosticStaleMailbox']);
  let inverse=value.xml;
  for(const name of targets){
   const enabled='<event name="'+name+'"><setting name="enabled">true</setting>';
   assert.equal(inverse.split(enabled).length,2,name);
   inverse=inverse.replace(enabled,'<event name="'+name+'"><setting name="enabled">false</setting>');
  }
  assert.equal(inverse,goldenV1,'All remaining attributes, settings and custom events remain identical');
  f.assertNoInstallation();
 }finally{f.close();}
});

test('v1 fields cannot opt into a recording profile',()=>{
 const f=recordingFixture(1);try{
  Object.assign(f.binding,{recordingProfileSha256:'A'.repeat(64)});f.refresh();
  const result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/Closed fixture input schema/);
  f.assertNoInstallation();
 }finally{f.close();}
});

test('v2 requires its recording profile digest field',()=>{
 const f=recordingFixture();try{
  Reflect.deleteProperty(f.binding,'recordingProfileSha256');writeFileSync(f.bindingPath,JSON.stringify(f.binding));
  const result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/Closed fixture input schema/);f.assertNoInstallation();
 }finally{f.close();}
});

for(const field of ['kind','id'])for(const value of [null,17,{},'singleton-array'])test('profile '+field+' refuses '+JSON.stringify(value)+' as a non-string',()=>{
 const f=recordingFixture();try{
  const wrong=value==='singleton-array'?[field==='kind'?'NATIVE99_JFR_STARTUP_PROFILE_V1':'CLASSLOAD_EXCEPTIONS_V1']:value;
  const text=JSON.stringify({...f.profile,[field]:wrong});writeFileSync(f.profilePath,text);
  f.binding.recordingProfileSha256=hash(text);writeFileSync(f.bindingPath,JSON.stringify(f.binding));
  const result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/Fixed startup recording profile/);f.assertNoInstallation();
 }finally{f.close();}
});

test('a junction at the fixed profile path is refused and only that contained link is removed',()=>{
 const f=recordingFixture();const target=resolve(f.root,'junction-target'),marker=resolve(target,'marker.txt');
 try{
  for(const path of [target,f.profilePath]){const suffix=relative(f.root,path);assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix));}
  mkdirSync(target);writeFileSync(marker,'owned junction target remains unchanged');unlinkSync(f.profilePath);
  symlinkSync(target,f.profilePath,'junction');assert.equal(lstatSync(f.profilePath).isSymbolicLink(),true);
  const result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/Ordinary recording profile input/);f.assertNoInstallation();
 }finally{
  if(lstatSync(f.profilePath,{throwIfNoEntry:false})?.isSymbolicLink())unlinkSync(f.profilePath);
  assert.equal(readFileSync(marker,'utf8'),'owned junction target remains unchanged');f.close();
 }
});

/** @type {[string,(f:ReturnType<typeof recordingFixture>)=>void,RegExp][]} */
const invalidProfiles=[
 ['duplicate event',f=>{f.profile.enabledEvents=['jdk.ClassLoad','jdk.ClassLoad','jdk.JavaErrorThrow'];},/unique recording events/],
 ['unknown event',f=>{f.profile.enabledEvents[2]='jdk.ThreadStart';},/unique recording events/],
 ['changed duration',f=>{f.profile.recordingDurationSeconds=421;},/Fixed recording profile bounds/],
 ['changed size',f=>{f.profile.recordingMaxSizeMiB=65;},/Fixed recording profile bounds/],
 ['disabled stack',f=>{f.profile.stackTrace=false;},/Fixed startup recording profile/],
 ['changed threshold',f=>{f.profile.classLoadThresholdMs=1;},/Fixed recording profile bounds/],
 ['unknown profile',f=>{f.profile.id='OTHER';},/Fixed startup recording profile/],
 ['changed JVM pin',f=>{f.profile.gameJvmReleaseSHA256='F'.repeat(64);},/source JVM or base JFC changed/],
 ['changed base pin',f=>{f.profile.baseDefaultJfcSHA256='F'.repeat(64);},/source JVM or base JFC changed/],
];
for(const [name,mutate,reason] of invalidProfiles)test('profile refuses '+name+' before fixture effects',()=>{
 const f=recordingFixture();try{mutate(f);f.refresh();const result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,reason);f.assertNoInstallation();}finally{f.close();}
});

test('changed profile bytes and duplicate JSON fields refuse even with a refreshed outer binding',()=>{
 const f=recordingFixture();try{
  writeFileSync(f.profilePath,readFileSync(f.profilePath,'utf8')+' ');
  let result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/profile hash or size mismatch/);
  const duplicate=JSON.stringify(f.profile).replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1');
  writeFileSync(f.profilePath,duplicate);f.binding.recordingProfileSha256=hash(duplicate);writeFileSync(f.bindingPath,JSON.stringify(f.binding));
  result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/Duplicate recording profile field/);
  f.assertNoInstallation();
 }finally{f.close();}
});

test('closed profile rejects unknown fields',()=>{
 const f=recordingFixture();try{
  writeFileSync(f.profilePath,JSON.stringify({...f.profile,settingsPath:'outside.jfc'}));
  f.binding.recordingProfileSha256=hash(readFileSync(f.profilePath,'utf8'));writeFileSync(f.bindingPath,JSON.stringify(f.binding));
  const result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/Closed recording profile/);f.assertNoInstallation();
 }finally{f.close();}
});

test('missing or oversized fixed profile refuses before fixture effects',()=>{
 const f=recordingFixture();try{
  unlinkSync(f.profilePath);let result=f.invoke();assert.notEqual(result.status,0);f.assertNoInstallation();
  writeFileSync(f.profilePath,' '.repeat(16385));f.binding.recordingProfileSha256=hash(readFileSync(f.profilePath,'utf8'));writeFileSync(f.bindingPath,JSON.stringify(f.binding));
  result=f.invoke();assert.notEqual(result.status,0);assert.match(result.stderr,/profile hash or size mismatch/);f.assertNoInstallation();
 }finally{f.close();}
});

test('staged recording JVM cannot differ from its separately pinned source',()=>{
 const f=recordingFixture();try{
  f.stage();writeFileSync(resolve(f.root,'installation/jre/release'),'different JVM');
  const result=f.invoke('Assert-NativeRecordingRecipe $b ([pscustomobject]@{recordingProfileSha256=$b.recordingProfileSha256})');
  assert.notEqual(result.status,0);assert.match(result.stderr,/Staged recording JVM or base JFC changed/);
 }finally{f.close();}
});

test('expected recording recipe refuses changed XML even after its receipt digest is refreshed',()=>{
 const f=recordingFixture();try{
  f.stage();
  const receipt='$r=[pscustomobject]@{recordingProfileSha256=$b.recordingProfileSha256;jfcSha256=(Get-FileHash -LiteralPath '+quote(resolve(f.root,'capture/active.jfc'))+' -Algorithm SHA256).Hash}; ';
  let result=f.invoke(receipt+'Assert-NativeRecordingRecipe $b $r');assert.equal(result.status,0,result.stderr);
  const path=resolve(f.root,'capture/active.jfc');writeFileSync(path,readFileSync(path,'utf8').replace('<setting name="threshold">0 ms</setting>','<setting name="threshold">1 ms</setting>'));
  result=f.invoke(receipt+'Assert-NativeRecordingRecipe $b $r');assert.notEqual(result.status,0);assert.match(result.stderr,/complete recording configuration/);
 }finally{f.close();}
});

test('recording flag remains exactly one private 420s 64m declaration',()=>{
 const f=recordingFixture();try{
  f.stage();
  const body='Assert-NativeRecordingRecipe $b ([pscustomobject]@{recordingProfileSha256=$b.recordingProfileSha256})';
  for(const flags of [f.flag.replace('duration=420s','duration=421s'),f.flag.replace('maxsize=64m','maxsize=65m'),f.flag+' '+f.flag,
   f.flag.replace(f.root+'/capture/current.jfr','C:/foreign/current.jfr'),
   f.flag+' "-XX:StartFlightRecording=name=Other,duration=999s,filename=C:/foreign.jfr"',
   '"'+f.flag+'"','prefix'+f.flag,f.flag.replace('-XX:StartFlightRecording=','-XX:StartFlightRecordingExtra=')]){
   writeFileSync(resolve(f.root,'installation/vmparams'),'-Xmx1g '+flags+' -classpath test');
   const result=f.invoke(body);assert.notEqual(result.status,0);assert.match(result.stderr,/Fixed private recording flag/);
  }
 }finally{f.close();}
});

test('profile agreement requires binding receipt plan and lease together',()=>{
 const f=recordingFixture();try{
  const body='$same=[pscustomobject]@{recordingProfileSha256=$b.recordingProfileSha256}; ';
  let result=f.invoke(body+'Assert-NativeRecordingAuthority $b $same $same $same');assert.equal(result.status,0,result.stderr);
  for(const position of [0,1,2]){
   const args=['$same','$same','$same'];args[position]='([pscustomobject]@{})';
   result=f.invoke(body+'Assert-NativeRecordingAuthority $b '+args.join(' '));assert.notEqual(result.status,0);assert.match(result.stderr,/receipt plan and lease must agree/);
  }
  f.assertNoInstallation();
 }finally{f.close();}
});

test('legacy receipt plan and lease cannot carry any recording profile field',()=>{
 const f=recordingFixture(1);try{
  const body='$plain=[pscustomobject]@{}; $profile=[pscustomobject]@{recordingProfileSha256=$null}; ';
  let result=f.invoke(body+'Assert-NativeRecordingAuthority $b $plain $plain $plain');assert.equal(result.status,0,result.stderr);
  for(const position of [0,1,2]){const args=['$plain','$plain','$plain'];args[position]='$profile';
   result=f.invoke(body+'Assert-NativeRecordingAuthority $b '+args.join(' '));assert.notEqual(result.status,0);assert.match(result.stderr,/Legacy fixture cannot carry/);
  }
  f.assertNoInstallation();
 }finally{f.close();}
});

