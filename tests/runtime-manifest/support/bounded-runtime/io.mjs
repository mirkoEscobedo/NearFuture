// @ts-check
import { open, lstat } from 'node:fs/promises';
import { createHash } from 'node:crypto';

/** @param {Buffer} bytes */
export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
/** @param {Buffer} bytes */
export const gitBlob = bytes => createHash('sha1')
  .update(Buffer.from('blob ' + bytes.length + '\0')).update(bytes).digest('hex');
/** @param {import('node:fs').BigIntStats} s */
export const identity = s => ({
  dev: s.dev.toString(), ino: s.ino.toString(), mode: s.mode.toString(),
  nlink: s.nlink.toString(), size: s.size.toString(),
  mtime: s.mtimeNs.toString(), ctime: s.ctimeNs.toString(),
});
/** @param {import('./clock.mjs').CaseClock} clock
 * @param {string | URL} path @param {boolean} [cleanup]
 * @param {number} [remainingBytes] */
export async function readStable(clock, path, cleanup = false, remainingBytes = 1_048_576) {
  if (!Number.isSafeInteger(remainingBytes) || remainingBytes < 0
      || remainingBytes > 8_388_608) throw new Error('CASE_READ_ALLOWANCE');
  const maximum = BigInt(Math.min(1_048_576, remainingBytes));
  /** @type {import('node:fs/promises').FileHandle | undefined} */
  let handle;
  try {
    const target = await clock.guard(() => lstat(path, { bigint: true }), cleanup);
    if (!target.isFile() || target.isSymbolicLink() || target.nlink !== 1n
        || target.size < 0n || target.size > maximum) throw new Error('UNSAFE_OR_OVERSIZE_CASE_FILE');
    clock.check(cleanup);
    const file = await open(path, 'r');
    handle = file;
    clock.check(cleanup);
    const before = await clock.guard(() => file.stat({ bigint: true }), cleanup);
    if (!before.isFile() || before.nlink !== 1n || before.size < 0n
        || before.size > maximum
        || JSON.stringify(identity(before)) !== JSON.stringify(identity(target)))
      throw new Error('CASE_SOURCE_CHANGED');
    const buffer = Buffer.alloc(32_768);
    /** @type {Buffer[]} */
    const chunks = [];
    let position = 0;
    for (;;) {
      const length = Math.min(buffer.length, Number(before.size) - position + 1);
      const result = await clock.guard(
        () => file.read(buffer, 0, length, position), cleanup);
      if (result.bytesRead === 0) break;
      if (position + result.bytesRead > Number(before.size))
        throw new Error('CASE_SOURCE_CHANGED');
      chunks.push(Buffer.from(buffer.subarray(0, result.bytesRead)));
      position += result.bytesRead;
    }
    if (position !== Number(before.size)) throw new Error('CASE_SOURCE_CHANGED');
    const after = await clock.guard(() => file.stat({ bigint: true }), cleanup);
    const finalTarget = await clock.guard(() => lstat(path, { bigint: true }), cleanup);
    if (JSON.stringify(identity(before)) !== JSON.stringify(identity(after))
        || JSON.stringify(identity(before)) !== JSON.stringify(identity(finalTarget)))
      throw new Error('CASE_SOURCE_CHANGED');
    return { bytes: Buffer.concat(chunks), identity: identity(before) };
  } finally {
    if (handle) {
      const owned = handle;
      await clock.close(() => owned.close());
    }
  }
}
/** No overwrite, delete, write retry, or GC-only close.
 * @param {import('./clock.mjs').CaseClock} clock
 * @param {string} path @param {Buffer} bytes @param {boolean} [cleanup] */
export async function writeNew(clock, path, bytes, cleanup = false) {
  /** @type {import('node:fs/promises').FileHandle | undefined} */
  let handle;
  try {
    clock.check(cleanup);
    const file = await open(path, 'wx');
    handle = file;
    clock.check(cleanup);
    let position = 0;
    while (position < bytes.length) {
      const result = await clock.guard(() => file.write(bytes, position,
        Math.min(32_768, bytes.length - position), position), cleanup);
      if (result.bytesWritten <= 0) throw new Error('CASE_ZERO_WRITE');
      position += result.bytesWritten;
    }
  } finally {
    if (handle) {
      const owned = handle;
      await clock.close(() => owned.close());
    }
  }
}
