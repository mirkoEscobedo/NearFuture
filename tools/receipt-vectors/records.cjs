'use strict';
const {cat,bytes,uint,hash,peerField,fixed} = require('./fields.cjs');
const {header,operation,proof} = require('./profiles.cjs');
function statusPrefix(c,phase,sequence=7n,sourceEvent=c.source_event,sourceStore=c.source_store) {
  const terminal=phase>=3,known=phase!==1;
  return cat(header(c,8),c.request,c.client.account,c.client.device,known?c.operation:bytes(16),known?c.binding:bytes(32),
    uint(1,phase),uint(1,terminal?1:0),uint(8,terminal?sequence:0),uint(1,phase===3?1:0),uint(2,1),
    uint(8,sourceStore),uint(8,sourceEvent),uint(8,c.membership));
}
function unsupportedPrefix(c,reason) {
  return cat(header(c,9),c.request,uint(1,reason),uint(2,1),uint(8,c.source_store),uint(8,c.source_event),uint(8,c.membership));
}
function reply(c,prefix) {
  const transcript=operation(c,2,hash(prefix)),p=proof(c,'server',hash(transcript));
  return {bytes:cat(prefix,p.bytes),prefix,transcript,proof:p};
}
function book(c,generation,previous,phase=0,sequence=7n) {
  const terminal=phase>=3;
  return cat(Buffer.from('NF-RECEIPT-BOOK-1\0'),uint(2,1),uint(2,1),uint(1,generation),fixed(previous,32),
    c.universe,c.history,c.ruleset,c.content,c.client.account,c.client.device,c.server.account,c.server.device,
    peerField(c.server.peer),c.request,c.operation,uint(4,c.operation_kind),c.payload_digest,c.binding,
    uint(8,c.minimum_event),uint(8,c.minimum_store),uint(8,c.minimum_membership),
    uint(1,phase),uint(1,terminal?1:0),uint(8,terminal?sequence:0),uint(1,phase===3?1:0));
}
function wrapper(record) {
  const prefix=cat(Buffer.from('NF-BLOB-1\0'),uint(4,record.length),record);
  return cat(prefix,hash(prefix));
}
module.exports={statusPrefix,unsupportedPrefix,reply,book,wrapper};
