'use strict';
const {createPrivateKey,createPublicKey,sign,verify}=require('node:crypto');
const {cat,bytes,fixed,uint,hash,peerField,id}=require('./fields.cjs');
const CLIENT_PUBLIC_RFC_SEED='9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
const SERVER_PUBLIC_RFC_SEED='4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb';
function identity(seed,account,device){
  const key=createPrivateKey({key:cat(Buffer.from('302e020100300506032b657004220420','hex'),fixed(Buffer.from(seed,'hex'),32)),format:'der',type:'pkcs8'});
  const public_key=Buffer.from(createPublicKey(key).export({format:'der',type:'spki'})).subarray(-32);
  return {seed,key,public_key,account:bytes(16,account),device:bytes(16,device),peer:cat(Buffer.from('002408011220','hex'),public_key)};
}
function proof(c,who,challenge){
  const i=c[who],prefix=cat(id(c.universe),id(c.history),id(i.account),id(i.device),uint(8,c.membership));
  const canonical=cat(Buffer.from('NF-CANON-1\0'),uint(2,8),uint(2,5),uint(2,1),prefix,uint(4,i.peer.length),i.peer,fixed(challenge,32));
  const signed_digest=hash(canonical),signature=sign(null,signed_digest,i.key);
  return {bytes:cat(prefix,peerField(i.peer),challenge,signature),canonical,signed_digest,signature,public_key:i.public_key};
}
function verifyDigest(digest,signature,key){
  const publicKey=createPublicKey({key:cat(Buffer.from('302a300506032b6570032100','hex'),fixed(key,32)),format:'der',type:'spki'});
  return verify(null,fixed(digest,32),publicKey,fixed(signature,64));
}
module.exports={identity,proof,verifyDigest,CLIENT_PUBLIC_RFC_SEED,SERVER_PUBLIC_RFC_SEED};
