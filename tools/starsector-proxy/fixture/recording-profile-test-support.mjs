// @ts-check
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync,existsSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {resolve,relative,isAbsolute,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';

export const repository=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
const scenarios=resolve(repository,'.tmp/starsector-proxy/scenarios');
export const quote=(/** @type {string} */ s)=>"'"+s.replaceAll("'","''")+"'";
export const hash=(/** @type {string} */ s)=>createHash('sha256').update(s).digest('hex').toUpperCase();
export const targets=['jdk.ClassLoad','jdk.JavaExceptionThrow','jdk.JavaErrorThrow'];
export const defaultJfc='<configuration version="2.0" label="Default" description="default"><event name="jdk.JVMInformation"><setting name="enabled" control="x">true</setting></event><event name="jdk.ActiveRecording"><setting name="enabled">true</setting></event><event name="jdk.ActiveSetting"><setting name="enabled">true</setting></event><event name="jdk.DataLoss"><setting name="enabled">true</setting></event><event name="jdk.ClassLoad"><setting name="enabled">false</setting><setting name="stackTrace">true</setting><setting name="threshold">0 ms</setting></event><event name="jdk.JavaExceptionThrow"><setting name="enabled">false</setting><setting name="stackTrace">true</setting></event><event name="jdk.JavaErrorThrow"><setting name="enabled">false</setting><setting name="stackTrace">true</setting></event><event name="jdk.Other"><setting name="enabled">true</setting><setting name="period">everyChunk</setting></event></configuration>';
// Fixed independent compatibility oracle: not generated with the production recipe.
export const goldenV1='<configuration version="2.0" label="NF99 disposable campaign lifecycle" description="Private read-only lifecycle binding; no performance or full campaign acceptance."><event name="jdk.JVMInformation"><setting name="enabled">true</setting></event><event name="jdk.ActiveRecording"><setting name="enabled">true</setting></event><event name="jdk.ActiveSetting"><setting name="enabled">true</setting></event><event name="jdk.DataLoss"><setting name="enabled">true</setting></event><event name="jdk.ClassLoad"><setting name="enabled">false</setting><setting name="stackTrace">true</setting><setting name="threshold">0 ms</setting></event><event name="jdk.JavaExceptionThrow"><setting name="enabled">false</setting><setting name="stackTrace">true</setting></event><event name="jdk.JavaErrorThrow"><setting name="enabled">false</setting><setting name="stackTrace">true</setting></event><event name="jdk.Other"><setting name="enabled">false</setting><setting name="period">everyChunk</setting></event><event name="nf.PresentInterval"><setting name="enabled">true</setting><setting name="stackTrace">false</setting></event><event name="nf.FrameProbeRejected"><setting name="enabled">true</setting><setting name="stackTrace">false</setting></event><event name="nf.ProbeConfiguration"><setting name="enabled">true</setting><setting name="stackTrace">false</setting></event><event name="nf.LocalLifecycle"><setting name="enabled">true</setting><setting name="stackTrace">false</setting></event><event name="nf.DiagnosticHandoffDelay"><setting name="enabled">true</setting><setting name="stackTrace">false</setting><setting name="threshold">0 ms</setting></event><event name="nf.DiagnosticStaleMailbox"><setting name="enabled">true</setting><setting name="stackTrace">false</setting></event></configuration>';

/** Declarative ordinary files only; no game, JVM, native executable or vault.
 * @param {1|2} [version] */
export function recordingFixture(version=2){
 mkdirSync(scenarios,{recursive:true});
 const root=mkdtempSync(resolve(scenarios,'recording-profile-test-'));
 const source=resolve(root,'source'),runtime=resolve(root,'runtime'),agent=resolve(root,'agent.jar');
 mkdirSync(runtime);mkdirSync(resolve(source,'jre/lib/jfr'),{recursive:true});
 writeFileSync(resolve(source,'jre/lib/jfr/default.jfc'),defaultJfc);
 writeFileSync(resolve(source,'jre/release'),'JAVA_VERSION="test-only"');writeFileSync(agent,'not an executable agent');
 const profile={schemaVersion:1,kind:'NATIVE99_JFR_STARTUP_PROFILE_V1',id:'CLASSLOAD_EXCEPTIONS_V1',
  baseDefaultJfcSHA256:hash(defaultJfc),gameJvmReleaseSHA256:hash('JAVA_VERSION="test-only"'),
  enabledEvents:[...targets],stackTrace:true,classLoadThresholdMs:0,recordingDurationSeconds:420,recordingMaxSizeMiB:64};
 const profilePath=resolve(root,'recording-profile.private.json'),bindingPath=resolve(root,'fixture-binding.private.json');
 const binding={schemaVersion:version,fixtureRoot:root,sourceInstallation:source,qualifiedRuntimeRoot:runtime,
  candidateHead:'1234567890123456789012345678901234567890',runtimeManifestSha256:'A'.repeat(64),
  presentationAgentPath:agent,presentationAgentSha256:hash('not an executable agent'),presentationClassSha256:'B'.repeat(64),driverRole:'/test/profile',
  ...(version===2?{recordingProfileSha256:''}:{})};
 const refresh=()=>{writeFileSync(profilePath,JSON.stringify(profile));if(version===2)binding.recordingProfileSha256=hash(readFileSync(profilePath,'utf8'));writeFileSync(bindingPath,JSON.stringify(binding));};
 refresh();
 const sourcePath=resolve(repository,'tools/starsector-proxy/fixture/fixture-inputs.ps1');
 /** @param {string} [body] */
 function invoke(body='$b|ConvertTo-Json -Depth 8'){
  return spawnSync(process.env.NF_TEST_POWERSHELL??'pwsh',['-NoProfile','-NonInteractive','-Command',
   "$ErrorActionPreference='Stop'; . "+quote(sourcePath)+"; $b=Read-NativeFixtureBinding "+quote(bindingPath)+' '+quote(hash(readFileSync(bindingPath,'utf8')))+'; '+body],
   {cwd:repository,encoding:'utf8',timeout:15000});
 }
 /** @param {string} [body] */
 function recipe(body='[ordered]@{xml=$jfc.OuterXml;enabled=@($jfc.configuration.event|Where-Object {$_.SelectSingleNode("setting[@name=\'enabled\']").InnerText-ceq\'true\'}|ForEach-Object {$_.name})}|ConvertTo-Json -Depth 5'){
  return invoke('[xml]$base=[IO.File]::ReadAllText('+quote(resolve(source,'jre/lib/jfr/default.jfc'))+'); $p=if($b.schemaVersion-eq2){$b.recordingProfile}else{$null}; $jfc=New-NativeRecordingConfiguration $base $p; '+body);
 }
 const flag='-XX:StartFlightRecording=name=NF99,settings='+root+'/capture/active.jfc,duration=420s,maxsize=64m,dumponexit=true,filename='+root+'/capture/current.jfr';
 /** Test data installation, never a JVM/application execution. */
 function stage(){
  const xml=recipe('$jfc.OuterXml');assert.equal(xml.status,0,xml.stderr);
  for(const [name,text] of [['capture/active.jfc',xml.stdout.trim()],['installation/vmparams','-Xmx1g '+flag+' -classpath test'],
   ['installation/jre/lib/jfr/default.jfc',defaultJfc],['installation/jre/release','JAVA_VERSION="test-only"']]){
   const path=resolve(root,name);mkdirSync(dirname(path),{recursive:true});writeFileSync(path,text);
  }
 }
 return {root,source,profile,profilePath,binding,bindingPath,refresh,invoke,recipe,stage,flag,
  assertNoInstallation(){assert.equal(existsSync(resolve(root,'installation')),false);},
  close(){const suffix=relative(scenarios,root);assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix));rmSync(root,{recursive:true});}
 };
}

