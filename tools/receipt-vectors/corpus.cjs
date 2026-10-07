'use strict';
const {cat,bytes,uint,limits,hash} = require('./fields.cjs');
const {header,handshake,operation,binding,proof,intent} = require('./profiles.cjs');
const {fixture} = require('./fixture.cjs');
const {statusPrefix,unsupportedPrefix,reply,book,wrapper} = require('./records.cjs');
function corpus() {
  const c=fixture(),rows=[],raw=new Map();
  const row=(category,name,layer,expectation,value,extra={})=>{
    const result={category,name,layer,expectation,hex:value.toString('hex'),sha256:hash(value).toString('hex'),...extra};
    rows.push(result);return result;
  };
  const positive=(name,kind,value,extra={})=>{
    raw.set(name,value);row('record',name,'shape','ADMIT_DATA',value,{kind,...extra});
  };
  const addProof=(name,p)=>row('signature',name,'primitive','VALID_ED25519_DIGEST',p.signature,
    {public_key:p.public_key.toString('hex'),signed_digest:p.digest.toString('hex'),canonical_hex:p.canonical.toString('hex')});
  row('value','full-profile1-intent','value','NOT_WIRE_REGISTERED',c.intent);
  row('value','original-binding','value','133_BYTES',binding(c),{intent_payload_digest:c.payload_digest.toString('hex')});
  row('value','client-peer','value','CANONICAL_PEER',c.client.peer);
  row('value','server-peer','value','CANONICAL_PEER',c.server.peer);
  for(const stage of [0,1,2,3]) row('transcript',`handshake-${stage}`,'value','619_BYTES',handshake(c,stage,stage===0?0:c.membership));
  const first=operation(c,1),clientProof=proof(c,'client',hash(first));
  row('transcript','operation-1','value','245_BYTES',first);addProof('operation-client',clientProof);
  positive('hello',1,cat(header(c,1,bytes(16)),c.client.account,c.client.device,c.client_nonce,
    uint(4,c.required),uint(4,c.optional),limits(c.offered)));
  const serverHello=proof(c,'server',hash(handshake(c,1,c.membership)));addProof('server-hello',serverHello);
  positive('server-hello',2,cat(header(c,2),c.server_nonce,uint(4,1),uint(4,1),limits(c.server_limits),limits(c.selected_limits),serverHello.bytes));
  const cp=proof(c,'client',hash(handshake(c,2,c.membership))),finished=proof(c,'server',hash(handshake(c,3,c.membership)));
  addProof('client-proof',cp);addProof('finished',finished);
  positive('client-proof',3,cat(header(c,3),cp.bytes));positive('finished',4,cat(header(c,4),finished.bytes));
  positive('begin-receipt',5,cat(header(c,5),c.request,c.operation,c.binding,uint(2,1),c.operation_nonce,
    uint(8,c.minimum_membership),uint(8,c.minimum_event),uint(8,c.minimum_store)));
  positive('receipt-challenge',6,cat(header(c,6),c.request,c.operation,c.binding,uint(2,1),c.operation_nonce,
    c.server_operation_nonce,uint(8,c.membership),hash(first)));
  positive('prove-receipt',7,cat(header(c,7),c.request,c.operation_nonce,clientProof.bytes));
  for(const [name,phase] of [['unknown',1],['pending',2],['rejected',3],['committed',4]]) {
    const r=reply(c,statusPrefix(c,phase));
    row('prefix',`status-${name}`,'value','259_BYTES',r.prefix);
    row('transcript',`reply-${name}`,'value','245_BYTES',r.transcript);addProof(`reply-${name}`,r.proof);
    positive(`status-${name}`,8,r.bytes,{historical_commit_sequence:phase>=3?'7':null,source_event:'100',minimum_source_event:'90'});
  }
  for(const [name,reason] of [['binding-conflict',1],['source-below-minima',2]]) {
    const source=name==='source-below-minima'?{...c,source_event:89n,source_store:189n}:c;
    const r=reply(source,unsupportedPrefix(source,reason));
    row('prefix',name,'value','169_BYTES',r.prefix);row('transcript',name,'value','245_BYTES',r.transcript);
    addProof(name,r.proof);positive(name,9,r.bytes);
  }
  const maximum=18446744073709551615n;
  const maxContext={...c,source_event:maximum,source_store:maximum,membership:maximum,minimum_membership:maximum,minimum_event:maximum,minimum_store:maximum};
  const maxReply=reply(maxContext,statusPrefix(maxContext,4,maximum));
  addProof('reply-maximum-counters',maxReply.proof);positive('status-maximum-counters',8,maxReply.bytes);
  let previous=bytes(32);
  for(let generation=0;generation<8;generation++) {
    const bookContext=generation===0?c:{...c,minimum_event:99n+BigInt(generation),minimum_store:199n+BigInt(generation),minimum_membership:12n};
    const b=book(bookContext,generation,previous,generation===0?0:generation===1?2:4);
    row('book',`generation-${generation}`,'recovery','VALID_CHAIN_DATA',b,{generation,previous_digest:previous.toString('hex')});
    row('blob',`wrapped-generation-${generation}`,'value','525_BYTES',wrapper(b));previous=hash(b);
  }
  addMalformed(c,rows,raw,row);
  return {schema_version:1,contract:{path:'docs/transport/receipt-contract.md',sha256:'763fbd65d5d6df42268c6b30d3ddaf3bea23972303fa7b0ab5f618ab58f31efd',line_ending_identity:'LF_NORMALIZED'},
    disclaimer:'Independent data and primitive fixtures only. Shape/auth/recovery decisions require later public production tests. RFC8032 seeds are public test data, never installed credentials.',
    identities:[c.client,c.server].map(i=>({public_test_seed:i.seed,public_key:i.public_key.toString('hex'),account:i.account.toString('hex'),device:i.device.toString('hex'),peer:i.peer.toString('hex')})),
    rows};
}
function addMalformed(c,rows,raw,row) { require('./malformed.cjs').addMalformed(c,rows,raw,row); }
module.exports={corpus};
