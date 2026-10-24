// @ts-check
import test from 'node:test';
import {exerciseWizardTransport} from './wizard-transport-test-support.mjs';

test('actual stdio and HTTP compose generation Play, New Game, Generate and first Continue across three known-closed providers with inherited history', async () => {
    await exerciseWizardTransport(true);
});
