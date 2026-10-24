// @ts-check
// Actual preparation script, generated inputs, deliberate XML refusal before any executable invocation.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,readdirSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {generationBindingFixture,repository} from './generation-fixture-test-support.mjs';
import {json,write,filePin,powershell} from './seed-test-support.mjs';

test('actual generation preparation writes the closed private recipe and preserves originals before a deliberate pre-init refusal',()=>{
    const f=generationBindingFixture();
    try {
        const source=f.binding.sourceInstallation,runtime=f.binding.qualifiedRuntimeRoot;
        const originals=[...Array.from({length:178},(_,i)=>['saves/save-'+i+'.txt','generated-save-'+i]),
            ['vmparams','generated vmparams'],['mods/enabled_mods.json','{"enabledMods":["near_future"]}'],
            ['starsector-core/data/config/settings.json','{"maxAutosaveSlots":3,"autosaveFeatureEnabled":true,"autosaveEnabledByDefault":true,"compressSaveGameData":false}'],
            ['starsector-core/compiler_directives.txt','generated directives']];
        const baseline=originals.map(([path,text])=>({path,...write(resolve(source,path),text)}));
        write(resolve(source,'starsector.exe'),'Generated marker, not executable');
        write(resolve(source,'jre/lib/jfr/default.jfc'),'<deliberately malformed XML');
        const artifacts=['adapter-0.1.0.jar','bcprov-jdk18on-1.81.jar','contract-0.1.0.jar','ipc-0.1.0.jar',
            'protobuf-java-4.31.1.jar','wire-0.1.0.jar'].map(name=>{
            const path=resolve(runtime,'jars',name);
            return {relative:'mods/near_future/jars/'+name,source:path,...write(path,'generated artifact '+name)};
        });
        const node=resolve(runtime,'diagnostic/nf-ipc-node.exe');
        artifacts.push({relative:'diagnostic/nf-ipc-node.exe',source:node,...write(node,'Generated marker, not executable')});
        f.binding.runtimeManifestSha256=json(resolve(f.root,'runtime-artifacts.private.json'),{schemaVersion:1,
            kind:'NATIVE99_CAMPAIGN_BINDING_RUNTIME_INPUTS',candidateHead:f.binding.candidateHead,artifacts}).sha256;
        f.sha256=json(f.path,f.binding).sha256;
        json(resolve(f.root,'plan.json'),{});
        const result=powershell(['-File',resolve(repository,'tools/starsector-proxy/fixture/prepare-fixture.ps1'),
            '-BindingPath',f.path,'-BindingSha256',f.sha256]);
        assert.notEqual(result.status,0,'this case deliberately stops before private node initialization');
        assert.match(result.stderr,/XmlDocument|Cannot convert value/);
        const installation=resolve(f.root,'installation');
        assert.deepEqual(JSON.parse(readFileSync(resolve(installation,'mods/enabled_mods.json'),'utf8')),{enabledMods:[]});
        const settings=JSON.parse(readFileSync(resolve(installation,'starsector-core/data/config/settings.json'),'utf8'));
        assert.equal(settings.compressSaveGameData,false);assert.equal(settings.autosaveFeatureEnabled,false);
        assert.equal(settings.autosaveEnabledByDefault,false);assert.equal(settings.maxAutosaveSlots,3);
        assert.deepEqual(readdirSync(resolve(installation,'saves')),[]);
        for(const row of baseline) {
            assert.deepEqual(filePin(resolve(source,row.path)),{bytes:row.bytes,sha256:row.sha256});
            assert.deepEqual(filePin(resolve(f.root,'original-backup',row.path)),{bytes:row.bytes,sha256:row.sha256});
        }
        for(const name of ['diagnostic-vault','vault-initialization.private.json','protected-fixture-receipt.private.json']) {
            assert.equal(existsSync(resolve(f.root,name)),false,name+' must remain unreached');
        }
    } finally {f.close();}
});
