// @ts-check
import test from 'node:test';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {exerciseSeedCreationTransport} from './seed-creation-transport-support.mjs';

test('stdio/HTTP resolves one initial activation before geometry and preserves all eight wizard click ordinals',async()=>{
    await exerciseSeedCreationTransport({fixture:initialFocusFixture,initialActivation:true});
});
