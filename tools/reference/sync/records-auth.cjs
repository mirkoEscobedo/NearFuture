'use strict';
const {cat,uint,hash}=require('./fields.cjs');
const {header,limits}=require('./profile.cjs');
const {handshake}=require('./transcripts.cjs');
const {proof}=require('./identity.cjs');
function auth(c,lane){
  const rows=[];
  const hello=cat(header(c,1,lane),uint(1,1),c.client.account,c.client.device,c.client_nonce,uint(4,1),uint(4,0),limits(c.offered),uint(2,1),c.implementation,c.schema);
  rows.push({kind:1,name:'hello',bytes:hello});
  for(const [kind,stage,signer,name]of [[2,1,'server','server-hello'],[3,2,'client','client-proof'],[4,3,'server','finished']]){
    const transcript=handshake(c,lane,stage,c.membership),p=proof(c,signer,hash(transcript));
    const prefix=kind===2?cat(header(c,kind,lane),c.server_nonce,uint(4,1),uint(4,1),limits(c.server_limits),limits(c.selected_limits),c.membership_digest):header(c,kind,lane);
    rows.push({kind,name,bytes:cat(prefix,p.bytes),prefix,transcript,proof:p,signer});
  }
  return rows;
}
module.exports={auth};
