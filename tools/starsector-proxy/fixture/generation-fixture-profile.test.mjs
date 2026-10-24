// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {generationBindingFixture,readGenerationBinding} from './generation-fixture-test-support.mjs';
import {writeFileSync} from 'node:fs';
import {hash,json} from './seed-test-support.mjs';

test('an explicitly pinned vanilla generation binding selects its closed no-mods profile',()=>{
    const fixture=generationBindingFixture();
    try {
        const result=readGenerationBinding(fixture);
        assert.equal(result.status,0,result.stderr);
        const actual=JSON.parse(result.stdout);
        assert.equal(actual.schemaVersion,3);
        assert.equal(actual.fixtureProfile,'VANILLA_UNCOMPRESSED_GENERATION');
        assert.equal(actual.fixtureRoot,fixture.root);
        assert.equal(Object.hasOwn(actual,'recordingProfile'),false,'the new profile retains legacy recording');
    } finally {fixture.close();}
});

test('generation binding refuses duplicate profile declarations even with a refreshed digest',()=>{
    const fixture=generationBindingFixture();
    try {
        const text=JSON.stringify(fixture.binding).replace('"fixtureProfile":',
            '"fixtureProfile":"VANILLA_UNCOMPRESSED_GENERATION","fixtureProfile":');
        writeFileSync(fixture.path,text);fixture.sha256=hash(text);
        const result=readGenerationBinding(fixture);
        assert.notEqual(result.status,0);
        assert.match(result.stderr,/Duplicate generation binding field/);
    } finally {fixture.close();}
});

for(const value of [null,[],['VANILLA_UNCOMPRESSED_GENERATION'],'UNKNOWN'])test('generation binding refuses profile '+JSON.stringify(value),()=>{
    const fixture=generationBindingFixture();
    try {
        Reflect.set(fixture.binding,'fixtureProfile',value);fixture.sha256=json(fixture.path,fixture.binding).sha256;
        const result=readGenerationBinding(fixture);assert.notEqual(result.status,0);
        assert.match(result.stderr,/Closed vanilla generation fixture profile/);
    } finally {fixture.close();}
});

test('generation and legacy bindings cannot carry each other recording or fixture profile fields',()=>{
    const fixture=generationBindingFixture();
    try {
        Reflect.set(fixture.binding,'recordingProfileSha256','a'.repeat(64));
        fixture.sha256=json(fixture.path,fixture.binding).sha256;
        let result=readGenerationBinding(fixture);assert.notEqual(result.status,0);assert.match(result.stderr,/Closed fixture input schema/);
        Reflect.deleteProperty(fixture.binding,'recordingProfileSha256');fixture.binding.schemaVersion=1;
        fixture.sha256=json(fixture.path,fixture.binding).sha256;
        result=readGenerationBinding(fixture);assert.notEqual(result.status,0);assert.match(result.stderr,/Closed fixture input schema/);
    } finally {fixture.close();}
});

test('the explicit generation profile keeps legacy recording without a startup profile',()=>{
    const fixture=generationBindingFixture();
    try {
        const result=readGenerationBinding(fixture,
            "$profile=Read-NativeRecordingProfile $binding; if($null-ne$profile){throw 'Legacy recording expected'}; '{\"recording\":\"LEGACY\"}'");
        assert.equal(result.status,0,result.stderr);
        assert.deepEqual(JSON.parse(result.stdout),{recording:'LEGACY'});
    } finally {fixture.close();}
});

test('generation recipe and phase authority retain the legacy recording agreement',()=>{
    const fixture=generationBindingFixture();
    try {
        const result=readGenerationBinding(fixture,
            "$empty=[pscustomobject]@{}; Assert-NativeRecordingRecipe $binding $empty; "+
            "Assert-NativeRecordingAuthority $binding $empty $empty $empty; '{\"agreement\":\"LEGACY\"}'");
        assert.equal(result.status,0,result.stderr);
        assert.deepEqual(JSON.parse(result.stdout),{agreement:'LEGACY'});
    } finally {fixture.close();}
});

test('the generation preparation recipe enables no mods and requires uncompressed saves',()=>{
    const fixture=generationBindingFixture();
    try {
        const result=readGenerationBinding(fixture,'Read-NativeFixtureRecipe $binding | ConvertTo-Json');
        assert.equal(result.status,0,result.stderr);
        const actual=JSON.parse(result.stdout);
        assert.deepEqual(JSON.parse(actual.enabledModsJson),{enabledMods:[]});
        assert.equal(actual.requireUncompressed,true);
    } finally {fixture.close();}
});

test('generation configuration accepts uncompressed data and refuses compressed saves',()=>{
    const fixture=generationBindingFixture();
    try {
        const valid=readGenerationBinding(fixture,
            "Assert-NativeFixtureSettings $binding '{\"compressSaveGameData\":false}'; 'ACCEPTED'");
        assert.equal(valid.status,0,valid.stderr);
        assert.equal(valid.stdout.trim(),'ACCEPTED');
        const compressed=readGenerationBinding(fixture,
            "Assert-NativeFixtureSettings $binding '{\"compressSaveGameData\":true}'");
        assert.notEqual(compressed.status,0);
        assert.match(compressed.stderr,/Generation requires uncompressed save data/);
    } finally {fixture.close();}
});

test('generation settings preserve commented game syntax but require exactly one literal false compression flag',()=>{
    const fixture=generationBindingFixture();
    const quote=(/** @type {string} */ value)=>"'"+value.replaceAll("'","''")+"'";
    try {
        const accepted='{// disabled compression\n"compressSaveGameData":false, /* trailing comma */\n"note":"https://example/#value",}';
        const valid=readGenerationBinding(fixture,'Assert-NativeFixtureSettings $binding '+quote(accepted));
        assert.equal(valid.status,0,valid.stderr);
        for(const text of [
            '{/* "compressSaveGameData":false */}',
            '{"note":"compressSaveGameData:false"}',
            '{"compressSaveGameData":"false"}',
            '{"compressSaveGameData":false,"compressSaveGameData":true}',
            '{"compressSaveGameData":false,"CompressSaveGameData":false}',
            '{"CompressSaveGameData":false}',
            '[{"compressSaveGameData":false}]',
        ]) {
            const result=readGenerationBinding(fixture,'Assert-NativeFixtureSettings $binding '+quote(text));
            assert.notEqual(result.status,0,text);
            assert.match(result.stderr,/Generation requires uncompressed save data/,text);
        }
    } finally {fixture.close();}
});
