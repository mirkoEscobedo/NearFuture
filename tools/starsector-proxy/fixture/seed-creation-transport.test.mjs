// @ts-check
import test from 'node:test';
import {exerciseSeedCreationTransport} from './seed-creation-transport-support.mjs';

test('stdio and HTTP create one disposable initial save through eight inspected actions and three providers',async()=>{
    await exerciseSeedCreationTransport();
});
