// @ts-check
// A second generated CURRENT consumer root. No historical code or game is executed.
import assert from 'node:assert/strict';
import {copyFileSync,mkdirSync,mkdtempSync,rmSync,constants} from 'node:fs';
import {dirname,isAbsolute,relative,resolve,sep} from 'node:path';
import {pathToFileURL} from 'node:url';
import {requiredOwnedHttpFiles} from '../owned-http-batch.mjs';
import {repository,filePin} from './seed-test-support.mjs';

export async function copiedModernConsumerTarget() {
    const powershell=process.env.NF_TEST_POWERSHELL;
    assert.ok(powershell&&isAbsolute(powershell),'A canonical test PowerShell is required');
    const supervisor=resolve(repository,'node_modules/workspace-template-win32-x64/bin/workspace-template.exe');
    mkdirSync(resolve(repository,'.tmp'),{recursive:true});
    const root=mkdtempSync(resolve(repository,'.tmp/consumer-'));
    const extra=['prepare-fixture.ps1','run-native.ps1','read-owned-client-geometry.ps1','seed-test-support.mjs'];
    const paths=[...new Set([...requiredOwnedHttpFiles(repository,powershell,supervisor),
        ...extra.map(name=>resolve(repository,'tools/starsector-proxy/fixture',name))])];
    assert.ok(paths.length<=1025,'Only the bounded public tooling closure and one fixture helper may be copied');
    function close() {
        const suffix=relative(resolve(repository,'.tmp'),root);
        assert.ok(suffix&&!isAbsolute(suffix)&&!suffix.startsWith('..')&&suffix.startsWith('consumer-'));
        rmSync(root,{recursive:true});
    }
    try{
        for(const source of paths){
            const suffix=relative(repository,source);
            if(isAbsolute(suffix)||suffix==='..'||suffix.startsWith('..'+sep))continue;
            const destination=resolve(root,suffix);
            assert.ok(destination.startsWith(root+sep));mkdirSync(dirname(destination),{recursive:true});
            copyFileSync(source,destination,constants.COPYFILE_EXCL);
            assert.deepEqual(filePin(destination),filePin(source));
        }
        const helper=await import(pathToFileURL(resolve(root,'tools/starsector-proxy/fixture/seed-test-support.mjs')).href);
        const target=helper.seedFixture({driverRole:'/root/recovery_game_driver'});
        assert.ok(target.target.startsWith(root+sep));
        return{root,target,close};
    }catch(error){close();throw error;}
}
