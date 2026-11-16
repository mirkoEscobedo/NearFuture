// @ts-check
// Preparation only. None of these records is a native launch/input/seed-creation authority.
import {readFileSync, readdirSync, lstatSync, realpathSync, mkdirSync, writeFileSync, copyFileSync, constants} from 'node:fs';
import {createHash} from 'node:crypto';
import {resolve, relative, isAbsolute, dirname, join, sep} from 'node:path';
import {fileURLToPath} from 'node:url';
import {parseGameJson} from '../../runtime-manifest/metadata.mjs';

export const neutralProfile = 'NEUTRAL_ACK32_TOT_NEX_V1';
/** @type {Record<string,{version:string,dependencies:string[]}>} */
export const neutralMods = Object.freeze({
    MagicLib: {version:'1.5.6',dependencies:['lw_lazylib']},
    aotd_theory_of_toolbox: {version:'1.0.11',dependencies:['ashlib','bmo','lw_lazylib']},
    ashlib: {version:'2.2.3',dependencies:['lw_lazylib']},
    bmo: {version:'2.1.0',dependencies:['ashlib']},
    lw_lazylib: {version:'3.0.0',dependencies:[]},
    nexerelin: {version:'0.12.2c',dependencies:['MagicLib','lw_lazylib']},
});
for(const spec of Object.values(neutralMods)){Object.freeze(spec.dependencies);Object.freeze(spec);}
const repository = resolve(fileURLToPath(new URL('../../../', import.meta.url)));
const scenarios = resolve(repository,'.tmp/starsector-proxy/scenarios');
const configPaths = ['vmparams','mods/enabled_mods.json','starsector-core/data/config/settings.json','starsector-core/compiler_directives.txt'];
/** @param {any} ok @param {string} message */
function need(ok,message) { if (!ok) throw new Error('Neutral recipe refused: '+message); }
/** @param {any} value @param {string[]} keys */
function closed(value,keys) { need(value && !Array.isArray(value) && typeof value==='object' && Object.keys(value).length===keys.length && Object.keys(value).sort().join()===keys.slice().sort().join(),'closed fields'); }
/** @param {string} parent @param {string} child */
function inside(parent,child) { const p=relative(parent,child); return p!==''&&!p.startsWith('..'+sep)&&p!=='..'&&!isAbsolute(p); }
/** @param {string} path @param {boolean} [missing] */
function ordinary(path,missing=false) {
    let at=resolve(path);
    while (true) {
        try { need(!lstatSync(at).isSymbolicLink(),'reparse ancestry'); }
        catch(error) { if (!missing || !(error instanceof Error && 'code' in error && error.code==='ENOENT')) throw error; }
        const next=dirname(at); if (next===at) break; at=next;
    }
}
/** @param {any} path */
function relativePath(path) { need(typeof path==='string' && path.length<=240 && /^[A-Za-z0-9 _./+-]+$/.test(path) && !path.includes('\\') && !isAbsolute(path) && path.split('/').every(p=>p!==''&&p!=='.'&&p!=='..'),'relative path'); return path; }
/** @param {string} root @param {string} path */
function contained(root,path) { const p=resolve(root,relativePath(path)); need(inside(root,p),'contained path'); ordinary(p); return p; }
/** @param {string} path @param {number} [maximum] */
function bytes(path,maximum=67_108_864) { ordinary(path); const s=lstatSync(path); need(s.isFile()&&s.size>0&&s.size<=maximum,'file bound'); return readFileSync(path); }
/** @param {string} path @param {string} label */
function pin(path,label) { const b=bytes(path); return {path:label,bytes:b.length,sha256:createHash('sha256').update(b).digest('hex')}; }
/** @param {string} root @param {any} value @param {string} [expected] */
function pinned(root,value,expected) {
    closed(value,['path','bytes','sha256']);
    const label=relativePath(value.path); need(!expected||label===expected,'fixed artifact path');
    need(Number.isSafeInteger(value.bytes)&&value.bytes>0&&value.bytes<=67_108_864&&typeof value.sha256==='string'&&/^[a-f0-9]{64}$/.test(value.sha256),'artifact pin');
    const path=contained(root,label), actual=pin(path,label);
    need(actual.bytes===value.bytes&&actual.sha256===value.sha256,'artifact bytes changed'); return path;
}
/** @param {string[]} a @param {string[]} b */
function sameSet(a,b) { return a.length===b.length&&a.every(x=>typeof x==='string'&&!x.includes('\0'))&&a.length===new Set(a).size&&a.slice().sort().join('\0')===b.slice().sort().join('\0'); }
/** Strict JSON duplicate keys are rejected at every object before JSON.parse.
 * @param {Buffer} b */
function strictJson(b) {
    const text=new TextDecoder('utf-8',{fatal:true}).decode(b).replace(/^\uFEFF/,'');
    let i=0;
    function ws(){while(/\s/.test(text[i]??'')&&i<text.length)i++;}
    function string(){const start=i++;while(i<text.length){if(text[i++]==='"')return JSON.parse(text.slice(start,i));if(text[i-1]==='\\')i++;}throw new Error('Unclosed JSON string');}
    function value(depth=0){
        need(depth<=32,'JSON depth');ws();
        if(text[i]==='{') { i++;ws();const keys=new Set();if(text[i]==='}'){i++;return;}
            while(true){ws();need(text[i]==='"','JSON key');const key=string();need(!keys.has(key),'duplicate JSON key');keys.add(key);ws();need(text[i++]===':','JSON colon');value(depth+1);ws();if(text[i]==='}'){i++;return;}need(text[i++]===',','JSON separator');}
        } else if(text[i]==='['){i++;ws();if(text[i]===']'){i++;return;}while(true){value(depth+1);ws();if(text[i]===']'){i++;return;}need(text[i++]===',','JSON separator');}}
        else if(text[i]==='"') string();
        else {const start=i;while(i<text.length&&!/[\s,}\]]/.test(text[i]))i++;need(i>start,'JSON value');}
    }
    value();ws();need(i===text.length,'JSON tail');return JSON.parse(text);
}
/** @param {any} version */
function version(version) { if(typeof version==='string')return version; closed(version,['major','minor','patch']);return [version.major,version.minor,version.patch].join('.'); }
/** Inspect the ZIP central directory only; not bytecode/loader certification.
 * @param {string} path */
function noCoreApi(path) {
    const b=bytes(path);let end=-1;
    for(let i=b.length-22;i>=Math.max(0,b.length-65557);i--)if(b.readUInt32LE(i)===0x06054b50&&i+22+b.readUInt16LE(i+20)===b.length){end=i;break;}
    need(end>=0&&b.readUInt16LE(end+4)===0&&b.readUInt16LE(end+6)===0,'ordinary single-disk JAR');
    const count=b.readUInt16LE(end+10), size=b.readUInt32LE(end+12), offset=b.readUInt32LE(end+16);
    need(count===b.readUInt16LE(end+8)&&count<65535&&size<=16_777_216&&offset+size<=end,'JAR directory bound');
    let at=offset;
    for(let n=0;n<count;n++){
        need(at+46<=offset+size&&b.readUInt32LE(at)===0x02014b50,'JAR entry');
        const length=b.readUInt16LE(at+28), extra=b.readUInt16LE(at+30), comment=b.readUInt16LE(at+32);
        need(at+46+length+extra+comment<=offset+size,'JAR entry bound');
        const name=new TextDecoder('utf-8',{fatal:true}).decode(b.subarray(at+46,at+46+length));
        need(!name.includes('\0')&&!name.includes('\\'),'ordinary JAR name');
        need(!/^(?:META-INF\/versions\/[0-9]+\/)?com\/fs\/starfarer\/api\//.test(name),'competing core API in mod JAR');at+=46+length+extra+comment;
    }
    need(at===offset+size,'JAR directory tail');
}

/** Read-only exact recipe validation. Pins establish supplied byte identity, not native support.
 * @param {string} manifestPath @param {string} expectedSha256 */
export function readNeutralRecipe(manifestPath,expectedSha256) {
    const raw=bytes(manifestPath,65_536);
    need(/^[a-f0-9]{64}$/.test(expectedSha256)&&createHash('sha256').update(raw).digest('hex')===expectedSha256,'recipe pin');
    const m=strictJson(raw);
    closed(m,['schemaVersion','profile','gameVersion','protocol','companionVersion','sourceInstallation','packageRoot','hostRoot','scenarioRoot','runtime','package','host','mods','protected']);
    need(m.schemaVersion===1&&m.profile===neutralProfile&&m.gameVersion==='0.98a-RC8'&&m.protocol==='neutral-probe-v1-ack32'&&m.companionVersion==='0.1.0','named profile');
    for(const key of ['sourceInstallation','packageRoot','hostRoot','scenarioRoot']) {need(typeof m[key]==='string'&&isAbsolute(m[key]),'absolute roots');ordinary(m[key]);need(lstatSync(m[key]).isDirectory(),'root directory');m[key]=realpathSync(m[key]);}
    need(dirname(m.scenarioRoot)===realpathSync(scenarios)&&dirname(resolve(manifestPath))===m.scenarioRoot,'private direct-child scenario');
    for(const root of [m.sourceInstallation,m.packageRoot,m.hostRoot])need(root!==m.scenarioRoot&&!inside(root,m.scenarioRoot)&&!inside(m.scenarioRoot,root),'disjoint input/output roots');
    closed(m.runtime,['api','totPatch','javaRelease','javaExecutable','jvmLibrary']);
    const api=pinned(m.sourceInstallation,m.runtime.api,'starsector-core/starfarer.api.jar');
    pinned(m.sourceInstallation,m.runtime.javaExecutable,'jre/bin/java.exe');pinned(m.sourceInstallation,m.runtime.jvmLibrary,'jre/bin/server/jvm.dll');
    const release=readFileSync(pinned(m.sourceInstallation,m.runtime.javaRelease,'jre/release'),'utf8');
    for(const [key,wanted] of Object.entries({JAVA_VERSION:'17.0.10',JAVA_RUNTIME_VERSION:'17.0.10+7-LTS',IMPLEMENTOR:'Azul Systems, Inc.',OS_NAME:'Windows',OS_ARCH:'x86_64'}))need(new RegExp('^'+key+'="'+wanted.replace(/[.*+?^${}()|[\]\\]/g,'\\$&')+'"\\r?$','m').test(release),'JVM profile');
    need(Array.isArray(m.mods)&&sameSet(m.mods.map((/** @type {any} */ x)=>x.id),Object.keys(neutralMods)),'exact six-mod closure');
    for(const mod of m.mods) {
        closed(mod,['id','version','directory','metadata','jars','dependencies']);const spec=neutralMods[mod.id];
        need(spec&&mod.version===spec.version&&Array.isArray(mod.dependencies)&&sameSet(mod.dependencies,spec.dependencies),'mod version/dependency closure');
        const directory=relativePath(mod.directory);need(directory.startsWith('mods/')&&directory.split('/').length===2,'mod directory');
        const info=parseGameJson(readFileSync(pinned(m.sourceInstallation,mod.metadata,directory+'/mod_info.json'),'utf8'),'profile mod');
        need(info.id===mod.id&&version(info.version)===mod.version,'mod metadata identity');
        const deps=(info.dependencies??[]).map((/** @type {any} */ x)=>x.id);need(sameSet(deps,spec.dependencies),'declared dependency closure');
        need(Array.isArray(mod.jars)&&Array.isArray(info.jars)&&sameSet(mod.jars.map((/** @type {any} */ x)=>x.path),info.jars.map((/** @type {string} */ x)=>directory+'/'+relativePath(x))),'declared JAR closure');
        for(const jar of mod.jars)noCoreApi(pinned(m.sourceInstallation,jar));
    }
    const tot=m.mods.find((/** @type {any} */ x)=>x.id==='aotd_theory_of_toolbox');
    const patch=pinned(m.sourceInstallation,m.runtime.totPatch,tot.directory+'/0.98a/starfarer.api.jar');
    need(m.runtime.api.sha256===m.runtime.totPatch.sha256&&readFileSync(api).equals(readFileSync(patch)),'core equals supplied ToT patch');
    closed(m.package,['manifest','modInfo','adapter','probe']);
    const pkg=strictJson(bytes(pinned(m.packageRoot,m.package.manifest,'probe-package.json'),16_384));
    closed(pkg,['formatVersion','packageVersion','protocol','javaTarget','compiler','gameRequirement','enabledByDefault','nativeValidated','sidecarBundled','maxRequestsPerBinding','maxOutstanding','artifacts']);
    need(pkg.formatVersion===1&&pkg.packageVersion===m.companionVersion&&pkg.protocol===m.protocol&&pkg.javaTarget===17&&pkg.compiler==='21.0.8'&&pkg.gameRequirement===m.gameVersion&&pkg.enabledByDefault===false&&pkg.nativeValidated===false&&pkg.sidecarBundled===false&&pkg.maxRequestsPerBinding===8&&pkg.maxOutstanding===1,'companion compatibility metadata');
    closed(pkg.artifacts,['mod_info.json','jars/near-future-adapter.jar','jars/near-future-probe.jar']);
    for(const [key,label] of Object.entries({modInfo:'mod_info.json',adapter:'jars/near-future-adapter.jar',probe:'jars/near-future-probe.jar'})) {const p=pinned(m.packageRoot,m.package[key],label);need(pkg.artifacts[label]===m.package[key].sha256,'package artifact binding');if(key!=='modInfo')noCoreApi(p);}
    const info=strictJson(bytes(pinned(m.packageRoot,m.package.modInfo,'mod_info.json'),16_384));
    need(info.id==='near_future_probe'&&info.version===m.companionVersion&&info.gameVersion===m.gameVersion&&info.modPlugin==='nf.adapter.probe.ProbeModPlugin'&&Array.isArray(info.jars)&&sameSet(info.jars,['jars/near-future-adapter.jar','jars/near-future-probe.jar']),'companion entrypoint/JARs');
    closed(m.host,['artifact','sourceCommit','protocol']);need(/^[a-f0-9]{40}$/.test(m.host.sourceCommit)&&m.host.protocol===m.protocol,'host source/protocol pin');pinned(m.hostRoot,m.host.artifact,'nf-sidecar.exe');
    need(Array.isArray(m.protected)&&m.protected.length>=configPaths.length&&m.protected.length<=1004,'protected inventory bound');
    const names=m.protected.map((/** @type {any} */ x)=>x.path);need(names.length===new Set(names).size&&configPaths.every(p=>names.includes(p))&&names.every((/** @type {string} */ p)=>configPaths.includes(p)||p.startsWith('saves/')),'exact protected paths');
    for(const entry of m.protected)pinned(m.sourceInstallation,entry);
    const saves=inventory(m.sourceInstallation,'saves');need(sameSet(saves.map(x=>x.path),names.filter((/** @type {string} */ p)=>p.startsWith('saves/'))),'original save census changed');
    privateSettings(readFileSync(contained(m.sourceInstallation,configPaths[2]),'utf8'));
    return m;
}

/** @param {string} root @param {string} subtree */
function inventory(root,subtree) {
    /** @type {{path:string,bytes:number,sha256:string}[]} */ const files=[];
    /** @param {string} path @param {number} depth */
    function scan(path,depth) {need(depth<=32,'tree depth');ordinary(resolve(root,path));for(const entry of readdirSync(resolve(root,path),{withFileTypes:true})){need(!entry.isSymbolicLink(),'tree reparse');const next=path+'/'+entry.name;if(entry.isDirectory())scan(next,depth+1);else{need(entry.isFile(),'ordinary tree file');files.push(pin(resolve(root,next),next));need(files.length<=10000,'tree file bound');}}}
    scan(subtree,0);return files.sort((a,b)=>a.path.localeCompare(b.path));
}
/** Preserve all non-autosave bytes, including the game's comments/numeric extensions.
 * @param {string} text */
export function privateSettings(text) {
    /** @type {Record<string,{start:number,end:number,raw:string}[]>} */
    const found={maxAutosaveSlots:[],autosaveFeatureEnabled:[],autosaveEnabledByDefault:[]};
    /** @param {number} at */
    function skip(at){
        while(at<text.length){
            if(/\s/.test(text[at])){at++;continue;}
            if(text[at]==='#'||text.slice(at,at+2)==='//'){while(at<text.length&&text[at]!=='\n')at++;continue;}
            if(text.slice(at,at+2)==='/*'){const end=text.indexOf('*/',at+2);need(end>=0,'settings comment');at=end+2;continue;}
            break;
        }
        return at;
    }
    /** @param {number} at */
    function stringEnd(at){const quote=text[at++];while(at<text.length){const c=text[at++];if(c===quote)return at;if(c==='\\')at++;}throw new Error('Neutral recipe refused: settings string');}
    for(let i=0;i<text.length;){
        i=skip(i);if(i>=text.length)break;
        if(text[i]!=='"'&&text[i]!=="'"){
            if(/[A-Za-z_$]/.test(text[i])){
                let end=i+1;while(end<text.length&&/[A-Za-z0-9_$]/.test(text[end]))end++;
                need(!(Object.hasOwn(found,text.slice(i,end))&&text[skip(end)]===':'),'protected settings key must be quoted');i=end;
            }else i++;
            continue;
        }
        const start=i,end=stringEnd(i);i=end;const colon=skip(end);if(text[colon]!==':')continue;
        const key=parseGameJson(text.slice(start,end),'settings key');if(!Object.hasOwn(found,key))continue;
        const value=skip(colon+1);let finish=value;
        if(text[value]==='"'||text[value]==="'")finish=stringEnd(value);
        else while(finish<text.length&&!/[\s,};]/.test(text[finish])&&text[finish]!=='#'&&!['//','/*'].includes(text.slice(finish,finish+2)))finish++;
        found[key].push({start:value,end:finish,raw:text.slice(value,finish)});
    }
    need(found.maxAutosaveSlots.length===1&&found.maxAutosaveSlots[0].raw==='3','original autosave retention');
    const replacements=[];
    for(const key of ['autosaveFeatureEnabled','autosaveEnabledByDefault']) {
        need(found[key].length===1&&found[key][0].raw==='true','original autosave setting');replacements.push(found[key][0]);
    }
    for(const item of replacements.sort((a,b)=>b.start-a.start))text=text.slice(0,item.start)+'false'+text.slice(item.end);
    return text;
}
/** @param {string} text @param {string} installation @param {string} evidence @param {string} hostHash */
export function neutralVmparams(text,installation,evidence,hostHash) {
    need(!/[\s"'\r\n]/.test(installation+evidence)&&/^[a-f0-9]{64}$/.test(hostHash),'safe prepared launcher paths');
    need(text.trim().startsWith('java.exe ')&&!/-Dnf\.|-javaagent:|-agentlib:|-agentpath:/.test(text),'uninstrumented source launcher');
    need(!/(?:^|\s)["']?(?:@|-XX:(?:Flags|VMOptionsFile)(?:=|\s|$))/.test(text),'indirect JVM option source');
    for(const flag of ['-noverify','-Xverify:none','-XX:-BytecodeVerificationLocal','-XX:-BytecodeVerificationRemote'])text=text.replaceAll(flag,'');
    need(!/-Xverify:|-XX:[+-]BytecodeVerification/.test(text),'ambiguous verifier flags');
    for(const key of ['saves','screenshots','mods','logs']) {const pattern=new RegExp('-Dcom\\.fs\\.starfarer\\.settings\\.paths\\.'+key+'=\\S+','g');need([...text.matchAll(pattern)].length===1,'launcher path shape');text=text.replace(pattern,'-Dcom.fs.starfarer.settings.paths.'+key+'='+installation+'/'+(key==='logs'?'starsector-core':key));}
    const args='-Xverify:all -Dnf.probe.enabled=true -Dnf.probe.executable='+installation+'/diagnostic/nf-sidecar.exe -Dnf.probe.sha256='+hostHash+' -Dnf.probe.evidence='+evidence+' ';
    return text.trim().replace('java.exe ','java.exe '+args)+'\n';
}

/** Stage only small configs and verified protected-original backups. Runtime/mod assets are
 * described by pins, not copied. The new receipt is deliberately rejected by old launch guards.
 * @param {string} manifestPath @param {string} expectedSha256 */
export function prepareNeutralRecipe(manifestPath,expectedSha256) {
    const m=readNeutralRecipe(manifestPath,expectedSha256), output=join(m.scenarioRoot,'neutral-preparation');ordinary(output,true);
    const installation=join(m.scenarioRoot,'installation').replaceAll('\\','/'), evidence=join(m.scenarioRoot,'native-facts.private.jsonl').replaceAll('\\','/');
    const vm=neutralVmparams(readFileSync(contained(m.sourceInstallation,'vmparams'),'utf8'),installation,evidence,m.host.artifact.sha256);
    const settings=privateSettings(readFileSync(contained(m.sourceInstallation,configPaths[2]),'utf8'));
    const total=m.protected.reduce((/** @type {number} */ n,/** @type {any} */ p)=>n+p.bytes,0);need(total<=10_737_418_240,'protected backup byte bound');
    mkdirSync(output);const start=Date.now();
    for(const entry of m.protected) {
        need(Date.now()-start<150000,'preparation deadline');const from=pinned(m.sourceInstallation,entry), to=join(output,'original-backup',entry.path);ordinary(to,true);mkdirSync(dirname(to),{recursive:true});copyFileSync(from,to,constants.COPYFILE_EXCL);need(pin(to,entry.path).sha256===entry.sha256,'backup hash mismatch');
    }
    for(const [name,value] of Object.entries({'vmparams.prepared':vm,'settings.prepared.json':settings,'enabled_mods.prepared.json':JSON.stringify({enabledMods:[...Object.keys(neutralMods).sort(),'near_future_probe']})+'\n'}))writeFileSync(join(output,name),value,{flag:'wx'});
    readNeutralRecipe(manifestPath,expectedSha256); // Every original pin and save census must still match.
    const receipt={schemaVersion:1,kind:'NEUTRAL_ACK32_PREPARATION_ONLY_V1',profile:neutralProfile,recipeSha256:expectedSha256,originalWrites:false,backupVerified:true,protectedFiles:m.protected.length,protectedBytes:total,privateAutosaveEnabled:false,verification:'-Xverify:all',runtimeAssetsCopied:false,seedCreated:false,seedIdentityValidated:false,nativeLaunched:false,launchAdmitted:false,nativeAcceptance:'UNPROVED',prepared:['vmparams.prepared','settings.prepared.json','enabled_mods.prepared.json'].map(p=>pin(join(output,p),p))};
    writeFileSync(join(output,'receipt.private.json'),JSON.stringify(receipt,null,2)+'\n',{flag:'wx'});return receipt;
}
