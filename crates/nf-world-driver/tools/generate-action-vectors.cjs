'use strict';
// Independent public NF-MINI-ID-1 vectors. No Rust serializer/resolver is invoked.
const crypto=require('node:crypto'),fs=require('node:fs'),path=require('node:path');
const cases=[['site1',3,1],['site2',3,2],['faction0',2,0],['faction1',2,1],['fleet0',4,0],['system1',1,1]];
const records=cases.map(([name,kind,ordinal])=>{
 const n=Buffer.alloc(4);n.writeUInt32LE(ordinal);
 const preimage=Buffer.concat([Buffer.from('NF-MINI-ID-1\0'),Buffer.alloc(16,1),Buffer.alloc(16,2),Buffer.alloc(32,3),Buffer.from([kind]),n]);
 return name+'\t'+crypto.createHash('sha256').update(preimage).digest().subarray(0,16).toString('hex');
});
const text='# Public scope: universe=01x16 history=02x16 seed=03x32; kind/ordinal fixed in independent producer\n'+records.join('\n')+'\n';
const target=path.join(__dirname,'../fixtures/action-ids.tsv');
if(process.argv.includes('--check')){if(fs.readFileSync(target,'utf8')!==text)throw new Error('action entity corpus differs');}else fs.writeFileSync(target,text);
console.log('PASS '+cases.length+' independent public action entity vectors');
