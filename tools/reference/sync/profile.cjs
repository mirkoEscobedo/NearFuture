'use strict';
const {cat,bytes,fixed,uint,id}=require('./fields.cjs');
const PROTOCOLS={1:'/nearfuture/peer/sync/1',2:'/nearfuture/peer/sync-transfer/1'};
const MAX={control_frame:1024,transfer_frame:9216,chunk_bytes:8192,control_queue_bytes:16384,transfer_queue_bytes:32768,control_items:4,transfer_items:2,pending_challenges:1};
const LIMIT_KEYS=Object.keys(MAX);
function validateLimits(l){
  for(const name of LIMIT_KEYS)if(!Number.isSafeInteger(l[name])||l[name]<1||l[name]>MAX[name])throw RangeError('sync limit range');
  if(l.control_frame!==1024||l.transfer_frame<1024||l.chunk_bytes<256||l.chunk_bytes>l.transfer_frame-200||l.control_queue_bytes<l.control_frame||l.transfer_queue_bytes<l.transfer_frame)throw RangeError('sync coupled limits');
  return l;
}
function limits(l){
  validateLimits(l);return cat(...LIMIT_KEYS.map((name,i)=>uint(i<5?4:2,l[name])));
}
function negotiate(a,b){
  validateLimits(a);validateLimits(b);return validateLimits(Object.fromEntries(LIMIT_KEYS.map(k=>[k,Math.min(a[k],b[k])])));
}
function session(c,lane){return c.sessions?.[lane]??c.session;}
function header(c,kind,lane,s=kind===1?bytes(16):session(c,lane)){
  if(!Number.isInteger(kind)||kind<1||kind>17||!PROTOCOLS[lane])throw RangeError('sync closed registry');
  if(kind>=5&&((kind<=9||kind>=16)?lane!==1:lane!==2))throw RangeError('sync kind lane');
  fixed(s,16);if(kind===1?!s.equals(bytes(16)):s.equals(bytes(16)))throw RangeError('sync session shape');
  return cat(Buffer.from('NF-SYNC-1\0'),uint(2,1),uint(1,kind),uint(1,lane),s,id(c.universe),id(c.history),fixed(c.ruleset,32),fixed(c.content,32));
}
function frame(raw){
  const prefix=bytes(4);prefix.writeUInt32BE(raw.length);return cat(prefix,raw);
}
function admitLength(n,lane,selected=MAX){
  validateLimits(selected);const cap=lane===1?selected.control_frame:lane===2?Math.min(selected.transfer_frame,8392):0;
  if(!Number.isSafeInteger(n)||n<126||n>cap)throw RangeError('sync frame length');return n;
}
function chunkCount(total,l){
  validateLimits(l);const maximum=Math.min(2105344,257*l.chunk_bytes);
  if(!Number.isSafeInteger(total)||total<1||total>maximum)throw RangeError('sync document limit');
  return Math.ceil(total/l.chunk_bytes);
}
module.exports={PROTOCOLS,MAX,LIMIT_KEYS,validateLimits,limits,negotiate,session,header,frame,admitLength,chunkCount};
