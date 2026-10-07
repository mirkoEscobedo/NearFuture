'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {fixture} = require('./fixture.cjs');
const {corpus} = require('./corpus.cjs');
const {hash,verifyDigest,uint,cat,canon} = require('./fields.cjs');
const {handshake,subscribe,notice,noticePrefix,ackPrefix} = require('./profiles.cjs');
test('ten record kinds have exact body widths and minimum negotiation remains data-only',()=>{
  const data=corpus(),records=data.rows.filter(r=>r.category==='record');
  assert.deepEqual([...new Set(records.map(r=>r.kind))].sort((a,b)=>a-b),[1,2,3,4,5,6,7,8,9,10]);
  const widths=new Map([[1,214],[2,493],[3,425],[4,425],[5,251],[6,248],[7,473],[8,516],[9,545],[10,482]]);
  for(const r of records)assert.equal(Buffer.from(r.hex,'hex').length,widths.get(r.kind),r.name);
  assert.deepEqual(data,corpus());
});
test('all contributed peer/session/selector fields affect their separate purpose digest',()=>{
  const c=fixture(),baseline=hash(handshake(c,1,c.membership));
  for(const name of ['session','universe','history','ruleset','content','client_nonce','server_nonce']) {
    const changed={...c,[name]:Buffer.from(c[name])};changed[name][0]^=1;
    assert.notDeepEqual(hash(handshake(changed,1,c.membership)),baseline,name);
  }
  for(const who of ['client','server'])for(const name of ['account','device','peer']) {
    const changed={...c,[who]:{...c[who],[name]:Buffer.from(c[who][name])}};changed[who][name][0]^=1;
    assert.notDeepEqual(hash(handshake(changed,1,c.membership)),baseline,who+'/'+name);
  }
  for(const set of ['offered','server_limits','selected'])for(const name of Object.keys(c[set])) {
    const changed={...c,[set]:{...c[set],[name]:c[set][name]-1}};
    assert.notDeepEqual(hash(handshake(changed,1,c.membership)),baseline,set+'/'+name);
  }
  for(const name of ['subscription','request','operation','binding','subscribe_nonce','server_subscribe_nonce']) {
    const changed={...c,[name]:Buffer.from(c[name])};changed[name][0]^=1;
    assert.notDeepEqual(hash(subscribe(changed,1)),hash(subscribe(c,1)),name);
  }
  for(const name of ['membership','minimum_membership','lifetime'])
    assert.notDeepEqual(hash(subscribe({...c,[name]:typeof c[name]==='bigint'?c[name]+1n:c[name]+1},1)),hash(subscribe(c,1)),name);
  for(const name of ['subscription','request','operation','binding','notice_nonce']) {
    const changed={...c,[name]:Buffer.from(c[name])};changed[name][0]^=1;
    assert.notDeepEqual(hash(notice(changed,1,hash(noticePrefix(changed)))),hash(notice(c,1,hash(noticePrefix(c)))),name);
  }
});
test('every supplied positive primitive signature verifies and changed digest rejects',()=>{
  const data=corpus(),rows=data.rows.filter(r=>r.category==='signature');assert.equal(rows.length,12);
  for(const row of rows) {
    const digest=Buffer.from(row.signed_digest,'hex'),sig=Buffer.from(row.hex,'hex'),key=Buffer.from(row.public_key,'hex');
    assert.deepEqual(hash(Buffer.from(row.canonical_hex,'hex')),digest);
    assert.equal(verifyDigest(digest,sig,key),true,row.name);digest[0]^=1;
    assert.equal(verifyDigest(digest,sig,key),false,row.name);
  }
});
test('signed auth negatives can have valid primitive signatures and still fail context',()=>{
  const c=fixture(),rows=corpus().rows.filter(r=>r.valid_signature===true);assert.equal(rows.length,7);
  for(const row of rows) {
    const p=Buffer.from(row.hex,'hex').subarray(-297),length=p[72];
    const canonical=cat(canon(8,5),p.subarray(0,72),uint(4,length),p.subarray(73,73+length),p.subarray(201,233));
    assert.equal(verifyDigest(hash(canonical),p.subarray(233),c[row.signer].public_key),true,row.name);
    assert.equal(row.expectation,'REJECT_CONTEXT');
  }
});
test('exact maximum sequence changes both prefixes and one-over integer cannot wrap',()=>{
  const c=fixture(),max={...c,sequence:18446744073709551615n};
  assert.equal(noticePrefix(max).readBigUInt64LE(144),18446744073709551615n);
  assert.equal(ackPrefix(max).readBigUInt64LE(144),18446744073709551615n);
  assert.throws(()=>uint(8,18446744073709551616n),RangeError);
  assert.throws(()=>uint(8,9007199254740992),RangeError);
});
test('every positive body has a malformed tail and timing inputs are explicitly not wire records',()=>{
  const rows=corpus().rows,names=new Set(rows.map(r=>r.name));
  for(const row of rows.filter(r=>r.category==='record')) {
    assert.ok(names.has(row.name+'-truncated'));assert.ok(names.has(row.name+'-trailing'));
  }
  const model=rows.filter(r=>r.category==='model');assert.equal(model.length,12);
  for(const row of model)assert.equal(row.wire_record,false);
});
