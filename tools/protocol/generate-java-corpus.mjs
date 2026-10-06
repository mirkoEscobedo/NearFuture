// @ts-check
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';

const root = new URL('../../', import.meta.url);
const corpus = JSON.parse(await readFile(new URL('protocol/vectors/nf-canon-1.json', root), 'utf8'));
/** @param {string} value */
const quote = value => JSON.stringify(value).replace(/[\u0080-\uffff]/g, char => `\\u${char.charCodeAt(0).toString(16).padStart(4, '0')}`);
/** @param {string} value */
const bytes = value => `hex(${quote(value)})`;
/** @param {string} value */
const id = value => `new Records.Id(${bytes(value)})`;
/** @param {string} value */
const digest = value => `new Records.Digest(${bytes(value)})`;
/** @param {string | number} value */
const u64 = value => `new Records.U64(Long.parseUnsignedLong(${quote(String(value))}))`;
/** @param {any[]} values */
const list = values => `List.of(${values.join(', ')})`;
/** @param {any} record */
function binding(record) {
    return `new Records.Document(1, 1, ${list([id(record.request_id), id(record.account_id), id(record.device_id), id(record.universe_id), id(record.history_id), `new Records.U32(${record.operation_kind}L)`, digest(record.payload_digest)])})`;
}
/** @param {number} schema @param {string} body */
const payload = (schema, body) => `new Records.Document(6, 1, List.of(new Records.U32(${schema}L), new Records.Bytes(${bytes(body)})))`;
/** @param {number[]} caps @param {number[]} schemas */
const required = (caps, schemas) => `new Records.Document(6, 2, List.of(new Records.SetValues(${list(caps.map(value => `${value}L`))}), new Records.SetValues(${list(schemas.map(value => `${value}L`))})))`;
/** @param {string} document */
const nested = document => `new Records.Nested(${document})`;
const revisions = 'new Records.Revisions(List.of())';
const documents = 'new Records.Documents(List.of())';
/** @param {any} fixture */
function semantic(fixture) {
    const f = fixture.fields;
    const req = nested(required(f.required_capabilities ?? [], f.required_schemas ?? []));
    let values;
    if (fixture.domain === 6 && fixture.type === 1) return payload(f.schema_id, f.canonical_body_hex);
    if (fixture.domain === 1) values = [nested(binding(f.binding)), revisions, nested(payload(f.payload_schema_id, f.payload_body_hex)), `new Records.OptionalU64(${f.expiry_tick === null ? 'null' : `Long.parseUnsignedLong(${quote(f.expiry_tick)})`})`, req];
    else if (fixture.domain === 2) values = [id(f.universe_id), id(f.history_id), u64(f.event_seq), u64(f.world_tick), digest(f.ruleset_hash), revisions, documents, documents, documents, documents, documents, documents, req];
    else if (fixture.domain === 3) values = [id(f.job_id), id(f.provider_id), `new Records.U32(${f.provider_version}L)`, digest(f.ruleset_hash), digest(f.input_hash), u64(f.runtime_session), revisions, revisions, documents, documents, req];
    else if (fixture.domain === 4) values = [id(f.history_id), u64(f.event_seq), digest(f.previous_hash), u64(f.authority_term), u64(f.world_tick), digest(f.ruleset_hash), documents, documents, documents, digest(f.state_hash), req];
    else throw new Error('Unsupported fixture record');
    return `new Records.Document(${fixture.domain}, ${fixture.type}, ${list(values)})`;
}
/** @param {any} fixture */
function profile(fixture) {
    const f = fixture.record;
    return `new Canonical.Profile(${f.optional_bytes_hex === null ? 'null' : bytes(f.optional_bytes_hex)}, ${f.optional_text === null ? 'null' : quote(f.optional_text)}, ${list(f.integers.map(/** @param {string} value */ value => `Long.parseLong(${quote(value)})`))}, ${list(f.counters.map(/** @param {{key: string, value: string}} value */ value => `new Canonical.Counter(${quote(value.key)}, Long.parseUnsignedLong(${quote(value.value)}))`))})`;
}
const positive = corpus.positive.map(/** @param {any} f */ f => `new ProfileFixture(${quote(f.name)}, ${profile(f)}, ${quote(f.canonical_hex)}, ${quote(f.sha256)})`);
const bad = corpus.malformed.map(/** @param {any} f */ f => `new BadFixture(${quote(f.name)}, ${quote(f.canonical_hex)})`);
const bindings = corpus.request_bindings.map(/** @param {any} f */ f => `new RecordFixture(${quote(f.name)}, ${binding(f.record)}, ${quote(f.canonical_hex)}, ${quote(f.sha256)})`);
const semantics = corpus.semantic_records.map(/** @param {any} f */ f => `new RecordFixture(${quote(f.name)}, ${semantic(f)}, ${quote(f.canonical_hex)}, ${quote(f.sha256)})`);
const requirements = corpus.required_semantics.map(/** @param {any} f */ f => `new RequiredFixture(${quote(f.name)}, ${required(f.capability_ids, f.schema_ids)}, ${quote(f.canonical_hex)}, ${f.expected === 'ACCEPT'})`);
const signatures = corpus.signatures.map(/** @param {any} f */ f => `new SignatureFixture(${quote(f.name)}, ${quote(f.test_seed_hex)}, ${quote(f.public_key_hex)}, ${quote(f.canonical_digest_hex)}, ${quote(f.signature_hex)})`);
const negativeSignatures = corpus.negative_signatures.map(/** @param {any} f */ f => `new BadSignatureFixture(${quote(f.name)}, ${quote(f.public_key_hex)}, ${quote(f.canonical_digest_hex)}, ${quote(f.signature_hex)})`);
const source = `package nf.contract.canonical;
import java.util.HexFormat;
import java.util.List;
// Generated fixture DATA from independent checked-in JSON. No production codec calls.
final class GoldenCorpus {
    private GoldenCorpus() { }
    static byte[] hex(String value) { return HexFormat.of().parseHex(value); }
    record ProfileFixture(String name, Canonical.Profile record, String hex, String hash) { }
    record RecordFixture(String name, Records.Document record, String hex, String hash) { }
    record BadFixture(String name, String hex) { }
    record SignatureFixture(String name, String seed, String publicKey, String digest, String signature) { }
    record BadSignatureFixture(String name, String publicKey, String digest, String signature) { }
    record RequiredFixture(String name, Records.Document record, String hex, boolean accepted) { }
    static final List<ProfileFixture> POSITIVE = ${list(positive)};
    static final List<BadFixture> MALFORMED = ${list(bad)};
    static final List<RecordFixture> BINDINGS = ${list(bindings)};
    static final List<RecordFixture> SEMANTIC = ${list(semantics)};
    static final List<SignatureFixture> SIGNATURES = ${list(signatures)};
    static final List<BadSignatureFixture> NEGATIVE_SIGNATURES = ${list(negativeSignatures)};
    static final List<RequiredFixture> REQUIRED = ${list(requirements)};
}
`;
const path = fileURLToPath(new URL('java/contract/build/generated/sources/corpus/nf/contract/canonical/GoldenCorpus.java', root));
await mkdir(dirname(path), { recursive: true });
await writeFile(path, source);
console.log(`Converted ${positive.length} positive/${bad.length} malformed/${bindings.length} binding/${semantics.length} semantic/${requirements.length} required fixtures to Java test data`);
