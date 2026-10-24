// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdirSync,writeFileSync,readFileSync,unlinkSync,rmdirSync,renameSync,linkSync,symlinkSync} from 'node:fs';
import {resolve} from 'node:path';
import {withGenerationEntry,completeGeneratedNameEntry} from './generation-input-test-support.mjs';
import {seedInputSession,seedTarget} from './campaign-input-test-support.mjs';
import {readBatchAuthorization,claimBatchAuthorization} from '../batch-authorization.mjs';
import {readCampaignAuthorization} from '../campaign-authority.mjs';
import {campaignLaunchFixture} from './campaign-launch-test-support.mjs';
import {claimSeededLauncher} from './campaign-input-test-support.mjs';

const preference='{"campaignHelpPopupsOptionChecked": true}';
/** @param {any} f */
function createPreference(f) {
    const common=resolve(f.installation,'saves/common');mkdirSync(common);
    const path=resolve(common,'core_shared_settings.json.data');writeFileSync(path,preference);
    return {common,path};
}

test('common output remains forbidden by prelaunch, generation B1 and the legacy first-page profile',()=>{
    for(const options of [{generation:true},{inspection:true}]) {
        const f=campaignLaunchFixture(options);
        try {
            createPreference(f);
            assert.notEqual(f.admit().status,0);
            const b1=claimSeededLauncher(f,'early-common-output',false);
            assert.throws(()=>readBatchAuthorization(b1.source,{root:f.root,runID:b1.runID}),/empty first-page/);
        } finally {f.close();}
    }
});

test('common output is refused before Generate and before the first Continue, even with a valid E2 claim',async()=>{
    await withGenerationEntry('preference-too-early',async(f,h,effects)=>{
        let output=createPreference(f);
        assert.throws(()=>readBatchAuthorization(f.source,{root:f.root,runID:f.runID}),/empty first-page/);
        unlinkSync(output.path);rmdirSync(output.common);
        const next=await completeGeneratedNameEntry(f,h);
        claimBatchAuthorization(readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
        output=createPreference(f);
        assert.throws(()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID}),/known first Continue/);
        assert.deepEqual(effects,['left_click','left_click']);
    });
});

test('post-Continue observations refuse changed paths, contents, links, uncertain history and new input',async()=>{
    await withGenerationEntry('preference-refusals',async(f,h,effects)=>{
        const next=await completeGeneratedNameEntry(f,h);
        claimBatchAuthorization(readBatchAuthorization(next.source,{root:f.root,runID:next.runID}));
        const e2=seedInputSession(next,tool=>{effects.push(tool);createPreference(f);},
            {window:()=>({...seedTarget,windowId:21})});
        const admit=()=>readBatchAuthorization(next.source,{root:f.root,runID:next.runID});
        try {
            await e2.before();
            await e2.call('left_click',{target_window_id:21,target_app:'java.exe',focus_strategy:'strict',coordinate:[100,120]});
            const common=resolve(f.installation,'saves/common'),path=resolve(common,'core_shared_settings.json.data');
            assert.equal(readFileSync(path,'utf8'),preference);
            assert.doesNotThrow(admit);
            for(const bytes of ['{"campaignHelpPopupsOptionChecked":false}',preference+'\n','\uFEFF'+preference,
                '{"campaignHelpPopupsOptionChecked":false,"campaignHelpPopupsOptionChecked":true}',
                '{"campaignHelpPopupsOptionChecked":true,"other":true}','', 'x'.repeat(42)]) {
                writeFileSync(path,bytes);assert.throws(admit);writeFileSync(path,preference);
            }
            for(const extra of [resolve(f.installation,'saves/campaign.xml'),resolve(common,'extra')]) {
                writeFileSync(extra,'generated forbidden output');assert.throws(admit);unlinkSync(extra);
            }
            const extraDirectory=resolve(common,'extra-directory');mkdirSync(extraDirectory);
            assert.throws(admit);rmdirSync(extraDirectory);
            const other=resolve(common,'other.json');renameSync(path,other);
            assert.throws(admit);renameSync(other,path);
            renameSync(path,other);mkdirSync(path);
            assert.throws(admit);rmdirSync(path);renameSync(other,path);
            const hardLink=resolve(f.scenario,'preference-link');linkSync(path,hardLink);
            assert.throws(admit);unlinkSync(hardLink);
            const target=resolve(f.scenario,'generated-common-target');renameSync(common,target);
            symlinkSync(target,common,process.platform==='win32'?'junction':'dir');
            try {assert.throws(admit);} finally {unlinkSync(common);renameSync(target,common);}
            const ledgerPath=resolve(f.scenario,'guard-launch-history.private.json'),original=readFileSync(ledgerPath);
            /** @type {Array<(s:any)=>void>} */const changes=[
                s=>{s.uncertain=true;},s=>{s.pending.status='SENT';},s=>{s.pending.status='RESERVED';},
                s=>{s.pending=null;},s=>{s.pending.runID+='wrong';},s=>{s.pending.sourceSha256='0'.repeat(64);},
                s=>{s.pending.reservationID+='wrong';},s=>{s.pending.args.coordinate=[101,120];},
                s=>{s.pending.args.target_window_id=22;},s=>{delete s.pending.knownResultAtMs;},
                s=>{s.pending.knownResultAtMs=Date.now()+60000;},
                s=>{s.pending.sentAtMs=s.providerClock.startMs-1;},
            ];
            for(const change of changes) {
                const value=JSON.parse(original.toString('utf8'));change(value);
                writeFileSync(ledgerPath,JSON.stringify(value));assert.throws(admit);writeFileSync(ledgerPath,original);
            }
            const claimPath=resolve(f.scenario,'guard-E2-consumed.private.json'),claim=readFileSync(claimPath);
            const changed=JSON.parse(claim.toString('utf8'));changed.binding.runID+='wrong';
            writeFileSync(claimPath,JSON.stringify(changed));assert.throws(admit);writeFileSync(claimPath,claim);
            assert.throws(()=>readCampaignAuthorization(next.source,{root:f.root,runID:next.runID,input:true}),/empty first-page/);
            assert.doesNotThrow(admit);await e2.before();
            assert.deepEqual(effects,['left_click','left_click','left_click']);
            assert.equal(f.ledger().history.reservedActions,4);assert.equal(f.ledger().history.sentActions,4);
        } finally {await e2.embedded.server.close();}
    });
});
