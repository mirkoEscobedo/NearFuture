'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {header}=require('../../../tools/reference/sync/profile.cjs');
test('closed Hello header is126 bytes with zero session and actual control namespace lane',()=>{
  const c={session:Buffer.alloc(16,5),universe:Buffer.alloc(16,6),history:Buffer.alloc(16,7),
    ruleset:Buffer.alloc(32,8),content:Buffer.alloc(32,9)};
  const bytes=header(c,1,1,Buffer.alloc(16));
  assert.equal(bytes.length,126);
  assert.equal(bytes.subarray(0,10).toString('ascii'),'NF-SYNC-1\0');
  assert.equal(bytes.readUInt16LE(10),1);
  assert.equal(bytes[12],1);assert.equal(bytes[13],1);
  assert.deepEqual(bytes.subarray(14,30),Buffer.alloc(16));
  assert.deepEqual(bytes.subarray(30,46),c.universe);
});
test('closed registry, actual lane, nonzero session and scope cannot default silently',()=>{
  const c={session:Buffer.alloc(16,5),universe:Buffer.alloc(16,6),history:Buffer.alloc(16,7),ruleset:Buffer.alloc(32,8),content:Buffer.alloc(32,9)};
  for(const kind of [0,18])assert.throws(()=>header(c,kind,1));
  for(const [kind,lane]of [[1,0],[1,3],[5,2],[9,2],[16,2],[17,2],[10,1],[15,1]])assert.throws(()=>header(c,kind,lane));
  assert.throws(()=>header(c,1,1,c.session));assert.throws(()=>header(c,2,1,Buffer.alloc(16)));
  assert.throws(()=>header({...c,universe:Buffer.alloc(16)},1,1));
  assert.throws(()=>header({...c,history:Buffer.alloc(15)},1,1));
});
