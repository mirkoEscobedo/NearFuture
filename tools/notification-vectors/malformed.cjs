'use strict';
const {bytes,cat,uint,hash} = require('./fields.cjs');
const {PROTOCOL,handshake,subscribe,notice,proof,header,subscribedPrefix,noticePrefix,ackPrefix} = require('./profiles.cjs');
function addMalformed(c,raw,row) {
  const mutate=(name,base,layer,edit,extra={})=>{
    const value=Buffer.from(raw.get(base));edit(value);
    row('negative',name,layer,`REJECT_${layer.toUpperCase()}`,value,{reference:base,...extra});
  };
  for(const [name,value] of raw) {
    row('negative',`${name}-truncated`,'shape','REJECT_SHAPE',value.subarray(0,-1),{reference:name});
    row('negative',`${name}-trailing`,'shape','REJECT_SHAPE',cat(value,uint(1,0)),{reference:name});
  }
  for(const cut of [0,11,12,13,14,15,16,31,32,47,48,63,64,95,96,127])
    row('negative',`header-truncated-${cut}`,'shape','REJECT_SHAPE',raw.get('begin-subscribe').subarray(0,cut));
  mutate('wrong-magic','hello','shape',v=>v[0]^=1);mutate('wrong-version','hello','shape',v=>v.writeUInt16LE(2,12));
  mutate('wrong-lane','hello','shape',v=>v[15]=1);mutate('unknown-kind','hello','shape',v=>v[14]=11);
  mutate('nonzero-hello-session','hello','shape',v=>v[16]=1);mutate('zero-finished-session','finished','shape',v=>v.fill(0,16,32));
  for(const [name,offset] of [['account',128],['device',144]])mutate(`hello-zero-${name}`,'hello','shape',v=>v.fill(0,offset,offset+16));
  mutate('zero-client-nonce','hello','shape',v=>v.fill(0,160,192));mutate('zero-server-nonce','server-hello','shape',v=>v.fill(0,128,160));
  mutate('required-unknown-bit','hello','shape',v=>v.writeUInt32LE(2,192));mutate('optional-must-be-zero','hello','shape',v=>v.writeUInt32LE(1,196));
  mutate('wrong-selected-capability','server-hello','shape',v=>v.writeUInt32LE(3,164));
  for(const [name,offset,size,value] of [['body-over',200,2,1025],['body-under',200,2,767],['queue-under-body',202,4,1023],
    ['queue-over',202,4,16385],['items-zero',206,2,0],['items-over',206,2,17],['rate-zero',208,2,0],['rate-over',208,2,9],
    ['burst-zero',210,2,0],['burst-over',210,2,9],['challenges-zero',212,2,0],['challenges-over',212,2,2]])
    mutate(`limit-${name}`,'hello','limit',v=>size===4?v.writeUInt32LE(value,offset):v.writeUInt16LE(value,offset));
  mutate('selected-is-not-componentwise-minimum','server-hello','context',v=>v.writeUInt16LE(15,188));
  mutate('unknown-topic','begin-subscribe','shape',v=>v[144]=2);
  for(const [name,offset] of [['subscription',128],['request',145],['operation',161]])
    mutate(`begin-zero-${name}`,'begin-subscribe','shape',v=>v.fill(0,offset,offset+16));
  for(const lifetime of [0,31])mutate(`lifetime-${lifetime}`,'begin-subscribe','shape',v=>v.writeUInt16LE(lifetime,249));
  mutate('subscribed-first-sequence-not-one','subscribed','shape',v=>v.writeBigUInt64LE(2n,209));
  mutate('notice-zero-sequence','notice','shape',v=>v.fill(0,144,152));mutate('ack-not-admitted','notice-ack','shape',v=>v[184]=0);
  mutate('proof-peer-zero-length','finished','shape',v=>v[200]=0);mutate('proof-peer-over-width','finished','shape',v=>v[200]=129);
  mutate('proof-peer-noncanonical','finished','shape',v=>v[201]=255);mutate('proof-peer-padding','finished','shape',v=>v[328]=1);
  mutate('invalid-detached-signature','finished','signature',v=>v[v.length-1]^=1);
  mutate('signed-notice-prefix-tampered','notice','context',v=>v[184]^=1);
  mutate('old-session','notice','context',v=>v[16]^=1);mutate('different-scope','notice','context',v=>v[32]^=1);
  mutate('different-ruleset','notice','context',v=>v[64]^=1);mutate('different-content','notice','context',v=>v[96]^=1);
  mutate('different-subscription','notice','context',v=>v[128]^=1);mutate('different-request','notice','context',v=>v[152]^=1);
  mutate('different-operation','notice','context',v=>v[168]^=1);mutate('different-binding','notice','context',v=>v[184]^=1);
  mutate('wrong-next-sequence','notice','context',v=>v.writeBigUInt64LE(2n,144));mutate('wrong-ack-sequence','notice-ack','context',v=>v.writeBigUInt64LE(2n,144));
  mutate('wrong-ack-notice-digest','notice-ack','context',v=>v[152]^=1);
  mutate('different-client-operation-nonce','subscribe-challenge','context',v=>v[144]^=1);
  mutate('below-known-membership','subscribe-challenge','context',v=>v.writeBigUInt64LE(10n,208));
  const signedNegative=(name,context,kind,prefix,t,who='server')=>{
    const p=proof(context,who,hash(t));
    row('negative',name,'context','REJECT_CONTEXT',cat(prefix,p.bytes),{valid_signature:true,signer:who});
  };
  const protocolWrong=Buffer.from(handshake(c,3,c.membership));hash(Buffer.from(PROTOCOL+'\0')).copy(protocolWrong,21);
  signedNegative('protocol-digest-includes-nul',c,4,header(c,4),protocolWrong);
  signedNegative('wrong-handshake-stage',c,4,header(c,4),handshake(c,2,c.membership));
  const changedSelector={...c,binding:bytes(32,23)},changedNotice=noticePrefix(changedSelector);
  signedNegative('signed-selector-substitution',changedSelector,9,changedNotice,notice(changedSelector,1,hash(changedNotice)));
  const wrongMembership={...c,membership:11n},wrongPrefix=noticePrefix(wrongMembership);
  signedNegative('signed-stale-policy-epoch',wrongMembership,9,wrongPrefix,notice(wrongMembership,1,hash(wrongPrefix)));
  const wrongNonce={...c,notice_nonce:bytes(32,23)},wrongAck=ackPrefix(wrongNonce);
  signedNegative('signed-ack-for-another-notice',wrongNonce,10,wrongAck,notice(wrongNonce,2,hash(wrongAck)),'client');
  const wrongLifetime={...c,lifetime:11},wrongSubscribed=subscribedPrefix(wrongLifetime);
  signedNegative('signed-lifetime-substitution',wrongLifetime,8,wrongSubscribed,subscribe(wrongLifetime,2,hash(wrongSubscribed)));
  signedNegative('subscribe-proof-used-for-notice',c,9,noticePrefix(c),subscribe(c,2,hash(subscribedPrefix(c))));
  row('negative','wrong-actual-peer','context','REJECT_CONTEXT',raw.get('notice'),{actual_peer_hex:c.client.peer.toString('hex')});
  row('negative','wrong-actual-connection','context','REJECT_CONTEXT',raw.get('notice'),{expected_connection:'A',actual_connection:'B'});
  row('negative','duplicate-notice','context','REJECT_CONTEXT',raw.get('notice'),{attempts:2,admit_attempt:1});
  row('negative','duplicate-ack','context','REJECT_CONTEXT',raw.get('notice-ack'),{attempts:2,admit_attempt:1});
  row('frame','hard-over-with-missing-body','limit','REJECT_LIMIT',Buffer.from('00000401','hex'));
  row('frame','selected-over-with-missing-body','limit','REJECT_LIMIT',Buffer.from('00000301','hex'),{selected_cap:768});
}
module.exports={addMalformed};
