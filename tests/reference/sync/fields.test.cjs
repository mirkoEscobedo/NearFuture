'use strict';
const {test}=require('node:test');
const assert=require('node:assert/strict');
const {bytes,uint,point,optionalPoint,stamp,peerField}=require('../../../tools/reference/sync/fields.cjs');
const {fixture}=require('../../../tools/reference/sync/fixtures.cjs');
const {limits,MAX,negotiate,chunkCount,admitLength}=require('../../../tools/reference/sync/profile.cjs');
test('fixed unsigned widths are exact and cannot wrap or lose integer precision',()=>{
  for(const n of [1,2,4,8]){
    const maximum=(1n<<BigInt(n*8))-1n;
    assert.deepEqual(uint(n,maximum),bytes(n,255));
    assert.throws(()=>uint(n,maximum+1n),RangeError);assert.throws(()=>uint(n,-1n),RangeError);
  }
  for(const value of [NaN,Infinity,1.5,9007199254740992,'1'])assert.throws(()=>uint(8,value),RangeError);
});
test('Point112, presence113, stamp40 and peer129 preserve full-u64 and zero padding',()=>{
  const c=fixture(),maximum=(1n<<64n)-1n,p=point({...c.target,store:maximum,event:maximum});
  assert.equal(p.length,112);assert.equal(p.readBigUInt64LE(0),maximum);assert.equal(p.readBigUInt64LE(8),maximum);
  assert.deepEqual(optionalPoint(null),bytes(113));assert.equal(optionalPoint(c.target)[0],1);
  assert.equal(stamp(c).length,40);assert.equal(stamp(c).readBigUInt64LE(0),12n);
  for(const n of [1,128]){const f=peerField(bytes(n,1));assert.equal(f.length,129);assert.equal(f[0],n);assert.deepEqual(f.subarray(1+n),bytes(128-n));}
  assert.throws(()=>peerField(bytes(0)));assert.throws(()=>peerField(bytes(129)));
});
test('limits26 component minima are exact and coupled invalid values refuse',()=>{
  const c=fixture();assert.equal(limits(c.offered).length,26);
  const lower={...MAX,transfer_frame:1024,chunk_bytes:256,control_queue_bytes:1024,transfer_queue_bytes:1024,control_items:1,transfer_items:1};
  assert.deepEqual(negotiate(c.offered,lower),lower);
  for(const patch of [{control_frame:1023},{transfer_frame:1023},{transfer_frame:9217},{chunk_bytes:255},{chunk_bytes:8193},{transfer_frame:1024,chunk_bytes:825},{control_queue_bytes:1023},{transfer_queue_bytes:1023},{control_items:0},{control_items:5},{transfer_items:3},{pending_challenges:2}])assert.throws(()=>limits({...MAX,...patch}),RangeError);
});
test('hard versus selected preallocation caps and257-chunk product boundaries stay separate',()=>{
  assert.equal(admitLength(1024,1),1024);assert.throws(()=>admitLength(1025,1));
  assert.equal(admitLength(8392,2),8392);assert.throws(()=>admitLength(8393,2));assert.throws(()=>admitLength(9216,2));
  const lower={...MAX,transfer_frame:1024,chunk_bytes:256};assert.equal(admitLength(1024,2,lower),1024);assert.throws(()=>admitLength(1025,2,lower));
  assert.equal(chunkCount(2105344,MAX),257);assert.throws(()=>chunkCount(2105345,MAX));
  assert.equal(chunkCount(65792,lower),257);assert.throws(()=>chunkCount(65793,lower));
  assert.equal(chunkCount(1,MAX),1);assert.throws(()=>chunkCount(0,MAX));
});
