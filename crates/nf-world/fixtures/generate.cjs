'use strict';
// Independent public NF-MINI-1 typed genesis writer; never executes Rust.
const fs=require('node:fs');const path=require('node:path');const crypto=require('node:crypto');
const b=n=>Buffer.alloc(16,n),u32=n=>{const x=Buffer.alloc(4);x.writeUInt32LE(n);return x},u64=n=>{const x=Buffer.alloc(8);x.writeBigUInt64LE(BigInt(n));return x};
const universe=b(1),history=b(2),seed=Buffer.alloc(32,3),accounts=[b(4),b(5),b(6)];
const config=Buffer.from('NF-MINI-1\0systems=3;factions=3;markets=6;fleets=3;home=even;routes=triangle;credits=200;supplies=100;colony=100,40;industry=60,20,2;travel=0,20,3;relation=-10000,10000;frontier=64;schedules=32;slots=2;winner=1;hour=1;seedrng=none');
const id=(kind,n)=>crypto.createHash('sha256').update(Buffer.concat([Buffer.from('NF-MINI-ID-1\0'),universe,history,seed,Buffer.from([kind]),u32(n)])).digest().subarray(0,16);
const rows=(count,kind,writer)=>Buffer.concat([u32(count),...Array.from({length:count},(_,i)=>({i,id:id(kind,i)})).sort((a,c)=>Buffer.compare(a.id,c.id)).map(writer)]);
const system=rows(3,1,({i,id})=>Buffer.concat([id,Buffer.from([i])]));
const faction=rows(3,2,({i,id})=>Buffer.concat([id,Buffer.from([i]),accounts[i],u64(200),u64(100),u64(0),u64(0)]));
const market=rows(6,3,({i,id:site})=>Buffer.concat([site,Buffer.from([i]),id(1,Math.floor(i/2)),Buffer.from([i%2===0?1:0]),...(i%2===0?[id(2,Math.floor(i/2))]:[]),Buffer.from([0,0])]));
const fleet=rows(3,4,({i,id:fleet})=>Buffer.concat([fleet,id(2,i),Buffer.from([1]),id(1,i)]));
const relation=rows(3,5,({i,id:pair})=>{const [a,c]=[[0,1],[0,2],[1,2]][i];const x=[id(2,a),id(2,c)].sort(Buffer.compare);return Buffer.concat([pair,...x,u64(0)])});
const header=Buffer.concat([Buffer.from('NF-CANON-1\0'),Buffer.from([7,0,5,0,2,0])]);
const value=Buffer.concat([header,universe,history,seed,crypto.createHash('sha256').update(config).digest(),...accounts,u64(0),system,faction,market,fleet,relation,u32(0)]);
const target=path.join(__dirname,'genesis-component.hex');const text=value.toString('hex')+'\n';
if(process.argv.includes('--check')){if(fs.readFileSync(target,'utf8')!==text)throw new Error('World component fixture differs');}else fs.writeFileSync(target,text);
console.log('NF-MINI-1 independent genesis: '+value.length+' bytes; SHA256 '+crypto.createHash('sha256').update(value).digest('hex'));
