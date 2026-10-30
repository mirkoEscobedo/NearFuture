// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';

test('modern staging refuses changed custody, unknown Start, incomplete XML and partial tooling before consumption',async()=>{
    const f=await modernSeedFixture();
    try {
        writeFileSync(f.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
        const manifestText=JSON.stringify(f.manifest),closeoutText=readFileSync(f.closeoutPath),ledgerText=readFileSync(f.ledgerPath);
        const pinsPath=resolve(f.target.target,'source-pins.private.json'),authorityPath=f.target.authorityPath;
        const pinsText=readFileSync(pinsPath),claim=resolve(f.target.target,'seed-stage-claim.private.json');
        const xmlPath=resolve(f.source.installation,'saves',f.manifest.saveDirectory,'campaign.xml'),xml=readFileSync(xmlPath);
        const finalPng=resolve(f.source.scenario,'E2-created-seed/F1.original.png'),png=readFileSync(finalPng);
        /** @param {()=>void} change */
        function refuse(change) {
            assert.equal(existsSync(claim),false);change();
            const result=f.target.stage();assert.notEqual(result.status,0,'mutated source must refuse');
            assert.equal(existsSync(claim),false,'a refused source cannot consume staging authority');
            Object.assign(f.manifest,JSON.parse(manifestText));writeFileSync(f.closeoutPath,closeoutText);
            writeFileSync(f.ledgerPath,ledgerText);writeFileSync(xmlPath,xml);writeFileSync(finalPng,png);
            writeFileSync(pinsPath,pinsText);writeFileSync(f.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
            f.refresh();
        }
        refuse(()=>{f.manifest.sourceWorkspace=f.target.target;f.refresh();});
        refuse(()=>{f.manifest.saveDirectory='../personal';f.refresh();});
        refuse(()=>{const value=JSON.parse(closeoutText.toString());value.cleanup.zeroLiveGameProcesses=false;
            json(f.closeoutPath,value);f.manifest.provenance.closeout=filePin(f.closeoutPath);f.refresh();});
        refuse(()=>{const value=JSON.parse(ledgerText.toString());value.pending.status='SENT';json(f.ledgerPath,value);
            const closeout=JSON.parse(closeoutText.toString());closeout.evidence.ledger.sha256=filePin(f.ledgerPath).sha256.toLowerCase();
            json(f.closeoutPath,closeout);f.manifest.provenance.closeout=filePin(f.closeoutPath);f.refresh();});
        refuse(()=>writeFileSync(finalPng,png.subarray(0,png.length-1)));
        refuse(()=>writeFileSync(xmlPath,'changed XML'));
        refuse(()=>writeFileSync(f.commonPath,'{"unexpected":true}'));
        refuse(()=>{const pins=JSON.parse(pinsText.toString());pins.pins.pop();
            const authority=JSON.parse(readFileSync(authorityPath,'utf8'));
            authority.sourcePinsSha256=json(pinsPath,pins).sha256;json(authorityPath,authority);});
        const result=f.target.stage();assert.equal(result.status,0,result.stderr);
    }finally{f.close();}
});
