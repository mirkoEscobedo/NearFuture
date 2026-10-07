'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {fixture} = require('./fixture.cjs');
const {corpus} = require('./corpus.cjs');
const {hash,bytes,verifyDigest} = require('./fields.cjs');
const {book,wrapper,statusPrefix,reply} = require('./records.cjs');
test('all nine record kinds and four phases have exact contract widths',()=>{
  const data=corpus(),rows=data.rows.filter(r=>r.category==='record'),kinds=new Set(rows.map(r=>r.kind));
  assert.deepEqual([...kinds].sort(),[1,2,3,4,5,6,7,8,9]);
  const sizes=new Map([[1,224],[2,515],[3,423],[4,423],[5,248],[6,296],[7,471],[8,556],[9,466]]);
  for(const r of rows)assert.equal(Buffer.from(r.hex,'hex').length,sizes.get(r.kind),r.name);
  assert.deepEqual(data,corpus());
});
test('status source frontiers are distinct from historical commit sequence',()=>{
  const c=fixture(),p=statusPrefix(c,4),r=reply(c,p);
  assert.equal(p.length,259);assert.equal(p.readBigUInt64LE(224),7n);
  assert.equal(p.readBigUInt64LE(235),200n);assert.equal(p.readBigUInt64LE(243),100n);
  assert.equal(p.readBigUInt64LE(251),12n);assert.equal(c.minimum_event,90n);
  assert.equal(r.transcript.length,245);assert.deepEqual(r.transcript.subarray(-32),hash(p));
  assert.equal(r.bytes.length,556);
});
test('479-byte book and 46-byte wrapper preserve exact generation and anchored hash',()=>{
  const c=fixture(),root=book(c,0,bytes(32)),next=book(c,1,hash(root),2),wrapped=wrapper(root);
  assert.equal(root.length,479);assert.equal(root.readUInt16LE(18),1);assert.equal(root[22],0);
  assert.deepEqual(next.subarray(23,55),hash(root));assert.equal(root.readUInt32LE(376),3);
  assert.deepEqual(root.subarray(380,412),c.payload_digest);assert.deepEqual(root.subarray(412,444),c.binding);
  assert.equal(root.readBigUInt64LE(444),90n);assert.equal(root[468],0);
  assert.equal(wrapped.length,525);assert.equal(wrapped.readUInt32LE(10),479);
  assert.deepEqual(wrapped.subarray(-32),hash(wrapped.subarray(0,-32)));
});
test('every primitive signature is independently verifiable and changed digest fails',()=>{
  const rows=corpus().rows.filter(r=>r.category==='signature');assert.equal(rows.length,11);
  for(const r of rows) {
    const digest=Buffer.from(r.signed_digest,'hex'),key=Buffer.from(r.public_key,'hex'),sig=Buffer.from(r.hex,'hex');
    assert.deepEqual(hash(Buffer.from(r.canonical_hex,'hex')),digest);
    assert.equal(verifyDigest(digest,sig,key),true,r.name);digest[0]^=1;
    assert.equal(verifyDigest(digest,sig,key),false,r.name);
  }
});
test('malformed categories remain data-only and include each positive body truncation',()=>{
  const rows=corpus().rows,names=new Set(rows.map(r=>r.name));
  for(const r of rows.filter(r=>r.category==='record')) {
    assert.equal(names.has(r.name+'-truncated'),true);assert.equal(names.has(r.name+'-trailing'),true);
  }
  for(const layer of ['shape','limit','context','signature','recovery'])assert.ok(rows.some(r=>r.layer===layer&&r.category==='negative'));
  assert.ok(names.has('book-anchored-older-valid-prefix'));assert.ok(names.has('signed-current-below-minimum'));
});
