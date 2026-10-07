'use strict';
const {cat,uint,hash,point,stamp}=require('./fields.cjs');
const {header,chunkCount}=require('./profile.cjs');
const {operation,documentContext}=require('./transcripts.cjs');
const {proof}=require('./identity.cjs');
function beginDocument(c){
  return cat(header(c,10,2),c.transfer_id,c.export_id,c.document_digest,uint(4,c.document.length),uint(2,chunkCount(c.document.length,c.selected_limits)),c.document_client_nonce);
}
function chunk(c,index){
  const count=chunkCount(c.document.length,c.selected_limits);
  if(!Number.isSafeInteger(index)||index<0||index>=count)throw RangeError('sync chunk index');
  const data=c.document.subarray(index*c.selected_limits.chunk_bytes,Math.min((index+1)*c.selected_limits.chunk_bytes,c.document.length));
  return cat(header(c,14,2),c.transfer_id,c.export_id,c.document_digest,uint(2,index),uint(2,count),uint(4,c.document.length),uint(2,data.length),data);
}
function transfer(c){
  const begin=beginDocument(c),ctx=documentContext(c,begin),count=chunkCount(c.document.length,c.selected_limits);
  const transcript=operation(c,2,3,ctx),p=proof(c,'client',hash(transcript));
  const rows=[{kind:10,name:'begin-document',bytes:begin},
    {kind:11,name:'document-challenge',bytes:cat(header(c,11,2),c.transfer_id,c.export_id,c.document_digest,c.document_client_nonce,c.document_server_nonce,stamp(c),hash(transcript))},
    {kind:12,name:'prove-document',bytes:cat(header(c,12,2),c.transfer_id,c.document_client_nonce,p.bytes),transcript,proof:p,signer:'client'}];
  for(const [kind,purpose,name,prefix]of [
    [13,4,'document-ready',cat(header(c,13,2),c.transfer_id,c.export_id,c.document_digest,uint(4,c.document.length),uint(2,count),point(c.current),stamp(c))],
    [15,5,'document-end',cat(header(c,15,2),c.transfer_id,c.export_id,c.document_digest,uint(4,c.document.length),c.document_digest,point(c.current),stamp(c),c.completion_nonce)]]){
    const transcript=operation(c,2,purpose,{...ctx,response_digest:hash(prefix)}),p=proof(c,'server',hash(transcript));
    rows.push({kind,name,prefix,transcript,proof:p,signer:'server',bytes:cat(prefix,p.bytes),...(kind===15?{final_document_client_nonce:Buffer.from(ctx.client_nonce),final_completion_nonce:Buffer.from(c.completion_nonce)}:{})});
  }
  rows.push({kind:14,name:'sync-chunk',bytes:chunk(c,0)});return rows;
}
module.exports={beginDocument,chunk,transfer};
