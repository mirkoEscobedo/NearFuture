// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {seedCreationFixture} from './seed-creation-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {readBatchAuthorization} from '../batch-authorization.mjs';

test('seed creation refuses an ineligible shared preference at both real launch boundaries',()=>{
    const f=seedCreationFixture();
    try {
        const value=JSON.parse(readFileSync(f.preferencePath,'utf8'));value.value='false';json(f.preferencePath,value);
        f.plan.seedCreation.preference.sha256=filePin(f.preferencePath).sha256.toLowerCase();f.refreshLaunch();
        const result=f.admit();assert.notEqual(result.status,0);assert.match(result.stderr,/preference/i);
        const first=f.launcher();assert.throws(()=>readBatchAuthorization(first.source,{root:f.root,runID:first.runID}),/preference/i);
    } finally {f.close();}
});
