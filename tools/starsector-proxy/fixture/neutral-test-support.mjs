// @ts-check
// All artifacts/seed XML below are synthetic. Never native/runtime qualification.
import {mkdtempSync,mkdirSync,writeFileSync,readFileSync,rmSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve,dirname,relative,isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';
import {neutralMods,neutralProfile} from './neutral-recipe.mjs';

export const repository=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
/** @param {Buffer|string} value */
export const hash=value=>createHash('sha256').update(value).digest('hex');
/** @param {string} root @param {string} path @param {Buffer|string} data */
export function write(root,path,data){const full=resolve(root,path);mkdirSync(dirname(full),{recursive:true});writeFileSync(full,data);return {path,bytes:Buffer.byteLength(data),sha256:hash(data)};}
/** @param {string} root @param {string} path @param {any} value */
export function json(root,path,value){return write(root,path,JSON.stringify(value));}

export function neutralFixture(){
    const unit=resolve(repository,'.tmp/starsector-proxy/neutral-unit'),scenarios=resolve(repository,'.tmp/starsector-proxy/scenarios');mkdirSync(unit,{recursive:true});mkdirSync(scenarios,{recursive:true});
    const root=mkdtempSync(resolve(unit,'case-')),scenario=mkdtempSync(resolve(scenarios,'neutral-recipe-test-'));
    const source=resolve(root,'source'),pkg=resolve(root,'package'),host=resolve(root,'host');for(const p of [source,pkg,host])mkdirSync(p);
    const jar=Buffer.alloc(22);jar.writeUInt32LE(0x06054b50);
    const core=write(source,'starsector-core/starfarer.api.jar','synthetic ToT patch, NOT native');
    const release=write(source,'jre/release','JAVA_VERSION="17.0.10"\nJAVA_RUNTIME_VERSION="17.0.10+7-LTS"\nIMPLEMENTOR="Azul Systems, Inc."\nOS_NAME="Windows"\nOS_ARCH="x86_64"\n');
    const java=write(source,'jre/bin/java.exe','not executable'),jvm=write(source,'jre/bin/server/jvm.dll','not a JVM');
    const mods=Object.entries(neutralMods).map(([id,spec])=>{
        const directory='mods/'+id;
        const metadata=json(source,directory+'/mod_info.json',{id,version:spec.version,jars:['jars/main.jar'],dependencies:spec.dependencies.map(id=>({id}))});
        const jars=[write(source,directory+'/jars/main.jar',jar)];return {id,version:spec.version,directory,metadata,jars,dependencies:spec.dependencies};
    });
    const tot=write(source,'mods/aotd_theory_of_toolbox/0.98a/starfarer.api.jar','synthetic ToT patch, NOT native');
    const adapter=write(pkg,'jars/near-future-adapter.jar',jar),probe=write(pkg,'jars/near-future-probe.jar',jar);
    const modInfo=json(pkg,'mod_info.json',{id:'near_future_probe',version:'0.1.0',gameVersion:'0.98a-RC8',modPlugin:'nf.adapter.probe.ProbeModPlugin',jars:[adapter.path,probe.path]});
    const metadata={formatVersion:1,packageVersion:'0.1.0',protocol:'neutral-probe-v1-ack32',javaTarget:17,compiler:'21.0.8',gameRequirement:'0.98a-RC8',enabledByDefault:false,nativeValidated:false,sidecarBundled:false,maxRequestsPerBinding:8,maxOutstanding:1,artifacts:{[modInfo.path]:modInfo.sha256,[adapter.path]:adapter.sha256,[probe.path]:probe.sha256}};
    const manifest=json(pkg,'probe-package.json',metadata);
    const node=write(host,'nf-sidecar.exe','synthetic host NOT executable');
    const vm='java.exe -noverify -XX:-BytecodeVerificationLocal -XX:-BytecodeVerificationRemote -Dcom.fs.starfarer.settings.paths.saves=../saves -Dcom.fs.starfarer.settings.paths.mods=../mods -Dcom.fs.starfarer.settings.paths.logs=. -Dcom.fs.starfarer.settings.paths.screenshots=../screenshots -classpath starfarer.api.jar launcher.Main';
    const settings='{\n"maxAutosaveSlots":3, # retain original3\n"autosaveFeatureEnabled":true,\n"autosaveEnabledByDefault":true,\n"scale":.5f;"compressSaveGameData":false\n}';
    const protectedFiles=[write(source,'vmparams',vm),write(source,'mods/enabled_mods.json','{"enabledMods":["original-unrelated"]}'),write(source,'starsector-core/data/config/settings.json',settings),write(source,'starsector-core/compiler_directives.txt','original compiler configuration'),write(source,'saves/save_original/campaign.xml','protected original synthetic save')];
    const recipe={schemaVersion:1,profile:neutralProfile,gameVersion:'0.98a-RC8',protocol:'neutral-probe-v1-ack32',companionVersion:'0.1.0',sourceInstallation:source,packageRoot:pkg,hostRoot:host,scenarioRoot:scenario,runtime:{api:core,totPatch:tot,javaRelease:release,javaExecutable:java,jvmLibrary:jvm},package:{manifest,modInfo,adapter,probe},host:{artifact:node,sourceCommit:'9'.repeat(40),protocol:'neutral-probe-v1-ack32'},mods,protected:protectedFiles};
    const path=resolve(scenario,'neutral-recipe.private.json');
    const save=resolve(root,'native-created-candidate');mkdirSync(save);
    const campaign='<CampaignEngine><saveDirName>save_neutral</saveDirName><playerFleet ref="7"/><characterData><level>1</level></characterData><saveDesc/><Flt z="7"/></CampaignEngine>';
    const expected={...Object.fromEntries(Object.entries(neutralMods).map(([id,v])=>[id,v.version])),near_future_probe:'0.1.0'};
    const entries=Object.entries(expected).map(([id,v],i)=>{const [major,minor,patch]=v.split('.');return '<EnabledModData><spec z="'+(100+i)+'"><id>'+id+'</id><versionInfo><major>'+major+'</major><minor>'+minor+'</minor><patch>'+patch+'</patch><minorIsRC>false</minorIsRC></versionInfo></spec></EnabledModData>';}).join('');
    const descriptor='<SaveGameData><compressed>false</compressed><gameVersion>0.98a-RC8</gameVersion><enabledMods>'+entries+'</enabledMods></SaveGameData>';
    write(save,'campaign.xml',campaign);write(save,'descriptor.xml',descriptor);
    return {root,scenario,source,pkg,host,path,save,recipe,metadata,campaign,descriptor,vm,settings,
        saveRecipe(){writeFileSync(path,JSON.stringify(recipe));return hash(readFileSync(path));},
        close(){for(const [base,target] of [[unit,root],[scenarios,scenario]]){const p=relative(base,target);if(!p||p.startsWith('..')||isAbsolute(p))throw new Error('Cleanup escaped test root');rmSync(target,{recursive:true});}}
    };
}
