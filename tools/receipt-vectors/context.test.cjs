'use strict';
const {test} = require('node:test');
const assert = require('node:assert/strict');
const {fixture} = require('./fixture.cjs');
const {hash,bytes,uint,cat,canon,verifyDigest,peerField} = require('./fields.cjs');
const {handshake,operation} = require('./profiles.cjs');
const {corpus} = require('./corpus.cjs');
test('all contributed handshake fields affect authenticated context bytes',()=>{
  const c=fixture(),baseline=hash(handshake(c,1,12));
  for(const name of ['session','universe','history','ruleset','content','client_nonce','server_nonce']) {
    const changed={...c,[name]:Buffer.from(c[name])};changed[name][0]^=1;
    assert.notDeepEqual(hash(handshake(changed,1,12)),baseline,name);
  }
  for(const who of ['client','server']) for(const name of ['account','device','peer']) {
    const changed={...c,[who]:{...c[who],[name]:Buffer.from(c[who][name])}};changed[who][name][0]^=1;
    assert.notDeepEqual(hash(handshake(changed,1,12)),baseline,who+'/'+name);
  }
  for(const name of ['required','optional','available','selected'])
    assert.notDeepEqual(hash(handshake({...c,[name]:(c[name]^1)>>>0},1,12)),baseline,name);
  for(const set of ['offered','server_limits','selected_limits']) for(const name of Object.keys(c[set])) {
    const changed={...c,[set]:{...c[set],[name]:c[set][name]-1}};
    assert.notDeepEqual(hash(handshake(changed,1,12)),baseline,set+'/'+name);
  }
  assert.notDeepEqual(hash(handshake(c,1,13)),baseline,'proof frontier');
});
test('all receipt operation selectors/minima/nonces and signed prefix affect purpose digest',()=>{
  const c=fixture(),baseline=hash(operation(c,1));
  for(const name of ['request','operation','binding','operation_nonce','server_operation_nonce']) {
    const changed={...c,[name]:Buffer.from(c[name])};changed[name][0]^=1;
    assert.notDeepEqual(hash(operation(changed,1)),baseline,name);
  }
  for(const name of ['membership','minimum_membership','minimum_event','minimum_store'])
    assert.notDeepEqual(hash(operation({...c,[name]:c[name]+1n},1)),baseline,name);
  assert.notDeepEqual(hash(operation(c,2,bytes(32,1))),hash(operation(c,2,bytes(32))), 'reply prefix');
});
test('u64 exact maximum, one-over and fixed peer boundary use standard checked primitives',()=>{
  assert.equal(uint(8,18446744073709551615n).toString('hex'),'ffffffffffffffff');
  assert.throws(()=>uint(8,18446744073709551616n),RangeError);
  assert.throws(()=>uint(8,-1n),RangeError);
  assert.throws(()=>uint(8,9007199254740992),RangeError);
  assert.equal(peerField(bytes(128,1)).length,129);
  assert.throws(()=>peerField(bytes(129,1)),/width/);
  assert.throws(()=>peerField(bytes(0)),/width/);
  const rows=corpus().rows,capacity=rows.filter(r=>r.category==='capacity');
  assert.equal(capacity.length,2);assert.equal(capacity[0].next_generation,8);assert.equal(capacity[1].requested_slot,8);
});

test('signed context negatives retain valid primitive signatures without granting admission',()=>{
  const c=fixture(),rows=corpus().rows.filter(r=>r.valid_signature===true);
  assert.ok(rows.length>=8);
  for(const row of rows) {
    const raw=Buffer.from(row.hex,'hex'),p=raw.subarray(-297),length=p[72];
    const canonical=cat(canon(8,5),p.subarray(0,72),uint(4,length),p.subarray(73,73+length),p.subarray(201,233));
    assert.equal(verifyDigest(hash(canonical),p.subarray(233),c.server.public_key),true,row.name);
    assert.equal(row.expectation,'REJECT_CONTEXT');
  }
});