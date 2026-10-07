'use strict';
const {createHmac}=require('node:crypto');const {readFileSync,writeFileSync}=require('node:fs');const path=require('node:path');
const config={authVersion:1,capability:2,schema:2,protocol:1,role:1,port:12345,session:9,universe:'01'.repeat(16),history:'02'.repeat(16),ruleset:'03'.repeat(32),content:'04'.repeat(32),account:'05'.repeat(16),device:'06'.repeat(16),clientNonce:'08'.repeat(32),serverNonce:'09'.repeat(32),limits:[16384,8192,262144,32,16384,256,16,1048576]};
const key=Buffer.from(Array.from({length:32},(_,i)=>i));
const u32=n=>{const b=Buffer.alloc(4);b.writeUInt32LE(n);return b;};const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(n));return b;};const port=Buffer.alloc(2);port.writeUInt16LE(config.port);
const limits=Buffer.concat([...config.limits.slice(0,7).map(u32),u64(config.limits[7])]);
const tail=Buffer.concat([[1,2,2,1].map(u32),Buffer.from([1]),port,u64(9),['universe','history','ruleset','content','account','device','clientNonce','serverNonce'].map(k=>Buffer.from(config[k],'hex')),limits,limits].flat());
const vectors=[1,2,3].map(stage=>{const transcript=Buffer.concat([Buffer.from('NF-IPC-AUTH-1\0'),Buffer.from([stage]),tail]);if(transcript.length!==306)throw Error('Transcript width mismatch');return{stage,transcriptHex:transcript.toString('hex'),proofHex:createHmac('sha256',key).update(transcript).digest('hex')};});
// Independent protobuf fixture writer; no production codec import or generated-message oracle.
const vi=n=>{const result=[];do{let byte=n%128;n=Math.floor(n/128);if(n)byte|=128;result.push(byte);}while(n);return Buffer.from(result);};
const scalar=(number,n)=>Buffer.concat([vi(number*8),vi(n)]);const fixed=(number,n)=>Buffer.concat([vi(number*8+1),u64(n)]);const blob=(number,b)=>Buffer.concat([vi(number*8+2),vi(b.length),b]);const hex=s=>Buffer.from(s,'hex');
const resource=Buffer.concat([...config.limits.slice(0,7).map((n,i)=>scalar(i+1,n)),fixed(8,config.limits[7])]);
const principal=Buffer.concat([blob(1,blob(1,hex(config.account))),blob(2,blob(1,hex(config.device)))]);
const headers=(version=1,cap=2,schema=2,session=9)=>Buffer.concat([scalar(1,version),scalar(2,cap),scalar(3,schema),blob(4,fixed(1,session))]);
const hello=(nonce=hex(config.clientNonce),role=1,maximum=1)=>Buffer.concat([blob(1,nonce),scalar(2,role),blob(3,resource),blob(4,principal),blob(5,Buffer.concat([scalar(1,1),scalar(2,maximum)]))]);
const challenge=Buffer.concat([blob(1,hex(config.serverNonce)),blob(2,resource),blob(3,hex(vectors[0].proofHex))]);const proof=blob(1,hex(vectors[1].proofHex)),accepted=blob(1,hex(vectors[2].proofHex));
const valid=[['hello',10,hello()],['challenge',11,challenge],['proof',12,proof],['accepted',13,accepted]].map(([name,number,b])=>({name,wireHex:Buffer.concat([headers(),blob(number,b)]).toString('hex')}));
const invalid=[];function bad(name,error,b){invalid.push({name,error,wireHex:b.toString('hex')});}
bad('missing_header','SEMANTIC',Buffer.concat([scalar(1,1),scalar(3,2),blob(4,fixed(1,9)),blob(10,hello())]));bad('missing_body','SEMANTIC',headers());
for(const [name,args]of[['version',[2,2,2,9]],['capability',[1,1,2,9]],['schema',[1,2,1,9]]])bad(name,'UNSUPPORTED',Buffer.concat([headers(...args),blob(10,hello())]));
bad('zero_session','SEMANTIC',Buffer.concat([headers(1,2,2,0),blob(10,hello())]));bad('unknown_header','UNKNOWN_FIELD',Buffer.concat([headers(),blob(10,hello()),scalar(30,1)]));bad('duplicate_header','DUPLICATE',Buffer.concat([headers(),scalar(1,1),blob(10,hello())]));bad('two_bodies','DUPLICATE',Buffer.concat([headers(),blob(10,hello()),blob(12,proof)]));
bad('short_nonce','SEMANTIC',Buffer.concat([headers(),blob(10,hello(Buffer.alloc(31,8)))]));bad('oversized_nonce','LIMIT',Buffer.concat([headers(),blob(10,hello(Buffer.alloc(33,8)))]));bad('truncated_oversized_proof','LIMIT',Buffer.concat([headers(),blob(12,Buffer.from([10,33]))]));bad('unknown_role','SEMANTIC',Buffer.concat([headers(),blob(10,hello(hex(config.clientNonce),3))]));bad('future_protocol_offer','UNSUPPORTED',Buffer.concat([headers(),blob(10,hello(hex(config.clientNonce),1,2))]));
const document=JSON.stringify({schemaVersion:1,keyHex:key.toString('hex'),config,vectors,wire_valid:valid,wire_invalid:invalid},null,2)+'\n';const target=path.join(__dirname,'local-auth-v1.json');
if(process.argv.includes('--check')){if(readFileSync(target,'utf8')!==document){console.error('Local authentication fixtures differ; regenerate with this independent Node writer.');process.exitCode=1;}}else writeFileSync(target,document);
