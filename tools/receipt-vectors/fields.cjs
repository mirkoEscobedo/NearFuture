'use strict';
const {createHash,createPrivateKey,createPublicKey,sign,verify} = require('node:crypto');
const cat = (...parts) => Buffer.concat(parts);
const bytes = (n,value=0) => Buffer.alloc(n,value);
const fixed = (value,n) => {
  if (!Buffer.isBuffer(value) || value.length !== n) throw Error('fixture fixed width');
  return value;
};
function uint(n,value) {
  const out = bytes(n);
  if (n === 8) {
    if (typeof value === 'number' && !Number.isSafeInteger(value)) throw RangeError('fixture exact integer required');
    out.writeBigUInt64LE(BigInt(value));
  }
  else if (n === 4) out.writeUInt32LE(value);
  else if (n === 2) out.writeUInt16LE(value);
  else out.writeUInt8(value);
  return out;
}
const hash = value => createHash('sha256').update(value).digest();
const canon = (domain,kind) => cat(Buffer.from('NF-CANON-1\0'),uint(2,domain),uint(2,kind),uint(2,1));
function key(seed) {
  return createPrivateKey({key:cat(Buffer.from('302e020100300506032b657004220420','hex'),fixed(seed,32)),format:'der',type:'pkcs8'});
}
const publicKey = privateKey => Buffer.from(createPublicKey(privateKey).export({format:'der',type:'spki'})).subarray(-32);
const signDigest = (digest,privateKey) => sign(null,fixed(digest,32),privateKey);
const verifyDigest = (digest,signature,publicBytes) => verify(null,digest,createPublicKey({key:cat(Buffer.from('302a300506032b6570032100','hex'),fixed(publicBytes,32)),format:'der',type:'spki'}),signature);
function peerField(peer) {
  if (!Buffer.isBuffer(peer) || peer.length < 1 || peer.length > 128) throw Error('fixture peer width');
  return cat(uint(1,peer.length),peer,bytes(128-peer.length));
}
function limits(v) {
  return cat(...[v.control_frame,v.bulk_frame,v.chunk,v.control_queue,v.bulk_queue].map(x=>uint(4,x)),
    ...[v.control_items,v.bulk_items,v.challenges].map(x=>uint(2,x)));
}
module.exports = {cat,bytes,fixed,uint,hash,canon,key,publicKey,signDigest,verifyDigest,peerField,limits};
