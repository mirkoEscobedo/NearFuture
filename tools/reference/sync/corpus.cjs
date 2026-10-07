'use strict';
const {bytes,hash}=require('./fields.cjs');
const {frame}=require('./profile.cjs');
const {fixture}=require('./fixtures.cjs');
const {auth}=require('./records-auth.cjs');
const {control,beginSync,install}=require('./records-control.cjs');
const {transfer,chunk}=require('./records-transfer.cjs');
const {handshake}=require('./transcripts.cjs');
const {malformed}=require('./malformed.cjs');
const CONTRACT={path:'docs/transport/sync-contract.md',sha256:'e4223a690379b3c76db543a08dc8c8179da2d75df55dc96c8b473b2aed8c65bc'};
function row(category,name,layer,expectation,raw,extra={}){
  return {category,name,layer,expectation,hex:raw.toString('hex'),sha256:hash(raw).toString('hex'),...extra};
}
function records(c){
  const controlRows=control(c).map(r=>({...r,lane:1})),transferRows=transfer(c).map(r=>({...r,lane:2}));
  const end=transferRows.find(r=>r.kind===15);
  const rows=[...auth(c,1).map(r=>({...r,lane:1})),...auth(c,2).map(r=>({...r,lane:2})),...controlRows,...transferRows,{...install(c,end),lane:1}];
  const positive=rows.map(r=>({...r,name:r.name+'-lane'+r.lane}));
  positive.push({kind:5,lane:1,name:'begin-sync-snapshot',bytes:beginSync(c,2)});
  const lower={...c,offered:{...c.offered,chunk_bytes:256},server_limits:{...c.server_limits,chunk_bytes:256},selected_limits:{...c.selected_limits,chunk_bytes:256}};
  positive.push({kind:14,lane:2,name:'chunk-first-selected256',selected_chunk:256,bytes:chunk(lower,0)});
  positive.push({kind:14,lane:2,name:'chunk-final-selected256',selected_chunk:256,bytes:chunk(lower,1)});
  const maximum={...c,document:bytes(8192,0x2a)};maximum.document_digest=hash(maximum.document);
  positive.push({kind:14,lane:2,name:'chunk-hard-maximum8392',bytes:chunk(maximum,0)});
  return positive;
}
function corpus(){
  const c=fixture(),positive=records(c),rows=[];
  for(const r of positive){
    rows.push(row('record',r.name,'shape','ACCEPT_SHAPE',r.bytes,{kind:r.kind,lane:r.lane,selected_chunk:r.selected_chunk??c.selected_limits.chunk_bytes,selected_transfer_frame:c.selected_limits.transfer_frame,frame_hex:frame(r.bytes).toString('hex')}));
    if(r.prefix)rows.push(row('prefix',r.name+'-prefix','binding','REFERENCE_BYTES',r.prefix));
    if(r.proof){
      const p=r.proof;
      rows.push(row('transcript',r.name+'-transcript','binding','REFERENCE_BYTES',r.transcript));
      rows.push(row('signature',r.name+'-signature','primitive','VALID_PRIMITIVE',p.signature,{canonical_hex:p.canonical.toString('hex'),signed_digest:p.signed_digest.toString('hex'),public_key:p.public_key.toString('hex'),proof_hex:p.bytes.toString('hex'),signer:r.signer}));
    }
  }
  for(const lane of [1,2])rows.push(row('transcript','neutral-lane'+lane,'binding','REFERENCE_BYTES',handshake(c,lane,0,0)));
  rows.push(...malformed(positive));
  const inputs={session_control:c.sessions[1].toString('hex'),session_transfer:c.sessions[2].toString('hex'),universe:c.universe.toString('hex'),history:c.history.toString('hex'),ruleset:c.ruleset.toString('hex'),content:c.content.toString('hex'),membership:c.membership.toString(),membership_digest:c.membership_digest.toString('hex'),synthetic_expected_implementation:c.implementation.toString('hex'),source_schema:c.schema.toString('hex'),limits:c.offered,
    client_nonce:c.client_nonce.toString('hex'),server_nonce:c.server_nonce.toString('hex'),sync_client_nonce:c.sync_client_nonce.toString('hex'),sync_server_nonce:c.sync_server_nonce.toString('hex'),document_client_nonce:c.document_client_nonce.toString('hex'),document_server_nonce:c.document_server_nonce.toString('hex'),completion_nonce:c.completion_nonce.toString('hex'),client_account:c.client.account.toString('hex'),client_device:c.client.device.toString('hex'),server_account:c.server.account.toString('hex'),server_device:c.server.device.toString('hex'),original_sync_request:c.request.toString('hex'),export_id:c.export_id.toString('hex'),transfer_id:c.transfer_id.toString('hex'),client_public_rfc_seed:c.client.seed,server_public_rfc_seed:c.server.seed,client_public_key:c.client.public_key.toString('hex'),server_public_key:c.server.public_key.toString('hex'),client_peer:c.client.peer.toString('hex'),server_peer:c.server.peer.toString('hex')};
  return {format:'NF-SYNC-1 independent data only',contract:CONTRACT,source_schema_provenance:c.source_provenance,qualifications:['No compiled/registered implementation pin','ACCEPT_SHAPE does not admit semantic Store documents','Valid primitive signatures do not decide current role, session, expiry or durable activation','No production encoder imports'],inputs,rows};
}
module.exports={CONTRACT,row,records,corpus};
