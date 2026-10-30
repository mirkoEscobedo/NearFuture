// @ts-check
// Closed historical file graph. This reader issues no authority and executes no recorded command.
import {resolve,dirname,isAbsolute} from 'node:path';
import {TextDecoder} from 'node:util';
import {closed,file,requireThat,inside} from './campaign-proof.mjs';
import {zeroProviderObject} from './seed-zero-provider-terminal.mjs';

export const preContinueBudgetHistoryKind='ROOT_CLOSED_PRE_CONTINUE_PROVIDER_BUDGET_FAILED_SEED_ATTEMPT_HISTORY';
export const budgetHistoryNeed=(/** @type {any} */ v)=>requireThat(v,'closed pre-Continue provider budget history required');
export const budgetHistoryInteger=(/** @type {any} */ v,min=0)=>Number.isSafeInteger(v)&&v>=min;
export const budgetHistoryObject=(/** @type {any} */ v)=>v!==null&&typeof v==='object'&&!Array.isArray(v);
export const budgetHistoryTime=(/** @type {any} */ v)=>typeof v==='string'&&Number.isSafeInteger(Date.parse(v));
const need=budgetHistoryNeed,integer=budgetHistoryInteger;
/** @param {any} pin */
export function budgetHistoryPin(pin){need(closed(pin,['path','sha256'])&&typeof pin.path==='string'&&isAbsolute(pin.path)&&resolve(pin.path)===pin.path&&
    typeof pin.sha256==='string'&&/^[a-f0-9]{64}$/.test(pin.sha256));return pin;}
/** @param {Buffer} bytes */
export function budgetHistoryJson(bytes){let text='';try{text=new TextDecoder('utf-8',{fatal:true}).decode(bytes);}catch{need(false);}return zeroProviderObject(text,need);}
/** @param {any} v */
export function readBudgetHistoryIndex(v){
    need(closed(v,['schemaVersion','kind','issuer','driver','scenario','sourceWorkspace','sourceHead','allOwnedHandlesTerminalReaped','actions','evidence'])&&
        v.schemaVersion===1&&v.kind===preContinueBudgetHistoryKind&&v.issuer==='/root'&&v.driver==='/root/recovery_game_driver'&&
        v.allOwnedHandlesTerminalReaped===true&&typeof v.scenario==='string'&&isAbsolute(v.scenario)&&resolve(v.scenario)===v.scenario&&
        typeof v.sourceWorkspace==='string'&&isAbsolute(v.sourceWorkspace)&&resolve(v.sourceWorkspace)===v.sourceWorkspace&&
        dirname(v.scenario)===resolve(v.sourceWorkspace,'.tmp/starsector-proxy/scenarios')&&typeof v.sourceHead==='string'&&/^[a-f0-9]{40}$/.test(v.sourceHead)&&
        closed(v.evidence,['index']));
    const ip=budgetHistoryPin(v.evidence.index),actual=file(ip.path,262144);need(actual.sha256===ip.sha256);
    const index=budgetHistoryJson(actual.bytes);
    need(index.scenario===v.scenario&&index.authorityIssuedByIndex===false&&index.operationalPermission===false&&
        index.newSourceQualifiedByIndex===false&&index.originalFileNamesPreserved===true&&Array.isArray(index.files)&&
        index.files.length>0&&index.files.length<=256&&index.fileCount===index.files.length);
    /** @type {Map<string,{bytes:Buffer,sha256:string}>} */const files=new Map();let total=0;
    for(const row of index.files){
        need(closed(row,['path','sha256','bytes'])&&integer(row.bytes)&&row.bytes<=32*1024*1024);
        const pin=budgetHistoryPin({path:row.path,sha256:row.sha256});need(!files.has(pin.path)&&
            (inside(resolve(v.sourceWorkspace,'.tmp/starsector-proxy'),pin.path)||
                row.path===index.originalSourceAndFixture?.sourceQualification?.path&&row.sha256===index.originalSourceAndFixture.sourceQualification.sha256&&row.bytes===index.originalSourceAndFixture.sourceQualification.bytes));
        const found=file(pin.path,32*1024*1024);need(found.bytes.length===row.bytes&&found.sha256===pin.sha256);
        total+=found.bytes.length;need(total<=64*1024*1024);files.set(pin.path,found);
    }
    need(index.totalIndexedBytes===total);
    /** @param {string} path */
    const bytes=path=>{need(typeof path==='string'&&resolve(path)===path&&files.has(path));return /** @type {{bytes:Buffer,sha256:string}} */(files.get(path));};
    /** @param {string} path */
    const ref=path=>({path,sha256:bytes(path).sha256});
    /** @param {string} path */
    const json=path=>budgetHistoryJson(bytes(path).bytes);
    /** @param {any} pin @param {string} [expected] */
    const pinned=(pin,expected=pin?.path)=>{budgetHistoryPin(pin);need(pin.path===expected&&ref(expected).sha256===pin.sha256);return bytes(expected);};
    /** @param {any} pin @param {string} [expected] */
    const pinJson=(pin,expected=pin?.path)=>{pinned(pin,expected);return json(expected);};
    return{index,reader:{bytes,ref,json,pinned,pinJson},v};
}
