// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {preStartHistoryFixture} from './seed-pre-start-history-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('both gates require absence of a Job descendant recorded only by the final validated source snapshot',()=>{
    const f=preStartHistoryFixture({finalSnapshotExtraPid:999});
    try {
        const b=f.launcher();let admitted=false;
        try{readBatchAuthorization(b.source,{root:f.root,runID:b.runID});admitted=true;}catch{}
        const ps=f.admit();
        assert.deepEqual({javascript:admitted,powershell:ps.status===0},{javascript:false,powershell:false},ps.stderr);
    }finally{f.close();}
});

test('portable history fixture contains no installed user-directory PowerShell path',()=>{
    const text=readFileSync(new URL('./seed-pre-start-history-test-support.mjs',import.meta.url),'utf8');
    assert.equal(/[A-Za-z]:[\\/]Users[\\/]/u.test(text),false,'No installed user-directory path belongs in the portable fixture');
});

test('portable positive discovers the current canonical PowerShell when the optional explicit override is absent',()=>{
    const prior=process.env.NF_TEST_POWERSHELL;delete process.env.NF_TEST_POWERSHELL;
    let f;
    try {
        f=preStartHistoryFixture();const b=f.launcher();
        const admitted=readBatchAuthorization(b.source,{root:f.root,runID:b.runID});
        assert.ok('campaign' in admitted.binding);const ps=f.admit();assert.equal(ps.status,0,ps.stderr);
    }finally{f?.close();if(prior===undefined)delete process.env.NF_TEST_POWERSHELL;else process.env.NF_TEST_POWERSHELL=prior;}
});
