'use strict';
// Test-only explicit field writer. No production codecs, signatures or authority decisions.
const {createHash} = require('node:crypto');
const cat = (...parts) => Buffer.concat(parts);
const fixed = (value, width) => {
  if (!Buffer.isBuffer(value) || value.length !== width) throw Error('fixture field width');
  return value;
};
const u32 = value => { const b = Buffer.alloc(4); b.writeUInt32LE(value); return b; };
const u64 = value => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(value)); return b; };
const hash = value => createHash('sha256').update(value).digest();
const blob = value => cat(u32(value.length), value);
function auth(c) {
  return cat(Buffer.from('NF-MINI-AUTH-1\0'), Buffer.from([c.purpose]),
    fixed(c.universe,16), fixed(c.history,16), fixed(c.ruleset,32),
    ...[c.term,c.proposed_term,c.session,c.tick,c.membership].map(u64),
    fixed(c.actor,16), fixed(c.device,16), fixed(c.binding,32),
    u64(c.sequence), fixed(c.nonce,32));
}
function bootstrap(snapshot, membership, owner) {
  return cat(Buffer.from('NF-MINI-CREATE-1\0'), blob(snapshot), fixed(membership,32),
    fixed(owner.account,16), fixed(owner.account_key,32), fixed(owner.device,16),
    fixed(owner.device_key,32), blob(owner.peer));
}
function envelope(c) {
  const parts = [Buffer.from('NF-STORE-2\0\x02\0'), fixed(c.implementation,32), blob(c.snapshot)];
  const offsets = {authority: parts.reduce((n,p)=>n+p.length,0)};
  parts.push(Buffer.from([c.authority ? 1 : 0]));
  if (c.authority) {
    const a = c.authority;
    parts.push(u64(a.term),u64(a.session),fixed(a.account,16),fixed(a.device,16),u64(a.membership));
  }
  offsets.pending = parts.reduce((n,p)=>n+p.length,0);
  parts.push(Buffer.from([c.pending ? 1 : 0]));
  if (c.pending) parts.push(blob(c.pending));
  offsets.requests = parts.reduce((n,p)=>n+p.length,0);
  parts.push(u32(c.requests.length));
  for (const r of c.requests) {
    parts.push(fixed(r.request,16),blob(r.intent),Buffer.from([r.phase]));
    if (r.phase === 2) parts.push(u64(r.sequence),Buffer.from([r.rejection]));
  }
  offsets.holds = parts.reduce((n,p)=>n+p.length,0);
  parts.push(u32(c.holds.length));
  for (const h of c.holds) parts.push(fixed(h.operation,16),fixed(h.faction,16),u64(h.credits),u64(h.supplies));
  offsets.outbox = parts.reduce((n,p)=>n+p.length,0);
  parts.push(u32(c.outbox.length));
  for (const o of c.outbox) parts.push(fixed(o.operation,16),u64(o.sequence),fixed(o.digest,32),Buffer.from([o.rejection]));
  return {bytes:cat(...parts), offsets};
}
module.exports = {auth,bootstrap,envelope,hash,cat,u32,u64,blob};
