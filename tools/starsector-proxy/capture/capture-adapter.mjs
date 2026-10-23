// @ts-check
import { readFileSync } from 'node:fs';
import { isAbsolute } from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual } from 'node:util';
import { serveComputerUseStdio } from './vendor-runtime.mjs';
import { createCaptureCustody } from './capture-custody.mjs';
import { createCaptureIntentIO } from './capture-intent-io.mjs';
import { readCaptureCompletion } from './capture-completion-reader.mjs';
/** @param {Buffer} value */const sha=value=>createHash('sha256').update(value).digest('hex');
/** @param {string} path @param {string} expectedSHA256 */
export function loadCaptureConfiguration(path,expectedSHA256) {
    const bytes=readFileSync(path);
    if(bytes.length>1024*1024 || !/^[a-f0-9]{64}$/i.test(expectedSHA256) || sha(bytes)!==expectedSHA256.toLowerCase())
        throw Error('EXACT_CAPTURE_CONFIGURATION_PIN_REQUIRED');
    const configuration=JSON.parse(bytes.toString('utf8'));
    if(configuration.schemaVersion!==1 || configuration.kind!=='NATIVE99_CAPTURE_ADAPTER_CONFIGURATION_V1')
        throw Error('EXACT_CAPTURE_ADAPTER_CONFIGURATION_REQUIRED');
    for(const pin of configuration.admitted.sourcePins)
        if(sha(readFileSync(pin.path))!==pin.sha256.toLowerCase()) throw Error('CAPTURE_ADAPTER_SOURCE_PIN_CHANGED');
    const runtimePins=configuration.runtimePins ?? [];
    if(!Array.isArray(runtimePins)) throw Error('CAPTURE_ADAPTER_RUNTIME_PINS_REQUIRED');
    for(const pin of runtimePins)
        if(typeof pin?.path!=='string' || !isAbsolute(pin.path) || !/^[a-f0-9]{64}$/i.test(pin.sha256??'') ||
            sha(readFileSync(pin.path))!==pin.sha256.toLowerCase()) throw Error('CAPTURE_ADAPTER_RUNTIME_PIN_CHANGED');
    return configuration;
}
/** Uses the qualified original stdio implementation; policy still comes from unchanged vendor environment.
 * @param {{configuration:any,session?:any,stdio?:any}} options */
export function serveCaptureStdio({configuration,session,stdio={}}) {
    const io=createCaptureIntentIO(configuration);
    const custody=createCaptureCustody({admitted:configuration.admitted,readIntent:io.readIntent});
    return serveComputerUseStdio({...custody,...(session?{session}: {})},stdio);
}
/** Filesystem-only commands. No desktop session is created here.
 * @param {string} mode @param {any} configuration @param {any} expected */
export function captureFileCommand(mode,configuration,expected) {
    if(mode==='publish') return createCaptureIntentIO(configuration).publish(expected);
    if(mode!=='read') throw Error('CLOSED_CAPTURE_FILE_COMMAND_REQUIRED');
    const intent=expected.intent ?? expected;
    const binding=configuration.admitted.bindings.find((/** @type {any} */ b)=>b.id===intent.bindingId && b.batchId===intent.batchId);
    const index=binding?.frames.findIndex((/** @type {any} */ s)=>s.id===intent.frameId)??-1;
    if(intent.profileId!==configuration.admitted.profileId || index<0 || !isDeepStrictEqual(expected.slot,binding.frames[index]) || expected.captureOrdinal!==index+1)
        throw Error('EXACT_ADMITTED_COMPLETION_SLOT_REQUIRED');
    if(!expected.intent) {
        const text=readFileSync(binding.frames[index].intentPath,'utf8');
        if(Buffer.byteLength(text)>262144 || expected.intentPath!==binding.frames[index].intentPath ||
            sha(Buffer.from(text))!==expected.intentSHA256) throw Error('EXACT_CALLER_INTENT_DIGEST_REQUIRED');
        const frozen=JSON.parse(text);
        for(const key of ['profileId','batchId','bindingId','frameId'])
            if(frozen[key]!==expected[key]) throw Error('EXACT_CALLER_INTENT_IDENTITY_REQUIRED');
        expected={...expected,intent:frozen,intentText:text};
    }
    return readCaptureCompletion(expected);
}
