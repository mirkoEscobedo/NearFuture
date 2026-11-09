// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {preContinueHistoryFixture} from './seed-pre-continue-history-test-support.mjs';

/** Each change is applied before the generated acyclic graph is pinned.
 * @type {Array<[string,any]>} */
const vectors=[
    ['unknown E1 outcome relabelled as PASS',{e1Terminal:{verdict:'PASS',verifierStatus:0,signal:null,closeFailed:false,terminalBoundToChild:true}}],
    ['invented E1 child stream',{e1Stdout:'{"ok":true}'}],
    ['array handle-closure flag',{header:(/** @type {any} */ h)=>{h.allOwnedHandlesTerminalReaped=[true];}}],
    ['qualified seed claim',{result:(/** @type {any} */ r)=>{r.seed.qualified=true;}}],
    ['invented Continue dispatch',{result:(/** @type {any} */ r)=>{r.continueSent=true;}}],
    ['refunded Generate reservation',{ledger:(/** @type {any} */ l)=>{l.history.reservedActions--;}}],
    ['uncertain result concealed by known ACK',{ledger:(/** @type {any} */ l)=>{l.uncertain=true;}}],
    ['coerced protection slots',{protection:(/** @type {any} */ p)=>{p.originalAutosave.maximumSlots=[3];}}],
    ['array geometry HWND',{geometry:(/** @type {any} */ g,/** @type {string} */ label)=>{if(label==='B1-PLAY')g.windowId=[g.windowId];}}],
    ['fresh pre-click owned bounds changed',{call:(/** @type {any} */ row,/** @type {string} */ batch,/** @type {number} */ ordinal)=>{
        if(batch==='E1'&&ordinal===13){const data=JSON.parse(row.raw.content[0].text);data.bounds.x+=10;row.raw={content:[{type:'text',text:JSON.stringify(data)}],structuredContent:data};}
    }}],
    ['array raw error flag',{call:(/** @type {any} */ row,/** @type {string} */ batch,/** @type {number} */ ordinal)=>{if(batch==='B1'&&ordinal===1)row.raw.isError=[false];}}],
    ['duplicate indexed path with recomputed counts',{index:(/** @type {any} */ i)=>{i.files.push({...i.files[0]});i.fileCount++;i.totalIndexedBytes+=i.files[0].bytes;}}],
    ['index authority claim',{index:(/** @type {any} */ i)=>{i.authorityIssuedByIndex=true;}}],
    ['array current context',{result:(/** @type {any} */ r)=>{r.currentRunContext.overallAttempts=[54];}}],
    ['omitted known preceding actions',{call:(/** @type {any} */ row,/** @type {string} */ batch,/** @type {number} */ ordinal)=>{if(batch==='E1'&&ordinal===17)row.post.precedingActions=[];}}],
    ['invented ledger activation field',{ledger:(/** @type {any} */ l)=>{l.activationCount=1;}}],
    ['different birth in the E1 owned claim',{phaseOwned:(/** @type {any} */ owned,/** @type {string} */ batch)=>{if(batch==='E1')owned.processes[0].kernelCreationFileTime='134371266963297184';}}],
    ['missing provider error flag',{providerResponse:(/** @type {any} */ _begin,/** @type {any} */ finish,/** @type {string} */ batch)=>{if(batch==='E1')delete finish.raw.isError;}}],
    ['unknown provider response fields',{providerResponse:(/** @type {any} */ _begin,/** @type {any} */ finish,/** @type {string} */ batch)=>{if(batch==='E1')finish.raw.retryAllowed=true;}}],
    ['duplicate raw error fields',{
        call:(/** @type {any} */ row,/** @type {string} */ batch,/** @type {number} */ ordinal)=>{if(batch==='B1'&&ordinal===1)row.raw.isError=false;},
        rawDocument:(/** @type {Buffer} */ bytes,/** @type {string} */ batch,/** @type {number} */ ordinal)=>batch==='B1'&&ordinal===1?Buffer.from(bytes.toString().replace('"isError":false','"isError":false,"isError":false')):undefined,
    }],
    ['duplicate embedded doctor fields',{call:(/** @type {any} */ row,/** @type {string} */ batch,/** @type {number} */ ordinal)=>{
        if(batch==='B1'&&ordinal===1)row.raw={content:[{type:'text',text:'{"ok":true,"ok":true,"checks":[]}'}],structuredContent:{ok:true,checks:[]}};
    }}],
    ['invalid UTF-8 in a coherently pinned raw packet',{
        call:(/** @type {any} */ row,/** @type {string} */ batch,/** @type {number} */ ordinal)=>{if(batch==='B1'&&ordinal===1){const data={ok:true,checks:[{summary:'\uFFFD'}]};row.raw={content:[{type:'text',text:JSON.stringify(data)}],structuredContent:data};}},
        rawDocument:(/** @type {Buffer} */ bytes,/** @type {string} */ batch,/** @type {number} */ ordinal)=>{if(batch!=='B1'||ordinal!==1)return;const at=bytes.indexOf(Buffer.from('\uFFFD'));assert.ok(at>0);return Buffer.concat([bytes.subarray(0,at),Buffer.from([255]),bytes.subarray(at+3)]);},
    }],
];
for(const [name,options] of vectors)test('both public history readers refuse '+name,()=>{
    const f=preContinueHistoryFixture(options);
    try{
        let refused=false;try{seedCreationActionLimit(f.plan,f.launchLease,f.scenario);}catch(error){refused=true;assert.match(/** @type {Error} */(error).message,/Campaign authority refused|PNG|geometry|client/i);}
        // Always execute the second real gate, including when the first refuses.
        const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({jsRefused:refused,psStatus:ps.status},{jsRefused:true,psStatus:1},ps.stderr);
        assert.match(ps.stderr,/closed|pre-Continue|scalar|integer|histor/i);
    }finally{f.close();}
});
