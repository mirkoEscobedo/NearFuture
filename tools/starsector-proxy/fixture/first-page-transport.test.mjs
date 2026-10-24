// @ts-check
import test from 'node:test';
import {exerciseWizardTransport} from './wizard-transport-test-support.mjs';

test('actual finite stdio and HTTP compose first-page Play and one inspected New Game click with the original clipped clock', async () => {
    await exerciseWizardTransport(false);
});
