'use strict';
// Independent private schema2 writer: explicit fields and public NF-world genesis fixture, never Rust.
const fs=require('node:fs'),path=require('node:path'),crypto=require('node:crypto');
const u16=n=>{const x=Buffer.alloc(2);x.writeUInt16LE(n);return x},u32=n=>{const x=Buffer.alloc(4);x.writeUInt32LE(n);return x},u64=n=>{const x=Buffer.alloc(8);x.writeBigUInt64LE(BigInt(n));return x};
const id=n=>Buffer.alloc(16,n),seed=Buffer.alloc(32,3),hash=x=>crypto.createHash('sha256').update(x).digest(),join=xs=>Buffer.concat(xs),header=n=>join([Buffer.from('NF-CANON-1\0'),u16(7),u16(n),u16(2)]);
const universe=id(1),history=id(2),aggregate=id(20),provider=id(21),providerAggregate=id(22);
const component=Buffer.from(fs.readFileSync(path.join(__dirname,'../../../../nf-world/fixtures/genesis-component.hex'),'utf8').trim(),'hex');
const rules=component.subarray(81,113);
const entity=(kind,n)=>hash(join([Buffer.from('NF-MINI-ID-1\0'),universe,history,seed,Buffer.from([kind]),u32(n)])).subarray(0,16);
const snapshot=(tick,seq,outcomes=[])=>join([header(1),universe,history,seed,rules,u64(tick),u64(seq),aggregate,provider,providerAggregate,u64(tick),u32(component.length),component,u32(outcomes.length),...outcomes]);
const intent=join([header(4),id(30),id(31),id(32),id(4),id(33),universe,history,provider,u32(2),aggregate,u64(0),providerAggregate,u64(0),Buffer.from([1]),entity(3,1),entity(2,0)]);
const bytes=x=>join([u32(x.length),x]);
const initial=snapshot(0,0),hold=join([id(31),entity(2,0),u64(100),u64(40)]);
const frontier=join([header(3),hash(initial),u64(7),u64(9),u64(1),u64(2),bytes(initial),u32(1),bytes(intent),Buffer.from([1]),hold]);
const batch=(disposition,tick,seq,admitted,outcomes,after)=>join([header(2),hash(initial),hash(after),Buffer.from([disposition]),u64(tick),u64(seq),u64(7),u64(9),u64(2),u32(admitted.length),...admitted.map(bytes),Buffer.from([0]),u32(0),u32(outcomes.length),...outcomes]);
const cancelled=join([id(31),id(32),Buffer.from([21])]);
const fixtures={'snapshot-genesis':initial,'intent-colony':intent,'frontier-colony':frontier,'batch-empty':batch(1,1,1,[],[],snapshot(1,1)),'batch-cancel':batch(2,0,1,[intent],[cancelled],snapshot(0,1,[cancelled]))};
for(const [name,value] of Object.entries(fixtures)){
 const target=path.join(__dirname,name+'.hex'),text=value.toString('hex')+'\n';
 if(process.argv.includes('--check')){if(fs.readFileSync(target,'utf8')!==text)throw new Error(name+' differs')}else fs.writeFileSync(target,text);
 console.log(name+': '+value.length+' bytes; SHA256 '+hash(value).toString('hex'));
}
