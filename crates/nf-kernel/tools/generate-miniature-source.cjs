'use strict';
// Owned-source identity only. No compiled-binary, library-runtime, or game equivalence claim.
const fs = require('node:fs');
const path = require('node:path');
const crypto = require('node:crypto');
const root = path.resolve(__dirname, '../../..');
const pin = 'crates/nf-kernel/src/miniature/generated_pin.rs';
const trees = ['crates/nf-kernel/src/miniature', 'crates/nf-world/src'];
const hash = data => crypto.createHash('sha256').update(data).digest();
function files(relative) {
  return fs.readdirSync(path.join(root, relative), {withFileTypes:true}).flatMap(entry => {
    const item = relative + '/' + entry.name;
    return entry.isDirectory() ? files(item) : item.endsWith('.rs') && item !== pin ? [item] : [];
  });
}
const inventory = trees.flatMap(files).sort().map(relative => {
  const source = fs.readFileSync(path.join(root, relative), 'utf8').replace(/\r\n/g, '\n');
  if (source.includes('\r')) throw new Error('bare CR: ' + relative);
  const bytes = Buffer.from(source, 'utf8');
  return {path:relative, bytes:bytes.length, sha256:hash(bytes).toString('hex')};
});
const parts = [Buffer.from('NF-MINIATURE-SOURCE-1\0')];
for (const entry of inventory) {
  const name = Buffer.from(entry.path, 'utf8');
  const width = Buffer.alloc(4); width.writeUInt32LE(name.length);
  const length = Buffer.alloc(8); length.writeBigUInt64LE(BigInt(entry.bytes));
  parts.push(width, name, length, Buffer.from(entry.sha256, 'hex'));
}
const digest = hash(Buffer.concat(parts));
const manifest = JSON.stringify({profile:'NF-MINIATURE-SOURCE-1', normalization:'UTF-8, CRLF to LF; no other rewriting', excluded:pin, files:inventory, bundle_sha256:digest.toString('hex')}, null, 2) + '\n';
const literal = '// Generated fixed literal only; excluded from its own source bundle.\npub const SOURCE_HASH: [u8; 32] =\n    *b"' + [...digest].map(n=>'\\x'+n.toString(16).padStart(2,'0')).join('') + '";\n';
for (const [relative, content] of [[pin,literal], ['crates/nf-kernel/tests/fixtures/miniature/source-bundle.json',manifest]]) {
  const target = path.join(root, relative);
  if (process.argv.includes('--check')) {
    if (fs.readFileSync(target,'utf8').replace(/\r\n/g,'\n') !== content) throw new Error('source identity differs: '+relative);
  } else fs.writeFileSync(target,content);
}
console.log('NF-MINIATURE-SOURCE-1 '+inventory.length+' files SHA256 '+digest.toString('hex'));
