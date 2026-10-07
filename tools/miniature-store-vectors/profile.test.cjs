'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {auth,bootstrap,envelope,hash} = require('./profile.cjs');
const b=(n,v)=>Buffer.alloc(n,v);
const context=()=>({purpose:7,universe:b(16,1),history:b(16,2),ruleset:b(32,3),term:4,proposed_term:4,session:5,tick:6,membership:7,actor:b(16,8),device:b(16,9),binding:b(32,10),sequence:11,nonce:b(32,12)});
test('224-byte auth fields use declared fixed offsets and little endian boundaries',()=>{
 const c=context(),raw=auth(c);assert.equal(raw.length,224);assert.equal(raw[15],7);
 assert.equal(raw.readBigUInt64LE(80),4n);assert.equal(raw.readBigUInt64LE(104),6n);
 assert.equal(raw.readBigUInt64LE(112),7n);assert.equal(raw.readBigUInt64LE(184),11n);
 assert.deepEqual(raw.subarray(192),b(32,12));
 c.sequence=18446744073709551615n;assert.equal(auth(c).readBigUInt64LE(184),18446744073709551615n);
});
test('every explicit context field changes its digest without acting as an auth grant',()=>{
 const original=context(),digest=hash(auth(original));
 for(const key of Object.keys(original)) {
  const changed=context();changed[key]=Buffer.isBuffer(changed[key])?Buffer.from(changed[key]):changed[key]+1;
  if(Buffer.isBuffer(changed[key]))changed[key][0]^=1;
  assert.notDeepEqual(hash(auth(changed)),digest,key);
 }
 assert.throws(()=>auth({...original,device:b(15,9)}),/width/);
});
test('bootstrap and minimal envelope widths distinguish presence and integer lengths',()=>{
 const owner={account:b(16,1),account_key:b(32,2),device:b(16,3),device_key:b(32,4),peer:b(3,5)};
 const raw=bootstrap(b(2,7),b(32,8),owner);assert.equal(raw.length,158);
 assert.equal(raw.subarray(0,17).toString(),'NF-MINI-CREATE-1\0');assert.equal(raw.readUInt32LE(17),2);
 const c={implementation:b(32,1),snapshot:b(2,2),authority:null,pending:null,requests:[],holds:[],outbox:[]};
 const e=envelope(c);assert.equal(e.bytes.length,65);assert.equal(e.offsets.authority,51);
 assert.equal(e.bytes.subarray(0,13).toString('hex'),'4e462d53544f52452d32000200');
});
