// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {providerBudgetFixture} from './seed-provider-budget-test-support.mjs';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {preContinueHistoryFixture} from './seed-pre-continue-history-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';
import {json,filePin} from './seed-test-support.mjs';

for(const activation of [false,true])test('eight-minute original providers complete and stage eight clicks plus '+Number(activation)+' activation',async()=>{
    const f=await modernSeedFixture({fixture:providerBudgetFixture,initialActivation:activation,callerBudgetMs:1740000,expectedProviderTimeoutMs:480000});
    try{
        const sha=filePin(f.closeoutPath).sha256.toLowerCase(),js=inspectSeedCreationCustody(f.source.scenario,sha),ledger=JSON.parse(readFileSync(f.ledgerPath,'utf8'));
        assert.equal(js.inputs,8);assert.equal(js.activations,Number(activation));
        for(const clock of [ledger.resolvedB1.providerClock,ledger.resolvedE1.providerClock,ledger.providerClock])assert.equal(clock.deadlineMs-clock.startMs,480000);
        assert.deepEqual(f.closeout.actions.actualGuard,{overallAttempts:53,uiAttempts:50,reservedActions:99+Number(activation),sentActions:97+Number(activation)});
        if(!activation){
            const history=preContinueHistoryFixture(),original=readFileSync(f.closeoutPath);
            try{
                json(f.closeoutPath,history.closeout);f.manifest.provenance.closeout=filePin(f.closeoutPath);f.refresh();
                let refused=false;try{inspectSeedCreationCustody(f.source.scenario,filePin(f.closeoutPath).sha256.toLowerCase());}catch{refused=true;}
                const ps=f.target.stage();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
                assert.deepEqual({jsRefused:refused,psStatus:ps.status},{jsRefused:true,psStatus:1},ps.stderr);
                assert.match(ps.stderr,/seed|closeout|schema/i);assert.equal(existsSync(resolve(f.target.target,'seed-stage-claim.private.json')),false);
            }finally{history.close();writeFileSync(f.closeoutPath,original);f.manifest.provenance.closeout=filePin(f.closeoutPath);f.refresh();}
        }
        const staged=f.target.stage();assert.equal(staged.error,undefined);assert.equal(staged.signal,null);assert.equal(staged.status,0,staged.stderr);
    }finally{f.close();}
});
