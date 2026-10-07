'use strict';
const {cat,bytes,uint,hash,peerField,fixed,signDigest,canon} = require('./fields.cjs');
const PROTOCOL='/nearfuture/peer/notify/1';
function limits(v) {
  return cat(uint(2,v.body),uint(4,v.queue),...[v.items,v.rate,v.burst,v.challenges].map(x=>uint(2,x)));
}
function header(c,kind,session=c.session) {
  return cat(Buffer.from('NF-NOTIFY-1\0'),uint(2,1),uint(1,kind),uint(1,3),fixed(session,16),
    c.universe,c.history,c.ruleset,c.content);
}
function handshake(c,stage,frontier) {
  return cat(Buffer.from('NF-NOTIFY-AUTH-1\0'),uint(1,stage),uint(2,1),uint(1,3),hash(Buffer.from(PROTOCOL,'ascii')),
    peerField(c.client.peer),peerField(c.server.peer),c.client.account,c.client.device,c.server.account,c.server.device,
    c.session,c.universe,c.history,c.ruleset,c.content,c.client_nonce,c.server_nonce,
    ...[1,0,1,1].map(v=>uint(4,v)),limits(c.offered),limits(c.server_limits),limits(c.selected),uint(8,frontier));
}
function subscribe(c,stage,prefix=bytes(32)) {
  return cat(Buffer.from('NF-NOTIFY-SUB-1\0'),uint(1,stage),hash(handshake(c,0,0)),c.subscription,uint(1,1),
    c.request,c.operation,c.binding,c.subscribe_nonce,c.server_subscribe_nonce,uint(8,c.membership),
    uint(8,c.minimum_membership),uint(2,c.lifetime),fixed(prefix,32));
}
function notice(c,stage,prefix) {
  return cat(Buffer.from('NF-NOTIFY-NOTICE-1\0'),uint(1,stage),hash(handshake(c,0,0)),c.subscription,uint(8,c.sequence),
    c.request,c.operation,c.binding,c.notice_nonce,uint(8,c.membership),fixed(prefix,32));
}
function proof(c,who,challenge) {
  const i=c[who],fields=cat(c.universe,c.history,i.account,i.device,uint(8,c.membership));
  const canonical=cat(canon(8,5),fields,uint(4,i.peer.length),i.peer,challenge);
  const digest=hash(canonical),signature=signDigest(digest,i.key);
  return {bytes:cat(fields,peerField(i.peer),challenge,signature),canonical,digest,signature,public_key:i.public_key};
}
function subscribedPrefix(c) {
  return cat(header(c,8),c.subscription,uint(1,1),c.request,c.operation,c.binding,uint(8,1),uint(2,c.lifetime));
}
function noticePrefix(c) {
  return cat(header(c,9),c.subscription,uint(8,c.sequence),c.request,c.operation,c.binding,c.notice_nonce);
}
function ackPrefix(c) {
  return cat(header(c,10),c.subscription,uint(8,c.sequence),hash(noticePrefix(c)),uint(1,1));
}
module.exports={PROTOCOL,limits,header,handshake,subscribe,notice,proof,subscribedPrefix,noticePrefix,ackPrefix};
