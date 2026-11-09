// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {writeFileSync,readFileSync,linkSync} from 'node:fs';
import {postStartHistoryFixture} from './seed-post-start-history-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {json} from './seed-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';

/** @param {any} f */
function refused(f){
    let jsAccepted=false;try{jsAccepted=seedCreationActionLimit(f.plan,f.launchLease,f.scenario)===99;}catch{}
    const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);assert.equal(ps.status,1,ps.stderr);
    assert.match(ps.stderr,/post-Start|Shared post-Start|previous seed|Campaign authority/i);
    assert.deepEqual({jsAccepted,psAccepted:ps.status===0},{jsAccepted:false,psAccepted:false});
}
/** These independently chosen boundary violations are fully repinned through result and next history. */
const cases=/** @type {[string,(f:any)=>void][]} */([
    ['E2 UNKNOWN cannot become PASS',f=>f.mutateE2Terminal((/** @type {any} */v)=>{v.verdict='PASS';v.verifierStatus=0;v.signal=null;v.closeFailed=false;v.terminalBoundToChild=true;})],
    ['terminalBoundToChild singleton array is not false',f=>f.mutateE2Terminal((/** @type {any} */v)=>{v.terminalBoundToChild=[false];})],
    ['UNKNOWN verifier status singleton null is not null',f=>f.mutateE2Terminal((/** @type {any} */v)=>{v.verifierStatus=[null];})],
    ['a recorded absent PID cannot be omitted',f=>{
        const p=resolve(f.priorScenario,'owned-terminal-full-pid-absence.actual.private.json'),v=value(p);v.pids.pop();f.result.cleanup.exactRecordedPids=v.pids;json(p,v);}],
    ['an extra post call cannot be hidden by the index count',f=>writeFileSync(resolve(f.ctx.phases.E2.output,'call-029.private.json'),'{}')],
    ['a different provider source cannot borrow its begin receipt',f=>{
        const p=resolve(f.priorScenario,'driver-provider-calls.actual.private.json'),v=value(p);v.E2.begin.args.source=resolve(f.priorScenario,'B1-authorization.private.json');json(p,v);}],
    ['a singleton-array begin isError cannot be truthy-normalized',f=>{
        const p=resolve(f.priorScenario,'driver-provider-calls.actual.private.json'),v=value(p);v.E2.begin.raw.isError=[false];json(p,v);}],
    ['a renewed root deadline cannot be inferred from late inspection',f=>{
        const p=resolve(f.priorScenario,'driver-chronology.actual.private.json'),v=value(p);v.rootDeadlineRenewed=true;json(p,v);}],
    ['the final inspection cannot erase its original measured gap',f=>{
        const p=resolve(f.priorScenario,'driver-chronology.actual.private.json'),v=value(p);v.postCaptureToInspectionMs=0;json(p,v);}],
    ['a successful stability exit cannot carry the margin refusal',f=>{
        const p=resolve(f.priorScenario,'driver-metadata-calls.actual.private.json'),v=value(p);v.calls[0].raw.exit_code=0;json(p,v);
        const q=resolve(f.priorScenario,'driver-chronology.actual.private.json'),c=value(q);c.stabilityInvocation=v.calls[0];json(q,c);}],
    ['duplicate JSON terminal keys are refused despite a fresh digest',f=>{
        const p=resolve(f.priorScenario,'launch-caller-terminal.actual.private.json');writeFileSync(p,readFileSync(p,'utf8').replace('"exit_code":1','"exit_code":1,"exit_code":1'));}],
    ['invalid UTF8 terminal bytes are refused despite a fresh digest',f=>{
        const p=resolve(f.priorScenario,'launch-caller-terminal.actual.private.json');writeFileSync(p,Buffer.concat([readFileSync(p),Buffer.from([0xff])]));}],
    ['protected autosave maximum cannot be a singleton array',f=>{
        const p=resolve(f.priorScenario,'after-smoke-protection.private.json'),v=value(p);v.originalAutosave.maximumSlots=[3];f.result.protection=v;json(p,v);}],
    ['XML outputs cannot alias another file through a hardlink',f=>linkSync(f.closeout.evidence.unqualifiedOutput[0].path,resolve(f.priorScenario,'xml-hardlink.private'))],
    ['a created receipt cannot be smuggled into incomplete verification',f=>json(resolve(f.priorScenario,'E2-created-seed-inspected.private.json'),{})],
    ['a generic result cannot claim successful seed qualification',f=>{f.result.seedQualified=true;}],
]);
for(const [name,mutate] of cases)test(name,()=>{
    const f=postStartHistoryFixture();
    try{mutate(f);f.rebindLateRecords();refused(f);}finally{f.close();}
});
