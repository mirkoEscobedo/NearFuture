'use strict';
const fs=require('node:fs'), path=require('node:path');
const target=path.join(__dirname,'wire-v1.json');
const canonical=JSON.parse(fs.readFileSync(path.join(__dirname,'nf-canon-1.json'),'utf8'));
const cat=(...v)=>Buffer.concat(v);
const varint=n=>{let v=BigInt(n),out=[];do {let b=Number(v&127n);v>>=7n;if(v)b|=128;out.push(b);}while(v);return Buffer.from(out);};
const scalar=(n,v)=>cat(varint(n*8),varint(v));
const bytes=(n,v)=>cat(varint(n*8+2),varint(v.length),v);
const fixed=(n,v)=>{let b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(v));return cat(varint(n*8+1),b);};
const utf=(n,s)=>bytes(n,Buffer.from(s,'utf8'));
const id=(n,v)=>bytes(n,bytes(1,Buffer.from(v,'hex')));
const session=bytes(2,fixed(1,1));
const required=(caps=[],schemas=[])=>cat(caps.length?bytes(1,cat(...caps.map(varint))):Buffer.alloc(0),schemas.length?bytes(2,cat(...schemas.map(varint))):Buffer.alloc(0));
const error=(reason='bad')=>cat(scalar(1,1),utf(2,reason));
const envelope=(body,number=18,req=required(),extra=Buffer.alloc(0))=>cat(scalar(1,1),session,bytes(3,req),bytes(number,body),extra);
const limits=cat(scalar(1,1048576),scalar(2,262144),scalar(3,16777216),scalar(4,256),scalar(5,1048576),scalar(6,4096),scalar(7,32),fixed(8,67108864));
const binding=canonical.request_bindings[0].record;
const bodyHex=canonical.positive[0].canonical_hex;
const payload=cat(scalar(1,1),bytes(2,Buffer.from(bodyHex,'hex')));
const probeRequired=required([1],[1]);
const principal=cat(id(1,binding.account_id),id(2,binding.device_id));
const intent=(req=probeRequired,sha=binding.payload_digest,body=payload)=>cat(id(1,binding.request_id),bytes(2,principal),id(3,binding.universe_id),id(4,binding.history_id),scalar(5,1),bytes(7,body),id(8,sha),bytes(10,req));
const handshake=cat(bytes(1,cat(scalar(1,1),scalar(2,1))),bytes(2,required()),id(4,binding.universe_id),id(5,binding.history_id),bytes(6,fixed(1,1)),id(7,'99'.repeat(32)),id(8,'aa'.repeat(32)),bytes(9,limits),bytes(10,Buffer.alloc(32,0xbb)));
const positive=[
  {name:'bounded-error',wire_hex:envelope(error()).toString('hex')},
  {name:'ignored-unknown-transport-field',wire_hex:envelope(error(),18,required(),bytes(30,scalar(99,7))).toString('hex')},
  {name:'known-required-contract-probe',wire_hex:envelope(intent(),12,probeRequired).toString('hex')},
  {name:'local-handshake',wire_hex:envelope(handshake,10).toString('hex')},
];
const bad=(name,value,error)=>({name,wire_hex:value.toString('hex'),error});
const malformed=[
  bad('empty-envelope',Buffer.alloc(0),'SEMANTIC'),
  bad('duplicate-protocol-version',cat(scalar(1,1),scalar(1,1)),'DUPLICATE'),
  bad('duplicate-envelope-body-oneof',cat(envelope(error()),bytes(12,Buffer.alloc(0))),'DUPLICATE'),
  bad('unknown-envelope-field',cat(scalar(99,1)),'UNKNOWN_FIELD'),
  bad('unknown-error-semantic-field',envelope(cat(error(),scalar(99,1))),'UNKNOWN_FIELD'),
  bad('wrong-wire-type',cat(fixed(1,1)),'MALFORMED'),
  bad('overlong-varint',Buffer.from([8,129,0]),'MALFORMED'),
  bad('uint32-overflow',scalar(1,4294967296n),'MALFORMED'),
  bad('truncated-length',Buffer.from([0x1a,2,0]),'MALFORMED'),
  bad('protobuf-group-rejected',Buffer.from([0x0b,0x0c]),'MALFORMED'),
  bad('invalid-zero-field-tag',Buffer.from([0]),'MALFORMED'),
  bad('unknown-required-capability',envelope(error(),18,required([999],[])),'UNSUPPORTED'),
  bad('unknown-required-schema',envelope(error(),18,required([],[999])),'UNSUPPORTED'),
  bad('duplicate-required-capability',envelope(error(),18,required([1,1],[])),'SEMANTIC'),
  bad('unordered-required-capability',envelope(error(),18,required([2,1],[])),'SEMANTIC'),
  bad('required-mismatch-envelope-body',envelope(intent(),12,required()),'SEMANTIC'),
  bad('short-request-id',envelope(cat(id(1,'11'.repeat(15)),intent().subarray(20)),12,probeRequired),'SEMANTIC'),
  bad('invalid-signature-width',envelope(cat(intent(),bytes(11,Buffer.alloc(65))),12,probeRequired),'SEMANTIC'),
  bad('wrong-payload-digest',envelope(intent(probeRequired,'66'.repeat(32)),12,probeRequired),'SEMANTIC'),
  bad('unknown-payload-schema',envelope(intent(probeRequired,binding.payload_digest,cat(scalar(1,999),bytes(2,Buffer.from(bodyHex,'hex')))),12,probeRequired),'UNSUPPORTED'),
  bad('malformed-canonical-payload',envelope(intent(probeRequired,binding.payload_digest,cat(scalar(1,1),bytes(2,Buffer.from([0])))),12,probeRequired),'SEMANTIC'),
  bad('unicode-after-profile-error-reason',envelope(error(String.fromCodePoint(0x105d2))),'SEMANTIC'),
  bad('noncharacter-error-reason',envelope(error(String.fromCodePoint(0xfdd0))),'SEMANTIC'),
  bad('non-nfc-error-reason',envelope(error('e\u0301')),'SEMANTIC'),
  bad('invalid-utf8-error-reason',envelope(cat(scalar(1,1),bytes(2,Buffer.from([0xc0,0x80])))),'MALFORMED'),
  bad('error-reason-limit',envelope(error('a'.repeat(513))),'LIMIT'),
  bad('declared-bytes-limit-before-allocation',envelope(error(),18,required(),bytes(30,cat(varint(99*8+2),varint(262145)))),'LIMIT'),
  bad('required-count-limit',envelope(error(),18,required(Array(65).fill(1),[])),'LIMIT'),
  bad('packed-required-duplicate-occurrence',envelope(error(),18,cat(required([1]),required([1]))),'DUPLICATE'),
  bad('packed-unpacked-required-mix',envelope(error(),18,cat(required([1]),scalar(1,1))),'DUPLICATE'),
  bad('invalid-boolean',envelope(cat(error(),scalar(5,2))),'MALFORMED'),
  bad('error-code-zero',envelope(cat(scalar(1,0))),'SEMANTIC'),
  bad('zero-optional-capability',envelope(cat(handshake,bytes(3,varint(0))),10),'SEMANTIC'),
];
const json=JSON.stringify({profile:'NF-WIRE-1',method:'Independent explicit protobuf tag/varint assembly; no generated encoder or production validator imports.',positive,malformed,
  frame_malformed:[{name:'zero-frame',frame_hex:'00000000',error:'MALFORMED'},{name:'oversized-declared-frame',frame_hex:'00100001',error:'LIMIT'},{name:'truncated-frame',frame_hex:'0000000208',error:'MALFORMED'}]},null,2)+'\n';
if(process.argv.includes('--check')) {
  if(!fs.existsSync(target)||fs.readFileSync(target,'utf8')!==json) {process.stderr.write('Wire fixtures differ.\n');process.exitCode=1;}
  else process.stdout.write(`Wire fixtures reproduce: ${positive.length} positive/${malformed.length} malformed.\n`);
} else fs.writeFileSync(target,json,'utf8');
