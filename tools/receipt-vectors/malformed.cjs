'use strict';
const {bytes,cat,uint,hash} = require('./fields.cjs');
const {handshake,operation,proof,intent,binding,header} = require('./profiles.cjs');
const {book,reply,statusPrefix} = require('./records.cjs');
function addMalformed(c,rows,raw,row) {
  const mutate=(name,base,layer,edit,extra={})=>{
    const value=Buffer.from(raw.get(base));edit(value);
    row('negative',name,layer,`REJECT_${layer.toUpperCase()}`,value,{reference:base,...extra});
  };
  for(const [name,value] of raw) {
    row('negative',`${name}-truncated`,'shape','REJECT_SHAPE',value.subarray(0,-1),{reference:name});
    row('negative',`${name}-trailing`,'shape','REJECT_SHAPE',cat(value,uint(1,0)),{reference:name});
  }
  for(const cut of [0,9,10,11,12,13,14,29,30,45,46,61,62,93,94,125])
    row('negative',`header-truncated-${cut}`,'shape','REJECT_SHAPE',raw.get('begin-receipt').subarray(0,cut));
  mutate('wrong-magic','begin-receipt','shape',v=>v[0]^=1);
  mutate('wrong-version','begin-receipt','shape',v=>v.writeUInt16LE(1,10));
  mutate('wrong-lane','begin-receipt','shape',v=>v[13]=2);
  mutate('unknown-kind','begin-receipt','shape',v=>v[12]=10);
  mutate('zero-later-session','begin-receipt','shape',v=>v.fill(0,14,30));
  mutate('nonzero-hello-session','hello','shape',v=>v[14]=1);
  mutate('hello-zero-account','hello','shape',v=>v.fill(0,126,142));
  mutate('hello-zero-device','hello','shape',v=>v.fill(0,142,158));
  mutate('hello-zero-nonce','hello','shape',v=>v.fill(0,158,190));
  mutate('server-hello-zero-nonce','server-hello','shape',v=>v.fill(0,126,158));
  mutate('required-unknown-bit','hello','shape',v=>v.writeUInt32LE(4,190));
  mutate('wrong-selected-capability','server-hello','shape',v=>v.writeUInt32LE(3,162));
  mutate('over-control-limit','hello','limit',v=>v.writeUInt32LE(4097,198));
  mutate('under-control-floor','hello','limit',v=>v.writeUInt32LE(1023,198));
  mutate('zero-control-items','hello','limit',v=>v.writeUInt16LE(0,218));
  mutate('chunk-exceeds-bulk','hello','limit',v=>v.writeUInt32LE(9071,206));
  for(const [name,offset] of [['begin-receipt',190],['receipt-challenge',190],['status-committed',233],['binding-conflict',143]])
    mutate(`${name}-unknown-profile`,name,'shape',v=>v.writeUInt16LE(2,offset));
  for(const [name,offset,width] of [['request',126,16],['operation',142,16],['operation-nonce',192,32]])
    mutate(`begin-zero-${name}`,'begin-receipt','shape',v=>v.fill(0,offset,offset+width));
  mutate('status-unknown-phase','status-committed','shape',v=>v[222]=5);
  mutate('status-invalid-presence','status-committed','shape',v=>v[223]=2);
  mutate('status-committed-zero-sequence','status-committed','shape',v=>v.fill(0,224,232));
  mutate('status-committed-rejection','status-committed','shape',v=>v[232]=1);
  mutate('status-pending-sequence','status-pending','shape',v=>v[224]=1);
  mutate('status-unknown-binding','status-unknown','shape',v=>v[190]=1);
  mutate('unsupported-unknown-reason','binding-conflict','shape',v=>v[142]=3);
  mutate('proof-peer-zero-length','finished','shape',v=>v[198]=0);
  mutate('proof-peer-over-width','finished','shape',v=>v[198]=129);
  mutate('proof-peer-noncanonical-prefix','finished','shape',v=>v[199]=255);
  mutate('proof-peer-nonzero-padding','finished','shape',v=>v[326]=1);
  mutate('proof-invalid-signature','finished','signature',v=>v[v.length-1]^=1);
  mutate('signed-result-prefix-tamper','status-committed','context',v=>v[235]^=1);
  mutate('wire-minimum-normalized','begin-receipt','context',v=>v.writeBigUInt64LE(91n,232));
  mutate('wrong-operation','begin-receipt','context',v=>v[142]^=1);
  mutate('wrong-binding','begin-receipt','context',v=>v[158]^=1);
  mutate('wrong-request','prove-receipt','context',v=>v[126]^=1);
  mutate('wrong-principal','prove-receipt','context',v=>v[206]^=1);
  mutate('wrong-proof-scope','finished','context',v=>v[126]^=1);
  mutate('old-session','finished','context',v=>v[14]^=1);
  mutate('different-ruleset','finished','context',v=>v[62]^=1);
  mutate('different-content','finished','context',v=>v[94]^=1);
  row('negative','wrong-actual-peer','context','REJECT_CONTEXT',raw.get('finished'),{actual_peer_hex:c.client.peer.toString('hex')});
  row('negative','wrong-actual-connection','context','REJECT_CONTEXT',raw.get('finished'),{expected_connection:'A',actual_connection:'B'});
  row('negative','repeated-reply','context','REJECT_CONTEXT',raw.get('status-committed'),{attempts:2,admit_attempt:1});
  for(const [name,challenge] of [['client-stage-used-for-server',hash(handshake(c,2,c.membership))],
    ['control1-domain-reflection',control1Challenge(c)],
    ['old-membership-proof',hash(handshake(c,3,11))]]) {
    const p=proof(c,'server',challenge);
    row('negative',name,'context','REJECT_CONTEXT',cat(header(c,4),p.bytes),{valid_signature:true});
  }
  const changed= {...c,binding:hash(binding(c,hash(intent(c,-6n))))};
  row('negative','changed-full-intent-binding','context','REJECT_CONTEXT',reply(changed,statusPrefix(changed,4)).bytes,{original_binding:c.binding.toString('hex'),valid_signature:true});
  row('negative','signed-current-below-commit','context','REJECT_CONTEXT',reply(c,statusPrefix(c,4,101n)).bytes,{valid_signature:true});
  row('negative','signed-current-below-minimum','context','REJECT_CONTEXT',reply(c,statusPrefix(c,4,7n,89n)).bytes,{valid_signature:true});
  row('negative','proof-frontier-disagrees-status','context','REJECT_CONTEXT',reply({...c,membership:13n},statusPrefix(c,4)).bytes,{valid_signature:true});
  row('negative','signed-store-below-minimum','context','REJECT_CONTEXT',reply(c,statusPrefix(c,4,7n,100n,189n)).bytes,{valid_signature:true});
  const wrongPrincipal=statusPrefix(c,4);wrongPrincipal[142]^=1;
  row('negative','signed-wrong-original-principal','context','REJECT_CONTEXT',reply(c,wrongPrincipal).bytes,{valid_signature:true});
  row('frame','hard-over-limit-with-missing-body','limit','REJECT_LIMIT',Buffer.from('00001001','hex'));
  row('frame','selected-over-limit-with-missing-body','limit','REJECT_LIMIT',Buffer.from('00000401','hex'),{selected_cap:1024});
  require('./recovery.cjs').addRecovery(c,row);
}
function control1Challenge(c) {
  const transcript=cat(Buffer.from('NF-PEER-AUTH-1\0'),handshake(c,3,c.membership).subarray(15));
  transcript.writeUInt16LE(1,16);return hash(transcript);
}
module.exports={addMalformed};
