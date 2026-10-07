// @ts-check
// Independent test fixture producer: Node Buffer/crypto only; no production codec is imported.
// Public synthetic seeds are fixture constants, never an installed user's private identity.
const {createHash,createPrivateKey,createPublicKey,sign}=require('node:crypto');
const {readFileSync,writeFileSync}=require('node:fs');
const {resolve}=require('node:path');
const b=(n,v)=>Buffer.alloc(n,v);
const cat=(...xs)=>Buffer.concat(xs);
const u=(n,v)=>{const out=Buffer.alloc(n);if(n===8)out.writeBigUInt64LE(BigInt(v));else if(n===4)out.writeUInt32LE(v);else out.writeUInt16LE(v);return out;};
const hash=x=>createHash('sha256').update(x).digest();
function key(seed){return createPrivateKey({key:cat(Buffer.from('302e020100300506032b657004220420','hex'),b(32,seed)),format:'der',type:'pkcs8'});}
const pub=k=>Buffer.from(createPublicKey(k).export({format:'der',type:'spki'})).subarray(-32);
const clientPeer=cat(Buffer.from('002408011220','hex'),pub(key(21)));
const serverPeer=cat(Buffer.from('002408011220','hex'),pub(key(22)));
const pf=p=>cat(Buffer.from([p.length]),p,b(128-p.length,0));
const limits=cat(...[4096,9216,8192,65536,131072].map(v=>u(4,v)),...[16,4,8].map(v=>u(2,v)));
function auth(lane,stage,frontier){return cat(Buffer.from('NF-PEER-AUTH-1\0'),Buffer.from([stage]),u(2,1),Buffer.from([lane]),pf(clientPeer),pf(serverPeer),...[1,2,3,4,5,6,7].map(v=>b(16,v)),...[8,9,10,11].map(v=>b(32,v)),u(4,lane),u(4,lane===1?2:1),u(4,3),u(4,3),limits,limits,limits,u(8,frontier));}
function header(kind,lane,session=5){return cat(Buffer.from('NF-PEER-1\0'),u(2,1),Buffer.from([kind,lane]),b(16,session),b(16,6),b(16,7),b(32,8),b(32,9));}
const descriptor=cat(b(16,13),u(8,6),u(2,2),hash(Buffer.from('abcdef')));
function operation(lane,stage,prefixDigest){return cat(Buffer.from(lane===1?'NF-PEER-QUERY-1\0':'NF-PEER-BULK-1\0'),Buffer.from([stage]),hash(auth(lane,0,0)),b(16,13),b(32,14),b(32,15),u(8,12),u(8,11),...(lane===2?[hash(descriptor)]:[]),prefixDigest);}
function proof(challenge,server){const peer=server?serverPeer:clientPeer;const fields=cat(b(16,6),b(16,7),b(16,server?3:1),b(16,server?4:2),u(8,12));const canonical=cat(Buffer.from('NF-CANON-1\0'),u(2,8),u(2,5),u(2,1),fields,u(4,peer.length),peer,challenge);return cat(fields,pf(peer),challenge,sign(null,hash(canonical),key(server?23:24)));}
const rows=[];const row=(kind,name,lane,expected,bytes,digest=Buffer.alloc(0))=>rows.push([kind,name,lane,expected,bytes.toString('hex'),digest.toString('hex')].join('\t'));
row('value','client-peer',0,'VALUE',clientPeer);row('value','server-peer',0,'VALUE',serverPeer);row('value','server-device-key',0,'VALUE',pub(key(23)));row('value','client-device-key',0,'VALUE',pub(key(24)));
for(const lane of [1,2]){for(const stage of [0,1,2,3]){const bytes=auth(lane,stage,stage===0?0:12);if(bytes.length!==619)throw Error('auth width');row('transcript',`auth-${lane}-${stage}`,lane,'619',bytes,hash(bytes));}}
for(const lane of [1,2]){const bytes=operation(lane,1,b(32,0));if(bytes.length!==(lane===1?177:208))throw Error('operation width');row('transcript',`op-${lane}-1`,lane,String(bytes.length),bytes,hash(bytes));}
const raw=new Map();function positive(name,lane,body){raw.set(name,body);row('raw',name,lane,'OK',body);}
for(const lane of [1,2]){positive(`hello-${lane}`,lane,cat(header(1,lane,0),b(16,1),b(16,2),b(32,10),u(4,lane),u(4,lane===1?2:1),limits));positive(`server-hello-${lane}`,lane,cat(header(2,lane),b(32,11),u(4,3),u(4,3),limits,limits,proof(hash(auth(lane,1,12)),true)));positive(`client-proof-${lane}`,lane,cat(header(3,lane),proof(hash(auth(lane,2,12)),false)));positive(`finished-${lane}`,lane,cat(header(4,lane),proof(hash(auth(lane,3,12)),true)));}
positive('begin-query',1,cat(header(5,1),b(16,13),b(32,14),u(8,11)));
positive('query-challenge',1,cat(header(6,1),b(16,13),b(32,14),b(32,15),u(8,12),hash(operation(1,1,b(32,0)))));
positive('prove-query',1,cat(header(7,1),b(16,13),b(32,14),proof(hash(operation(1,1,b(32,0))),false)));
const status=cat(header(8,1),b(16,13),b(16,0),b(32,0),Buffer.from([1,0]),u(8,0),Buffer.from([0]));positive('retained-unknown',1,cat(status,proof(hash(operation(1,2,hash(status))),true)));
const unsupported=cat(header(9,1),b(16,13),Buffer.from([1]));positive('unsupported-kernel',1,cat(unsupported,proof(hash(operation(1,2,hash(unsupported))),true)));
positive('begin-bulk',2,cat(header(10,2),descriptor,b(32,14),u(8,11)));
positive('bulk-challenge',2,cat(header(11,2),b(16,13),b(32,14),b(32,15),u(8,12),hash(operation(2,1,b(32,0)))));
positive('prove-bulk',2,cat(header(12,2),b(16,13),b(32,14),proof(hash(operation(2,1,b(32,0))),false)));
positive('bulk-chunk',2,cat(header(13,2),b(16,13),u(2,0),u(2,3),Buffer.from('abc')));
for(const [name,kind,fields]of [['bulk-ready',16,b(16,13)],['bulk-progress',15,cat(b(16,13),u(2,1),u(8,3))],['bulk-verified',14,cat(b(16,13),u(8,6),hash(Buffer.from('abcdef')))]]){const prefix=cat(header(kind,2),fields);const t=operation(2,2,hash(prefix));row('transcript',`${name}-reply`,2,'208',t,hash(t));positive(name,2,cat(prefix,proof(hash(t),true)));}
function bad(name,base,lane,expected,edit){const copy=Buffer.from(raw.get(base));edit(copy);row('raw',name,lane,expected,copy);}
bad('bad-version','begin-query',1,'Unsupported',x=>x.writeUInt16LE(2,10));bad('bad-lane','begin-query',1,'Unauthorized',x=>x[13]=2);bad('zero-session','begin-query',1,'Session',x=>x.fill(0,14,30));bad('unknown-kind','begin-query',1,'Unsupported',x=>x[12]=17);bad('required-cap','hello-1',1,'Unsupported',x=>x.writeUInt32LE(4,190));bad('over-chunk-offer','hello-1',1,'Limit',x=>x.writeUInt32LE(8193,206));bad('proof-padding','finished-1',1,'Malformed',x=>x[126+72+1+127]=1);bad('proof-scope','finished-1',1,'Scope',x=>x[126]=8);bad('zero-bulk-count','begin-bulk',2,'Limit',x=>x.writeUInt16LE(0,150));bad('zero-progress','bulk-progress',2,'Limit',x=>x.writeUInt16LE(0,142));
row('raw','oversized-truncated-chunk',2,'Limit',cat(header(13,2),b(16,13),u(2,0),u(2,8193)));row('raw','trailing-query',1,'Malformed',cat(raw.get('begin-query'),Buffer.from([0])));row('raw','oversized-frame',1,'Limit',b(4097,0));
const text='# NF-PEER-1 independent Node crypto/Buffer public synthetic vectors; columns category,name,lane,expected,hex,digest-or-key\n'+rows.join('\n')+'\n';const path=resolve(__dirname,'../../../docs/transport/vectors/peer-v1.tsv');if(process.argv.slice(2).join(' ')==='--check'){if(readFileSync(path,'utf8')!==text)throw Error('peer vectors differ');process.stdout.write('PEER_VECTORS_OK\n');}else if(process.argv.length===2){writeFileSync(path,text);process.stdout.write('PEER_VECTORS_WRITTEN\n');}else throw Error('closed arguments');
