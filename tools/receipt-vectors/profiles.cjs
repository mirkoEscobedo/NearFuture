'use strict';
const {cat,bytes,uint,hash,canon,peerField,limits,fixed,signDigest} = require('./fields.cjs');
function header(c,kind,session=c.session) {
  return cat(Buffer.from('NF-PEER-2\0'),uint(2,2),uint(1,kind),uint(1,1),fixed(session,16),
    fixed(c.universe,16),fixed(c.history,16),fixed(c.ruleset,32),fixed(c.content,32));
}
function handshake(c,stage,frontier) {
  return cat(Buffer.from('NF-PEER-AUTH-2\0'),uint(1,stage),uint(2,2),uint(1,1),
    peerField(c.client.peer),peerField(c.server.peer),c.client.account,c.client.device,c.server.account,c.server.device,
    c.session,c.universe,c.history,c.ruleset,c.content,c.client_nonce,c.server_nonce,
    ...[c.required,c.optional,c.available,c.selected].map(v=>uint(4,v)),
    limits(c.offered),limits(c.server_limits),limits(c.selected_limits),uint(8,frontier));
}
function operation(c,stage,prefixDigest=bytes(32)) {
  return cat(Buffer.from('NF-PEER-RECEIPT-2\0'),uint(1,stage),hash(handshake(c,0,0)),
    c.request,c.operation,c.binding,uint(2,1),c.operation_nonce,c.server_operation_nonce,
    uint(8,c.membership),uint(8,c.minimum_membership),uint(8,c.minimum_event),uint(8,c.minimum_store),fixed(prefixDigest,32));
}
function binding(c,payload=c.payload_digest) {
  return cat(canon(1,1),c.request,c.client.account,c.client.device,c.universe,c.history,uint(4,c.operation_kind),fixed(payload,32));
}
function intent(c,delta=-5n) {
  const command = bytes(8); command.writeBigInt64LE(delta);
  return cat(canon(7,4),c.request,c.operation,c.job,c.client.account,c.universe,c.history,c.provider,
    uint(4,2),c.component,uint(8,0),c.provider_aggregate,uint(8,0),uint(4,3),c.market,command);
}
function proof(c,who,challenge) {
  const identity = c[who];
  const fields = cat(c.universe,c.history,identity.account,identity.device,uint(8,c.membership));
  const canonical = cat(canon(8,5),fields,uint(4,identity.peer.length),identity.peer,fixed(challenge,32));
  const digest = hash(canonical),signature = signDigest(digest,identity.key);
  return {bytes:cat(fields,peerField(identity.peer),challenge,signature),canonical,digest,signature,public_key:identity.public_key};
}
module.exports = {header,handshake,operation,binding,intent,proof};
