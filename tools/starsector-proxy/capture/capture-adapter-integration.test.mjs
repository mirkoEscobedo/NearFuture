// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync,writeFileSync,unlinkSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { resolve } from 'node:path';
import { PassThrough } from 'node:stream';
import { spawnSync } from 'node:child_process';
import { Client } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import { StdioServerTransport } from '../../computer-use-mcp/node_modules/@modelcontextprotocol/server/dist/stdio.mjs';
import { serveCaptureStdio,loadCaptureConfiguration,captureFileCommand } from './capture-adapter.mjs';
import { custodyFixture,expectedImage,screenshotArgs } from './capture-custody-fixture.mjs';
import { sha256Ascii } from './capture-ascii-sha256.mjs';
const sha=(/** @type {Buffer} */ v)=>createHash('sha256').update(v).digest('hex');
test('real adapter joins immutable publication, stdio callback and filesystem CLI consumption', {timeout:15000},async()=>{
    const f=custodyFixture('adapter-integration-'),delta=Date.now()-1000000,intent=f.intent;
    for(const name of /** @type {const} */ (['startMs','deadlineMs','callerAdmissionDeadlineMs','originalCallerAdmissionDeadlineMs','effectiveDeadlineMs'])) intent.rootClock[name]+=delta;
    for(const name of /** @type {const} */ (['startMs','lastMs','deadlineMs'])) intent.batchClock[name]+=delta;
    intent.createdMs+=delta;intent.rootCallRecord.value.startMs+=delta;intent.ownedWitness.atMs+=delta;
    intent.ownedWitness.source.startMs+=delta;intent.ownedWitness.source.endMs+=delta;
    intent.producerWitness.effectiveDeadlineMs+=delta;
    for(const name of /** @type {const} */ (['conservativeOuterDeadlineUtc','controllerDeadlineUtc'])) intent.producerWitness[name]=
        new Date(Date.parse(intent.producerWitness[name])+delta).toISOString().replace(/(\.\d{3})Z$/,'$10000Z');
    unlinkSync(f.intentPath);
    const pointerPath=resolve(f.output,'current-capture.private.json');
    writeFileSync(pointerPath,JSON.stringify({schemaVersion:1,kind:'NATIVE99_CAPTURE_POINTER_V1',status:'UNSET',profileId:f.admitted.profileId})+'\n');
    const configuration={schemaVersion:1,kind:'NATIVE99_CAPTURE_ADAPTER_CONFIGURATION_V1',admitted:f.admitted,pointerPath};
    const configPath=resolve(f.output,'adapter-config.private.json');
    writeFileSync(configPath,JSON.stringify(configuration)+'\n');
    const configSHA=sha(readFileSync(configPath)), loaded=loadCaptureConfiguration(configPath,configSHA),intentText=JSON.stringify(intent)+'\n';
    const publication=captureFileCommand('publish',loaded,{intentText,expectedPointerSHA256:sha(readFileSync(pointerPath))});
    assert.equal(publication.intentPath,f.intentPath);
    let dispatches=0;
    const input=new PassThrough(),output=new PassThrough();
    const server=serveCaptureStdio({configuration:loaded,session:{dispatch:async()=>{dispatches++;return expectedImage;}},
        stdio:{transport:new StdioServerTransport(input,output)}});
    const client=new Client({name:'capture-adapter-real-stdio',version:'1'},{capabilities:{},supportedProtocolVersions:['2025-11-25']});
    try {
        await client.connect(new StdioServerTransport(output,input));
        const captureStartMs=Date.now(),result=await client.callTool({name:'screenshot',arguments:screenshotArgs}),captureEndMs=Date.now();
        assert.deepEqual(result,expectedImage);assert.equal(dispatches,1);
        const expected={intent,intentText,slot:f.slot,captureOrdinal:1,captureStartMs,captureEndMs,
            inlineImageSHA256:sha256Ascii(expectedImage.content[0].data),inlineImageCharacters:expectedImage.content[0].data.length,nonImageContent:[]};
        const {intent:ignoredIntent,intentText:ignoredText,...small}=expected;
        const compact={...small,profileId:intent.profileId,batchId:intent.batchId,bindingId:intent.bindingId,frameId:intent.frameId,
            intentPath:f.slot.intentPath,intentSHA256:sha(Buffer.from(intentText))};
        const cli=spawnSync(process.execPath,[new URL('./capture-file-cli.mjs',import.meta.url).pathname.replace(/^\/([A-Z]:)/,'$1'),
            'read',configPath,configSHA,JSON.stringify(compact)],{encoding:'utf8',timeout:10000,windowsHide:true});
        assert.equal(cli.status,0,cli.stderr); const proof=JSON.parse(cli.stdout);
        assert.equal(proof.inlineImageDigestMatched,true);assert.equal(proof.wholeNativeAcceptance,false);
        assert.throws(()=>captureFileCommand('read',loaded,{...compact,intentSHA256:'0'.repeat(64)}),/EXACT_CALLER_INTENT_DIGEST/);
        assert.throws(()=>captureFileCommand('read',loaded,{...expected,captureOrdinal:2}),/EXACT_ADMITTED_COMPLETION_SLOT/);
        writeFileSync(configPath,'{}');assert.throws(()=>loadCaptureConfiguration(configPath,configSHA),/EXACT_CAPTURE_CONFIGURATION_PIN/);
    } finally {await client.close();await server.close();input.destroy();output.destroy();}
});
