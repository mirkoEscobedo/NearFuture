// @ts-check
import { lstat, mkdir, opendir, realpath } from 'node:fs/promises';
import { randomBytes } from 'node:crypto';
import { join, resolve } from 'node:path';
import { readStable, writeNew, identity, sha256 } from './bounded-runtime/io.mjs';

let next = 0;
/** Finite caller-owned evidence. Nothing deletes on success, failure or disposal. */
export class RuntimeCase {
  /** @param {import('./bounded-runtime/clock.mjs').CaseClock} clock
   * @param {string} root */
  constructor(clock, root) {
    this.clock = clock;
    this.root = root;
    this.entries = new Set(['']);
    this.directories = new Set(['']);
    this.bytes = 0;
  }
  /** @param {import('./bounded-runtime/clock.mjs').CaseClock} clock */
  static async create(clock) {
    clock.check();
    const supplied = process.env.NF_RUNTIME_OBSERVER_EVIDENCE_ROOT;
    if (!supplied) throw new Error('MISSING_CALLER_RUNTIME_EVIDENCE');
    const parent = resolve(supplied);
    const metadata = await clock.guard(() => lstat(parent, { bigint: true }));
    if (!metadata.isDirectory() || metadata.isSymbolicLink())
      throw new Error('UNSAFE_CALLER_PARENT');
    if (await clock.guard(() => realpath(parent)) !== parent)
      throw new Error('UNSAFE_CALLER_PARENT');
    const caller = await readStable(clock, join(parent, 'caller-receipt.json'));
    if (!caller.bytes.equals(Buffer.from(
      '{"version":1,"kind":"owned-public-runtime-observer-fixtures","secretFiles":false,"children":false,"caseSeconds":20,"caseBytes":8388608,"caseEntries":64}\n')))
      throw new Error('INVALID_CALLER_RUNTIME_RECEIPT');
    const nonce = await clock.guard(() => randomBytes(16).toString('hex'));
    const root = join(parent, 'runtime-' + process.pid + '-' + next++ + '-' + nonce);
    await clock.guard(() => mkdir(root));
    return new RuntimeCase(clock, root);
  }
  /** @param {string} relative */
  path(relative) {
    if (!relative || relative.includes('\\') || relative.includes(':')
        || relative.split('/').some(x => !x || x === '.' || x === '..'))
      throw new Error('UNSAFE_CASE_LEAF');
    return join(this.root, ...relative.split('/'));
  }
  /** @param {string} relative */
  accountEntry(relative) {
    if (this.entries.has(relative) || this.entries.size >= 64)
      throw new Error('CASE_ENTRY_LIMIT_OR_OCCUPIED');
    this.entries.add(relative);
  }
  /** @param {string} relative @param {boolean} [cleanup] */
  async parents(relative, cleanup = false) {
    let directory = '';
    for (const part of relative.split('/').slice(0, -1)) {
      directory += (directory ? '/' : '') + part;
      if (this.directories.has(directory)) continue;
      this.accountEntry(directory);
      await this.clock.guard(() => mkdir(this.path(directory)), cleanup);
      this.directories.add(directory);
    }
  }
  /** @param {string} relative @param {Buffer} bytes @param {boolean} [cleanup] */
  async put(relative, bytes, cleanup = false) {
    this.clock.check(cleanup);
    this.path(relative);
    if (bytes.length > 1_048_576 || this.bytes + bytes.length > 8_388_608)
      throw new Error('CASE_BYTE_LIMIT');
    await this.parents(relative, cleanup);
    this.accountEntry(relative);
    this.bytes += bytes.length;
    await writeNew(this.clock, this.path(relative), bytes, cleanup);
  }
  /** @param {string} relative @param {unknown} value @param {boolean} [cleanup] */
  async record(relative, value, cleanup = false) {
    this.clock.check(cleanup);
    await this.put(relative, Buffer.from(JSON.stringify(value, null, 2) + '\n'), cleanup);
  }
  /** Actual bounded census of this unique synthetic namespace only.
   * @param {string} [relative] @param {boolean} [cleanup] */
  async snapshot(relative = '', cleanup = false) {
    /** @type {Record<string, {kind: string, identity: ReturnType<typeof identity>, bytes?: Buffer}>} */
    const result = {};
    let entries = 0;
    let bytes = 0;
    /** @param {string} item @param {number} depth @returns {Promise<void>} */
    const visit = async (item, depth) => {
      if (++entries > 64 || depth > 16) throw new Error('CASE_TREE_LIMIT');
      const path = item ? this.path(item) : this.root;
      const metadata = await this.clock.guard(
        () => lstat(path, { bigint: true }), cleanup);
      if (metadata.isSymbolicLink()) throw new Error('CASE_TREE_LINK');
      if (metadata.isFile()) {
        const file = await readStable(this.clock, path, cleanup, 8_388_608 - bytes);
        bytes += file.bytes.length;
        if (bytes > 8_388_608) throw new Error('CASE_TREE_BYTE_LIMIT');
        result[item] = { kind: 'file', identity: file.identity, bytes: file.bytes };
        return;
      }
      if (!metadata.isDirectory()) throw new Error('CASE_TREE_TYPE');
      result[item] = { kind: 'directory', identity: identity(metadata) };
      this.clock.check(cleanup);
      const directory = await opendir(path, { bufferSize: 1, recursive: false });
      try {
        this.clock.check(cleanup);
        for (;;) {
          const entry = await this.clock.guard(() => directory.read(), cleanup);
          if (!entry) break;
          if (entry.name.includes('/') || entry.name.includes('\\'))
            throw new Error('CASE_TREE_LEAF');
          await visit((item ? item + '/' : '') + entry.name, depth + 1);
        }
      } finally {
        await this.clock.close(() => directory.close());
      }
    };
    await visit(relative, 0);
    return result;
  }
  /** @param {Awaited<ReturnType<RuntimeCase['snapshot']>>} snapshot */
  summary(snapshot) {
    return Object.fromEntries(Object.entries(snapshot).sort(([a], [b]) =>
      a < b ? -1 : a > b ? 1 : 0).map(([path, value]) => [path, {
        kind: value.kind, identity: value.identity,
        bytes: value.bytes?.length ?? null,
        sha256: value.bytes ? sha256(value.bytes) : null,
      }]));
  }
}
