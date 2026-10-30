// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {writeFileSync,readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';
import {filePin} from './seed-test-support.mjs';

test('modern wizard provenance stages the exact disposable XML through the real consumer with its complete source inventory',async()=>{
    const f=await modernSeedFixture();
    try {
        writeFileSync(f.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
        const pins=readFileSync(resolve(f.target.target,'source-pins.private.json'));
        assert.ok(pins.length>131072&&pins.length<=262144,'the actual complete inventory exercises the modern bound');
        assert.equal(inspectSeedCreationCustody(f.source.scenario,filePin(f.closeoutPath).sha256.toLowerCase()).ok,true);
        const result=f.target.stage();assert.equal(result.status,0,result.stderr);
        for(const row of f.manifest.files)assert.deepEqual(readFileSync(resolve(f.target.installation,'saves',f.manifest.saveDirectory,row.relative)),
            readFileSync(resolve(f.source.installation,'saves',f.manifest.saveDirectory,row.relative)));
    } finally {f.close();}
});
