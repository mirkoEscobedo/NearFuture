// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {writeFileSync,readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {earlyWindowFixture} from './seed-early-window-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {json} from './seed-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';
/** @param {any} f @param {boolean} [expected] */
function paired(f,expected=false){
    let jsAccepted=false;try{jsAccepted=seedCreationActionLimit(f.plan,f.launchLease,f.scenario)===99;}catch{}
    const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
    assert.deepEqual({jsAccepted,psStatus:ps.status},{jsAccepted:expected,psStatus:expected?0:1},ps.stderr);
    if(!expected)assert.match(ps.stderr,/early window|history/i);
}
for(const [name,options] of /** @type {[string,any][]} */([
    ['array Java HWND',{handle:[19]}],['noncanonical Java HWND',{handle:'19.0'}],
    ['lost historical unsent reservation',{baseline:{overallAttempts:52,uiAttempts:49,reservedActions:91,sentActions:90}}],
]))test(name+' refuses as fully regenerated history',()=>{const f=earlyWindowFixture(options);try{paired(f);}finally{f.close();}});

const cases=/** @type {[string,(f:any)=>void][]} */([
    ['remediation true does not match this exact doctor prefix',f=>f.reviseCall(0,(/** @type {any} */ {post})=>{post.args={include_remediation:true};})],
    ['focused window cannot be described as this focus stop',f=>{f.result.focus.ownedWindowFocused=true;f.reviseCall(2,(/** @type {any} */ {raw})=>{
        raw.structuredContent.isFocused=true;raw.content[0].text=JSON.stringify(raw.structuredContent);});}],
    ['a foreign window cannot inherit owned Java custody',f=>{f.result.focus.ownedPid=999;f.reviseCall(2,(/** @type {any} */ {raw})=>{
        raw.structuredContent.pid=999;raw.content[0].text=JSON.stringify(raw.structuredContent);});}],
    ['MCP isError array is not false',f=>f.reviseCall(1,(/** @type {any} */ {raw})=>{raw.isError=[false];})],
    ['embedded duplicate doctor JSON is refused',f=>f.reviseCall(0,(/** @type {any} */ {raw})=>{raw.content[0].text=raw.content[0].text.replace('"ok":true','"ok":true,"ok":true');})],
    ['an unclaimed fourth vendor call is refused',f=>writeFileSync(resolve(f.output,'call-004.private.json'),'{}')],
    ['an unclaimed PNG cannot accompany zero originals',f=>writeFileSync(resolve(f.output,'unexpected.png'),Buffer.from([1,2,3]))],
    ['the exact eight-PID union cannot omit a PID',f=>{const p=resolve(f.priorScenario,'owned-terminal-pid-absence.actual.private.json'),v=value(p);
        v.queriedExactRecordedPids.pop();f.result.cleanup.allExactRecordedPids=v.queriedExactRecordedPids;json(p,v);}],
    ['the observedAtUtc field cannot be silently renamed',f=>{const p=resolve(f.priorScenario,'owned-terminal-pid-absence.actual.private.json'),v=value(p);
        v.atUtc=v.observedAtUtc;delete v.observedAtUtc;json(p,v);}],
    ['root terminal zero exit cannot borrow planned cleanup failure',f=>{const p=resolve(f.priorScenario,'launch-caller-terminal.actual.private.json'),v=value(p);
        v.actual.exit_code=0;v.launchPolls[v.launchPolls.length-1]=v.actual;json(p,v);f.result.cleanup.actualOuterExit=0;}],
    ['invalid UTF8 caller bytes retain their failure despite fresh pins',f=>{const p=resolve(f.priorScenario,'launch-caller-terminal.actual.private.json');
        writeFileSync(p,Buffer.concat([readFileSync(p),Buffer.from([255])]));}],
    ['a later E1 claim cannot be hidden behind the early stop',f=>json(resolve(f.priorScenario,'guard-E1-consumed.private.json'),{})],
    ['the generic original cannot claim successful seed acceptance',f=>{f.result.seedAccepted=true;}],
]);
for(const [name,change] of cases)test(name,()=>{const f=earlyWindowFixture();try{change(f);f.refresh();paired(f);}finally{f.close();}});
test('doctor earlier diagnostic may name the owned Java without becoming a contemporaneous foreground observation',()=>{
    const f=earlyWindowFixture({diagnosticOwnedJava:true});try{paired(f,true);}finally{f.close();}
});
