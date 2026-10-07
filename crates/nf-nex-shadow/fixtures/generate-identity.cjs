'use strict';
const {createHash}=require('node:crypto');const {readFileSync,writeFileSync}=require('node:fs');const path=require('node:path');
// Independent standard-crypto private identity fixture; no Rust codec or expected output import.
const u16=n=>{let b=Buffer.alloc(2);b.writeUInt16LE(n);return b;};const u32=n=>{let b=Buffer.alloc(4);b.writeUInt32LE(n);return b;};const u64=n=>{let b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(n));return b;};const text=s=>Buffer.concat([u32(Buffer.byteLength(s)),Buffer.from(s)]);
const prefix=Buffer.concat([Buffer.from('NF-NEX-SHADOW-1\0'),u16(1),text('a669f4d0740e95a4acbb6b894dde09ade67aa754'),...Array.from({length:8},(_,i)=>Buffer.alloc(32,i+1)),...[11,12,15,16,17].map(n=>Buffer.alloc(16,n)),text('hegemony'),Buffer.from([0]),u64(13),u64(14),Buffer.from([1,1])]);
const tags=['diplomacy','canMakePeace','trait_pacifist','trait_weak-willed','!trait_stalwart'];
const war=Buffer.concat([Buffer.from([1,1]),u32(0x456a6000),u32(0x459c4000),Buffer.from([0,1]),u32(0),...[0x3f000000,0x3e800000,0x3fa66666,0x3f333333].map(u32),u32(0),Buffer.from([0]),u32(5),...tags.map(text),u32(1),text('value'),Buffer.from([1]),u32(0x3f800000)]);
const bytes=Buffer.concat([prefix,war]);const output=`# name|private_input_hex|sha256\nequal|${bytes.toString('hex')}|${createHash('sha256').update(bytes).digest('hex')}\n`;const target=path.join(__dirname,'identity-v1.tsv');
if(process.argv.includes('--check')) {if(readFileSync(target,'utf8')!==output) {console.error('Private shadow identity fixture mismatch');process.exitCode=1;}}else writeFileSync(target,output);
