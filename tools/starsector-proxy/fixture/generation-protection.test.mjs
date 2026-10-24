// @ts-check
// Generated boundary files only; no game, source installation or native provenance.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {filePin} from './seed-test-support.mjs';
import {generationProtectedFixture as fixture} from './generation-fixture-test-support.mjs';

test('the real protection boundary accepts a vanilla uncompressed generation fixture with originals unchanged',()=>{
    const f=fixture();
    try {
        const original=filePin(resolve(f.original,'starsector-core/data/config/settings.json'));
        const result=f.protection();assert.equal(result.status,0,result.stderr);
        const evidence=JSON.parse(readFileSync(resolve(f.admission,'after-smoke-protection.private.json'),'utf8'));
        assert.equal(evidence.protectedOriginalFiles,182);assert.equal(evidence.protectedSaveFiles,178);
        assert.equal(evidence.privateAutosaveEnabled,false);assert.equal(evidence.privateSaveEntries,0);
        assert.equal(evidence.liveGameProcesses,0);assert.equal(evidence.originalWrites,false);
        assert.deepEqual(filePin(resolve(f.original,'starsector-core/data/config/settings.json')),original);
    } finally {f.close();}
});

for(const [label,compressed,mods,reason] of /** @type {[string,boolean,string[],RegExp][]} */ ([
    ['compressed save data',true,[],/Generation requires uncompressed save data/],
    ['an enabled mod',false,['near_future'],/Generation requires no enabled mods/],
])) test('generation protection refuses '+label+' even with refreshed receipt hashes',()=>{
    const f=fixture();
    try {
        f.configure(compressed,mods);
        const result=f.protection();assert.notEqual(result.status,0,result.stdout);
        assert.match(result.stderr,reason);
    } finally {f.close();}
});
