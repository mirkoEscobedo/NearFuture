// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync,writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { createHash } from 'node:crypto';
import { custodyFixture } from './capture-custody-fixture.mjs';
import { loadCaptureConfiguration } from './capture-adapter.mjs';
const sha=(/** @type {Buffer} */ b)=>createHash('sha256').update(b).digest('hex');
test('compact caller pins do not replace verification of the complete configured runtime graph',()=>{
    const f=custodyFixture('runtime-pin-'),path=resolve(f.output,'configuration.private.json'),code=resolve(f.output,'pinned-runtime.js');
    writeFileSync(code,'export const x=1;\n');
    const configuration={schemaVersion:1,kind:'NATIVE99_CAPTURE_ADAPTER_CONFIGURATION_V1',admitted:f.admitted,
        pointerPath:resolve(f.output,'current.private.json'),runtimePins:[{path:code,sha256:sha(readFileSync(code))}]};
    writeFileSync(path,JSON.stringify(configuration)+'\n'); const pin=sha(readFileSync(path));
    assert.deepEqual(loadCaptureConfiguration(path,pin),configuration);
    writeFileSync(code,'export const x=2;\n');
    assert.throws(()=>loadCaptureConfiguration(path,pin),/CAPTURE_ADAPTER_RUNTIME_PIN_CHANGED/);
});
