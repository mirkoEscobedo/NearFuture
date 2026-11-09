// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {postStartHistoryFixture} from './seed-post-start-history-test-support.mjs';
import {seedCreationActionLimit} from '../seed-creation-history.mjs';
import {resolve} from 'node:path';
import {json} from './seed-test-support.mjs';
import {value} from './seed-failed-history-test-support.mjs';

/** Both public gates must reject independently, including when the first rejects. @param {any} f */
function refused(f){
    let jsAccepted=false;try{jsAccepted=seedCreationActionLimit(f.plan,f.launchLease,f.scenario)===99;}catch{}
    const ps=f.admit();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);assert.equal(ps.status,1,ps.stderr);
    assert.match(ps.stderr,/post-Start|Shared post-Start|previous seed|Campaign authority/i);
    assert.deepEqual({jsAccepted,psAccepted:ps.status===0},{jsAccepted:false,psAccepted:false});
}

test('a coherently pinned disabled Start control cannot carry acknowledged input as an admitted history',()=>{
    const f=postStartHistoryFixture({startEnabled:false});
    try{
        refused(f);
    }finally{f.close();}
});

test('a freshly hash-closed arbitrary root caller is not the bounded recorded launch',()=>{
    const f=postStartHistoryFixture({rootCommand:"Write-Output 'unbounded root command'"});
    try{refused(f);}finally{f.close();}
});

test('an extra command in the coherently linked stability invocation is refused',()=>{
    const f=postStartHistoryFixture();
    try{
        const path=resolve(f.priorScenario,'driver-metadata-calls.actual.private.json'),metadata=value(path);
        metadata.calls[0].args.cmd+='; Write-Output extra';json(path,metadata);
        const chronologyPath=resolve(f.priorScenario,'driver-chronology.actual.private.json'),chronology=value(chronologyPath);
        chronology.stabilityInvocation=metadata.calls[0];json(chronologyPath,chronology);f.rebindLateRecords();refused(f);
    }finally{f.close();}
});

test('a coherently linked different launched child cannot borrow the root terminal failure',()=>{
    const f=postStartHistoryFixture();
    try{
        const path=resolve(f.priorScenario,'launch-supervisor.private.json'),spool=value(path);
        spool.result.steps[0].args[3]=resolve(f.ctx.root,'tools/starsector-proxy/fixture/other-child.ps1');json(path,spool);
        const callerPath=resolve(f.priorScenario,'launch-caller-terminal.actual.private.json'),caller=value(callerPath);
        const step=spool.result.steps[0];
        caller.output=JSON.stringify({verdict:'FAIL',blocked:[],steps:{status:step.status,timedOut:step.timedOut,processOwnership:step.processOwnership,stdout:step.stdout,stderr:step.stderr}});json(callerPath,caller);
        f.result.cleanup.rootActualTerminal=caller;f.rebindLateRecords();refused(f);
    }finally{f.close();}
});

test('a hash-closed phase plan must be the exact bounded root spread',()=>{
    const f=postStartHistoryFixture({phasePlanExtra:{unadmittedAuthority:true}});
    try{refused(f);}finally{f.close();}
});

test('a fully pinned Start ACK cannot borrow a different phase source',()=>{
    const f=postStartHistoryFixture({startSourceSha:'f'.repeat(64)});
    try{refused(f);}finally{f.close();}
});
