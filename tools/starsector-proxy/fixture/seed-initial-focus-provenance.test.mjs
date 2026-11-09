// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {initialFocusFixture} from './seed-initial-focus-test-support.mjs';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';
import {filePin} from './seed-test-support.mjs';
import {json} from './seed-test-support.mjs';
import {readFileSync,writeFileSync,existsSync} from 'node:fs';
import {resolve} from 'node:path';

test('both completed seed readers retain initial-focus custody through coherent refusals and final staging',async t=>{
    const f=await modernSeedFixture({fixture:initialFocusFixture,initialActivation:true});
    try{
        /** @returns {any} */function inspect(){try{return inspectSeedCreationCustody(f.source.scenario,filePin(f.closeoutPath).sha256.toLowerCase());}catch(error){return {error:error instanceof Error?error.message:String(error)};}}
        const ledgerBytes=readFileSync(f.ledgerPath),closeoutBytes=readFileSync(f.closeoutPath),manifest=JSON.stringify(f.manifest);
        /** @type {Map<string,Buffer>} */const originals=new Map();
        /** @param {string} path @param {any} value */function change(path,value){if(!originals.has(path))originals.set(path,readFileSync(path));json(path,value);return{path,sha256:filePin(path).sha256.toLowerCase()};}
        /** @param {string} name @param {(ledger:any,c:any)=>void} mutate */
        async function refuse(name,mutate){let passed=false;await t.test(name,()=>{
            try{
                const ledger=JSON.parse(ledgerBytes.toString()),c=JSON.parse(closeoutBytes.toString());mutate(ledger,c);
                json(f.ledgerPath,ledger);c.evidence.ledger.sha256=filePin(f.ledgerPath).sha256.toLowerCase();
                json(f.closeoutPath,c);f.manifest.provenance.closeout=filePin(f.closeoutPath);f.refresh();
                const js=inspect(),ps=f.target.stage();
                assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
                assert.deepEqual({jsAccepted:!js.error,psStatus:ps.status},{jsAccepted:false,psStatus:1},ps.stderr);
                assert.match(ps.stderr,/seed|activation|custody|modern|integer/i);assert.ok(js.error,JSON.stringify(js));
                assert.equal(existsSync(resolve(f.target.target,'seed-stage-claim.private.json')),false);
                passed=true;
            }finally{
                for(const[path,bytes]of originals)writeFileSync(path,bytes);originals.clear();
                writeFileSync(f.ledgerPath,ledgerBytes);writeFileSync(f.closeoutPath,closeoutBytes);
                Object.assign(f.manifest,JSON.parse(manifest));f.refresh();
            }
        });if(!passed)throw new Error('Refusal failed; later vectors and positive staging remain unreached');}
        await refuse('typed completed schema cannot be a singleton array',(_l,c)=>{c.schemaVersion=[1];});
        await refuse('typed root session cannot be a singleton array',(_l,c)=>{c.rootSessionId=[c.rootSessionId];});
        await refuse('typed cleanup status cannot be a singleton array',(_l,c)=>{c.cleanup.status=[c.cleanup.status];});
        await refuse('typed protection count cannot be a singleton array',(_l,c)=>{c.protection.protectedOriginalFiles=[182];});
        await refuse('typed original autosave retention cannot be a singleton array',(_l,c)=>{
            const path=c.evidence.protection.path,p=JSON.parse(readFileSync(path,'utf8'));
            p.originalAutosave.maximumSlots=[3];c.protection.originalAutosave.maximumSlots=[3];c.evidence.protection=change(path,p);
            f.manifest.provenance.protection=filePin(path);
        });
        await refuse('known activation cannot disappear into an eight-request closeout',(l,c)=>{
            delete l.resolvedInitialActivation;l.history.reservedActions--;l.history.sentActions--;c.actions.activations=0;c.actions.actualGuard=l.history;
        });
        await refuse('the old closeout kind cannot mask the new profile',(_l,c)=>{c.kind='ACTUAL_DISPOSABLE_SEED_CREATION_CLOSEOUT';});
        await refuse('singleton activation count is not a scalar',(_l,c)=>{c.actions.activations=[1];});
        await refuse('activation reservations cannot be refunded',(l,c)=>{l.history.reservedActions--;l.history.sentActions--;c.actions.actualGuard=l.history;});
        await refuse('capture custody cannot claim a post-capture focus observation',l=>{l.resolvedInitialActivation.custody.focusObservationPosition='AFTER_CAPTURE';});
        await refuse('fully repinned unfocused post-ACK window cannot prove recovery',l=>{
            const a=l.resolvedInitialActivation.custody,p=a.postWindow.raw.path,raw=JSON.parse(readFileSync(p,'utf8'));
            const value=JSON.parse(raw.content[0].text);value.isFocused=false;raw.content[0].text=JSON.stringify(value);raw.structuredContent=value;
            a.postWindow.raw=change(p,raw);
            const m=JSON.parse(readFileSync(a.after.metadata.path,'utf8'));m.window.isFocused=false;a.after.metadata=change(a.after.metadata.path,m);
        });
        await refuse('a repinned call attempt cannot carry an array source identity',l=>{
            const a=l.resolvedInitialActivation.custody,p=a.ack.attempt.path,v=JSON.parse(readFileSync(p,'utf8'));v.sourceSha256=[v.sourceSha256];a.ack.attempt=change(p,v);
        });
        const js=inspect();
        const ps=f.target.stage();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);
        assert.deepEqual({js,psStatus:ps.status,psError:ps.status===0?'':ps.stderr},
            {js:{schemaVersion:1,kind:'COMPLETED_SEED_WIZARD_CUSTODY',ok:true,inputs:8,activations:1,scope:'HISTORICAL_CUSTODY_ONLY'},psStatus:0,psError:''});
    }finally{f.close();}
});

test('the new opt-in profile also completes and stages with zero initial activations',async()=>{
    const f=await modernSeedFixture({fixture:initialFocusFixture,initialActivation:false});
    try{
        const js=inspectSeedCreationCustody(f.source.scenario,filePin(f.closeoutPath).sha256.toLowerCase());
        const ps=f.target.stage();assert.equal(ps.error,undefined);assert.equal(ps.signal,null);assert.equal(ps.status,0,ps.stderr);
        assert.equal(js.inputs,8);assert.equal(js.activations,0);
        assert.equal(f.closeout.actions.actualGuard.reservedActions,99);assert.equal(f.closeout.actions.actualGuard.sentActions,97);
    }finally{f.close();}
});
