// Independent synthetic profile vectors: fixed documented fields, Node standard SHA256 only.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const source = 'a669f4d0740e95a4acbb6b894dde09ade67aa754';
const rep = (n,size) => Buffer.alloc(size,n);
function writer() {
  const parts=[];
  const raw=b=>parts.push(Buffer.from(b));
  const u8=n=>raw([n]);
  const u16=n=>{const b=Buffer.alloc(2);b.writeUInt16LE(n);raw(b);};
  const u32=n=>{const b=Buffer.alloc(4);b.writeUInt32LE(n);raw(b);};
  const u64=n=>{const b=Buffer.alloc(8);b.writeBigUInt64LE(BigInt(n));raw(b);};
  const text=s=>{const b=Buffer.from(s,'utf8');u32(b.length);raw(b);};
  const list=(v,fn)=>{u32(v.length);v.forEach(fn);};
  const opt=(v,fn)=>{u8(v===null?0:1);if(v!==null)fn(v);};
  return {raw,u8,u16,u32,u64,text,list,opt,bytes:()=>Buffer.concat(parts)};
}
const concernDef=['warWeariness','exerelin.campaign.ai.concern.WarWearinessConcern',1];
const actionDef=['makePeace','exerelin.campaign.ai.action.MakePeaceAction',1];
const tags=['diplomacy','canMakePeace','trait_pacifist','trait_weak-willed','!trait_stalwart'];
function baseline() { return {
  observation:1,subject:'hegemony',factions:[['hegemony',1,['stalwart'],0x3f000000,[['warWeariness',0x3f800000]]]],relations:[],strengths:[],
  weariness:[1,1,0x459c4000,0x459c4000,[],0,0x459c4000],priorityRules:[0x3f000000,0x3fc00000,0x3f000000],
  concernConfig:[concernDef,1,0,tags,0x3f800000,0x3f800000],actionConfigs:[],
  concerns:[[9,concernDef,0,0x80000000,null,null,null,0,[['value',1,0x42480000]]]],
  timers:[4,0x3e000000,[],[]],extensions:[1,[],[],[]]
}; }
function rich() { const s=baseline();
  s.factions[0][2]=['stalwart','pacifist'];s.factions.push(['tritachyon',0,['weak-willed'],0,[]]);
  s.relations=[['hegemony','tritachyon',0xbf000000,0x80000000,1],['tritachyon','hegemony',0x3f000000,null,0]];
  s.strengths=[['hegemony',[['jangala',5,0x42c80000]],0x41200000],['tritachyon',[['eochu_bres',3,0x42480000]],0x40a00000]];
  s.weariness[4]=['tritachyon'];s.actionConfigs=[[actionDef,1,0,['diplomacy','canMakePeace','friendly'],0x3f000000,0x3f800000,0x40000000,'peace']];
  const c=s.concerns[0];c[4]=[8,actionDef,2,0,2];c[5]='tritachyon';c[6]='jangala';c[8].push(['alignment_diplomatic',3,0x3fa00000]);
  const d=structuredClone(c);d[0]=10;d[2]=1;d[4]=null;d[5]=null;d[6]=null;s.concerns.push(d);
  s.timers[2]=[['strategic-short',0x3f000000,0x3f800000,0x40000000],['diplomacy-brain',0,0,0x3f800000]];
  s.timers[3]=[['peace-choice',0,0x3fd0000000000000n],['diplomacy-interval',1,0x3fe0000000000000n]];return s;
}
function profile(s) {
  const w=writer();w.raw(Buffer.from('NF-NEX-WORLD-1\0'));w.u16(1);w.u8(1);w.u8(1);
  w.text(source);[1,2,3,4].forEach(n=>w.raw(rep(n,32)));[5,6].forEach(n=>w.raw(rep(n,16)));w.u64(1);w.u64(7);w.u8(s.observation);
  const def=d=>{w.text(d[0]);w.text(d[1]);w.u8(d[2]);};
  const mod=m=>{w.text(m[0]);w.u8(m[1]);w.u32(m[2]);};
  w.text(s.subject);w.list(s.factions,f=>{w.text(f[0]);w.u8(f[1]);w.list([...f[2]].sort((a,b)=>Buffer.compare(Buffer.from(a),Buffer.from(b))),w.text);w.u32(f[3]);w.list(f[4],p=>{w.text(p[0]);w.u32(p[1]);});});
  w.list(s.relations,r=>{w.text(r[0]);w.text(r[1]);w.u32(r[2]);w.opt(r[3],w.u32);w.u8(r[4]);});
  w.list(s.strengths,s=>{w.text(s[0]);w.list(s[1],m=>{w.text(m[0]);w.u8(m[1]);w.u32(m[2]);});w.u32(s[2]);});
  const v=s.weariness;w.u8(v[0]);w.u8(v[1]);w.u32(v[2]);w.u32(v[3]);w.list(v[4],w.text);w.u8(1);w.u8(v[5]);w.u32(v[6]);s.priorityRules.forEach(w.u32);
  const c=s.concernConfig;def(c[0]);w.u8(c[1]);w.u8(c[2]);w.list(c[3],w.text);w.u32(c[4]);w.u32(c[5]);
  w.list(s.actionConfigs,a=>{def(a[0]);w.u8(a[1]);w.u8(a[2]);w.list(a[3],w.text);w.u32(a[4]);w.u32(a[5]);w.u32(a[6]);w.opt(a[7],w.text);});
  w.list(s.concerns,c=>{w.raw(rep(c[0],16));def(c[1]);w.u8(c[2]);w.u32(c[3]);w.opt(c[4],a=>{w.raw(rep(a[0],16));def(a[1]);w.u8(a[2]);w.u8(a[3]);w.u32(a[4]);});w.opt(c[5],w.text);w.opt(c[6],w.text);w.u32(c[7]);w.list(c[8],mod);});
  const t=s.timers;w.u64(t[0]);w.u32(t[1]);w.list(t[2],i=>{w.text(i[0]);i.slice(1).forEach(w.u32);});w.list(t[3],d=>{w.text(d[0]);w.u32(d[1]);w.u64(d[2]);});
  const e=s.extensions;w.u8(e[0]);for(const list of [e[1],e[2]])w.list(list,r=>{w.text(r[0]);w.text(r[1]);});w.list(e[3],def);return w.bytes();
}
const sha=b=>crypto.createHash('sha256').update(b).digest();
function copiedBaselineUpdate() {
  const w=writer();w.raw(Buffer.from('NF-NEX-SHADOW-1\0'));w.u16(1);w.text(source);
  [1,2,3,4,1,2,3,4].forEach(n=>w.raw(rep(n,32)));[5,6,1,2,3].forEach(n=>w.raw(rep(n,16)));
  w.text('hegemony');w.u8(1);w.raw(rep(9,16));w.u64(1);w.u64(7);w.u8(1);w.u8(1);w.u8(1);w.u8(2);
  w.u32(0x459c4000);w.u32(0x459c4000);w.u8(0);w.u8(0);w.u32(1);w.u8(1);w.u8(0);
  [0x3f000000,0x3f000000,0x3fc00000,0x3f000000].forEach(w.u32);w.list(['stalwart'],w.text);w.u8(1);w.u32(0x3f800000);w.list(tags,w.text);
  w.u32(1);w.text('value');w.u8(1);w.u32(0x42480000);return w.bytes();
}
const cases=[['baseline',baseline()],['rich',rich()]];
const trait=rich();trait.factions[0][2].reverse();cases.push(['rich_traits_reordered',trait]);
const ordered=rich();ordered.concerns.reverse();cases.push(['rich_concerns_reordered',ordered]);
const absent=rich();absent.relations[0][3]=null;cases.push(['rich_disposition_absent',absent]);
const zero=baseline();zero.concerns[0][3]=0;cases.push(['baseline_positive_zero',zero]);
let output='# public synthetic NF-NEX-WORLD-1 profile|bytes|sha256\n';
for(const [name,s]of cases){const b=profile(s);output+=`${name}|${b.toString('hex')}|${sha(b).toString('hex')}\n`;}
const world=sha(profile(baseline())),copied=sha(copiedBaselineUpdate());const bind=writer();bind.raw(Buffer.from('NF-NEX-WORLD-EVAL-1\0'));bind.u16(1);bind.raw(world);bind.raw(copied);bind.u8(2);bind.u8(1);bind.raw(rep(9,16));
output+=`baseline_update_binding|${bind.bytes().toString('hex')}|${sha(bind.bytes()).toString('hex')}\n`;
output+=`baseline_update_copied|${copiedBaselineUpdate().toString('hex')}|${copied.toString('hex')}\n`;
const target=path.join(__dirname,'world-v1.tsv');
if(process.argv.includes('--check')) {if(fs.readFileSync(target,'utf8')!==output)throw Error('synthetic world fixture drift');}
else fs.writeFileSync(target,output);
