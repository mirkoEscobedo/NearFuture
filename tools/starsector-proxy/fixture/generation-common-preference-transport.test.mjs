// @ts-check
import test from 'node:test';
import {exerciseWizardTransport} from './wizard-transport-test-support.mjs';

test('known first Continue may create the fixed common UI preference before original AFTER observations', async () => {
    await exerciseWizardTransport(true, {commonPreference: true});
});
