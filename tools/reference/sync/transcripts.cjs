'use strict';
const {cat,bytes,fixed,uint,id,hash,peerField}=require('./fields.cjs');
const {limits,PROTOCOLS,session}=require('./profile.cjs');
function handshake(c,lane,stage,frontier){
  if(![0,1,2,3].includes(stage)||!PROTOCOLS[lane]||(stage===0&&BigInt(frontier)!==0n))throw RangeError('sync handshake stage');
  return cat(Buffer.from('NF-SYNC-AUTH-1\0'),uint(1,stage),uint(2,1),uint(1,lane),peerField(c.client.peer),peerField(c.server.peer),
    id(c.client.account),id(c.client.device),id(c.server.account),id(c.server.device),id(session(c,lane)),id(c.universe),id(c.history),
    fixed(c.ruleset,32),fixed(c.content,32),fixed(c.client_nonce,32),fixed(c.server_nonce,32),
    ...[c.required,c.optional,c.available,c.selected].map(v=>uint(4,v)),limits(c.offered),limits(c.server_limits),limits(c.selected_limits),
    uint(8,frontier),uint(2,1),fixed(c.implementation,32),fixed(c.schema,32),fixed(c.membership_digest,32),hash(Buffer.from(PROTOCOLS[lane],'ascii')),uint(1,1));
}
function operation(c,lane,purpose,o){
  if(![1,2,3,4,5,6,7].includes(purpose))throw RangeError('sync operation purpose');
  return cat(Buffer.from('NF-SYNC-OP-1\0'),uint(1,purpose),hash(handshake(c,lane,0,0)),id(c.request),fixed(o.export_id,16),
    fixed(o.document_digest,32),fixed(o.initiating_digest,32),fixed(o.client_nonce,32),fixed(o.server_nonce,32),uint(8,c.membership),
    fixed(c.membership_digest,32),fixed(o.response_digest,32));
}
function syncContext(c,prefix,exportId=bytes(16),documentDigest=bytes(32)){
  return {export_id:exportId,document_digest:documentDigest,initiating_digest:hash(prefix),client_nonce:c.sync_client_nonce,server_nonce:c.sync_server_nonce,response_digest:bytes(32)};
}
function documentContext(c,prefix){return {export_id:c.export_id,document_digest:c.document_digest,initiating_digest:hash(prefix),client_nonce:c.document_client_nonce,server_nonce:c.document_server_nonce,response_digest:bytes(32)};}
module.exports={handshake,operation,syncContext,documentContext};
