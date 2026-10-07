'use strict';
// Independent test data producer. Only Node standard APIs and accepted public byte fixtures.
const fs=require('node:fs'),path=require('node:path');
const {auth,bootstrap,envelope,hash,cat,u32,u64}=require('./profile.cjs');
const root=path.resolve(__dirname,'../..'),b=(n,v)=>Buffer.alloc(n,v);
const fixture=name=>Buffer.from(fs.readFileSync(path.join(root,'crates/nf-kernel/tests/fixtures/miniature',name+'.hex'),'utf8').trim(),'hex');
const snapshot=fixture('snapshot-genesis'),intent=fixture('intent-colony'),frontier=fixture('frontier-colony');
const bundle=JSON.parse(fs.readFileSync(path.join(root,'crates/nf-kernel/tests/fixtures/miniature/source-bundle.json'),'utf8'));
const implementation=Buffer.from(bundle.bundle_sha256,'hex');
const ruleset=snapshot.subarray(81,113);
const owner={account:b(16,4),account_key:Buffer.from('d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a','hex'),device:b(16,33),device_key:Buffer.from('3d4017c3e843895a92b70aa74d1b7ebc9c982ccf2ec4968cc0cd55f12af4660c','hex'),peer:Buffer.from([1,2,3])};
const membership=b(32,7),creation=bootstrap(snapshot,membership,owner);
const serial=c=>Object.fromEntries(Object.entries(c).map(([k,v])=>[k,Buffer.isBuffer(v)?v.toString('hex'):String(v)]));
const value=(name,bytes,extra={})=>({name,bytes:bytes.length,hex:bytes.toString('hex'),sha256:hash(bytes).toString('hex'),...extra});
const authVectors=[];
for(const [purpose,name] of ['lease','claim','prepare','resume','cancel','bootstrap','commit'].map((n,i)=>[i+1,n])) {
 const c={purpose,universe:b(16,1),history:b(16,2),ruleset,term:7,proposed_term:7,session:9,tick:1,membership:2,actor:owner.account,device:owner.device,binding:b(32,0),sequence:purpose,nonce:b(32,70+purpose)};
 if(purpose===2){c.proposed_term=8;c.session=10;}
 if(purpose===3||purpose===4)c.binding=hash(intent);
 if(purpose===5||purpose===7)c.binding=hash(frontier);
 if(purpose===5)c.tick=0;
 if(purpose===6){c.term=0;c.proposed_term=0;c.session=0;c.tick=0;c.binding=hash(creation);}
 authVectors.push(value(name,auth(c),{fields:serial(c)}));
}
const authNegative=[];
function authBad(name,base,offset,edit,focus) {
 const bytes=Buffer.from(authVectors[base].hex,'hex');edit(bytes,offset);
 authNegative.push(value(name,bytes,{base:authVectors[base].name,expectation:'NO_AUTHORITY_FROM_CHANGED_OR_MALFORMED_TRANSCRIPT',failure_focus:focus}));
}
authBad('unknown-purpose',0,15,(x,o)=>x[o]=8,'closed purpose');
authBad('wrong-purpose',0,15,(x,o)=>x[o]=7,'purpose-bound issued registry');
authBad('stale-membership',0,112,(x,o)=>x.writeBigUInt64LE(1n,o),'current durable membership');
authBad('claim-non-increasing-term',1,88,(x,o)=>x.writeBigUInt64LE(7n,o),'checked greater Claim term');
authBad('wrong-runtime-session',6,96,(x,o)=>x.writeBigUInt64LE(10n,o),'current owner runtime session');
authBad('wrong-decision-tick',6,104,(x,o)=>x.writeBigUInt64LE(2n,o),'actual next tick');
authBad('wrong-binding',2,152,(x,o)=>x[o]^=1,'exact immutable intent digest');
authBad('reused-sequence',2,184,(x,o)=>x.writeBigUInt64LE(1n,o),'retained one-attempt ticket');
authBad('wrong-actor',2,120,(x,o)=>x[o]^=1,'exact account/device owner');
for(const [name,raw]of [['truncated',Buffer.from(authVectors[0].hex,'hex').subarray(0,223)],['trailing',cat(Buffer.from(authVectors[0].hex,'hex'),b(1,0))]])authNegative.push(value(name,raw,{expectation:'MALFORMED_TRANSCRIPT_DATA_ONLY'}));
const authority={term:7,session:9,account:owner.account,device:owner.device,membership:2};
const empty={implementation,snapshot,authority:null,pending:null,requests:[],holds:[],outbox:[]};
const claimed={...empty,authority};
const pending={...claimed,pending:frontier,requests:[{request:b(16,30),intent,phase:1}],holds:[{operation:b(16,31),faction:intent.subarray(-16),credits:100,supplies:40}]};
const cancelledSnapshot=Buffer.from(snapshot.subarray(0,-4));cancelledSnapshot.writeBigUInt64LE(1n,121);
const cancelledWorld=cat(cancelledSnapshot,u32(1),b(16,31),b(16,32),b(1,21));
const cancelled={...claimed,snapshot:cancelledWorld,requests:[{request:b(16,30),intent,phase:2,sequence:1,rejection:21}],outbox:[{operation:b(16,31),sequence:1,digest:hash(fixture('batch-cancel')),rejection:21}]};
const envelopeCases=new Map([['genesis',envelope(empty)],['claimed-genesis',envelope(claimed)],['pending-colony',envelope(pending)],['cancelled-colony',envelope(cancelled)]]);
const envelopes=[...envelopeCases].map(([name,c])=>value(name,c.bytes,{expectation:'CANONICAL_VALUE_ONLY',offsets:c.offsets}));
const malformed=[];
function bad(name,base,edit,focus) {const c=envelopeCases.get(base),raw=Buffer.from(c.bytes);edit(raw,c.offsets);malformed.push(value(name,raw,{base,expectation:'REJECT_OWNER_ADMISSION',failure_focus:focus}));}
bad('unknown-store-version','genesis',x=>x[11]=3,'fixed envelope header');
bad('wrong-implementation','genesis',x=>x[13]^=1,'compiled source profile identity');
bad('snapshot-overlength-before-body','genesis',x=>x.writeUInt32LE(1048577,45),'nested length before allocation');
bad('authority-presence-two','genesis',(x,o)=>x[o.authority]=2,'closed presence');
bad('pending-presence-two','genesis',(x,o)=>x[o.pending]=2,'closed presence');
bad('request-count-one-over','genesis',(x,o)=>x.writeUInt32LE(4097,o.requests),'bounded count before loop');
bad('hold-count-one-over','genesis',(x,o)=>x.writeUInt32LE(2,o.holds),'at most one derived hold');
bad('outbox-count-one-over','genesis',(x,o)=>x.writeUInt32LE(257,o.outbox),'bounded outbox');
bad('request-phase-unknown','pending-colony',(x,o)=>x[o.requests+4+16+4+intent.length]=3,'closed request phase');
bad('wrong-request-key','pending-colony',(x,o)=>x[o.requests+4]^=1,'key must equal bound intent request');
bad('wrong-derived-hold','pending-colony',(x,o)=>x.writeBigUInt64LE(99n,o.holds+4+32),'canonical reservation correspondence');
bad('stale-pending-input-hash','pending-colony',(x,o)=>x[o.pending+1+4+17]^=1,'exact source world hash');
bad('rejection-tag-unknown','cancelled-colony',x=>x[x.length-1]=22,'closed schema2 rejection');
const otherIntent=Buffer.from(intent);otherIntent.fill(29,17,33);
for(const [name,requests]of [['duplicate-request-order',[pending.requests[0],pending.requests[0]]],['descending-request-order',[pending.requests[0],{request:b(16,29),intent:otherIntent,phase:1}]]])malformed.push(value(name,envelope({...pending,requests}).bytes,{expectation:'REJECT_OWNER_ADMISSION',failure_focus:'strict ascending request IDs before insertion'}));
malformed.push(value('truncated-genesis',envelopeCases.get('genesis').bytes.subarray(0,-1),{expectation:'REJECT_OWNER_ADMISSION'}));
malformed.push(value('trailing-genesis',cat(envelopeCases.get('genesis').bytes,b(1,0)),{expectation:'REJECT_OWNER_ADMISSION'}));
const document={profile:'NF-MINIATURE-STORE-FIXTURES-1',method:'Node standard Buffer/crypto; explicit field writer; public accepted kernel fixtures; no production codec imports',authority:'Public data only. No transcript, digest or envelope is a signature, issued ticket, live lease or history certificate.',implementation_hash:implementation.toString('hex'),inputs:{snapshot:value('public-genesis',snapshot),intent:value('public-colony-intent',intent),frontier:value('public-colony-frontier',frontier)},bootstrap:value('creation-binding',creation,{fields:{membership_digest:membership.toString('hex'),owner:serial(owner)}}),auth:authVectors,auth_negative:authNegative,envelopes,malformed};
const content=JSON.stringify(document,null,2)+'\n',target=path.join(root,'docs/world/vectors/store-v2.json');
const args=process.argv.slice(2);
if(args.length===1&&args[0]==='--check'){if(fs.readFileSync(target,'utf8')!==content)throw Error('store fixtures differ');}
else if(args.length===0)fs.writeFileSync(target,content);else throw Error('closed fixture generator arguments');
const lines=[...document.auth.map(x=>['auth',x]),...document.auth_negative.map(x=>['auth-negative',x]),['bootstrap',document.bootstrap],...document.envelopes.map(x=>['envelope',x]),...document.malformed.map(x=>['malformed',x])].map(([kind,x])=>[kind,x.name,x.bytes,x.hex,x.sha256,x.expectation||'DATA_ONLY'].join('\t'));
const tsv='# NF-MINIATURE-STORE-FIXTURES-1; category,name,byte-length,hex,sha256,expectation\n'+lines.join('\n')+'\n';
const tsvPath=path.join(root,'docs/world/vectors/store-v2.tsv');
if(args.length===1){if(fs.readFileSync(tsvPath,'utf8')!==tsv)throw Error('store TSV differs');}else fs.writeFileSync(tsvPath,tsv);
console.log(`STORE_VECTORS_OK auth=${authVectors.length} envelope=${envelopes.length} malformed=${malformed.length} auth_negative=${authNegative.length}`);
