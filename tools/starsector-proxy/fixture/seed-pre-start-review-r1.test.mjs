// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {resolve} from 'node:path';
import {preStartHistoryFixture} from './seed-pre-start-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

/** Coherent mutations occur before downstream hashes are sealed.
 * @type {[string,Parameters<typeof preStartHistoryFixture>[0]][]} */
const cases=[
    ['copied wizard capture time different from the actual provider original',{copiedFrameTimeOffsetMs:1}],
    ['E2 helper outside the qualified runtime',{e2Helper:resolve('synthetic-missing-unqualified-helper.exe')}],
    ['E2 without the actual maintained Job query',{e2Query:'UNVERIFIED'}],
    ['E2 terminal deadline beyond its original provider clock',{e2DeadlineOffsetMs:1}],
    ['PID query that omits the independently recorded owned game',{omitOwnedGame:true}],
    ['Skip dispatched after the original input reserve cutoff',{lastClickDelayMs:1100}],
    ['an arbitrary original caller command',{rootCommand:'ARBITRARY_COMMAND_OUTSIDE_PINNED_LAUNCH'}],
];
for(const [label,options] of cases)test('both production gates refuse '+label,()=>{
    const f=preStartHistoryFixture(options);try{
        const b=f.launcher();let admitted=false;
        try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});admitted=true;}catch{}
        const ps=f.admit();
        assert.deepEqual({javascript:admitted,powershell:ps.status===0},{javascript:false,powershell:false},ps.stderr);
    }finally{f.close();}
});
