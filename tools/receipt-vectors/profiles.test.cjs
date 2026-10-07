'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {fixture} = require('./fixture.cjs');
const {header,handshake,operation,binding,proof,intent} = require('./profiles.cjs');
const {hash,verifyDigest} = require('./fields.cjs');
test('control2 version is authenticated independently of accepted control1',()=>{
  const c=fixture(),h=handshake(c,1,12);
  assert.equal(h.length,619);assert.equal(h.subarray(0,15).toString(),'NF-PEER-AUTH-2\0');
  assert.equal(h.readUInt16LE(16),2);assert.equal(h[18],1);
  assert.equal(header(c,5).length,126);assert.equal(header(c,5).readUInt16LE(10),2);
});
test('245-byte operation binds exact wire minima and distinguishes reply purpose',()=>{
  const c=fixture(),first=operation(c,1),reply=operation(c,2);
  assert.equal(first.length,245);assert.notDeepEqual(hash(first),hash(reply));
  assert.equal(first.readBigUInt64LE(197),c.minimum_event);
  assert.equal(first.readBigUInt64LE(205),c.minimum_store);
  assert.notDeepEqual(hash(first),hash(operation({...c,minimum_event:91n},1)));
});
test('133-byte binding commits complete canonical intent, including original operation',()=>{
  const c=fixture(),raw=binding(c);assert.equal(raw.length,133);
  assert.equal(raw.readUInt32LE(97),3);assert.deepEqual(raw.subarray(101),hash(intent(c)));
  assert.notDeepEqual(hash(raw),hash(binding(c,hash(intent(c,-6n)))));
  assert.equal(c.intent.length,209);assert.equal(c.intent.readUInt32LE(129),2);
});
test('297-byte proof signs maintained Ed25519 over canonical domain8 type5 digest',()=>{
  const c=fixture(),p=proof(c,'server',hash(handshake(c,1,12)));
  assert.equal(p.bytes.length,297);assert.equal(p.canonical.readUInt16LE(11),8);
  assert.equal(p.canonical.readUInt16LE(13),5);assert.equal(p.bytes[72],38);
  assert.equal(verifyDigest(p.digest,p.signature,p.public_key),true);
  const changed=Buffer.from(p.digest);changed[0]^=1;
  assert.equal(verifyDigest(changed,p.signature,p.public_key),false);
});
