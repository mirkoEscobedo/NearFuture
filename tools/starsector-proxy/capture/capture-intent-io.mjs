// @ts-check
import { readFileSync, writeFileSync, renameSync, lstatSync } from 'node:fs';
import { resolve, dirname, relative, isAbsolute } from 'node:path';
import { createHash } from 'node:crypto';
/** @param {Buffer|string} value */ const sha=value=>createHash('sha256').update(value).digest('hex');
/** @param {boolean} ok @param {string} code */ function check(ok,code) { if(!ok) throw Error(code); }
/** @param {string} path */
function ordinary(path) {
    let part=resolve(path);
    while(true) {check(!lstatSync(part).isSymbolicLink(),'ORDINARY_INTENT_PATH_REQUIRED');const next=dirname(part);if(next===part)return;part=next;}
}
/** Root selects pointer and admitted slots before execution; these are DATA, not native authority.
 * @param {{admitted:any,pointerPath:string}} options */
export function createCaptureIntentIO({admitted,pointerPath}) {
    const config=JSON.parse(JSON.stringify(admitted)), pointer=resolve(pointerPath);
    const suffix=relative(resolve(config.workspaceRoot,'.tmp'),pointer);
    check(suffix.length>0 && !suffix.startsWith('..') && !isAbsolute(suffix),'PRIVATE_ROOT_SELECTED_POINTER_REQUIRED');
    ordinary(dirname(pointer));
    /** @param {any} intent */
    function slotFor(intent) {
        check(intent?.schemaVersion===1 && intent.kind==='NATIVE99_CAPTURE_INTENT_V1' &&
            intent.profileId===config.profileId,'EXACT_INTENT_PROFILE_REQUIRED');
        const rows=config.bindings.filter((/** @type {any} */ b)=>b.id===intent.bindingId && b.batchId===intent.batchId);
        const slots=rows.length===1?rows[0].frames.filter((/** @type {any} */ s)=>s.id===intent.frameId):[];
        check(slots.length===1,'ONE_DECLARED_INTENT_SLOT_REQUIRED'); return slots[0];
    }
    function pointerBytes() {
        ordinary(pointer);const bytes=readFileSync(pointer);
        check(bytes.length>0 && bytes.length<=16384,'BOUNDED_CURRENT_POINTER_REQUIRED');return bytes;
    }
    /** @param {{intentText:string,expectedPointerSHA256:string}} options */
    function publish({intentText,expectedPointerSHA256}) {
        check(typeof intentText==='string' && Buffer.byteLength(intentText)>0 && Buffer.byteLength(intentText)<=262144 &&
            /^[a-f0-9]{64}$/.test(expectedPointerSHA256),'BOUNDED_FROZEN_INTENT_REQUIRED');
        const intent=JSON.parse(intentText),slot=slotFor(intent),before=pointerBytes();
        check(sha(before)===expectedPointerSHA256,'EXACT_PREVIOUS_POINTER_REQUIRED');
        const prior=JSON.parse(before.toString('utf8'));
        check(prior.schemaVersion===1 && prior.kind==='NATIVE99_CAPTURE_POINTER_V1' &&
            prior.profileId===config.profileId && ['UNSET','SELECTED'].includes(prior.status),'EXACT_CURRENT_POINTER_PROFILE_REQUIRED');
        ordinary(dirname(slot.intentPath));
        const bytes=Buffer.from(intentText),intentSHA256=sha(bytes);
        writeFileSync(slot.intentPath,bytes,{flag:'wx'});
        check(readFileSync(slot.intentPath).equals(bytes),'EXACT_IMMUTABLE_INTENT_READBACK_REQUIRED');
        const selected={schemaVersion:1,kind:'NATIVE99_CAPTURE_POINTER_V1',status:'SELECTED',profileId:intent.profileId,
            batchId:intent.batchId,bindingId:intent.bindingId,frameId:intent.frameId,intentPath:slot.intentPath,intentSHA256};
        const next=Buffer.from(JSON.stringify(selected)+'\n'),temporary=pointer+'.'+intentSHA256+'.next.private.json';
        writeFileSync(temporary,next,{flag:'wx'});
        check(readFileSync(temporary).equals(next) && sha(pointerBytes())===expectedPointerSHA256,'POINTER_HANDOFF_PRECONDITION_REQUIRED');
        renameSync(temporary,pointer);
        check(pointerBytes().equals(next),'EXACT_CURRENT_POINTER_READBACK_REQUIRED');
        return {...selected,
            kind:'NATIVE99_CAPTURE_INTENT_PUBLICATION_V1',pointerPath:pointer,pointerSHA256:sha(next),
            intentBytes:bytes.length,immutableIntentReadbackExact:true,atomicSameDirectoryRename:true,
            crashDurabilityAttested:false,nativeAuthorityGranted:false};
    }
    function readIntent() {
        const selected=JSON.parse(pointerBytes().toString('utf8'));
        check(selected.schemaVersion===1 && selected.kind==='NATIVE99_CAPTURE_POINTER_V1' && selected.status==='SELECTED','CURRENT_CAPTURE_POINTER_REQUIRED');
        const slot=slotFor({...selected,kind:'NATIVE99_CAPTURE_INTENT_V1'});
        check(resolve(selected.intentPath)===resolve(slot.intentPath),'EXACT_POINTER_INTENT_PATH_REQUIRED');
        ordinary(slot.intentPath);const bytes=readFileSync(slot.intentPath);
        check(bytes.length>0 && bytes.length<=262144 && sha(bytes)===selected.intentSHA256,'FROZEN_SELECTED_INTENT_BYTES_REQUIRED');
        const intent=JSON.parse(bytes.toString('utf8'));
        check(intent.profileId===selected.profileId && intent.batchId===selected.batchId &&
            intent.bindingId===selected.bindingId && intent.frameId===selected.frameId,'POINTER_INTENT_IDENTITY_REQUIRED');
        return {path:slot.intentPath,bytes};
    }
    return {publish,readIntent};
}
