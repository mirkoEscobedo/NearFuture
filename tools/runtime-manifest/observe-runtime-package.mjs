// @ts-check
import { lstat, open, realpath } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { performance } from 'node:perf_hooks';
import { inspect } from 'node:util';
import { dirname, isAbsolute, normalize, parse, resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { registeredFiles, parseDeclaration } from './runtime-package-registry.mjs';

const codes = Object.freeze(['InvalidInput', 'Bounds', 'Expired', 'SourceChanged',
  'UnsafeSource', 'MissingPublicArtifact', 'Io', 'InvalidObservation']);
/** A fixed reason, never an observation or authority witness. */
export class RuntimeObservationError extends Error {
  /** @param {string} code */
  constructor(code) {
    const reason = codes.includes(code) ? code : 'InvalidInput';
    super(reason);
    this.name = 'RuntimeObservationError';
    this.code = reason;
    this.stack = 'RuntimeObservationError: ' + reason;
    Object.freeze(this);
  }
}
/** @param {string} code @returns {never} */
function fail(code) { throw new RuntimeObservationError(code); }
/** @param {unknown} error */
function closed(error) {
  if (error instanceof RuntimeObservationError) return error;
  const missing = error !== null && typeof error === 'object'
    && 'code' in error && error.code === 'ENOENT';
  return new RuntimeObservationError(missing ? 'MissingPublicArtifact' : 'Io');
}
/** @typedef {import('node:fs').BigIntStats} Stat */
/** @typedef {ReturnType<typeof fileIdentity>} FileIdentity */
/** @typedef {{path:string,identity:string}} Ancestor */
/** @typedef {{path:string,canonical:string,identity:FileIdentity,ancestors:ReadonlyArray<Ancestor>}} SourceGuard */
/** @typedef {{ordinal:number,moduleOrdinal:number,role:string,classification:string,bytes:number,sha256:string}} Artifact */
/** @typedef {{remaining:number,expiry:number,observations:number,consumed:boolean,guards:ReadonlyArray<SourceGuard>}} Capture */
/** @type {WeakMap<object,Capture>} */
const captures = new WeakMap();
const limitations = Object.freeze(['subset_not_complete_runtime', 'declarations_not_observation',
  'live_flags_unverified', 'build_equivalence_unverified', 'full_acl_unverified',
  'local_volume_unverified', 'hostile_races_unqualified', 'blocking_io_not_preempted',
  'separate_captures_not_atomic', 'not_current_authority']);

class Budget {
  /** @param {unknown} deadline @param {number} start @param {number} remaining
   * @param {number} expiry @param {number} observations */
  constructor(deadline, start, remaining, expiry, observations) {
    if (typeof deadline !== 'number' || !Number.isFinite(deadline)) fail('InvalidInput');
    if (deadline > start + 20000) fail('Bounds');
    this.start = start;
    this.deadline = Math.min(deadline, start + remaining, expiry);
    this.observations = observations;
    this.check();
  }
  /** @param {boolean} [cleanup] */
  check(cleanup = false) {
    if (performance.now() >= this.deadline - (cleanup ? 0 : 2000)) fail('Expired');
  }
  pathCheck() {
    this.check();
    if (++this.observations > 384) fail('Bounds');
  }
  /** @template T @param {()=>Promise<T>} operation @param {boolean} [path] */
  async effect(operation, path = false) {
    if (path) this.pathCheck(); else this.check();
    const value = await operation();
    this.check();
    return value;
  }
}
/** @param {Stat} value */
function fileIdentity(value) {
  return [value.dev, value.ino, value.size, value.mode, value.nlink,
    value.mtimeNs, value.ctimeNs].join(':');
}
/** Directory siblings are outside selection; their timestamps are not a source guard.
 * @param {Stat} value */
function ancestorIdentity(value) { return [value.dev, value.ino, value.mode].join(':'); }
/** @param {Stat} value @param {number} size */
function safeFile(value, size) {
  if (value.isSymbolicLink() || !value.isFile() || value.nlink !== 1n) fail('UnsafeSource');
  if (value.size !== BigInt(size)) fail('SourceChanged');
}
/** @param {string} relative @param {Budget} budget */
function selectedPath(relative, budget) {
  budget.check();
  const url = new URL(relative, import.meta.url);
  if (url.protocol !== 'file:' || url.hostname !== '') fail('UnsafeSource');
  const path = fileURLToPath(url);
  const root = parse(path).root;
  if (!isAbsolute(path) || path !== normalize(path) || path !== resolve(path)
    || path.startsWith('\\\\') || path.slice(root.length).includes(':')) fail('UnsafeSource');
  const components = path.slice(root.length).split(sep).filter(Boolean);
  if (components.length > 32) fail('Bounds');
  budget.check();
  return path;
}
/** @param {string} path @param {Budget} budget */
async function ancestors(path, budget) {
  /** @type {string[]} */
  const paths = [];
  let current = dirname(path);
  for (;;) {
    if (paths.length >= 32) fail('Bounds');
    paths.push(current);
    const parent = dirname(current);
    if (parent === current) break;
    current = parent;
  }
  /** @type {Ancestor[]} */
  const result = [];
  for (const item of paths.reverse()) {
    const stat = await budget.effect(() => lstat(item, { bigint: true }), true);
    if (stat.isSymbolicLink() || !stat.isDirectory()) fail('UnsafeSource');
    result.push(Object.freeze({ path: item, identity: ancestorIdentity(stat) }));
  }
  return Object.freeze(result);
}
/** @param {ReadonlyArray<Ancestor>} left @param {ReadonlyArray<Ancestor>} right */
function sameAncestors(left, right) {
  return left.length === right.length && left.every((value, index) =>
    value.path === right[index].path && value.identity === right[index].identity);
}
/** @param {Budget} budget */
function admitCatalog(budget) {
  budget.check();
  if (registeredFiles.length !== 3) fail('Bounds');
  let total = 0;
  for (const [index, item] of registeredFiles.entries()) {
    budget.check();
    if (item.ordinal !== index || item.moduleOrdinal !== 0 || !Number.isSafeInteger(item.bytes)
      || item.bytes < 1 || item.bytes > 16384 || Buffer.byteLength(item.body) !== item.bytes)
      fail('Bounds');
    total += item.bytes;
  }
  if (total !== 6563 || total > 65536) fail('Bounds');
  budget.check();
}
/** @param {(typeof registeredFiles)[number]} item @param {Budget} budget
 * @param {SourceGuard|undefined} previous */
async function readRegistered(item, budget, previous) {
  const path = selectedPath(item.relative, budget);
  const beforeAncestors = await ancestors(path, budget);
  const before = await budget.effect(() => lstat(path, { bigint: true }), true);
  safeFile(before, item.bytes);
  const canonical = await budget.effect(() => realpath(path), true);
  if (canonical !== path) fail('UnsafeSource');
  if (previous && (previous.path !== path || previous.canonical !== canonical
    || previous.identity !== fileIdentity(before)
    || !sameAncestors(previous.ancestors, beforeAncestors))) fail('SourceChanged');
  budget.check();
  const expected = Buffer.from(item.body, 'utf8');
  const bytes = Buffer.alloc(item.bytes);
  const chunk = Buffer.alloc(32768);
  budget.check();
  /** @type {import('node:fs/promises').FileHandle|undefined} */
  let handle;
  /** @type {RuntimeObservationError|undefined} */
  let failure;
  try {
    budget.check();
    // Own the returned descriptor before a post-call deadline check can throw.
    handle = await open(path, 'r');
    budget.check();
    const owned = handle;
    const opened = await budget.effect(() => owned.stat({ bigint: true }), true);
    safeFile(opened, item.bytes);
    if (fileIdentity(opened) !== fileIdentity(before)) fail('SourceChanged');
    let position = 0;
    let calls = 0;
    for (;;) {
      budget.check();
      if (++calls > item.bytes + 1) fail('Bounds');
      const length = Math.min(chunk.length, item.bytes - position + 1);
      const { bytesRead } = await budget.effect(() => owned.read(chunk, 0, length, position));
      if (bytesRead === 0) {
        if (position !== item.bytes) fail('SourceChanged');
        break;
      }
      if (position + bytesRead > item.bytes
        || !chunk.subarray(0, bytesRead).equals(expected.subarray(position, position + bytesRead)))
        fail('SourceChanged');
      chunk.copy(bytes, position, 0, bytesRead);
      position += bytesRead;
    }
    const finished = await budget.effect(() => owned.stat({ bigint: true }), true);
    safeFile(finished, item.bytes);
    const after = await budget.effect(() => lstat(path, { bigint: true }), true);
    safeFile(after, item.bytes);
    const afterCanonical = await budget.effect(() => realpath(path), true);
    const afterAncestors = await ancestors(path, budget);
    if (fileIdentity(finished) !== fileIdentity(before) || fileIdentity(after) !== fileIdentity(before)
      || afterCanonical !== canonical || !sameAncestors(beforeAncestors, afterAncestors))
      fail('SourceChanged');
  } catch (error) {
    failure = closed(error);
  } finally {
    if (handle) {
      try { budget.check(true); } catch (error) { failure ??= closed(error); }
      try { await handle.close(); } catch { failure ??= new RuntimeObservationError('Io'); }
      try { budget.check(true); } catch (error) { failure ??= closed(error); }
    }
  }
  if (failure) throw failure;
  // Equality and every awaited close precede hashing; unknown replacement bytes never get a digest.
  budget.check();
  const sha256 = createHash('sha256').update(bytes).digest('hex');
  budget.check();
  if (sha256 !== item.sha256) fail('SourceChanged');
  return {
    guard: Object.freeze({ path, canonical, identity: fileIdentity(before), ancestors: beforeAncestors }),
    artifact: Object.freeze({ ordinal: item.ordinal, moduleOrdinal: item.moduleOrdinal,
      role: item.role, classification: item.classification, bytes: item.bytes, sha256 }),
  };
}
/** @param {Budget} budget @param {ReadonlyArray<SourceGuard>|undefined} previous */
async function collect(budget, previous) {
  admitCatalog(budget);
  const guards = [];
  const artifacts = [];
  for (const [index, item] of registeredFiles.entries()) {
    budget.check();
    const result = await readRegistered(item, budget, previous?.[index]);
    guards.push(result.guard);
    artifacts.push(result.artifact);
  }
  budget.check();
  return { guards: Object.freeze(guards), artifacts: Object.freeze(artifacts) };
}
/** No prototype constructor or backing fields: only frozen closed getter methods.
 * @template {object} T @param {T} methods @returns {Readonly<T>} */
function view(methods) { return Object.freeze(Object.assign(Object.create(null), methods)); }
/** @param {Readonly<import('./runtime-package-registry.mjs').Declaration>|null} value */
function declarationView(value) {
  if (value === null) return null;
  const flags = Object.freeze(value.jvm_flags.map(flag =>
    view({ kind: () => flag.kind, value: () => flag.value })));
  return view({ origin: () => 'externally_supplied_unverified', os: () => value.os,
    arch: () => value.arch, runtimeFamily: () => value.runtime_family,
    runtimeVersion: () => value.runtime_version, jvmFlags: () => flags });
}
/** @param {ReadonlyArray<Artifact>} values
 * @param {Readonly<import('./runtime-package-registry.mjs').Declaration>|null} declared */
function observationView(values, declared) {
  const coverage = view({ knownTotal: () => 3, included: () => 3, omitted: () => 0,
    completeWithinScope: () => true, origin: () => 'quiescent_read_only_file',
    atomicity: () => 'port_local_only' });
  const modules = Object.freeze([view({ ordinal: () => 0, publicPackage: () => 'near-future',
    version: () => '0.1.0', stateSchema: () => null,
    provenance: () => 'registered_public_bytes_observed' })]);
  const artifacts = Object.freeze(values.map(value => view({ ordinal: () => value.ordinal,
    moduleOrdinal: () => value.moduleOrdinal, role: () => value.role,
    byteLength: () => value.bytes, sha256: () => value.sha256,
    classification: () => value.classification })));
  const declaration = declarationView(declared);
  return view({ schema: () => 'NF-RUNTIME-PACKAGE-OBSERVATION-1',
    scope: () => 'registered_package_public_files', coverage: () => coverage,
    modules: () => modules, artifacts: () => artifacts, declaredRuntime: () => declaration,
    liveRuntime: () => 'unavailable', embeddedSourceRevision: () => null,
    certification: () => 'unknown', limitations: () => limitations,
    [inspect.custom]: () => 'RuntimePackageObservationV1 { data_only: true, live_runtime: unavailable, certification: unknown }' });
}
/** Observe the registered public subset, never the live game or a current authority.
 * @param {number} deadlineMs @param {string|null} [declaredRuntimeJSON] */
export async function observeRuntimePackage(deadlineMs, declaredRuntimeJSON = null) {
  const start = performance.now();
  if (arguments.length < 1 || arguments.length > 2
    || (arguments.length === 2 && arguments[1] !== null && typeof arguments[1] !== 'string'))
    fail('InvalidInput');
  const budget = new Budget(deadlineMs, start, 20000, start + 300000, 0);
  try {
    budget.check();
    if (declaredRuntimeJSON !== null && declaredRuntimeJSON.length > 8192) fail('Bounds');
    const declared = parseDeclaration(declaredRuntimeJSON);
    budget.check();
    if (declared === undefined) fail('InvalidInput');
    const actual = await collect(budget, undefined);
    budget.check();
    const observation = observationView(actual.artifacts, declared);
    budget.check();
    const capture = { remaining: 20000, expiry: start + 300000,
      observations: budget.observations, consumed: false, guards: actual.guards };
    captures.set(observation, capture);
    budget.check();
    capture.remaining = Math.max(0, 20000 - (performance.now() - start));
    return observation;
  } catch (error) { throw closed(error); }
}
/** A single attempt, bound to this module's actual capture; no fresh work allowance.
 * @param {unknown} observation @param {number} deadlineMs */
export async function revalidateRuntimePackage(observation, deadlineMs) {
  const start = performance.now();
  if (arguments.length !== 2) fail('InvalidInput');
  if (observation === null || typeof observation !== 'object') fail('InvalidObservation');
  const capture = captures.get(observation);
  if (!capture || capture.consumed) fail('InvalidObservation');
  capture.consumed = true;
  /** @type {Budget|undefined} */
  let budget;
  try {
    budget = new Budget(deadlineMs, start, capture.remaining, capture.expiry, capture.observations);
    await collect(budget, capture.guards);
    budget.check();
  } catch (error) { throw closed(error); }
  finally {
    capture.remaining = Math.max(0, capture.remaining - (performance.now() - start));
    if (budget) capture.observations = budget.observations;
  }
}
