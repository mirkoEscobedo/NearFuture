// @ts-check
// Generated boundary data only. These receipts do not represent an actual game or runtime qualification.
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {spawnSync} from 'node:child_process';
import {resolve,relative,isAbsolute,basename,dirname} from 'node:path';
import {fileURLToPath} from 'node:url';

export const repository=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
const scenarios=resolve(repository,'.tmp/starsector-proxy/scenarios');
export const quote=(/** @type {string} */ s)=>"'"+s.replaceAll("'","''")+"'";
export const hash=(/** @type {string|Buffer} */ data)=>createHash('sha256').update(data).digest('hex').toUpperCase();
export const filePin=(/** @type {string} */ path)=>({bytes:readFileSync(path).length,sha256:hash(readFileSync(path))});
export function write(/** @type {string} */ path,/** @type {string} */ data){mkdirSync(dirname(path),{recursive:true});writeFileSync(path,data);return filePin(path);}
export function json(/** @type {string} */ path,/** @type {unknown} */ value){return write(path,JSON.stringify(value));}
export function powershell(/** @type {string[]} */ args){return spawnSync(process.env.NF_TEST_POWERSHELL??'pwsh',['-NoProfile','-NonInteractive',...args],{cwd:repository,encoding:'utf8',timeout:15000});}

export function seedFixture(){
 mkdirSync(scenarios,{recursive:true});
 const source=mkdtempSync(resolve(scenarios,'seed-source-test-'));
 const evidence=mkdtempSync(resolve(scenarios,'seed-evidence-test-'));
 const target=mkdtempSync(resolve(scenarios,'seed-target-test-'));
 const admission=mkdtempSync(resolve(scenarios,'seed-admission-test-'));
 const original=resolve(target,'original'),runtime=resolve(target,'runtime'),installation=resolve(target,'installation');
 const saveDirectory='save_fixture_1',sourceSave=resolve(source,'installation/saves',saveDirectory);
 const campaign='<CampaignEngine><saveDirName>save_fixture_1</saveDirName><playerFleet ref="7"/><characterData><level>1</level></characterData><saveDesc/><Flt z="7"><fleet/></Flt></CampaignEngine>';
 const descriptor='<SaveGameData><compressed>false</compressed><gameVersion>0.98a-RC8</gameVersion><enabledMods/></SaveGameData>';
 const sourceFiles=[{relative:'campaign.xml',...write(resolve(sourceSave,'campaign.xml'),campaign)},{relative:'descriptor.xml',...write(resolve(sourceSave,'descriptor.xml'),descriptor)}];
 const sourceReceipt=resolve(source,'protected-fixture-receipt.private.json');
 json(sourceReceipt,{schemaVersion:1,phase:'FROZEN_PREPARATION_ONLY',installation:resolve(source,'installation'),privateSaveRoot:resolve(source,'installation/saves'),privateSaveRootEmpty:true,nativeLaunched:false,gameLaunched:false,originalWrites:false,backupVerified:true,originalFilesVerified:182,originalSaveFiles:178,logBaseline:{existed:false,bytes:0}});
 const sourceProtection={phase:'after-smoke',protectedOriginalFiles:182,protectedSaveFiles:178,allOriginalAndBackupHashesMatch:true,saveCensusUnchanged:true,liveGameProcesses:0,originalWrites:false,privateAutosaveEnabled:false,privateSaveEntries:1,originalAutosave:{enabled:true,maximumSlots:3}};
 const sourceProtectionPath=resolve(evidence,'after-smoke-protection.private.json');json(sourceProtectionPath,sourceProtection);
 const sourceLaunchPath=resolve(evidence,'launch-supervisor.private.json');json(sourceLaunchPath,{schemaVersion:1,command:'verify',ok:true,result:{verdict:'FAIL',steps:[{status:1,timedOut:false,processOwnership:'windows-job-object'}]}});
 const sourceRootPath=resolve(evidence,'root-call.actual.private.json');json(sourceRootPath,{key:'native7ActualRootCall',value:{startMs:123456789,args:{cmd:'generated test data only'},raw:{session_id:1234}}});
 const closeoutPath=resolve(evidence,'driver-closeout.actual7.private.json');
 const closeout={schemaVersion:1,kind:'ACTUAL7_DRIVER_FINAL_CLOSEOUT',scenarioId:basename(source),status:'CLOSED_FAIL_NO_NATIVE_RESUME',history:{overall:7,rootSessionId:1234},cleanup:{verdict:'FAIL',status:1,timedOut:false,processOwnership:'windows-job-object',zeroLiveGameProcesses:true},protection:sourceProtection,evidence:{fixtureReceipt:sourceReceipt,launchSupervisor:sourceLaunchPath,launchSupervisorSHA256:filePin(sourceLaunchPath).sha256,protection:sourceProtectionPath,protectionSHA256:filePin(sourceProtectionPath).sha256}};
 json(closeoutPath,closeout);
 const provenance={fixtureReceipt:filePin(sourceReceipt),rootCall:filePin(sourceRootPath),closeout:filePin(closeoutPath),protection:filePin(sourceProtectionPath),launch:filePin(sourceLaunchPath)};
 const manifest={schemaVersion:1,kind:'NATIVE99_DISPOSABLE_SEED_MANIFEST_V1',sourceScenario:basename(source),sourceEvidenceScenario:basename(evidence),sourceRun:7,saveDirectory,gameVersion:'0.98a-RC8',files:sourceFiles,provenance};

 const settings='{"maxAutosaveSlots":3,"autosaveFeatureEnabled":true,"autosaveEnabledByDefault":true}';
 const originals=[['starsector-core/data/config/settings.json',settings],...Array.from({length:3},(_,i)=>['metadata-'+i+'.txt','original-'+i]),...Array.from({length:178},(_,i)=>['saves/item-'+i+'.txt','original-save-'+i])];
 const snapshot=originals.map(([name,text])=>{const path=resolve(original,name),pin=write(path,text);write(resolve(target,'original-backup',name),text);return {relative:relative(original,path),...pin};});
 json(resolve(target,'protected-original-snapshot.private.json'),snapshot);
 const jarNames=['adapter-0.1.0.jar','bcprov-jdk18on-1.81.jar','contract-0.1.0.jar','ipc-0.1.0.jar','protobuf-java-4.31.1.jar','wire-0.1.0.jar'];
 const artifacts=jarNames.map(name=>{const path=resolve(runtime,'jars',name),pin=write(path,'synthetic-jar-'+name);write(resolve(installation,'mods/near_future/jars',name),'synthetic-jar-'+name);return {relative:'mods/near_future/jars/'+name,source:path,...pin};});
 const nodePath=resolve(runtime,'diagnostic/nf-ipc-node.exe'),nodePin=write(nodePath,'synthetic-node-not-executable');write(resolve(installation,'diagnostic/nf-ipc-node.exe'),'synthetic-node-not-executable');artifacts.push({relative:'diagnostic/nf-ipc-node.exe',source:nodePath,...nodePin});
 const candidateHead='1234567890123456789012345678901234567890';
 const runtimePin=json(resolve(target,'runtime-artifacts.private.json'),{schemaVersion:1,kind:'NATIVE99_CAMPAIGN_BINDING_RUNTIME_INPUTS',candidateHead,artifacts});
 const agentPath=resolve(target,'input-agent.jar'),agentPin=write(agentPath,'synthetic-agent');
 write(resolve(target,'agent/nf-presentation-agent.jar'),'synthetic-agent');
 const binding={schemaVersion:1,fixtureRoot:target,sourceInstallation:original,qualifiedRuntimeRoot:runtime,candidateHead,runtimeManifestSha256:runtimePin.sha256,presentationAgentPath:agentPath,presentationAgentSha256:agentPin.sha256,presentationClassSha256:'B'.repeat(64),driverRole:'/test/seed-driver'};
 const bindingPath=resolve(target,'fixture-binding.private.json'),bindingPin=json(bindingPath,binding);
 const pins={
  classSha256:binding.presentationClassSha256,agentSha256:agentPin.sha256,runtimeManifestSha256:runtimePin.sha256,
  modInfoSha256:json(resolve(installation,'mods/near_future/mod_info.json'),{gameVersion:'0.98a-RC8'}).sha256,
  enabledModsSha256:json(resolve(installation,'mods/enabled_mods.json'),{enabledMods:['near_future']}).sha256,
  settingsSha256:write(resolve(installation,'starsector-core/data/config/settings.json'),'{"autosaveFeatureEnabled":false,"autosaveEnabledByDefault":false}').sha256,
  vaultInitializationSha256:json(resolve(target,'vault-initialization.private.json'),{status:0,command:'diagnostic/nf-ipc-node.exe',scope:'OWNER_PRIVATE_VAULT_INITIALIZATION_ONLY'}).sha256,
  vmparamsSha256:write(resolve(installation,'vmparams'),'synthetic vmparams').sha256,
  jfcSha256:write(resolve(target,'capture/active.jfc'),'synthetic jfc').sha256
 };
 mkdirSync(resolve(installation,'saves'),{recursive:true});mkdirSync(resolve(target,'diagnostic-vault'),{recursive:true});
 const frozenPath=resolve(target,'protected-fixture-receipt.private.json');
 json(frozenPath,{schemaVersion:1,phase:'FROZEN_PREPARATION_ONLY',installation,originalFilesVerified:182,originalSaveFiles:178,originalWrites:false,backupVerified:true,privateAutosaveEnabled:false,privateSaveRoot:resolve(installation,'saves'),privateSaveRootEmpty:true,nativeLaunched:false,gameLaunched:false,nodeVaultInitialized:true,nativeAcceptance:'UNMEASURED',logBaseline:{path:resolve(installation,'starsector-core/starsector.log'),existed:false,bytes:0},originalAutosave:{enabledByDefault:true,featureEnabled:true,maximumSlots:3},...pins});
 const sourcePinPath=resolve(target,'source-pins.private.json');
 json(sourcePinPath,{pins:['fixture-inputs.ps1','seed-fixture.ps1','prepare-fixture.ps1','protection-check.ps1'].map(name=>{const path=resolve(repository,'tools/starsector-proxy/fixture',name);return {path,...filePin(path)};})});
 const manifestPath=resolve(target,'seed-manifest.private.json'),authorityPath=resolve(target,'seed-stage-authority.private.json');
 function refresh(){
  const manifestPin=json(manifestPath,manifest);
  const planPin=json(resolve(target,'seed-stage-plan.private.json'),{seedStage:{mode:'STAGE_ONE_PRIVATE_DISPOSABLE_SEED',manifestSha256:manifestPin.sha256}});
  return json(authorityPath,{schemaVersion:1,kind:'NATIVE99_SEED_STAGE_AUTHORITY_V1',authorized:true,scenarioId:basename(target),planSha256:planPin.sha256,fixtureReceiptSha256:filePin(frozenPath).sha256,sourcePinsSha256:filePin(sourcePinPath).sha256,manifestSha256:manifestPin.sha256});
 }
 refresh();
 function stage(){return powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/prepare-fixture.ps1'),'-BindingPath',bindingPath,'-BindingSha256',bindingPin.sha256,'-StageSeedAuthorityPath',authorityPath,'-StageSeedAuthoritySha256',filePin(authorityPath).sha256]);}
 /** @param {'both'|'plan'|'lease'|'neither'} which */
 function launch(which='both'){
  const receiptPath=resolve(target,'seed-stage-receipt.private.json');
  const spec=which==='neither'?null:{mode:'LOAD_ONE_PRIVATE_DISPOSABLE_SEED',manifestSha256:filePin(manifestPath).sha256,stagingReceiptSha256:filePin(receiptPath).sha256};
  const plan={ownership:{driver:binding.driverRole,onlyGameInputDriver:true},...((which==='both'||which==='plan')?{seed:spec}:{})};
  const planPin=json(resolve(admission,'plan.json'),plan);
  json(resolve(admission,'native-lane-lease.private.json'),{schemaVersion:1,scenarioId:basename(target),authorized:true,driver:binding.driverRole,phase:'CAMPAIGN_BINDING_ONLY',planSha256:planPin.sha256,fixtureReceiptSha256:filePin(frozenPath).sha256,...((which==='both'||which==='lease')?{seed:spec}:{})});
 }
 function admit(phase='admit-launch'){return powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/protection-check.ps1'),'-BindingPath',bindingPath,'-BindingSha256',bindingPin.sha256,'-Phase',phase,'-EvidenceRoot',admission]);}
 return {source,evidence,target,admission,original,installation,sourceSave,saveDirectory,campaign,descriptor,manifest,manifestPath,authorityPath,bindingPath,bindingPin,frozenPath,closeout,closeoutPath,sourceRootPath,refresh,stage,launch,admit,
 close(){for(const root of [admission,target,evidence,source]){const suffix=relative(scenarios,root);if(!suffix||suffix.startsWith('..')||isAbsolute(suffix))throw Error('Test cleanup escaped scenarios');rmSync(root,{recursive:true});}}};
}
