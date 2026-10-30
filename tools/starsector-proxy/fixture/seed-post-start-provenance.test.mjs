// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {postStartHistoryFixture} from './seed-post-start-history-test-support.mjs';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';

test('saved campaign with incomplete verification is not completed provenance in either real seed consumer',async()=>{
    const source=await modernSeedFixture(),history=postStartHistoryFixture();
    try{
        writeFileSync(source.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
        json(source.closeoutPath,history.closeout);source.manifest.provenance.closeout=filePin(source.closeoutPath);source.refresh();
        assert.throws(()=>inspectSeedCreationCustody(source.source.scenario,filePin(source.closeoutPath).sha256.toLowerCase()));
        const result=source.target.stage();assert.equal(result.error,undefined);assert.equal(result.signal,null);assert.equal(result.status,1,result.stderr);
        assert.match(result.stderr,/closed seed|closeout/i);assert.equal(existsSync(resolve(source.target.target,'seed-stage-claim.private.json')),false);
    }finally{history.close();source.close();}
});
