'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {fixture}=require('../../../tools/reference/sync/fixtures.cjs');
const {corpus,records}=require('../../../tools/reference/sync/corpus.cjs');
const {hash,uint,cat}=require('../../../tools/reference/sync/fields.cjs');
const {verifyDigest}=require('../../../tools/reference/sync/identity.cjs');
test('all17 closed kinds have exact fixed body widths and25 independently named cases',()=>{
  const positive=records(fixture());assert.equal(positive.length,25);
  assert.deepEqual([...new Set(positive.map(r=>r.kind))].sort((a,b)=>a-b),Array.from({length:17},(_,i)=>i+1));
  const widths=[0,165,421,297,297,294,264,345,639,579,102,200,345,519,0,581,465,482];
  for(const r of positive){if(r.kind!==14)assert.equal(r.bytes.length,126+widths[r.kind],r.name);assert.equal(r.bytes[12],r.kind);assert.equal(r.bytes[13],r.lane);}
  assert.equal(positive.find(r=>r.name==='chunk-hard-maximum8392').bytes.length,8392);
});
test('all response prefixes have independently fixed lengths and proofs excluded',()=>{
  const widths=new Map([[8,468],[9,408],[13,348],[15,410],[16,294],[17,311]]);
  for(const r of records(fixture()).filter(r=>widths.has(r.kind))){assert.equal(r.prefix.length,widths.get(r.kind),r.name);assert.equal(r.bytes.length-r.prefix.length,297);}
});
test('complete output is deterministic and every positive has truncated/trailing counterparts',()=>{
  const a=corpus(),b=corpus();assert.deepEqual(a,b);
  const names=new Set(a.rows.map(r=>r.name));assert.equal(names.size,a.rows.length);
  for(const r of a.rows){assert.equal(hash(Buffer.from(r.hex,'hex')).toString('hex'),r.sha256,r.name);if(r.category==='record'){assert.ok(names.has(r.name+'-truncated'));assert.ok(names.has(r.name+'-trailing'));const frame=Buffer.from(r.frame_hex,'hex');assert.equal(frame.readUInt32BE(0),Buffer.from(r.hex,'hex').length);}}
  assert.equal(a.source_schema_provenance.literal_bytes,2859);assert.equal(a.inputs.source_schema,a.source_schema_provenance.literal_lf_sha256);
});
test('valid signatures on altered record contexts are intentionally not auth-decision evidence',()=>{
  const rows=corpus().rows.filter(r=>r.valid_primitive_signature===true),c=fixture();assert.equal(rows.length,6);
  for(const r of rows){assert.equal(r.expectation,'REJECT_CONTEXT');const p=Buffer.from(r.hex,'hex').subarray(-297),n=p[72];const canonical=cat(Buffer.from('NF-CANON-1\0'),uint(2,8),uint(2,5),uint(2,1),p.subarray(0,72),uint(4,n),p.subarray(73,73+n),p.subarray(201,233));assert.equal(verifyDigest(hash(canonical),p.subarray(233),c[r.signer].public_key),true,r.name);}
});
test('absence/base and empty-chunk negatives mutate the actual presence/length slots',()=>{
  const rows=corpus().rows,base=Buffer.from(rows.find(r=>r.name==='absent-base-has-data').hex,'hex');
  assert.equal(base[249],0);assert.notDeepEqual(base.subarray(250,362),Buffer.alloc(112));
  const empty=Buffer.from(rows.find(r=>r.name==='empty-chunk').hex,'hex');assert.equal(empty.readUInt16LE(198),0);assert.ok(empty.length>200);
});
