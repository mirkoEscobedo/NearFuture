// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {providerBudgetFixture} from './seed-provider-budget-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {json} from './seed-test-support.mjs';
import {ref} from './seed-failed-history-test-support.mjs';

for(const which of ['plan','lease'])for(const duplicate of [false,true])test('both actual readers '+(duplicate?'refuse duplicate ':'admit reclosed ')+which+' provider interval',()=>{
    const f=providerBudgetFixture();
    try{
        const path=which==='plan'?f.planPath:f.leasePath,text=readFileSync(path,'utf8'),needle='"providerTimeoutMs":480000';
        assert.equal(text.split(needle).length,2);writeFileSync(path,duplicate?text.replace(needle,needle+','+needle):text);
        if(which==='plan'){f.launchLease.planSha256=ref(f.planPath).sha256;json(f.leasePath,f.launchLease);}
        f.controller.rootLeaseSha256=ref(f.leasePath).sha256;json(f.controllerPath,f.controller);
        const cp=ref(f.controllerPath);f.campaign.rootClock.producerRecordKey='controller-start:'+cp.sha256;f.campaign.producerWitness.producerRecordKey=f.campaign.rootClock.producerRecordKey;
        Object.assign(f.predecessor,{plan:ref(f.planPath),lease:ref(f.leasePath),controller:cp});json(f.originalLaunchPath,f.predecessor);f.campaign.launchPredecessor=ref(f.originalLaunchPath);
        const ledgerPath=resolve(f.scenario,'guard-launch-history.private.json'),ledger=JSON.parse(readFileSync(ledgerPath,'utf8'));
        assert.equal(ledger.pending,null);assert.equal(ledger.uncertain,false);assert.equal(ledger.provider,undefined);
        ledger.controllerSha256=cp.sha256;json(ledgerPath,ledger);
        const b=f.launcher();let jsRefused=false,jsError='';
        try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});}catch(error){jsRefused=true;jsError=/** @type {Error} */(error).message;}
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({jsRefused,psStatus:ps.status},{jsRefused:duplicate,psStatus:duplicate?1:0},jsError+'\n'+ps.stderr);
        if(duplicate){assert.match(jsError,/duplicate|timing|JSON/i);assert.match(ps.stderr,/duplicate|timing|JSON/i);}
    }finally{f.close();}
});
