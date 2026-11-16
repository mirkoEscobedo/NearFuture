// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {spawnSync} from 'node:child_process';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {neutralFixture,repository,hash} from './neutral-test-support.mjs';
/** @param {string} value */
const quote=value=>"'"+value.replaceAll("'","''")+"'";
/** @param {ReturnType<typeof neutralFixture>} f @param {boolean} [legacy] */
function inspect(f,legacy=false){
    const sha=f.saveRecipe();
    const call=legacy?'. '+quote(resolve(repository,'tools/starsector-proxy/fixture/seed-fixture.ps1'))+'; Read-NativeSeedXml '+quote(f.save)+" 'save_neutral' '0.98a-RC8'":
        '. '+quote(resolve(repository,'tools/starsector-proxy/fixture/neutral-seed.ps1'))+'; Read-NeutralSeedIdentity '+quote(f.save)+" 'save_neutral' "+quote(f.path)+' '+quote(sha);
    const result=spawnSync(process.env.NF_TEST_POWERSHELL??'pwsh',['-NoProfile','-NonInteractive','-Command',"$ErrorActionPreference='Stop'; "+call+' | ConvertTo-Json -Depth 6 -Compress'],{cwd:repository,encoding:'utf8',timeout:15000});
    assert.notEqual(result.status,null,'Seed inspection must close its command: '+result.error?.message);return result;
}

test('exact declared native ModSpec/versionInfo shapes inspect read-only; legacy reader still refuses',()=>{
    const f=neutralFixture();try{
        const before=hash(readFileSync(resolve(f.save,'descriptor.xml'))),result=inspect(f);assert.equal(result.status,0,result.stderr);
        const value=JSON.parse(result.stdout);assert.equal(value.declaredMods.length,7);assert.equal(value.nativeCreatedProof,false);assert.equal(value.launchAdmitted,false);
        assert.equal(hash(readFileSync(resolve(f.save,'descriptor.xml'))),before);
        const legacy=inspect(f,true);assert.notEqual(legacy.status,0);assert.match(legacy.stderr,/Seed descriptor format/);
    }finally{f.close();}
});

test('native spec references resolve only to one bounded matching identity',()=>{
    const f=neutralFixture();try{
        const inline=/<spec z="100">([\s\S]*?)<\/spec>/.exec(f.descriptor);assert.ok(inline);
        const referenced=f.descriptor.replace(inline[0],'<spec ref="100"/>').replace('</SaveGameData>','<ModSpec z="100">'+inline[1]+'</ModSpec></SaveGameData>');
        writeFileSync(resolve(f.save,'descriptor.xml'),referenced);let result=inspect(f);assert.equal(result.status,0,result.stderr);
        writeFileSync(resolve(f.save,'descriptor.xml'),referenced.replace('ref="100"','ref="999"'));result=inspect(f);assert.notEqual(result.status,0);assert.match(result.stderr,/reference/);
        writeFileSync(resolve(f.save,'descriptor.xml'),referenced.replace('</SaveGameData>','<ModSpec z="100"/></SaveGameData>'));result=inspect(f);assert.notEqual(result.status,0);assert.match(result.stderr,/reference/);
    }finally{f.close();}
});

test('missing, extra, duplicate or version-mismatched mod declarations refuse; no ignore-missing escape',()=>{
    const f=neutralFixture();try{
        const first=/<EnabledModData>[\s\S]*?<\/EnabledModData>/.exec(f.descriptor);assert.ok(first);
        for(const text of [f.descriptor.replace(first[0],''),f.descriptor.replace('</enabledMods>',first[0]+'</enabledMods>'),f.descriptor.replace('<id>near_future_probe</id>','<id>MagicLib</id>'),f.descriptor.replace('<patch>11</patch>','<patch>12</patch>')]){
            writeFileSync(resolve(f.save,'descriptor.xml'),text);const result=inspect(f);assert.notEqual(result.status,0);assert.match(result.stderr,/neutral|Neutral/);
        }
    }finally{f.close();}
});

test('DTD, duplicate fields, compression and wrong campaign identity refuse without seed writes',()=>{
    const f=neutralFixture();try{
        for(const text of ['<!DOCTYPE SaveGameData [<!ENTITY x SYSTEM "file:///unavailable">]>'+f.descriptor,f.descriptor.replace('</SaveGameData>','<gameVersion>0.98a-RC8</gameVersion></SaveGameData>'),f.descriptor.replace('<compressed>false','<compressed>true')]){
            writeFileSync(resolve(f.save,'descriptor.xml'),text);assert.notEqual(inspect(f).status,0);
        }
        writeFileSync(resolve(f.save,'descriptor.xml'),f.descriptor);writeFileSync(resolve(f.save,'campaign.xml'),f.campaign.replace('save_neutral','save_other'));assert.notEqual(inspect(f).status,0);
    }finally{f.close();}
});
