// @ts-check
/** Private public-byte catalog and pure declaration admission. No observation factory. */
const entries = [
  {
    "ordinal": 0,
    "moduleOrdinal": 0,
    "relative": "../../package.json",
    "role": "manifest",
    "classification": "verified_public_manifest",
    "bytes": 2870,
    "sha256": "3372b25671e1e9c068777435daf941fa03912ae9c398f906de4142049d311adc",
    "body": "{\n  \"name\": \"near-future\",\n  \"version\": \"0.1.0\",\n  \"private\": true,\n  \"description\": \"Local development tooling for the NearFuture Rust project.\",\n  \"scripts\": {\n    \"start\": \"cargo run --locked --offline\",\n    \"test:rust\": \"cargo test --workspace --locked --offline\",\n    \"test:tools\": \"node --test tests/architecture/*.test.mjs tests/build/*.test.mjs tests/runtime-manifest/*.test.mjs tests/performance/*.test.mjs tools/miniature-store-vectors/profile.test.cjs tools/receipt-vectors/*.test.cjs tools/notification-vectors/*.test.cjs tests/reference/sync/*.test.cjs tests/delivery/*.test.mjs tools/starsector-proxy/frame*.test.mjs tools/nex-first-proposal-repro.test.cjs\",\n    \"test\": \"npm run test:rust && npm run test:tools\",\n    \"lint:rust\": \"cargo fmt --all -- --check && cargo clippy --workspace --all-targets --locked --offline -- -D warnings\",\n    \"check:types\": \"tsc -p tsconfig.json\",\n    \"check:boundaries\": \"node tools/architecture/check.mjs\",\n    \"check:java\": \"node tools/build/java.mjs synthetic\",\n    \"check:game\": \"node tools/build/java.mjs game\",\n    \"lint\": \"npm run lint:rust && npm run check:types && npm run check:boundaries\",\n    \"check\": \"npm run lint && npm run check:schemas && npm test && npm run check:java\",\n    \"workspace-template\": \"workspace-template\",\n    \"skills\": \"workspace-template skills list\",\n    \"check:schemas\": \"node protocol/vectors/generate-fixtures.cjs --check && node protocol/vectors/generate-wire-fixtures.cjs --check && node protocol/text/generate-ranges.cjs --check && node protocol/vectors/generate-local-auth-fixtures.cjs --check && node crates/nf-nex-shadow/fixtures/generate-identity.cjs --check && node crates/nf-nex-shadow/fixtures/generate-world.cjs --check && node crates/nf-nex-shadow/fixtures/generate-action-identity.cjs --check && node crates/nf-world/fixtures/generate.cjs --check && node crates/nf-transport/tools/generate-peer-vectors.cjs --check && node crates/nf-kernel/tests/fixtures/miniature/generate.cjs --check && node crates/nf-kernel/tools/generate-miniature-source.cjs --check && node tools/miniature-store-vectors/generate.cjs --check && node crates/nf-world-driver/tools/generate-action-vectors.cjs --check && node tools/receipt-vectors/generate.cjs --check && node tools/notification-vectors/generate.cjs --check && node tools/reference/sync/generate.cjs --check\",\n    \"check:performance\": \"node --test tests/performance/*.test.mjs\",\n    \"check:profile\": \"node --test tests/delivery/*.test.mjs && cargo test -p near-future --test node_cli --locked --offline\"\n  },\n  \"devDependencies\": {\n    \"@types/node\": \"file:vendor/types-node-24.13.2.tgz\",\n    \"typescript\": \"file:vendor/typescript-5.9.3.tgz\",\n    \"undici-types\": \"file:vendor/undici-types-7.18.2.tgz\"\n  },\n  \"optionalDependencies\": {\n    \"workspace-template-win32-x64\": \"file:vendor/workspace-template-win32-x64-0.9.0-alpha.0.tgz\"\n  }\n}\n"
  },
  {
    "ordinal": 1,
    "moduleOrdinal": 0,
    "relative": "./metadata.mjs",
    "role": "module",
    "classification": "verified_public_code",
    "bytes": 2309,
    "sha256": "578cf38b6356116cdd2497fe7f29f98b09ac5284a2e5221df8e4b6b8c70719d2",
    "body": "// @ts-check\n/** Parse observed game JSON extensions without evaluating source code.\n * @param {string} text @param {string} label */\nexport function parseGameJson(text, label) {\n  try {\n    let clean = '';\n    for (let i = 0; i < text.length; i++) {\n      const c = text[i];\n      if (c === '#' || (c === '/' && text[i + 1] === '/')) {\n        while (i < text.length && text[i] !== '\\n') i++;\n        clean += '\\n';\n      } else if (c === '\"' || c === \"'\") {\n        const quote = c;\n        let raw = '', escaped = false, closed = false;\n        for (i++; i < text.length; i++) {\n          const next = text[i];\n          if (!escaped && next === quote) { closed = true; break; }\n          raw += next;\n          if (escaped) escaped = false;\n          else if (next === '\\\\') escaped = true;\n        }\n        if (!closed) throw new Error('unclosed string');\n        const value = quote === '\"' ? JSON.parse('\"' + raw + '\"') :\n          JSON.parse('\"' + raw.replace(/\\\\'/g, \"'\").replace(/\"/g, '\\\\\"') + '\"');\n        clean += JSON.stringify(value);\n      } else clean += c;\n    }\n    let result = '', quoted = false, escaped = false;\n    for (let i = 0; i < clean.length; i++) {\n      const c = clean[i];\n      if (!quoted && c === ',' && /^[\\s]*[}\\]]/.test(clean.slice(i + 1))) continue;\n      result += c;\n      if (escaped) escaped = false;\n      else if (quoted && c === '\\\\') escaped = true;\n      else if (c === '\"') quoted = !quoted;\n    }\n    return JSON.parse(result);\n  } catch { throw new Error(`INVALID_JSON: ${label}; repair the metadata/config JSON and retry.`); }\n}\n\n/** Keep numeric/runtime JVM flags; omit arbitrary properties, paths, agents and arguments.\n * @param {string} text */\nexport function safeJvmFlags(text) {\n  return (text.match(/\"[^\"]*\"|\\S+/g) ?? []).filter(token =>\n    /^-X(?:mx|ms|ss)\\d+[kKmMgG]$/.test(token) ||\n    /^-XX:[+-][A-Za-z0-9]+$/.test(token) ||\n    /^-XX:[A-Za-z0-9]+=-?[0-9]+(?:\\.[0-9]+)?[kKmMgG]?$/.test(token) ||\n    /^-XX:ShenandoahGCMode=(?:satb|iu|generational)$/.test(token) ||\n    /^-XX:ShenandoahGCHeuristics=(?:adaptive|static|compact|aggressive)$/.test(token) ||\n    /^--(?:enable-preview|add-opens=[A-Za-z0-9./]+=[A-Za-z0-9,-]+|add-exports=[A-Za-z0-9./]+=[A-Za-z0-9,-]+)$/.test(token) ||\n    /^-Xlog:async$/.test(token) || token === '-noverify');\n}\n"
  },
  {
    "ordinal": 2,
    "moduleOrdinal": 0,
    "relative": "../../config/runtime-support.json",
    "role": "manifest",
    "classification": "verified_public_manifest",
    "bytes": 1615,
    "sha256": "5a98cff8375686598578c4edc76e9412203b9fc27f04c6e37a0a697635333acf",
    "body": "{\n  \"schemaVersion\": 1,\n  \"unknownConfigurationMode\": \"legacy-read-only\",\n  \"certifications\": [],\n  \"statusDefinitions\": {\n    \"supported\": \"Exact runtime/content hashes passed required game integration and gate evidence under independent review.\",\n    \"experimentally-supported\": \"A named experimental capability passed on exact hashes; broader capabilities remain unavailable.\",\n    \"unknown\": \"Inventoried or unrecognized configuration without game integration evidence; no promoted authority.\"\n  },\n  \"observedTarget\": {\n    \"manifest\": \"docs/runtime/observed-installation.json\",\n    \"gameBuild\": \"0.98a-RC8\",\n    \"jvmVendor\": \"Azul Systems, Inc.\",\n    \"jvmVersion\": \"17.0.10\",\n    \"nexerelinVersion\": \"0.12.2c\",\n    \"consoleVersion\": \"4.0.9\",\n    \"status\": \"unknown\",\n    \"evidence\": [\"deterministic-installation-capture\", \"public-api-signature-inspection\"],\n    \"missingEvidence\": [\"live-launch-flags\", \"campaign-hooks\", \"save-durability\", \"console-listener-delivery\", \"Nex-single-writer-substitution\", \"G1\", \"G2\", \"G3\", \"G4\", \"G5\"]\n  },\n  \"referenceSources\": {\n    \"nexerelin\": {\n      \"repository\": \"https://github.com/Histidine91/Nexerelin\",\n      \"tag\": \"v0.12.2c\",\n      \"commit\": \"a669f4d0740e95a4acbb6b894dde09ade67aa754\",\n      \"scope\": \"reference inspection only; binary-to-source equivalence unverified\"\n    },\n    \"consoleCommands\": {\n      \"repository\": \"https://github.com/LazyWizard/console-commands\",\n      \"commit\": \"c5718df92c64d29c6a5fe7f4165f52127470d343\",\n      \"scope\": \"registration contract reference; installed 4.0.9 signature inspected, source-to-binary match unverified\"\n    }\n  }\n}\n"
  }
];
export const registeredFiles = Object.freeze(entries.map(entry => Object.freeze(entry)));

/** @typedef {{kind:string,value:number|boolean|string}} DeclaredFlag */
/** @typedef {{schema:string,os:string,arch:string,runtime_family:string,runtime_version:string|null,jvm_flags:ReadonlyArray<Readonly<DeclaredFlag>>}} Declaration */
const keys = ['schema', 'os', 'arch', 'runtime_family', 'runtime_version', 'jvm_flags'];
const kinds = ['heap_initial_bytes', 'heap_max_bytes', 'stack_bytes', 'use_g1_gc',
  'use_shenandoah_gc', 'max_gc_pause_millis', 'shenandoah_mode',
  'shenandoah_heuristics', 'enable_preview', 'noverify'];
/** @param {unknown} value @param {readonly string[]} expected */
function exactKeys(value, expected) {
  return value !== null && typeof value === 'object' && !Array.isArray(value)
    && Object.getPrototypeOf(value) === Object.prototype
    && JSON.stringify(Object.keys(value)) === JSON.stringify(expected);
}
/** @param {unknown} value @param {number} low @param {number} high */
function integer(value, low, high) {
  return typeof value === 'number' && Number.isSafeInteger(value)
    && value >= low && value <= high;
}
/** @param {number} index @param {unknown} value */
function flagValue(index, value) {
  if (index <= 2) return integer(value, 1, 1099511627776);
  if ([3, 4, 8, 9].includes(index)) return typeof value === 'boolean';
  if (index === 5) return integer(value, 0, 60000);
  if (index === 6) return typeof value === 'string'
    && ['satb', 'iu', 'generational'].includes(value);
  return index === 7 && typeof value === 'string'
    && ['adaptive', 'static', 'compact', 'aggressive'].includes(value);
}
/** A primitive bounded string only; a result remains an unverified declaration.
 * @param {string|null} text @returns {Readonly<Declaration>|null|undefined} */
export function parseDeclaration(text) {
  if (text === null) return null;
  if (typeof text !== 'string' || text.length > 8192 || /[^\x20-\x7e]/.test(text))
    return undefined;
  /** @type {any} */
  let value;
  try { value = JSON.parse(text); } catch { return undefined; }
  if (!exactKeys(value, keys) || value.schema !== 'NF-UNVERIFIED-RUNTIME-INPUT-1'
    || !['linux', 'windows', 'macos', 'unknown'].includes(value.os)
    || !['x86_64', 'aarch64', 'unknown'].includes(value.arch)
    || !['jvm', 'rust_native', 'unknown'].includes(value.runtime_family)
    || !(value.runtime_version === null || (typeof value.runtime_version === 'string'
      && value.runtime_version.length <= 64
      && /^[0-9]{1,4}(?:\.[0-9]{1,4}){0,3}(?:\+[0-9]{1,8})?$/.test(value.runtime_version)))
    || !Array.isArray(value.jvm_flags) || value.jvm_flags.length > 32
    || (value.runtime_family !== 'jvm' && value.jvm_flags.length !== 0)) return undefined;
  /** @type {Readonly<DeclaredFlag>[]} */
  const flags = [];
  let previous = -1;
  for (const flag of value.jvm_flags) {
    if (!exactKeys(flag, ['kind', 'value']) || typeof flag.kind !== 'string') return undefined;
    const index = kinds.indexOf(flag.kind);
    if (index <= previous || !flagValue(index, flag.value)) return undefined;
    previous = index;
    flags.push(Object.freeze({ kind: flag.kind, value: flag.value }));
  }
  const admitted = Object.freeze({ schema: value.schema, os: value.os, arch: value.arch,
    runtime_family: value.runtime_family, runtime_version: value.runtime_version,
    jvm_flags: Object.freeze(flags) });
  return JSON.stringify(admitted) === text ? admitted : undefined;
}
