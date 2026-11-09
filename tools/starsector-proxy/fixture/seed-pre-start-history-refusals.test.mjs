// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {existsSync,mkdirSync} from 'node:fs';
import {preStartHistoryFixture} from './seed-pre-start-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';
import {ref,value} from './seed-failed-history-test-support.mjs';
import {json} from './seed-test-support.mjs';

/** @param {any} f */
function refused(f) {
    const b=f.launcher();assert.throws(()=>readBatchAuthorization(b.source,{root:f.root,runID:b.runID}));
    const ps=f.admit();assert.notEqual(ps.status,0,ps.stdout);
}
/** @param {any} f @param {string} name @param {(v:any)=>void} change */
function changeEvidence(f,name,change) {
    const r=value(f.resultPath),pin=r.evidence[name],v=value(pin.path);change(v);json(pin.path,v);
    r.evidence[name]=ref(pin.path);json(f.resultPath,r);f.closeout.evidence.result=ref(f.resultPath);
}
/** @type {[string,(f:any)=>void][]} */
const cases=[
    ['a refunded known action',f=>{f.closeout.actions.actualGuard.sentActions--;}],
    ['string terminal-reap authority',f=>{f.closeout.allOwnedHandlesTerminalReaped='true';}],
    ['missing pinned call',f=>{f.closeout.evidence.calls.E2.pop();}],
    ['a provider that lacks maintained ownership',f=>{changeEvidence(f,'E2/owned-provider-terminal.private.json',v=>{v.steps[0].processOwnership='unowned';});}],
    ['an error-flag string in a raw acknowledgement',f=>{
        const pin=f.closeout.evidence.calls.E2[18].raw,v=value(pin.path);v.isError='false';json(pin.path,v);
        f.closeout.evidence.calls.E2[18].raw=ref(pin.path);
    }],
    ['an ambiguous unacknowledged click',f=>{
        const pin=f.closeout.evidence.calls.E2[18].raw,v=value(pin.path);v.content[0].text='Unknown result';json(pin.path,v);
        f.closeout.evidence.calls.E2[18].raw=ref(pin.path);
    }],
    ['a successful seed claim on the failed run',f=>{
        const r=value(f.resultPath);r.claims.seed=true;json(f.resultPath,r);f.closeout.evidence.result=ref(f.resultPath);
    }],
    ['cleanup with an input side effect',f=>{changeEvidence(f,'launch-caller-terminal.actual.private.json',v=>{
        const stopped=JSON.parse(v.cleanupRequest.raw.output);stopped.extraInput=true;v.cleanupRequest.raw.output=JSON.stringify(stopped);
    });}],
    ['a launch timeout relabelled as an input-margin stop',f=>{changeEvidence(f,'launch-supervisor.private.json',v=>{v.result.steps[0].timedOut=true;});}],
    ['a final geometry child despite the prechild refusal',f=>{mkdirSync(resolve(f.priorScenario,'geometry-owned-E2-START_GAME'));}],
    ['a changed historical clock',f=>{changeEvidence(f,'plan.json',v=>{v.sessionWatchdogMs=720000;});}],
    ['a remapped historical point with stale geometry',f=>{
        const r=value(f.resultPath),pin=r.successfulGeometry[7].point,v=value(pin.path);v.computedAtMs+=6000;json(pin.path,v);
        r.successfulGeometry[7].point=ref(pin.path);json(f.resultPath,r);f.closeout.evidence.result=ref(f.resultPath);
    }],
];
for(const [label,change] of cases)test('both history gates refuse '+label,()=>{
    const f=preStartHistoryFixture();try{change(f);f.refresh();refused(f);}finally{f.close();}
});

test('both gates refuse a current source closure that omits the new historical proof module',()=>{
    const f=preStartHistoryFixture();try{
        const p=f.launchLease.supportPinsPath,pins=value(p),omitted=resolve(f.root,'tools/starsector-proxy/seed-pre-start-margin-records.mjs');
        assert.ok(existsSync(omitted));assert.ok(pins.pins.some((/** @type {any} */ row)=>row.path===omitted));
        pins.pins=pins.pins.filter((/** @type {any} */ row)=>row.path!==omitted);json(p,pins);
        f.launchLease.supportPinsSha256=f.phaseLease.supportPinsSha256=ref(p).sha256;f.refreshLaunch();refused(f);
    }finally{f.close();}
});

test('both gates refuse a grant issuer wrapped in an array after coherent rehashing',()=>{
    const f=preStartHistoryFixture();try{
        const grant=value(f.grantPath);grant.issuer=['/root'];json(f.grantPath,grant);
        for(const v of [f.plan,f.launchLease,f.phasePlan,f.phaseLease])v.historyAdmission=ref(f.grantPath);
        f.refreshLaunch();refused(f);
    }finally{f.close();}
});
