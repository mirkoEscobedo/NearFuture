'use strict';
const {createHash}=require('node:crypto');
const {readFileSync,writeFileSync}=require('node:fs');
const path=require('node:path');
// Independent standard-crypto fixture from the closed V2 specification; no Rust import.
const u16=n=>{const b=Buffer.alloc(2);b.writeUInt16LE(n);return b;};
const u32=n=>{const b=Buffer.alloc(4);b.writeUInt32LE(n);return b;};
const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(n));return b;};
const text=s=>Buffer.concat([u32(Buffer.byteLength(s,'utf8')),Buffer.from(s,'utf8')]);
const prefix=Buffer.concat([
    Buffer.from('NF-NEX-SHADOW-2\0','utf8'),u16(2),
    text('a669f4d0740e95a4acbb6b894dde09ade67aa754'),
    ...Array.from({length:8},(_,i)=>Buffer.alloc(32,i+1)),
    ...[11,12,15,16,17].map(n=>Buffer.alloc(16,n)),
    text('hegemony'),Buffer.from([0]),u64(13),u64(14),Buffer.from([1,1]),
]);
// Closed V2 kind1, diplomacy=true, concern=true, target absent, faction disabled=false.
const bytes=Buffer.concat([prefix,Buffer.from([1,1,1,0,0])]);
const digest=createHash('sha256').update(bytes).digest('hex');
if(bytes.length!==434||digest!=='5910e944560a7655a232d48c53ea2934da74f38ee5f6b941a5fbd007d01df738')throw Error('V2 specification drift');
const output='# name|private_input_hex|sha256\nno-target|'+bytes.toString('hex')+'|'+digest+'\n';
const args=process.argv.slice(2);
if(args.length>1||(args.length===1&&args[0]!=='--check'))throw Error('unsupported generator arguments');
const target=path.join(__dirname,'action-identity-v2.tsv');
if(args[0]==='--check') {
    if(readFileSync(target,'utf8')!==output) {console.error('Action shadow V2 identity fixture mismatch');process.exitCode=1;}
} else writeFileSync(target,output,'utf8');
