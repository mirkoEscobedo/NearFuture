'use strict';
const {cat,bytes,fixed,uint,hash,point,optionalPoint,stamp}=require('./fields.cjs');
const {header}=require('./profile.cjs');
const {operation,syncContext}=require('./transcripts.cjs');
const {proof}=require('./identity.cjs');
function beginSync(c,mode=1){
  if(![1,2].includes(mode))throw RangeError('sync mode');
  return cat(header(c,5,1),uint(1,mode),uint(2,1),c.implementation,c.schema,c.sync_client_nonce,
    uint(8,c.minimum_event),uint(8,c.minimum_store),uint(8,c.minimum_membership),optionalPoint(mode===1?c.base:null),
    c.membership_digest,c.request,uint(2,257),uint(8,8388608n));
}
function signed(c,kind,name,prefix,purpose,context,signer='server'){
  const transcript=operation(c,1,purpose,{...context,response_digest:hash(prefix)}),p=proof(c,signer,hash(transcript));
  return {kind,name,prefix,transcript,proof:p,signer,bytes:cat(prefix,p.bytes)};
}
function control(c){
  const begin=beginSync(c),ctx=syncContext(c,begin);
  const challenge=hash(operation(c,1,1,ctx));
  const prove=proof(c,'client',challenge);
  const rows=[{kind:5,name:'begin-sync-delta',bytes:begin},
    {kind:6,name:'sync-challenge',bytes:cat(header(c,6,1),c.request,c.sync_client_nonce,c.sync_server_nonce,point(c.target),stamp(c),challenge)},
    {kind:7,name:'prove-sync',bytes:cat(header(c,7,1),c.request,c.sync_client_nonce,prove.bytes),transcript:operation(c,1,1,ctx),proof:prove,signer:'client'}];
  const offer=cat(header(c,8,1),c.request,c.export_id,point(c.target),c.manifest_digest,uint(4,c.manifest_length),uint(2,c.document_count),uint(8,c.total_data),point(c.current),stamp(c));
  rows.push(signed(c,8,'manifest-offer',offer,2,syncContext(c,begin,c.export_id,c.manifest_digest)));
  const gap=cat(header(c,9,1),c.request,uint(1,1),optionalPoint(c.base),point(c.current),stamp(c));
  rows.push(signed(c,9,'gap-prefix-pruned',gap,7,ctx));
  const refused=cat(header(c,17,1),c.request,bytes(16),uint(1,4),point(c.current),stamp(c));
  rows.push(signed(c,17,'refused-capacity',refused,7,ctx));
  return rows;
}
function install(c,end){
  const prefix=cat(header(c,16,1),c.export_id,c.manifest_digest,point(c.target),uint(8,c.activation_generation));
  const context={export_id:c.export_id,document_digest:c.manifest_digest,initiating_digest:hash(end.prefix),client_nonce:fixed(end.final_document_client_nonce,32),server_nonce:fixed(end.final_completion_nonce,32),response_digest:hash(prefix)};
  return signed(c,16,'replica-install-receipt',prefix,6,context,'client');
}
module.exports={beginSync,control,install};
