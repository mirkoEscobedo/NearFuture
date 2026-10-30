// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {inflateSync,deflateSync,crc32} from 'node:zlib';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {completePng} from '../direct-frames.mjs';

/** Change one pixel while retaining dimensions and a valid complete PNG.
 * @param {Buffer} png */
function replacementPng(png) {
    const chunks=[];let offset=8;
    while(offset<png.length){const length=png.readUInt32BE(offset),end=offset+length+12;
        chunks.push({type:png.toString('ascii',offset+4,offset+8),data:png.subarray(offset+8,end-4)});offset=end;}
    const pixels=inflateSync(Buffer.concat(chunks.filter(c=>c.type==='IDAT').map(c=>c.data)));pixels[1]^=1;
    let emitted=false;
    const result=[png.subarray(0,8),...chunks.flatMap(c=>{
        if(c.type==='IDAT'&&emitted)return [];if(c.type==='IDAT'){emitted=true;c={type:c.type,data:deflateSync(pixels)};}
        const body=Buffer.concat([Buffer.from(c.type),c.data]),head=Buffer.alloc(4),tail=Buffer.alloc(4);
        head.writeUInt32BE(c.data.length);tail.writeUInt32BE(crc32(body));return [Buffer.concat([head,body,tail])];})];
    const changed=Buffer.concat(result);assert.deepEqual(completePng(changed),completePng(png));assert.notDeepEqual(changed,png);return changed;
}

test('completed seed images remain tied to actual capture and input chronology',async t=>{
    const f=await modernSeedFixture();
    try {
        const closeoutHash=filePin(f.closeoutPath).sha256.toLowerCase();
        assert.equal(inspectSeedCreationCustody(f.source.scenario,closeoutHash).ok,true);
        const metadataPath=resolve(f.source.scenario,'E2-created-seed/capture-001.metadata.private.json');
        const finalPath=resolve(f.source.scenario,'E2-created-seed-inspected.private.json');
        const metadataBytes=readFileSync(metadataPath),finalBytes=readFileSync(finalPath);
        await t.test('a substituted earlier proof time cannot hide a late actual capture',()=>{
            try {
                const metadata=JSON.parse(metadataBytes.toString()),final=JSON.parse(finalBytes.toString());
                metadata.capturedAtMs+=480001;json(metadataPath,metadata);
                final.frame.metadataSha256=filePin(metadataPath).sha256.toLowerCase();json(finalPath,final);
                assert.throws(()=>inspectSeedCreationCustody(f.source.scenario,closeoutHash));
            }finally{writeFileSync(metadataPath,metadataBytes);writeFileSync(finalPath,finalBytes);}
        });
        const receiptPath=resolve(f.source.scenario,'E2-wizard-2-inspected.private.json');
        const receiptBytes=readFileSync(receiptPath),ledgerBytes=readFileSync(f.ledgerPath);
        await t.test('a repinned control inspection after its real dispatch refuses',()=>{
            try {
                const ledger=JSON.parse(ledgerBytes.toString()),receipt=JSON.parse(receiptBytes.toString());
                const action=ledger.resolvedActions.find((/** @type {any} */ a)=>a.batch==='E2'&&a.wizardStep.ordinal===2);
                receipt.inspection.inspectedAtMs=action.sentAtMs+1;json(receiptPath,receipt);
                action.wizardStep.inspectionSha256=filePin(receiptPath).sha256.toLowerCase();json(f.ledgerPath,ledger);
                assert.throws(()=>inspectSeedCreationCustody(f.source.scenario,closeoutHash));
            }finally{writeFileSync(receiptPath,receiptBytes);writeFileSync(f.ledgerPath,ledgerBytes);}
        });
        await t.test('a self-consistent replaced E1 control cannot replace the actual clicked original',()=>{
            const directory=resolve(f.source.scenario,'E1-wizard-2'),pngPath=resolve(directory,'F1.original.png');
            const rawPath=resolve(directory,'capture-001.raw.private.json'),metaPath=resolve(directory,'capture-001.metadata.private.json');
            const proofPath=resolve(f.source.scenario,'E1-wizard-2-inspected.private.json');
            const paths=[pngPath,rawPath,metaPath,proofPath,f.ledgerPath],originals=paths.map(p=>readFileSync(p));
            try {
                const changed=replacementPng(originals[0]);writeFileSync(pngPath,changed);
                const raw=JSON.parse(originals[1].toString());raw.content.find((/** @type {any} */ c)=>c.type==='image').data=changed.toString('base64');json(rawPath,raw);
                const meta=JSON.parse(originals[2].toString());meta.pngSha256=filePin(pngPath).sha256.toLowerCase();
                meta.rawSha256=filePin(rawPath).sha256.toLowerCase();json(metaPath,meta);
                const proof=JSON.parse(originals[3].toString());proof.frame.pngSha256=meta.pngSha256;
                proof.frame.metadataSha256=filePin(metaPath).sha256.toLowerCase();json(proofPath,proof);
                const ledger=JSON.parse(originals[4].toString()),entries=ledger.resolvedActions.filter((/** @type {any} */ a)=>a.batch==='E1');
                entries[0].after.pngSha256=meta.pngSha256;entries[0].seedWizardInspectionSha256=filePin(proofPath).sha256.toLowerCase();
                entries[1].wizardStep.inspectionSha256=entries[0].seedWizardInspectionSha256;json(f.ledgerPath,ledger);
                assert.throws(()=>inspectSeedCreationCustody(f.source.scenario,closeoutHash));
            }finally{paths.forEach((p,i)=>writeFileSync(p,originals[i]));}
        });
    }finally{f.close();}
});
