'use strict';
const {cat,bytes,uint,hash} = require('./fields.cjs');
const {fixture} = require('./fixture.cjs');
const {PROTOCOL,limits,header,handshake,subscribe,notice,proof,subscribedPrefix,noticePrefix,ackPrefix} = require('./profiles.cjs');
function corpus() {
  const c=fixture(),rows=[],raw=new Map();
  const row=(category,name,layer,expectation,value,extra={})=>{
    rows.push({category,name,layer,expectation,hex:value.toString('hex'),sha256:hash(value).toString('hex'),...extra});
  };
  const positive=(name,kind,value,extra={})=>{
    raw.set(name,value);row('record',name,'shape','ADMIT_DATA',value,{kind,...extra});
  };
  const signature=(name,p)=>row('signature',name,'primitive','VALID_ED25519_DIGEST',p.signature,
    {public_key:p.public_key.toString('hex'),signed_digest:p.digest.toString('hex'),canonical_hex:p.canonical.toString('hex')});
  const transcript=(name,value)=>row('transcript',name,'value',`${value.length}_BYTES`,value);
  const signed=(name,kind,prefix,t,who)=>{
    const p=proof(c,who,hash(t));signature(name,p);transcript(name,t);
    row('prefix',name,'value',`${prefix.length}_BYTES`,prefix);positive(name,kind,cat(prefix,p.bytes));
  };
  row('value','protocol-digest-input','value','ASCII_NO_NUL',Buffer.from(PROTOCOL,'ascii'));
  row('value','client-peer','value','CANONICAL_PEER',c.client.peer);
  row('value','server-peer','value','CANONICAL_PEER',c.server.peer);
  row('value','original-receipt-binding','value','OPAQUE_BOUND_INPUT',c.binding,
    {receipt_vector:'receipt-v2.json/original-binding',request:c.request.toString('hex'),operation:c.operation.toString('hex')});
  for(const stage of [0,1,2,3])transcript(`handshake-${stage}`,handshake(c,stage,stage===0?0:c.membership));
  positive('hello',1,cat(header(c,1,bytes(16)),c.client.account,c.client.device,c.client_nonce,uint(4,1),uint(4,0),limits(c.offered)));
  const sh=proof(c,'server',hash(handshake(c,1,c.membership))),cp=proof(c,'client',hash(handshake(c,2,c.membership))),fp=proof(c,'server',hash(handshake(c,3,c.membership)));
  signature('server-hello',sh);signature('client-proof',cp);signature('finished',fp);
  positive('server-hello',2,cat(header(c,2),c.server_nonce,uint(4,1),uint(4,1),limits(c.server_limits),limits(c.selected),sh.bytes));
  positive('client-proof',3,cat(header(c,3),cp.bytes));positive('finished',4,cat(header(c,4),fp.bytes));
  const first=subscribe(c,1),sp=proof(c,'client',hash(first));transcript('subscribe-1',first);signature('prove-subscribe',sp);
  positive('begin-subscribe',5,cat(header(c,5),c.subscription,uint(1,1),c.request,c.operation,c.binding,c.subscribe_nonce,
    uint(8,c.minimum_membership),uint(2,c.lifetime)));
  positive('subscribe-challenge',6,cat(header(c,6),c.subscription,c.subscribe_nonce,c.server_subscribe_nonce,uint(8,c.membership),hash(first)));
  positive('prove-subscribe',7,cat(header(c,7),c.subscription,c.subscribe_nonce,sp.bytes));
  const subPrefix=subscribedPrefix(c);signed('subscribed',8,subPrefix,subscribe(c,2,hash(subPrefix)),'server');
  const nPrefix=noticePrefix(c),aPrefix=ackPrefix(c);
  signed('notice',9,nPrefix,notice(c,1,hash(nPrefix)),'server');signed('notice-ack',10,aPrefix,notice(c,2,hash(aPrefix)),'client');
  for(const lifetime of [1,30]) {
    const bound={...c,lifetime},prefix=subscribedPrefix(bound),t=subscribe(bound,2,hash(prefix)),p=proof(bound,'server',hash(t));
    positive(`subscribed-lifetime-${lifetime}`,8,cat(prefix,p.bytes));signature(`subscribed-lifetime-${lifetime}`,p);
  }
  const maximum={...c,sequence:18446744073709551615n};
  for(const [name,kind,prefix,stage,who] of [['notice-maximum-sequence',9,noticePrefix(maximum),1,'server'],['ack-maximum-sequence',10,ackPrefix(maximum),2,'client']]) {
    const t=notice(maximum,stage,hash(prefix)),p=proof(maximum,who,hash(t));
    positive(name,kind,cat(prefix,p.bytes));signature(name,p);
  }
  const minLimit={body:768,queue:768,items:1,rate:1,burst:1,challenges:1},minimum={...c,offered:minLimit,server_limits:minLimit,selected:minLimit};
  positive('hello-minimum-limits',1,cat(header(c,1,bytes(16)),c.client.account,c.client.device,c.client_nonce,uint(4,1),uint(4,0),limits(minLimit)));
  const minProof=proof(minimum,'server',hash(handshake(minimum,1,minimum.membership)));
  positive('server-hello-minimum-limits',2,cat(header(c,2),c.server_nonce,uint(4,1),uint(4,1),limits(minLimit),limits(minLimit),minProof.bytes));signature('minimum-limits',minProof);
  require('./malformed.cjs').addMalformed(c,raw,row);
  require('./scheduling.cjs').addScheduling(c,row);
  return {schema_version:1,contract:{path:'docs/transport/notification-contract.md',sha256:'f7aebf226c0d9c03bf8c6c97945f371b333abf180bbc7aae498a70a4fbc4dc8d',reviewed_raw_sha256:'f3e818776bb3ee9fe0c660424bc420f938e852c3f7d3ba0eec51466440aa0dce',line_ending_identity:'LF_NORMALIZED'},
    disclaimer:'Independent protocol data/primitive fixtures only. Auth, SQL, timing, rate, queue and fairness decisions require actual production tests. RFC8032 seeds are public test data, not installed credentials.',
    identities:[c.client,c.server].map(i=>({public_test_seed:i.seed,public_key:i.public_key.toString('hex'),account:i.account.toString('hex'),device:i.device.toString('hex'),peer:i.peer.toString('hex')})),rows};
}
module.exports={corpus};
