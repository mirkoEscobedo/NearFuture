'use strict';
const {createHash}=require('node:crypto');
const cat=(...parts)=>Buffer.concat(parts);
const bytes=(length,value=0)=>Buffer.alloc(length,value);
function fixed(value,length){
  if(!Buffer.isBuffer(value)||value.length!==length)throw RangeError('sync fixed width');
  return value;
}
function uint(length,value){
  if(![1,2,4,8].includes(length))throw RangeError('sync integer width');
  if(typeof value==='number'&&!Number.isSafeInteger(value))throw RangeError('sync exact integer');
  if(typeof value!=='number'&&typeof value!=='bigint')throw RangeError('sync integer type');
  const exact=BigInt(value),maximum=(1n<<BigInt(length*8))-1n;
  if(exact<0n||exact>maximum)throw RangeError('sync integer range');
  const out=bytes(length);
  if(length===8)out.writeBigUInt64LE(exact);
  else if(length===4)out.writeUInt32LE(Number(exact));
  else if(length===2)out.writeUInt16LE(Number(exact));
  else out.writeUInt8(Number(exact));
  return out;
}
function id(value){
  fixed(value,16);if(value.equals(bytes(16)))throw RangeError('sync nonzero identity');return value;
}
const hash=value=>createHash('sha256').update(value).digest();
function peerField(peer){
  if(!Buffer.isBuffer(peer)||peer.length<1||peer.length>128)throw RangeError('sync peer width');
  return cat(uint(1,peer.length),peer,bytes(128-peer.length));
}
function point(p){
  return cat(uint(8,p.store),uint(8,p.event),fixed(p.world,32),fixed(p.state,32),fixed(p.head,32));
}
function optionalPoint(p){return p===null?cat(uint(1,0),bytes(112)):cat(uint(1,1),point(p));}
function stamp(c){return cat(uint(8,c.membership),fixed(c.membership_digest,32));}
module.exports={cat,bytes,fixed,uint,id,hash,peerField,point,optionalPoint,stamp};
