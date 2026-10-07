'use strict';
const {hash,cat,bytes}=require('./fields.cjs');
function changed(raw,offset,value){const out=Buffer.from(raw);out[offset]=value;return out;}
function malformed(records){
  const rows=[],add=(name,raw,expectation='REJECT_SHAPE',layer='shape',extra={})=>rows.push({category:'malformed',name,layer,expectation,hex:raw.toString('hex'),sha256:hash(raw).toString('hex'),...extra});
  for(const r of records){add(r.name+'-truncated',r.bytes.subarray(0,-1));add(r.name+'-trailing',cat(r.bytes,bytes(1)));}
  const hello=records.find(r=>r.kind===1&&r.lane===1).bytes;
  for(const [name,offset,value]of [['magic',0,0],['version',10,2],['kind',12,18],['lane',13,2],['hello-session',14,1],['zero-universe',30,0]]){
    let raw=changed(hello,offset,value);if(name==='zero-universe')raw.fill(0,30,46);
    // Hello is valid on either lane; label its wrong actual control slot as context.
    add(name,raw,name==='lane'?'REJECT_CONTEXT':'REJECT_SHAPE',name==='lane'?'context':'shape',name==='lane'?{actual_lane:1}:{});
  }
  add('unknown-required-capability',changed(hello,191,2),'REJECT_PROFILE');
  add('unknown-optional-capability',changed(hello,195,1),'REJECT_PROFILE');
  add('unknown-profile',changed(hello,225,2),'REJECT_PROFILE');
  add('unknown-implementation-pin',changed(hello,227,0x31),'REJECT_PROFILE');
  add('unknown-Store1-schema-pin',changed(hello,259,hello[259]^1),'REJECT_PROFILE');
  const begin=records.find(r=>r.kind===5).bytes;
  add('absent-base-has-data',changed(begin,249,0));
  const end=records.find(r=>r.kind===15);
  for(const [name,offset]of [['wrong-source-current-point',226],['wrong-completion-nonce',378],['wrong-document-digest',158]]){
    const raw=changed(end.bytes,offset,end.bytes[offset]^1);
    add(name,raw,'REJECT_CONTEXT','context',{valid_primitive_signature:true,signer:'server',public_key:end.proof.public_key.toString('hex'),signed_digest:end.proof.signed_digest.toString('hex')});
  }
  for(const [name,kind,offset,value]of [['wrong-preadmitted-membership-digest',2,218,0x2b],['moved-client-proof-session',3,14,0x2c],['finished-proof-in-client-stage',4,12,3]]){
    const r=records.find(r=>r.kind===kind&&r.lane===1),raw=changed(r.bytes,offset,value);
    add(name,raw,'REJECT_CONTEXT','context',{valid_primitive_signature:true,signer:r.signer,public_key:r.proof.public_key.toString('hex'),signed_digest:r.proof.signed_digest.toString('hex')});
  }
  const chunk=records.find(r=>r.kind===14).bytes;
  const empty=Buffer.from(chunk);empty.fill(0,198,200);add('empty-chunk',empty);add('wrong-chunk-count',changed(chunk,192,0));
  for(const [name,n]of [['control-one-over',1025],['transfer-hard-one-over',8393],['transfer-selected-one-over',1025],['short-header',125]]){
    const raw=bytes(4);raw.writeUInt32BE(n);
    add(name,raw,'REJECT_LIMIT','framing',{declared:n,actual_lane:name.startsWith('control')?1:2,selected_transfer_frame:name==='transfer-selected-one-over'?1024:9216,selected_chunk:name==='transfer-selected-one-over'?256:8192});
  }
  add('wrong-peer-padding',changed(records.find(r=>r.kind===3&&r.lane===1).bytes,126+73+38,1));
  return rows;
}
module.exports={changed,malformed};
