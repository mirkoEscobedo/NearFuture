// @ts-check
import test from 'node:test';
import {exerciseWizardTransport} from './wizard-transport-test-support.mjs';

test('truthful carried history leaves only four clicks under the immutable absolute72 ceiling', async () => {
    await exerciseWizardTransport(true, {commonPreference: true, remainingBudget: true});
});
