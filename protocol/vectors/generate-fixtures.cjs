'use strict';
// Independent specification fixture generator; not a production codec/parser.
// Buffer scalar operations assemble the published byte grammar directly.
// SHA-256 is supplied by Node's maintained standard crypto implementation.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const out = path.join(__dirname, 'nf-canon-1.json');
const byte = n => Buffer.from([n]);
const u16 = n => { const b = Buffer.alloc(2); b.writeUInt16LE(n); return b; };
const u32 = n => { const b = Buffer.alloc(4); b.writeUInt32LE(n); return b; };
const u64 = n => { const b = Buffer.alloc(8); b.writeBigUInt64LE(BigInt(n)); return b; };
const i64 = n => { const b = Buffer.alloc(8); b.writeBigInt64LE(BigInt(n)); return b; };
const cat = (...parts) => Buffer.concat(parts);
const header = (domain, type, version = 1) => cat(Buffer.from('NF-CANON-1\0', 'ascii'), u16(domain), u16(type), u16(version));
const sized = b => cat(u32(b.length), b);
const text = s => sized(Buffer.from(s, 'utf8'));
const digest = b => crypto.createHash('sha256').update(b).digest('hex');
const fixture = r => cat(header(r.domain, r.type, r.schema_version),
    r.optional_bytes_hex === null ? byte(0) : cat(byte(1), sized(Buffer.from(r.optional_bytes_hex, 'hex'))),
    r.optional_text === null ? byte(0) : cat(byte(1), text(r.optional_text)),
    u32(r.integers.length), ...r.integers.map(i64), u32(r.counters.length),
    ...r.counters.map(e => cat(text(e.key), u64(e.value))));
const record = overrides => ({ domain: 255, type: 1, schema_version: 1,
    optional_bytes_hex: null, optional_text: null, integers: [], counters: [], ...overrides });
const positiveRecords = [
    ['absent-optionals-empty-collections', record({})],
    ['present-empty-bytes', record({optional_bytes_hex: ''})],
    ['present-empty-text', record({optional_text: ''})],
    ['present-binary-bytes', record({optional_bytes_hex: '00ff80'})],
    ['nfc-unicode-supplementary', record({optional_text: 'caf\u00e9 \ud83d\ude80'})],
    ['signed-integer-boundaries', record({integers: ['-9223372036854775808', '-1', '0', '1', '9223372036854775807']})],
    ['unsigned-boundaries-utf8-map-order', record({counters: [
        {key: 'a', value: '0'}, {key: 'zz', value: '1'},
        {key: '\u00e9', value: '18446744073709551615'}, {key: '\ud83d\ude00', value: '9223372036854775808'}]})],
    ['ordered-array-is-semantic', record({integers: ['1', '0', '-1']})],
];
const positive = positiveRecords.map(([name, r]) => {
    const bytes = fixture(r);
    return {name, record: r, canonical_hex: bytes.toString('hex'), sha256: digest(bytes)};
});
const emptyBody = cat(byte(0), byte(0), u32(0), u32(0));
const hdr = header(255, 1);
const bad = (name, b, error) => ({name, canonical_hex: b.toString('hex'), error});
const textRecord = b => cat(hdr, byte(0), byte(1), sized(b), u32(0), u32(0));
const mapRecord = entries => cat(hdr, byte(0), byte(0), u32(0), u32(entries.length), ...entries);
const malformed = [
    bad('empty-input', Buffer.alloc(0), 'TRUNCATED'),
    bad('truncated-header', hdr.subarray(0, 16), 'TRUNCATED'),
    bad('wrong-profile-prefix', cat(Buffer.from('XF-CANON-1\0', 'ascii'), hdr.subarray(11), emptyBody), 'UNKNOWN_PROFILE'),
    bad('unknown-record-domain', cat(header(254, 1), emptyBody), 'UNKNOWN_RECORD'),
    bad('unknown-record-type', cat(header(255, 2), emptyBody), 'UNKNOWN_RECORD'),
    bad('unknown-record-schema', cat(header(255, 1, 2), emptyBody), 'UNKNOWN_RECORD'),
    bad('invalid-optional-presence', cat(hdr, byte(2), byte(0), u32(0), u32(0)), 'INVALID_PRESENCE'),
    bad('truncated-present-bytes', cat(hdr, byte(1), u32(2), byte(0xff)), 'TRUNCATED'),
    bad('trailing-byte', cat(hdr, emptyBody, byte(0)), 'TRAILING_BYTES'),
    bad('unicode-after-profile-version', textRecord(Buffer.from(String.fromCodePoint(0x105d2), 'utf8')), 'OUTSIDE_TEXT_PROFILE'),
    bad('unassigned-text-scalar', textRecord(Buffer.from(String.fromCodePoint(0x0378), 'utf8')), 'OUTSIDE_TEXT_PROFILE'),
    bad('noncharacter-text-scalar', textRecord(Buffer.from(String.fromCodePoint(0xfdd0), 'utf8')), 'OUTSIDE_TEXT_PROFILE'),
    bad('non-nfc-text', textRecord(Buffer.from('e\u0301', 'utf8')), 'NON_NFC'),
    bad('invalid-utf8-continuation', textRecord(Buffer.from([0xc3, 0x28])), 'INVALID_UTF8'),
    bad('overlong-utf8', textRecord(Buffer.from([0xc0, 0x80])), 'INVALID_UTF8'),
    bad('encoded-surrogate', textRecord(Buffer.from([0xed, 0xa0, 0x80])), 'INVALID_UTF8'),
    bad('unicode-out-of-range', textRecord(Buffer.from([0xf4, 0x90, 0x80, 0x80])), 'INVALID_UTF8'),
    bad('duplicate-map-key', mapRecord([cat(text('a'), u64('1')), cat(text('a'), u64('2'))]), 'DUPLICATE_KEY'),
    bad('unordered-map-keys', mapRecord([cat(text('\u00e9'), u64('1')), cat(text('zz'), u64('2'))]), 'KEY_ORDER'),
    bad('bytes-limit-before-allocation', cat(hdr, byte(1), u32(262145)), 'LIMIT_EXCEEDED'),
    bad('text-limit-before-allocation', cat(hdr, byte(0), byte(1), u32(4097)), 'LIMIT_EXCEEDED'),
    bad('list-count-limit-before-allocation', cat(hdr, byte(0), byte(0), u32(4097)), 'LIMIT_EXCEEDED'),
    bad('map-count-limit-before-allocation', cat(hdr, byte(0), byte(0), u32(0), u32(4097)), 'LIMIT_EXCEEDED'),
    bad('max-u32-length-no-allocation', cat(hdr, byte(1), u32(0xffffffff)), 'LIMIT_EXCEEDED'),
];
const probePayload = cat(header(6, 1), u32(1), sized(fixture(record({}))));
const baseBinding = {request_id: '11'.repeat(16), account_id: '22'.repeat(16),
    device_id: '33'.repeat(16), universe_id: '44'.repeat(16), history_id: '55'.repeat(16),
    operation_kind: 1, payload_digest: digest(probePayload)};
const bindingBytes = r => cat(header(1, 1), ...['request_id', 'account_id', 'device_id', 'universe_id', 'history_id'].map(k => Buffer.from(r[k], 'hex')),
    u32(r.operation_kind), Buffer.from(r.payload_digest, 'hex'));
const bindingVariants = [ ['original', {}], ['changed-account', {account_id:'66'.repeat(16)}],
    ['changed-device', {device_id:'66'.repeat(16)}], ['changed-universe', {universe_id:'66'.repeat(16)}],
    ['changed-history', {history_id:'66'.repeat(16)}], ['changed-kind', {operation_kind:2}],
    ['changed-payload', {payload_digest:'66'.repeat(32)}] ];
const request_bindings = bindingVariants.map(([name, changes]) => {
    const r = {...baseBinding, ...changes}; const b = bindingBytes(r);
    return {name, record:r, canonical_hex:b.toString('hex'), sha256:digest(b),
        same_request_id_as_original:true, expected_relation:name==='original'?'IDENTICAL':'DISTINCT_BINDING'};
});
const requiredRecords = [
    {name:'known-empty-required', capability_ids:[], schema_ids:[], expected:'ACCEPT'},
    {name:'known-probe-schema-required', capability_ids:[], schema_ids:[1], expected:'ACCEPT'},
    {name:'unknown-required-schema', capability_ids:[], schema_ids:[999], expected:'UNKNOWN_REQUIRED_SEMANTIC'},
    {name:'unknown-required-capability', capability_ids:[999], schema_ids:[], expected:'UNKNOWN_REQUIRED_SEMANTIC'},
    {name:'duplicate-required-schema', capability_ids:[], schema_ids:[1,1], expected:'DUPLICATE_KEY'},
    {name:'unordered-required-schema', capability_ids:[], schema_ids:[2,1], expected:'KEY_ORDER'},
].map(r => {
    const b=cat(header(6,2),u32(r.capability_ids.length),...r.capability_ids.map(u32),u32(r.schema_ids.length),...r.schema_ids.map(u32));
    return {...r,canonical_hex:b.toString('hex')};
});
// Closed semantic envelopes: deliberately empty world collections, not live game data.
const emptyRequired = cat(header(6,2),u32(0),u32(0));
const idBytes = hex => Buffer.from(hex,'hex');
const rulesetHash = '99'.repeat(32);
const originalBinding = bindingBytes(baseBinding);
const emptyIntent = cat(header(1,2),originalBinding,u32(0),probePayload,byte(0),emptyRequired);
const intentAtZeroExpiry = cat(header(1,2),originalBinding,u32(0),probePayload,byte(1),u64('0'),emptyRequired);
const emptySnapshot = cat(header(2,1),idBytes(baseBinding.universe_id),idBytes(baseBinding.history_id),u64('0'),u64('0'),idBytes(rulesetHash),
    u32(0),u32(0),u32(0),u32(0),u32(0),u32(0),u32(0),emptyRequired);
const emptyProposal = cat(header(3,1),idBytes('77'.repeat(16)),idBytes('88'.repeat(16)),u32(1),idBytes(rulesetHash),idBytes(digest(fixture(record({})))),
    u64('1'),u32(0),u32(0),u32(0),u32(0),emptyRequired);
const genesisBatch = cat(header(4,1),idBytes(baseBinding.history_id),u64('1'),idBytes('00'.repeat(32)),u64('1'),u64('0'),idBytes(rulesetHash),
    u32(0),u32(0),u32(0),idBytes(digest(emptySnapshot)),emptyRequired);
const semantic_records = [
    {name:'schema-payload-contract-probe',domain:6,type:1,fields:{schema_id:1,canonical_body_hex:fixture(record({})).toString('hex')},bytes:probePayload},
    {name:'intent-empty-revisions-absent-expiry',domain:1,type:2,fields:{binding:baseBinding,expected_revisions:[],payload_schema_id:1,payload_body_hex:fixture(record({})).toString('hex'),expiry_tick:null,required_capabilities:[],required_schemas:[]},bytes:emptyIntent},
    {name:'intent-empty-revisions-present-zero-expiry',domain:1,type:2,fields:{binding:baseBinding,expected_revisions:[],payload_schema_id:1,payload_body_hex:fixture(record({})).toString('hex'),expiry_tick:'0',required_capabilities:[],required_schemas:[]},bytes:intentAtZeroExpiry},
    {name:'empty-world-snapshot',domain:2,type:1,fields:{universe_id:baseBinding.universe_id,history_id:baseBinding.history_id,event_seq:'0',world_tick:'0',ruleset_hash:rulesetHash,aggregate_revisions:[],entities:[],module_state:[],scheduled_events:[],reservations:[],unresolved_operations:[],deduplication_state:[],required_capabilities:[],required_schemas:[]},bytes:emptySnapshot},
    {name:'empty-proposal',domain:3,type:1,fields:{job_id:'77'.repeat(16),provider_id:'88'.repeat(16),provider_version:1,ruleset_hash:rulesetHash,input_hash:digest(fixture(record({}))),runtime_session:'1',read_set:[],write_set:[],proposed_events:[],module_state_delta:[],required_capabilities:[],required_schemas:[]},bytes:emptyProposal},
    {name:'genesis-empty-event-batch',domain:4,type:1,fields:{history_id:baseBinding.history_id,event_seq:'1',previous_hash:'00'.repeat(32),authority_term:'1',world_tick:'0',ruleset_hash:rulesetHash,request_outcomes:[],events:[],module_state_changes:[],state_hash:digest(emptySnapshot),required_capabilities:[],required_schemas:[]},bytes:genesisBatch},
].map(({bytes,...r}) => ({...r,schema_version:1,canonical_hex:bytes.toString('hex'),sha256:digest(bytes)}));
// RFC 8032 section 7.1 public test seed/key; never an operational credential.
const testSeed='9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
const expectedPublic='d75a980182b10ab7d54bfed3c964073a0ee172f3daa62325af021a68f707511a';
const privateKey=crypto.createPrivateKey({key:Buffer.from('302e020100300506032b657004220420'+testSeed,'hex'),format:'der',type:'pkcs8'});
const publicKey=crypto.createPublicKey(privateKey);
if(publicKey.export({format:'der',type:'spki'}).subarray(-32).toString('hex')!==expectedPublic) throw new Error('RFC test key mismatch');
const signatures=semantic_records.filter(r=>r.domain===1&&r.type===2).map(r=>{
    const bytes=Buffer.from(r.sha256,'hex');
    const signature=crypto.sign(null,bytes,privateKey);
    if(!crypto.verify(null,bytes,publicKey,signature))throw new Error('Standard Ed25519 fixture verification failed');
    return {name:r.name,test_seed_hex:testSeed,public_key_hex:expectedPublic,canonical_digest_hex:r.sha256,signature_hex:signature.toString('hex')};
});
// Negative signature data: explicit malformed points, lengths and known-answer mutations.
// Mixed-torsion bytes were produced with maintained curve25519-dalek 4.1.3
// ED25519_BASEPOINT_POINT + EIGHT_TORSION[1] in an isolated review probe.
// They satisfy Dalek 2.2.0 verify_strict but violate this profile's prime-subgroup policy.
const signatureBad=(name,key,digest,sig,provenance='Explicit fixed bytes or mutation of independent RFC-key signature fixture.')=>
    ({name,public_key_hex:key,canonical_digest_hex:digest,signature_hex:sig,expected:'REJECT',provenance});
const identityPoint='01'+'00'.repeat(31), zeroPoint='00'.repeat(32);
const firstSignature=signatures[0];
const alteredDigest=Buffer.from(firstSignature.canonical_digest_hex,'hex');alteredDigest[0]^=1;
const alteredSignature=Buffer.from(firstSignature.signature_hex,'hex');alteredSignature[63]^=1;
const mixedPoint='98519eadf35b995233b51b5cd23e9cc5a28b639b5a4af0ec903cb960d81b7819';
const mixedProvenance='Maintained curve25519-dalek 4.1.3 isolated review probe: B + EIGHT_TORSION[1]; Dalek 2.2.0 verify_strict accepted before prime-subgroup boundary repair.';
const negative_signatures=[
    signatureBad('identity-key-forgery',identityPoint,zeroPoint,identityPoint+zeroPoint),
    signatureBad('identity-key-forgery-changed-digest',identityPoint,'ff'.repeat(32),identityPoint+zeroPoint),
    signatureBad('zero-small-order-key',zeroPoint,zeroPoint,zeroPoint+zeroPoint),
    signatureBad('mixed-torsion-key-valid-equation',mixedPoint,'01'+'00'.repeat(31),'586666666666666666666666666666666666666666666666666666666666666641f7d1fa3173fd39e1e32795778945fb4d7d340456be9875ee976b037cd91903',mixedProvenance),
    signatureBad('mixed-torsion-key-and-r-valid-equation',mixedPoint,'04'+'00'.repeat(31),'98519eadf35b995233b51b5cd23e9cc5a28b639b5a4af0ec903cb960d81b7819d857ddf5e552a8465f8e4c1982a2b06bdfa3d3935540f40148dd3429a57ec10b',mixedProvenance),
    signatureBad('noncanonical-public-key', 'ee'+'ff'.repeat(30)+'7f',zeroPoint,identityPoint+zeroPoint),
    signatureBad('identity-r',expectedPublic,firstSignature.canonical_digest_hex,identityPoint+firstSignature.signature_hex.slice(64)),
    signatureBad('noncanonical-r',expectedPublic,firstSignature.canonical_digest_hex,'ee'+'ff'.repeat(30)+'7f'+firstSignature.signature_hex.slice(64)),
    signatureBad('unreduced-s',expectedPublic,firstSignature.canonical_digest_hex,firstSignature.signature_hex.slice(0,64)+'edd3f55c1a631258d69cf7a2def9de1400000000000000000000000000000010'),
    signatureBad('changed-digest',expectedPublic,alteredDigest.toString('hex'),firstSignature.signature_hex),
    signatureBad('changed-signature',expectedPublic,firstSignature.canonical_digest_hex,alteredSignature.toString('hex')),
    signatureBad('wrong-key-width',expectedPublic.slice(2),firstSignature.canonical_digest_hex,firstSignature.signature_hex),
    signatureBad('wrong-digest-width',expectedPublic,firstSignature.canonical_digest_hex.slice(2),firstSignature.signature_hex),
    signatureBad('wrong-signature-width',expectedPublic,firstSignature.canonical_digest_hex,firstSignature.signature_hex+'00'),
];
const json = JSON.stringify({profile:'NF-CANON-1',schema_version:1,method:'Independent explicit Node Buffer assembly; standard node:crypto SHA-256. No production codec imports.',
    positive,malformed,request_bindings,semantic_records,signatures,negative_signatures,required_semantics:requiredRecords,
    invalid_domain_inputs:[
        {name:'i64-underflow',field:'integers',value:'-9223372036854775809',error:'INTEGER_RANGE'},
        {name:'i64-overflow',field:'integers',value:'9223372036854775808',error:'INTEGER_RANGE'},
        {name:'u64-negative',field:'counters',value:'-1',error:'INTEGER_RANGE'},
        {name:'u64-overflow',field:'counters',value:'18446744073709551616',error:'INTEGER_RANGE'},
        {name:'unpaired-java-surrogate',field:'optional_text',optional_text_utf16_units:[55296],error:'INVALID_UNICODE_SCALAR'},
        {name:'opaque-id-wrong-width',field:'request_id',value:'11'.repeat(15),error:'ID_WIDTH'}
    ]},null,2)+'\n';
if(process.argv.includes('--check')) {
    if(!fs.existsSync(out)||fs.readFileSync(out,'utf8')!==json) {
        process.stderr.write('Golden fixture corpus does not match the independent generator.\n'); process.exitCode=1;
    } else process.stdout.write(`Fixture reproducibility passed: ${positive.length} positive, ${malformed.length} malformed, ${request_bindings.length} binding, ${requiredRecords.length} required-semantic cases.\n`);
} else {fs.writeFileSync(out,json,'utf8');process.stdout.write(`Wrote ${out}\n`);}
