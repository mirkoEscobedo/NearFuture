'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {bytes,hash}=require('../../../tools/reference/sync/fields.cjs');
const {fixture}=require('../../../tools/reference/sync/fixtures.cjs');
const {handshake,operation,documentContext}=require('../../../tools/reference/sync/transcripts.cjs');
const {beginDocument,transfer}=require('../../../tools/reference/sync/records-transfer.cjs');
const {install}=require('../../../tools/reference/sync/records-control.cjs');
const {verifyDigest}=require('../../../tools/reference/sync/identity.cjs');
const {corpus}=require('../../../tools/reference/sync/corpus.cjs');
test('750 handshake and278 operation use distinct lanes, domains and closed stages',()=>{
  const c=fixture();for(const lane of [1,2])for(const stage of [0,1,2,3])assert.equal(handshake(c,lane,stage,stage===0?0:c.membership).length,750);
  assert.notDeepEqual(hash(handshake(c,1,0,0)),hash(handshake(c,2,0,0)));
  const context=documentContext(c,beginDocument(c));for(const purpose of [1,2,3,4,5,6,7])assert.equal(operation(c,2,purpose,context).length,278);
  assert.throws(()=>handshake(c,1,0,1));assert.throws(()=>handshake(c,1,4,12));assert.throws(()=>operation(c,2,8,context));
});
test('all caller and private context components influence the separate signed identity',()=>{
  const c=fixture(),baseline=hash(handshake(c,1,1,c.membership));
  for(const name of ['universe','history','ruleset','content','client_nonce','server_nonce','implementation','schema','membership_digest']){
    const changed={...c,[name]:Buffer.from(c[name])};changed[name][0]^=1;assert.notDeepEqual(hash(handshake(changed,1,1,c.membership)),baseline,name);
  }
  for(const who of ['client','server'])for(const name of ['peer','account','device']){
    const changed={...c,[who]:{...c[who],[name]:Buffer.from(c[who][name])}};changed[who][name][0]^=1;assert.notDeepEqual(hash(handshake(changed,1,1,c.membership)),baseline,who+'/'+name);
  }
  for(const name of ['required','optional','available','selected'])assert.notDeepEqual(hash(handshake({...c,[name]:c[name]+1},1,1,c.membership)),baseline,name);
  for(const set of ['offered','server_limits','selected_limits'])for(const name of ['transfer_frame','chunk_bytes','control_queue_bytes','transfer_queue_bytes','control_items','transfer_items']){
    const altered={...c,[set]:{...c[set],[name]:c[set][name]-1}};
    assert.notDeepEqual(hash(handshake(altered,1,1,c.membership)),baseline,set+'/'+name);
  }
  assert.notDeepEqual(hash(handshake(c,1,1,13n)),baseline,'proof frontier');
  const changed={...c,sessions:{...c.sessions,1:Buffer.from(c.sessions[1])}};changed.sessions[1][0]^=1;assert.notDeepEqual(hash(handshake(changed,1,1,c.membership)),baseline);
  const o=documentContext(c,beginDocument(c)),expected=hash(operation(c,2,3,o));
  for(const name of Object.keys(o)){const altered={...o,[name]:Buffer.from(o[name])};altered[name][0]^=1;assert.notDeepEqual(hash(operation(c,2,3,altered)),expected,name);}
  assert.notDeepEqual(hash(operation({...c,request:bytes(16,42)},2,3,o)),expected);
  assert.notDeepEqual(hash(operation({...c,membership:13n},2,3,o)),expected);
});
test('transfer correlation remains original SyncRequestId and End5 differs from Install6',()=>{
  const c=fixture(),end=transfer(c).find(r=>r.kind===15),installed=install(c,end);
  assert.deepEqual(end.transcript.subarray(46,62),c.request);
  assert.deepEqual(end.transcript.subarray(62,78),c.export_id);
  assert.deepEqual(end.transcript.subarray(174,206),c.document_server_nonce);
  assert.deepEqual(installed.transcript.subarray(174,206),c.completion_nonce);
  assert.deepEqual(installed.transcript.subarray(110,142),hash(end.prefix));
  assert.deepEqual(installed.transcript.subarray(78,110),c.manifest_digest);
  assert.notDeepEqual(end.transcript.subarray(14,46),installed.transcript.subarray(14,46));
});
test('all14 primitive signatures verify; tampering fails without granting runtime authority',()=>{
  const rows=corpus().rows.filter(r=>r.category==='signature');assert.equal(rows.length,14);
  for(const r of rows){const digest=Buffer.from(r.signed_digest,'hex'),signature=Buffer.from(r.hex,'hex'),key=Buffer.from(r.public_key,'hex');assert.deepEqual(hash(Buffer.from(r.canonical_hex,'hex')),digest);assert.equal(verifyDigest(digest,signature,key),true,r.name);digest[0]^=1;assert.equal(verifyDigest(digest,signature,key),false,r.name);}
});
test('Install6 retains final document nonces instead of substituting control operation nonces',()=>{
  const c=fixture();
  assert.notDeepEqual(c.sync_client_nonce,c.document_client_nonce);assert.notDeepEqual(c.sync_server_nonce,c.document_server_nonce);
  const end=transfer(c).find(r=>r.kind===15),installed=install(c,end);
  assert.deepEqual(installed.transcript.subarray(142,174),c.document_client_nonce);
  assert.deepEqual(end.transcript.subarray(142,174),c.document_client_nonce);
  assert.deepEqual(end.transcript.subarray(174,206),c.document_server_nonce);
  assert.deepEqual(installed.transcript.subarray(174,206),c.completion_nonce);
  const changedControl={...c,sync_client_nonce:bytes(32,40),sync_server_nonce:bytes(32,41)};
  assert.deepEqual(install(changedControl,transfer(changedControl).find(r=>r.kind===15)).transcript,installed.transcript);
  const changedDocument={...c,document_client_nonce:bytes(32,42)};
  const changedInstall=install(changedDocument,transfer(changedDocument).find(r=>r.kind===15));
  assert.notDeepEqual(changedInstall.transcript,installed.transcript);
  assert.deepEqual(changedInstall.transcript.subarray(142,174),changedDocument.document_client_nonce);
  assert.deepEqual(install(changedDocument,end).transcript,installed.transcript);
  const changedCompletion={...c,completion_nonce:bytes(32,43)};
  assert.deepEqual(install(changedCompletion,end).transcript,installed.transcript);
  const nextEnd=transfer(changedCompletion).find(r=>r.kind===15);
  assert.deepEqual(install(changedCompletion,nextEnd).transcript.subarray(174,206),changedCompletion.completion_nonce);
  assert.deepEqual(installed.transcript.subarray(174,206),end.final_completion_nonce);
  assert.deepEqual(installed.transcript.subarray(46,62),c.request);
  assert.deepEqual(installed.transcript.subarray(14,46),hash(handshake(c,1,0,0)));
});
test('Install6 refuses absent or malformed final End nonce metadata without a control fallback',()=>{
  const c=fixture(),end=transfer(c).find(r=>r.kind===15);
  for(const field of ['final_document_client_nonce','final_completion_nonce']){
    for(const value of [undefined,bytes(31),bytes(33)])assert.throws(()=>install(c,{...end,[field]:value}),field);
  }
});
