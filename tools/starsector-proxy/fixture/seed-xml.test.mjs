// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtempSync,mkdirSync,writeFileSync,rmSync} from 'node:fs';
import {resolve,relative,isAbsolute} from 'node:path';
import {fileURLToPath} from 'node:url';
import {spawnSync} from 'node:child_process';

const repository=resolve(fileURLToPath(new URL('../../../',import.meta.url)));
const temporary=resolve(repository,'.tmp/starsector-proxy/seed-unit-tests');
const quote=(/** @type {string} */ s)=>"'"+s.replaceAll("'","''")+"'";
function fixture(){
 mkdirSync(temporary,{recursive:true});const root=mkdtempSync(resolve(temporary,'xml-'));
 const campaign='<CampaignEngine><saveDirName>save_fixture_1</saveDirName><playerFleet ref="7"/><characterData><level>1</level></characterData><saveDesc/><Flt z="7"><fleet/></Flt></CampaignEngine>';
 const descriptor='<SaveGameData><compressed>false</compressed><gameVersion>0.98a-RC8</gameVersion><enabledMods/></SaveGameData>';
 const write=(/** @type {string} */ name,/** @type {string} */ text)=>writeFileSync(resolve(root,name),text);
 write('campaign.xml',campaign);write('descriptor.xml',descriptor);
 return {root,campaign,descriptor,write,close(){const suffix=relative(temporary,root);assert.ok(suffix&&!suffix.startsWith('..')&&!isAbsolute(suffix));rmSync(root,{recursive:true});}};
}
function read(/** @type {ReturnType<typeof fixture>} */ f){
 return spawnSync(process.env.NF_TEST_POWERSHELL??'pwsh',['-NoProfile','-NonInteractive','-Command',
 "$ErrorActionPreference='Stop';. "+quote(resolve(repository,'tools/starsector-proxy/fixture/seed-fixture.ps1'))+
 '; Read-NativeSeedXml '+quote(f.root)+" 'save_fixture_1' '0.98a-RC8' | ConvertTo-Json -Compress"],
 {cwd:repository,encoding:'utf8',timeout:15000});
}

test('a complete plain seed admits the serialized fleet reference and empty save description',()=>{
 const f=fixture();try{const r=read(f);assert.equal(r.status,0,r.stderr);assert.deepEqual(JSON.parse(r.stdout),{
 campaignRoot:'CampaignEngine',descriptorRoot:'SaveGameData',saveDirectory:'save_fixture_1',gameVersion:'0.98a-RC8',compressed:false,enabledMods:0,playerFleetResolved:true
 });}finally{f.close();}
});

test('a dangling or ambiguous fleet reference refuses the seed',()=>{
 const f=fixture();try{f.write('campaign.xml',f.campaign.replace('ref="7"','ref="9"'));let r=read(f);assert.notEqual(r.status,0);assert.match(r.stderr,/Unique seed fleet reference/);
 f.write('campaign.xml',f.campaign.replace('</CampaignEngine>','<Flt z="7"/></CampaignEngine>'));r=read(f);assert.notEqual(r.status,0);assert.match(r.stderr,/Unique seed fleet reference/);
 }finally{f.close();}
});

test('seed identity, version, and compression must match the admitted plain format',()=>{
 const f=fixture();try{f.write('campaign.xml',f.campaign.replace('save_fixture_1','save_other'));let r=read(f);assert.notEqual(r.status,0);assert.match(r.stderr,/Seed save identity/);
 f.write('campaign.xml',f.campaign);f.write('descriptor.xml',f.descriptor.replace('0.98a-RC8','0.97a'));r=read(f);assert.notEqual(r.status,0);assert.match(r.stderr,/Seed descriptor format/);
 f.write('descriptor.xml',f.descriptor.replace('false','true'));r=read(f);assert.notEqual(r.status,0);assert.match(r.stderr,/Seed descriptor format/);
 }finally{f.close();}
});

test('a DTD and duplicate required fields are refused without resolving external data',()=>{
 const f=fixture();try{f.write('campaign.xml','<!DOCTYPE CampaignEngine [<!ENTITY x SYSTEM "file:///unavailable">]>'+f.campaign);let r=read(f);assert.notEqual(r.status,0);
 f.write('campaign.xml',f.campaign.replace('</CampaignEngine>','<saveDirName>save_fixture_1</saveDirName></CampaignEngine>'));r=read(f);assert.notEqual(r.status,0);assert.match(r.stderr,/Unique seed XML field/);
 }finally{f.close();}
});
