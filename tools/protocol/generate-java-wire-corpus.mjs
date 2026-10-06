// @ts-check
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { dirname } from 'node:path';
const root = new URL('../../', import.meta.url);
const corpus = JSON.parse(await readFile(new URL('protocol/vectors/wire-v1.json', root), 'utf8'));
/** @param {string} value */
const quote = value => JSON.stringify(value);
/** @param {any[]} cases @param {string} key */
const fixtures = (cases, key) => cases.map(f => `new Fixture(${quote(f.name)}, ${quote(f[key])}, ${quote(f.error ?? '')})`).join(',\n');
const source = `package nf.wire;
import java.util.List;
// Independent checked-in raw bytes converted to fixture DATA, no generated encoder calls.
final class WireCorpus {
    private WireCorpus() { }
    record Fixture(String name, String hex, String failure) { }
    static final List<Fixture> POSITIVE = List.of(${fixtures(corpus.positive, 'wire_hex')});
    static final List<Fixture> MALFORMED = List.of(${fixtures(corpus.malformed, 'wire_hex')});
    static final List<Fixture> FRAME_MALFORMED = List.of(${fixtures(corpus.frame_malformed, 'frame_hex')});
}
`;
const path = fileURLToPath(new URL('java/wire/build/generated/sources/corpus/nf/wire/WireCorpus.java', root));
await mkdir(dirname(path), { recursive: true });
await writeFile(path, source);
console.log(`Converted ${corpus.positive.length} positive/${corpus.malformed.length} malformed wire cases`);
