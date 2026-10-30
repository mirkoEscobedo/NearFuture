// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync,readFileSync,readdirSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {filePin,json} from './seed-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';

// Build the producer's receipt before the transport pins any authority or calls.
/**
 * @param {(installation: string) => unknown} [privateRoot]
 * @param {() => ReturnType<typeof seedCreationFixture>} [create]
 */
function producerReceiptFixture(privateRoot=installation=>installation+'/saves',create=seedCreationFixture) {
    const f=create();
    const receipt=JSON.parse(readFileSync(f.frozenPath,'utf8'));
    receipt.privateSaveRoot=privateRoot(f.installation);
    const receiptHash=json(f.frozenPath,receipt).sha256;
    for(const value of [f.launchLease,f.phaseLease,f.controller])value.fixtureReceiptSha256=receiptHash;
    f.refreshLaunch();
    return f;
}

test('modern staging accepts the actual preparation producer private-save-root spelling',async()=>{
    const f=await modernSeedFixture({fixture:producerReceiptFixture});
    try {
        writeFileSync(f.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
        assert.equal(inspectSeedCreationCustody(f.source.scenario,filePin(f.closeoutPath).sha256.toLowerCase()).ok,true);
        const result=f.target.stage();
        assert.equal(result.error,undefined);assert.equal(result.signal,null);
        if(result.status!==0){
            assert.equal(existsSync(resolve(f.target.target,'seed-stage-claim.private.json')),false);
            assert.equal(existsSync(resolve(f.target.target,'seed-stage-receipt.private.json')),false);
            assert.deepEqual(readdirSync(resolve(f.target.installation,'saves')),[]);
        }
        assert.equal(result.status,0,result.stderr);
        for(const row of f.manifest.files)assert.deepEqual(
            readFileSync(resolve(f.target.installation,'saves',f.manifest.saveDirectory,row.relative)),
            readFileSync(resolve(f.source.installation,'saves',f.manifest.saveDirectory,row.relative)));
    }finally{f.close();}
});

test('initial-focus modern provenance also accepts the actual producer private-save-root spelling',async()=>{
    const f=await modernSeedFixture({fixture:()=>producerReceiptFixture(undefined,initialFocusFixture)});
    try{
        writeFileSync(f.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
        assert.equal(inspectSeedCreationCustody(f.source.scenario,filePin(f.closeoutPath).sha256.toLowerCase()).ok,true);
        const result=f.target.stage();
        assert.equal(result.error,undefined);assert.equal(result.signal,null);assert.equal(result.status,0,result.stderr);
        assert.equal(existsSync(resolve(f.target.target,'seed-stage-receipt.private.json')),true);
    }finally{f.close();}
});

test('modern staging refuses private-save roots outside the exact authorized source before claiming',async t=>{
    /** @type {{name:string,value:(installation:string)=>unknown}[]} */
    const cases=[
        {name:'a different absolute root',value:installation=>resolve(installation,'..','other-installation','saves')},
        {name:'a sibling of the authorized save directory',value:installation=>resolve(installation,'saves-other')},
        {name:'a relative root',value:()=> 'installation/saves'},
        {name:'a case-changed root',value:installation=>installation+'/SAVES'},
        {name:'a singleton array',value:installation=>[installation+'/saves']},
        {name:'a null root',value:()=>null},
        {name:'a boolean root',value:()=>false},
        {name:'a numeric root',value:()=>7},
    ];
    for(const {name,value} of cases)await t.test(name,async()=>{
        const f=await modernSeedFixture({fixture:()=>producerReceiptFixture(value)});
        try{
            writeFileSync(f.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
            const result=f.target.stage();
            assert.equal(result.error,undefined);assert.equal(result.signal,null);assert.equal(result.status,1,result.stderr);
            assert.match(result.stderr,/Modern empty preparation provenance required/);
            assert.equal(existsSync(resolve(f.target.target,'seed-stage-claim.private.json')),false);
            assert.equal(existsSync(resolve(f.target.target,'seed-stage-receipt.private.json')),false);
            assert.deepEqual(readdirSync(resolve(f.target.installation,'saves')),[]);
        }finally{f.close();}
    });
});
