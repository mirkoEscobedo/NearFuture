// Canonical-text data generator. Source and permission notice retained in this directory.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../..');
const source = fs.readFileSync(path.join(__dirname, 'DerivedAge-13.0.0.txt'));
const expected = 'e779a443d3aa2a3166a15becaa2b737c922480e32c0453d5956093633555078f';
if (crypto.createHash('sha256').update(source).digest('hex') !== expected) throw new Error('Unicode 13 source identity changed');
const admitted = new Uint8Array(0x110000);
for (const line of source.toString('utf8').split('\n')) {
  const match = /^([A-F0-9]+)(?:\.\.([A-F0-9]+))?\s*;\s*([0-9.]+)/.exec(line);
  if (!match || Number(match[3]) > 13) continue;
  const start = parseInt(match[1], 16), end = parseInt(match[2] || match[1], 16);
  for (let cp = start; cp <= end; cp++) {
    if (!(cp >= 0xd800 && cp <= 0xdfff) && !(cp >= 0xfdd0 && cp <= 0xfdef) && (cp & 0xffff) < 0xfffe) admitted[cp] = 1;
  }
}
const ranges = [];
for (let cp = 0; cp < admitted.length; cp++) {
  if (!admitted[cp]) continue;
  const start = cp;
  while (cp + 1 < admitted.length && admitted[cp + 1]) cp++;
  ranges.push([start, cp]);
}
const grouped = (values, width) => Array.from({length: Math.ceil(values.length / width)}, (_, i) => values.slice(i*width, (i+1)*width).join(', ')).join(',\n');
const rust = `// Generated from Unicode 13 DerivedAge; see protocol/text/Unicode-LICENSE.txt.\n// Source SHA-256 ${expected}; regenerate: node protocol/text/generate-ranges.cjs\npub const RANGES: &[(u32, u32)] = &[\n${grouped(ranges.map(([a,b]) => `    (0x${a.toString(16)}, 0x${b.toString(16)})`), 1)},\n];\n`;
const java = `// Generated from Unicode 13 DerivedAge; see protocol/text/Unicode-LICENSE.txt.\n// Source SHA-256 ${expected}; regenerate: node protocol/text/generate-ranges.cjs\npackage nf.contract;\n\npublic final class Unicode13 {\n    private Unicode13() { }\n    private static final int[] RANGES = {\n${grouped(ranges.flat().map(n => `0x${n.toString(16)}`), 8)}\n    };\n    public static boolean contains(int scalar) {\n        int low = 0, high = RANGES.length / 2;\n        while (low < high) {\n            int middle = (low + high) >>> 1;\n            if (scalar < RANGES[middle * 2]) high = middle;\n            else if (scalar > RANGES[middle * 2 + 1]) low = middle + 1;\n            else return true;\n        }\n        return false;\n    }\n}\n`;
for (const [relative, content] of [['crates/nf-contract/src/canonical/text_ranges.rs', rust], ['java/contract/src/main/java/nf/contract/Unicode13.java', java]]) {
  const target = path.join(root, relative);
  if (process.argv.includes('--check')) {
    if (fs.readFileSync(target, 'utf8').replace(/\r\n/g, '\n') !== content) throw new Error(`Stale generated Unicode table: ${relative}`);
  } else fs.writeFileSync(target, content);
}
console.log(`Unicode 13 scalar ranges: ${ranges.length}`);
