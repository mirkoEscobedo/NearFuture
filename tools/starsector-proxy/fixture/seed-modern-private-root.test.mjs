// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync,readFileSync,readdirSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {preContinueHistoryFixture} from './seed-pre-continue-history-test-support.mjs';
import {copiedModernConsumerTarget} from './seed-modern-consumer-test-support.mjs';
import {filePin,json,powershell,quote} from './seed-test-support.mjs';
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
    const consumer=await copiedModernConsumerTarget();
    /** @type {Awaited<ReturnType<typeof modernSeedFixture>>|undefined} */let f;
    /** @type {ReturnType<typeof preContinueHistoryFixture>|undefined} */let sourceOwner;
    try{
        f=await modernSeedFixture({fixture:()=>{
            const source=producerReceiptFixture(undefined,preContinueHistoryFixture),launcher=source.launcher;
            const close=source.close;let closed=false;
            source.close=()=>{if(!closed){closed=true;close();}};sourceOwner=source;
            // The history fixture retains its original schema-6 launcher factory.
            // Its fresh opt-in source has the same schema-7 phase data as the existing focus fixture.
            source.launcher=(/** @type {string|undefined} */ runID)=>{
                const batch=launcher(runID),authorization=JSON.parse(readFileSync(batch.source,'utf8'));
                authorization.schemaVersion=7;json(batch.source,authorization);return batch;
            };
            return source;
        },
            targetFixture:()=>consumer.target,callerBudgetMs:1740000,expectedProviderTimeoutMs:480000});
        assert.notEqual(f.manifest.sourceWorkspace,consumer.root,'Producer and current consumer roots must differ');
        assert.equal(f.manifest.sourceWorkspace,f.source.root);
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
        assert.equal(existsSync(resolve(f.target.target,'seed-stage-receipt.private.json')),true);
        for(const row of f.manifest.files)assert.deepEqual(
            readFileSync(resolve(f.target.installation,'saves',f.manifest.saveDirectory,row.relative)),
            readFileSync(resolve(f.source.installation,'saves',f.manifest.saveDirectory,row.relative)));
    }finally{try{f?.close();}finally{try{sourceOwner?.close();}finally{consumer.close();}}}
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

test('modern history keeps historical byte custody separate from explicit current proof pins',async t=>{
    const f=preContinueHistoryFixture();
    try{
        const currentPath=resolve(f.scenario,'current-consumer-pins.private.json');
        const original=readFileSync(f.launchLease.supportPinsPath);
        writeFileSync(currentPath,original);
        const current={path:currentPath,sha256:filePin(currentPath).sha256.toLowerCase()};
        /** @param {unknown} context @param {Record<string,unknown>} [leaseChanges] */
        function inspect(context,leaseChanges={}){
            const command="$ErrorActionPreference='Stop'; . "+quote(resolve(f.root,'tools/starsector-proxy/fixture/seed-fixture.ps1'))+
                '; $p=Get-Content -LiteralPath '+quote(f.planPath)+' -Raw|ConvertFrom-Json -DateKind String;'+
                ' $l=Get-Content -LiteralPath '+quote(f.leasePath)+' -Raw|ConvertFrom-Json -DateKind String;'+
                ' $changes='+quote(JSON.stringify(leaseChanges))+'|ConvertFrom-Json -DateKind String;'+
                ' foreach($property in $changes.PSObject.Properties){$l.($property.Name)=$property.Value};'+
                ' $context='+quote(JSON.stringify(context))+'|ConvertFrom-Json -DateKind String -NoEnumerate;'+
                " if($context-is[array]){'CONTEXT_ARRAY_DELIVERED'};"+
                ' Assert-NativeSeedCreationHistory $p $l '+quote(f.scenario)+' -CurrentProofPins $context;'+
                " 'HISTORY_PASS'";
            return powershell(['-Command',command]);
        }
        /** @param {import('node:child_process').SpawnSyncReturns<string>} result */
        function refuses(result){
            assert.equal(result.error,undefined);assert.equal(result.signal,null);assert.equal(result.status,1,result.stderr);
            assert.match(result.stderr,/seed|proof|pin|JSON|modern/i);assert.doesNotMatch(result.stdout,/HISTORY_PASS/);
        }
        const omitted=f.admit();assert.equal(omitted.error,undefined);assert.equal(omitted.signal,null);
        assert.equal(omitted.status,0,omitted.stderr);
        const selected=inspect(current);assert.equal(selected.error,undefined);assert.equal(selected.signal,null);
        assert.equal(selected.status,0,selected.stderr);assert.match(selected.stdout,/HISTORY_PASS/);
        /** @type {Array<[string,unknown]>} */
        const contexts=[
            ['explicit null is not an omitted context',null],
            ['an array is not a context object',[current]],
            ['missing hash',{path:currentPath}],
            ['wrong hash',{...current,sha256:'0'.repeat(64)}],
            ['array path',{...current,path:[currentPath]}],
            ['boolean path',{...current,path:false}],
            ['relative path',{...current,path:'current-consumer-pins.private.json'}],
            ['noncanonical absolute path',{...current,path:f.scenario+'/./current-consumer-pins.private.json'}],
            ['array hash',{...current,sha256:[current.sha256]}],
            ['unrelated authority field',{...current,authorized:true}],
        ];
        for(const [name,value]of contexts)await t.test(name,()=>{
            const result=inspect(value);
            if(Array.isArray(value))assert.match(result.stdout,/CONTEXT_ARRAY_DELIVERED/);
            refuses(result);
        });
        await t.test('valid current pins cannot replace the fixed historical lease path',()=>
            refuses(inspect(current,{supportPinsPath:currentPath})));
        await t.test('valid current pins cannot replace the historical manifest hash',()=>
            refuses(inspect(current,{supportPinsSha256:'0'.repeat(64)})));
        const incomplete=resolve(f.scenario,'incomplete-current-pins.private.json'),rows=JSON.parse(original.toString());
        rows.pins=rows.pins.filter((/** @type {any} */ row)=>!row.path.endsWith('fixture-inputs.ps1'));
        json(incomplete,rows);
        await t.test('a rehashed incomplete current inventory still refuses',()=>
            refuses(inspect({path:incomplete,sha256:filePin(incomplete).sha256.toLowerCase()})));
        const foreign=resolve(f.scenario,'other-root-fixture-inputs.ps1');
        writeFileSync(foreign,readFileSync(resolve(f.root,'tools/starsector-proxy/fixture/fixture-inputs.ps1')));
        const wrongRoot=resolve(f.scenario,'wrong-root-current-pins.private.json'),foreignRows=JSON.parse(original.toString());
        for(const row of foreignRows.pins)if(row.path.endsWith('fixture-inputs.ps1'))row.path=foreign;
        json(wrongRoot,foreignRows);
        await t.test('same-byte foreign tooling cannot replace the current root module',()=>
            refuses(inspect({path:wrongRoot,sha256:filePin(wrongRoot).sha256.toLowerCase()})));
        const isolated=resolve(f.scenario,'historical-ordinary-module.private.mjs');
        writeFileSync(isolated,'// Generated historical data; never executed.\n');
        const isolatedRow={path:isolated,sha256:filePin(isolated).sha256.toLowerCase()};
        /** @param {any[]} extra */
        function historicalRows(extra){
            json(f.launchLease.supportPinsPath,{pins:[...JSON.parse(original.toString()).pins,...extra]});
            f.launchLease.supportPinsSha256=f.phaseLease.supportPinsSha256=filePin(f.launchLease.supportPinsPath).sha256.toLowerCase();
            f.refreshLaunch();
        }
        historicalRows([isolatedRow]);
        const historical=inspect(current);assert.equal(historical.error,undefined);assert.equal(historical.signal,null);
        assert.equal(historical.status,0,historical.stderr);
        await t.test('valid current closure cannot hide a changed historical ordinary file',()=>{
            const bytes=readFileSync(isolated);writeFileSync(isolated,'Changed generated historical bytes');
            try{refuses(inspect(current));}finally{writeFileSync(isolated,bytes);}
        });
        await t.test('valid current closure cannot hide a coerced historical row',()=>{
            historicalRows([{...isolatedRow,sha256:[isolatedRow.sha256]}]);
            try{refuses(inspect(current));}finally{historicalRows([isolatedRow]);}
        });
        await t.test('valid current closure cannot hide duplicate historical rows',()=>{
            historicalRows([isolatedRow,isolatedRow]);
            try{refuses(inspect(current));}finally{historicalRows([isolatedRow]);}
        });
        await t.test('current closure does not replace older failed-attempt artifact pins',()=>{
            const path=f.ctx.old.plan.preclockHelpers['launch-owned-caller.private.ps1'].path,bytes=readFileSync(path);
            writeFileSync(path,'Changed isolated historical helper; never executed');
            try{refuses(inspect(current));}finally{writeFileSync(path,bytes);}
        });
    }finally{f.close();}
});
