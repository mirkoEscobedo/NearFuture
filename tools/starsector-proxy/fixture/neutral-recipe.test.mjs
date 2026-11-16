// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,existsSync,symlinkSync} from 'node:fs';
import {resolve} from 'node:path';
import {readNeutralRecipe,prepareNeutralRecipe,neutralVmparams,privateSettings} from './neutral-recipe.mjs';
import {neutralFixture,hash,write,json} from './neutral-test-support.mjs';

test('closed neutral recipe pins exact supplied profile and prepares only exclusive private configs/backups',()=>{
    const f=neutralFixture();try{
        const sha=f.saveRecipe(),m=readNeutralRecipe(f.path,sha);assert.equal(m.mods.length,6);
        const originals=f.recipe.protected.map(x=>hash(readFileSync(resolve(f.source,x.path))));
        const receipt=prepareNeutralRecipe(f.path,sha);assert.equal(receipt.launchAdmitted,false);assert.equal(receipt.seedCreated,false);assert.equal(receipt.runtimeAssetsCopied,false);
        const output=resolve(f.scenario,'neutral-preparation');assert.equal(existsSync(resolve(f.scenario,'installation')),false);
        const vm=readFileSync(resolve(output,'vmparams.prepared'),'utf8');assert.match(vm,/-Xverify:all/);assert.doesNotMatch(vm,/-noverify|-XX:-BytecodeVerification/);assert.match(vm,/-Dnf.probe.sha256=/);
        const settings=readFileSync(resolve(output,'settings.prepared.json'),'utf8');assert.match(settings,/"maxAutosaveSlots":3/);assert.match(settings,/"autosaveFeatureEnabled":false/);
        assert.equal(settings,f.settings.replace('"autosaveFeatureEnabled":true','"autosaveFeatureEnabled":false').replace('"autosaveEnabledByDefault":true','"autosaveEnabledByDefault":false'));
        for(const [i,p] of f.recipe.protected.entries()){assert.equal(hash(readFileSync(resolve(f.source,p.path))),originals[i]);assert.equal(hash(readFileSync(resolve(output,'original-backup',p.path))),p.sha256);}
        assert.throws(()=>prepareNeutralRecipe(f.path,sha),/exist|EEXIST/i);
    }finally{f.close();}
});

test('unknown/extra/duplicate recipe fields, wrong protocol and traversal refuse before output',()=>{
    const f=neutralFixture();try{
        for(const change of [()=>{f.recipe.profile='other';},()=>{f.recipe.protocol='old-ack16';},()=>{f.recipe.runtime.api.path='../outside';}]){
            const original=JSON.stringify(f.recipe);change();const sha=f.saveRecipe();assert.throws(()=>readNeutralRecipe(f.path,sha));Object.assign(f.recipe,JSON.parse(original));
        }
        const sha=f.saveRecipe();writeFileSync(f.path,readFileSync(f.path,'utf8').replace('"schemaVersion":1','"schemaVersion":1,"schemaVersion":1'));
        assert.throws(()=>readNeutralRecipe(f.path,hash(readFileSync(f.path))),/duplicate/);assert.throws(()=>readNeutralRecipe(f.path,sha),/pin/);
        assert.equal(existsSync(resolve(f.scenario,'neutral-preparation')),false);
    }finally{f.close();}
});

test('changed artifacts, core/ToT mismatch, wrong JVM and dependency/version drift refuse',()=>{
    const f=neutralFixture();try{
        const sha=f.saveRecipe(),path=resolve(f.source,f.recipe.runtime.totPatch.path);writeFileSync(path,'different patch');assert.throws(()=>readNeutralRecipe(f.path,sha),/changed/);
        f.recipe.runtime.totPatch=write(f.source,f.recipe.runtime.totPatch.path,'different patch');assert.throws(()=>readNeutralRecipe(f.path,f.saveRecipe()),/core equals/);
        f.recipe.runtime.totPatch=write(f.source,f.recipe.runtime.totPatch.path,'synthetic ToT patch, NOT native');
        f.recipe.runtime.javaRelease=write(f.source,'jre/release','JAVA_VERSION="21.0.8"');assert.throws(()=>readNeutralRecipe(f.path,f.saveRecipe()),/JVM/);
    }finally{f.close();}
    for(const key of ['version','dependencies']){const g=neutralFixture();try{if(key==='version')g.recipe.mods[0].version='1.5.7';else g.recipe.mods[0].dependencies=[];assert.throws(()=>readNeutralRecipe(g.path,g.saveRecipe()),/closure/);}finally{g.close();}}
});

test('backup census and bytes drift, symlink ancestry and prior destination refuse',()=>{
    const f=neutralFixture();try{
        const sha=f.saveRecipe();write(f.source,'saves/new-user-save/descriptor.xml','new save');assert.throws(()=>prepareNeutralRecipe(f.path,sha),/census/);
        assert.equal(existsSync(resolve(f.scenario,'neutral-preparation')),false);
    }finally{f.close();}
    const g=neutralFixture();try{
        const original=g.recipe.sourceInstallation,link=resolve(g.root,'linked-source');symlinkSync(original,link,process.platform==='win32'?'junction':'dir');g.recipe.sourceInstallation=link;
        assert.throws(()=>prepareNeutralRecipe(g.path,g.saveRecipe()),/reparse/);
    }finally{g.close();}
});

test('companion activation claims and extra declared JARs cannot enter the named recipe',()=>{
    const f=neutralFixture();try{
        f.metadata.nativeValidated=true;f.recipe.package.manifest=json(f.pkg,'probe-package.json',f.metadata);assert.throws(()=>readNeutralRecipe(f.path,f.saveRecipe()),/compatibility/);
        f.metadata.nativeValidated=false;f.recipe.package.manifest=json(f.pkg,'probe-package.json',f.metadata);
        f.recipe.mods[0].jars.push(write(f.source,f.recipe.mods[0].directory+'/jars/extra.jar','extra'));assert.throws(()=>readNeutralRecipe(f.path,f.saveRecipe()),/JAR closure/);
    }finally{f.close();}
});

test('private launch/settings construction has explicit verifier/autosave bounds',()=>{
    const f=neutralFixture();try{
        assert.throws(()=>neutralVmparams(f.vm+' -javaagent:unknown','E:/safe','E:/facts','a'.repeat(64)),/uninstrumented/);
        assert.throws(()=>neutralVmparams(f.vm,'E:/unsafe path','E:/facts','a'.repeat(64)),/safe/);
        assert.throws(()=>privateSettings(f.settings.replace('"maxAutosaveSlots":3','"maxAutosaveSlots":4')),/retention/);
        assert.throws(()=>privateSettings(f.settings.replace('"maxAutosaveSlots":3','"maxAutosaveSlots":3,"maxAutosaveSlots":4')),/retention/);
        assert.throws(()=>privateSettings(f.settings.replace('"maxAutosaveSlots":3','"maxAutosaveSlots":3,"\\u006daxAutosaveSlots":4')),/retention/);
        assert.throws(()=>privateSettings(f.settings.replace('"maxAutosaveSlots":3','"maxAutosaveSlots":3,maxAutosaveSlots:4')),/quoted/);
        assert.throws(()=>privateSettings(f.settings.replace('"autosaveFeatureEnabled":true','"autosaveFeatureEnabled":false')),/setting/);
        assert.throws(()=>privateSettings(f.settings.replace('"autosaveFeatureEnabled":true','"autosaveFeatureEnabled":true,"autosaveFeatureEnabled":null')),/setting/);
        assert.throws(()=>privateSettings(f.settings.replace('"autosaveFeatureEnabled":true','"autosaveFeatureEnabled":true,"\\u0061utosaveFeatureEnabled":null')),/setting/);
        for(const option of ['@unsafe.args','"@unsafe.args"','-XX:Flags=unsafe.flags','-XX:VMOptionsFile=unsafe.options'])
            assert.throws(()=>neutralVmparams(f.vm+' '+option,'E:/safe','E:/facts','a'.repeat(64)),/indirect/);
    }finally{f.close();}
});

test('a pinned declared mod JAR containing core API entries is refused',()=>{
    for(const label of ['com/fs/starfarer/api/BaseModPlugin.class','META-INF/versions/17/com/fs/starfarer/api/BaseModPlugin.class']){
        const f=neutralFixture();try{
        const name=Buffer.from(label),directory=Buffer.alloc(46+name.length);
        directory.writeUInt32LE(0x02014b50);directory.writeUInt16LE(name.length,28);name.copy(directory,46);
        const end=Buffer.alloc(22);end.writeUInt32LE(0x06054b50);end.writeUInt16LE(1,8);end.writeUInt16LE(1,10);end.writeUInt32LE(directory.length,12);
        const mod=f.recipe.mods[0];mod.jars[0]=write(f.source,mod.jars[0].path,Buffer.concat([directory,end]));
        assert.throws(()=>readNeutralRecipe(f.path,f.saveRecipe()),/competing core API/);
        }finally{f.close();}
    }
});
