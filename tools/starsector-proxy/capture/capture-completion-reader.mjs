// @ts-check
import { readFileSync, lstatSync } from 'node:fs';
import { resolve, dirname } from 'node:path';
import { createHash } from 'node:crypto';
import { isDeepStrictEqual as same } from 'node:util';
/** @param {Buffer|string} value */ const sha = value => createHash('sha256').update(value).digest('hex');
/** @param {boolean} ok @param {string} code */ function check(ok,code) { if(!ok) throw Error(code); }
/** @param {string} path @param {number} max */
function read(path,max) {
    let part=resolve(path);
    while(true) { check(!lstatSync(part).isSymbolicLink(),'ORDINARY_COMPLETION_PATH_REQUIRED'); const next=dirname(part); if(next===part) break; part=next; }
    const stat=lstatSync(path);
    check(stat.isFile() && stat.size>0 && stat.size<=max,'BOUNDED_COMPLETION_FILE_REQUIRED');
    const bytes=readFileSync(path); check(bytes.length<=max,'BOUNDED_COMPLETION_BYTES_REQUIRED'); return bytes;
}
/** Filesystem readback of one completed callback; no native call or authority grant.
 * Expected intent is the exact pre-dispatch caller value, not read back from a pointer.
 * @param {{intent:any,intentText:string,slot:any,inlineImageSHA256:string,inlineImageCharacters:number,
 * nonImageContent:any[],captureOrdinal:number,captureStartMs:number,captureEndMs:number}} expected */
export function readCaptureCompletion(expected) {
    const {intent,intentText,slot}=expected;
    check(typeof intentText==='string' && Buffer.byteLength(intentText)<=262144 &&
        same(JSON.parse(intentText),intent),'EXACT_CALLER_FROZEN_INTENT_REQUIRED');
    const proofBytes=read(slot.callbackReceipt,1024*1024), proof=JSON.parse(proofBytes.toString('utf8'));
    check(proof.schemaVersion===1 && proof.kind==='NATIVE99_CAPTURE_CALLBACK_COMPLETION_V1' &&
        proof.status==='PERSISTED_ORIGINAL_RECEIPT_ONLY' && proof.captureReceipt===true && proof.freshness==='UNKNOWN' &&
        proof.profileId===intent.profileId && proof.batchId===intent.batchId && proof.bindingId===intent.bindingId &&
        proof.frameId===intent.frameId && proof.frameId===slot.id &&
        resolve(proof.intentPath)===resolve(slot.intentPath) && proof.intentSha256===sha(intentText) &&
        proof.frozenIntentUtf8Base64===Buffer.from(intentText).toString('base64') &&
        same(proof.sourcePins,intent.sourcePins) && same(proof.callerRootClock,intent.rootClock) &&
        same(proof.callerBatchClock,intent.batchClock) && same(proof.callerOwnedWitness,intent.ownedWitness) &&
        proof.rootRecordSerializedSHA256===sha(JSON.stringify(intent.rootCallRecord)) &&
        proof.ownershipEvidence==='CALLER_ORIGIN_CURRENT_WITNESS_NOT_INDEPENDENT_HOST_ATTESTATION',
        'EXACT_CALLBACK_INTENT_PROOF_REQUIRED');
    check((typeof proof.protocolRequestID==='string' && proof.protocolRequestID.length<=256) ||
        Number.isSafeInteger(proof.protocolRequestID),'ACTUAL_CALLBACK_REQUEST_ID_REQUIRED');
    check(Number.isSafeInteger(expected.captureStartMs) && Number.isSafeInteger(expected.captureEndMs) &&
        expected.captureStartMs<=proof.startMs && proof.startMs<=proof.endMs && proof.endMs<=expected.captureEndMs &&
        proof.timingScope==='END_MS_SAMPLED_BEFORE_COMPLETION_RECEIPT_WRITE','EXACT_CALLER_CAPTURE_INTERVAL_REQUIRED');
    check(read(slot.intentPath,262144).equals(Buffer.from(intentText)),'IMMUTABLE_INTENT_READBACK_REQUIRED');
    check(Number.isSafeInteger(expected.captureOrdinal) && expected.captureOrdinal>=1 && expected.captureOrdinal<=6,
        'EXACT_DECLARED_CAPTURE_ORDINAL_REQUIRED');
    const label='capture-'+String(expected.captureOrdinal).padStart(3,'0');
    check(resolve(proof.png)===resolve(slot.png) && resolve(proof.payload)===resolve(slot.payload) &&
        resolve(proof.rawPath)===resolve(dirname(slot.png),label+'.raw.private.json') &&
        resolve(proof.metadataPath)===resolve(dirname(slot.png),label+'.metadata.private.json'),
        'EXACT_CALLBACK_ARTIFACT_PATHS_REQUIRED');
    const rawBytes=read(proof.rawPath,32*1024*1024), metadataBytes=read(proof.metadataPath,1024*1024),
        png=read(slot.png,4*1024*1024), payload=read(slot.payload,4194305);
    const raw=JSON.parse(rawBytes.toString('utf8')), metadata=JSON.parse(metadataBytes.toString('utf8')),
        images=raw.content?.filter((/** @type {any} */ block)=>block.type==='image');
    check(sha(rawBytes)===proof.rawSha256 && sha(metadataBytes)===proof.metadataSha256 &&
        sha(png)===proof.sha256 && sha(payload)===proof.payloadTextSHA256 &&
        metadata.captureReceipt===true && metadata.freshness==='UNKNOWN' &&
        metadata.rawSha256===proof.rawSha256 && metadata.pngSha256===proof.sha256 &&
        resolve(metadata.rawPath)===resolve(proof.rawPath) && resolve(metadata.pngPath)===resolve(slot.png) &&
        same(metadata.args,intent.args) && same(metadata.window,intent.window) &&
        metadata.intentSha256===proof.intentSha256 && metadata.protocolRequestID===proof.protocolRequestID &&
        same(metadata.sourcePins,intent.sourcePins),'EXACT_CALLBACK_ARTIFACT_HASHES_REQUIRED');
    check(raw.isError!==true && images?.length===1 && images[0].mimeType==='image/png' &&
        same(raw.content.filter((/** @type {any} */ block)=>block.type!=='image'),expected.nonImageContent),
        'EXACT_NATIVE_IMAGE_RESULT_REQUIRED');
    const canonical=png.toString('base64');
    check(canonical===images[0].data && payload.toString('utf8')===canonical+'\n' &&
        canonical.length===expected.inlineImageCharacters && canonical.length===proof.canonicalBase64Characters &&
        canonical.length>0 && canonical.length<=4194304 && sha(canonical)===proof.canonicalBase64SHA256 &&
        proof.canonicalBase64SHA256===expected.inlineImageSHA256 && proof.bytes===png.length &&
        proof.payloadTextCharacters===payload.length && proof.originalPayloadRoundtripExact===true,
        'EXACT_CALLER_INLINE_IMAGE_DIGEST_REQUIRED');
    check(proof.width===intent.window.bounds.width && proof.height===intent.window.bounds.height &&
        metadata.dimensions?.width===proof.width && metadata.dimensions?.height===proof.height &&
        png.subarray(0,8).toString('hex')==='89504e470d0a1a0a' && png.readUInt32BE(16)===proof.width &&
        png.readUInt32BE(20)===proof.height,'EXACT_ORIGINAL_DIMENSIONS_REQUIRED');
    const {frozenIntentUtf8Base64,callerOwnedWitness,...compact}=proof;
    return {...compact,kind:'NATIVE99_CAPTURE_COMPLETION_READBACK_V1',callbackReceipt:slot.callbackReceipt,
        callbackReceiptSHA256:sha(proofBytes),inlineImageDigestMatched:true,nonImageContentMatched:true,
        independentlyObservedProtocolRequestID:false,wholeNativeAcceptance:false};
}
