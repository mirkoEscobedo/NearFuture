// @ts-check
import { readFile, readdir, realpath, stat } from 'node:fs/promises';
import { createHash } from 'node:crypto';
import { join, relative, isAbsolute, sep } from 'node:path';
import { platform, arch, release } from 'node:os';
import { parseGameJson, safeJvmFlags } from './metadata.mjs';

/** Capture installation evidence only; this never launches or mutates the game.
 * @param {{ gameDir?: string, gameVersion?: string }} options
 */
export async function captureInstallation({ gameDir, gameVersion = 'unknown' }) {
  if (!gameDir) throw new Error('MISSING_GAME_DIR: pass --game-dir pointing to your installation.');
  if (!/^(unknown|[0-9]+\.[0-9]+[a-z]?(?:-RC[0-9]+)?)$/.test(gameVersion))
    throw new Error('INVALID_GAME_VERSION: use the exact game build (for example 0.98a-RC8).');
  let root = '';
  try { root = await realpath(gameDir); }
  catch { throw new Error('MISSING_INSTALLATION: --game-dir does not exist; choose an installed game.'); }
  /** @type {{path: string, bytes: number, sha256: string}[]} */
  const artifacts = [];
  /** @type {{code: string, action: string, modId?: string}[]} */
  const diagnostics = [];
  /** @param {string} path @param {boolean} required */
  async function bytes(path, required = true) {
    try {
      const actual = await realpath(join(root, path));
      const rel = relative(root, actual);
      if (rel === '..' || rel.startsWith(`..${sep}`) || isAbsolute(rel))
        throw new Error('OUTSIDE_INSTALLATION');
      return await readFile(actual);
    } catch (error) {
      if (error instanceof Error && error.message === 'OUTSIDE_INSTALLATION')
        throw new Error(`UNSAFE_PATH: ${path.replaceAll('\\', '/')}; inputs must remain inside the installation.`);
      if (!required && error instanceof Error && 'code' in error && error.code === 'ENOENT') return null;
      throw new Error(`MISSING_INPUT: ${path.replaceAll('\\', '/')}; restore this installation file and retry.`);
    }
  }
  /** @param {string} path @param {string} [label] @param {boolean} [required] */
  async function artifact(path, label = path, required = true) {
    const content = await bytes(path, required);
    if (content) artifacts.push({ path: label.replaceAll('\\', '/'), bytes: content.length,
      sha256: createHash('sha256').update(content).digest('hex') });
    return content;
  }
  for (const file of ['starfarer.api.jar', 'starfarer_obf.jar']) await artifact(`starsector-core/${file}`);
  for (const file of (await readdir(join(root, 'starsector-core'))).sort()) {
    if (file.endsWith('.jar') && !['starfarer.api.jar', 'starfarer_obf.jar'].includes(file))
      await artifact(`starsector-core/${file}`);
  }
  await artifact('starsector-core/data/config/settings.json');
  for (const file of ['compiler_directives.txt', 'starsector.bat', 'starsector.command'])
    await artifact(`starsector-core/${file}`, undefined, false);
  const jvmText = String(await artifact('jre/release'));
  const properties = Object.fromEntries([...jvmText.matchAll(/^([A-Z_]+)="([^"]*)"/gm)].map(x => [x[1], x[2]]));
  if (!properties.JAVA_VERSION || !properties.IMPLEMENTOR)
    throw new Error('INVALID_JVM_RELEASE: jre/release must declare JAVA_VERSION and IMPLEMENTOR.');
  if (!properties.JAVA_VERSION.startsWith('17.')) diagnostics.push({ code: 'UNVERIFIED_JVM', action: 'Use the recorded Java 17 target for experiments; keep other JVMs legacy/read-only until certified.' });
  if (gameVersion === 'unknown') diagnostics.push({ code: 'UNKNOWN_GAME_BUILD', action: 'Pass --game-version using the exact game version display or public Version constant.' });
  await artifact('jre/bin/java.exe', undefined, false);
  await artifact('jre/bin/java', undefined, false);
  const vmparams = String(await artifact('vmparams'));
  const enabled = parseGameJson(String(await artifact('mods/enabled_mods.json')), 'enabled mods');
  if (!enabled || typeof enabled !== 'object' || !Array.isArray(enabled.enabledMods) || !enabled.enabledMods.every(/** @param {unknown} x */ x => typeof x === 'string'))
    throw new Error('INVALID_ENABLED_MODS: enabledMods must be an array of mod IDs.');
  /** @type {string[]} */
  const enabledIds = [...new Set(enabled.enabledMods)].sort();
  const mods = [];
  for (const folder of (await readdir(join(root, 'mods'), { withFileTypes: true })).sort((a,b) => a.name.localeCompare(b.name, 'en'))) {
    if (!folder.isDirectory() || folder.isSymbolicLink()) continue;
    const modPath = `mods/${folder.name}`;
    const content = await bytes(`${modPath}/mod_info.json`, false);
    if (!content) continue;
    const info = parseGameJson(content.toString('utf8'), 'mod_info.json');
    if (!info || typeof info !== 'object') throw new Error('INVALID_MOD_METADATA: mod_info.json must contain an object.');
    if (typeof info.id !== 'string' || !enabledIds.includes(info.id)) continue;
    if (!/^[A-Za-z0-9 _+&.-]+$/.test(info.id)) throw new Error('INVALID_MOD_ID: mod IDs must be printable identifiers.');
    const version = typeof info.version === 'string' ? info.version :
      info.version && ['major','minor','patch'].map(k => info.version[k]).join('.');
    if (!version || !/^[A-Za-z0-9 ._+-]+$/.test(version)) throw new Error(`INVALID_MOD_VERSION: ${info.id}; repair its version metadata.`);
    const label = `mods/${info.id}/${version}`;
    await artifact(`${modPath}/mod_info.json`, `${label}/mod_info.json`);
    if (info.jars !== undefined && (!Array.isArray(info.jars) || !info.jars.every(/** @param {unknown} x */ x => typeof x === 'string')))
      throw new Error(`INVALID_MOD_JARS: ${info.id}; jars must be relative paths.`);
    for (const jar of (info.jars ?? []).sort()) {
      if (isAbsolute(jar) || jar.split(/[\\/]/).includes('..') || !jar.endsWith('.jar'))
        throw new Error(`UNSAFE_MOD_JAR: ${info.id}; JARs must be relative paths inside the mod.`);
      await artifact(`${modPath}/${jar}`, `${label}/${jar}`);
    }
    /** @param {string} path @param {string} logical */
    async function configFiles(path, logical) {
      const entries = await readdir(join(root, path), { withFileTypes: true });
      for (const entry of entries.sort((a,b) => a.name.localeCompare(b.name, 'en'))) {
        if (entry.isSymbolicLink()) throw new Error(`UNSAFE_CONFIG_LINK: ${info.id}; remove config links for capture.`);
        if (entry.isDirectory()) await configFiles(`${path}/${entry.name}`, `${logical}/${entry.name}`);
        else if (/\.(json|csv|ini|properties)$/.test(entry.name)) await artifact(`${path}/${entry.name}`, `${logical}/${entry.name}`);
      }
    }
    for (const entry of await readdir(join(root, modPath), { withFileTypes: true }))
      if (entry.isFile() && /\.(json|csv|ini|properties)$/.test(entry.name) && entry.name !== 'mod_info.json')
        await artifact(`${modPath}/${entry.name}`, `${label}/${entry.name}`);
    if (await bytes(`${modPath}/data/config/settings.json`, false)) { /* captured by config tree below */ }
    try { if ((await stat(join(root, modPath, 'data/config'))).isDirectory()) await configFiles(`${modPath}/data/config`, `${label}/data/config`); }
    catch (error) { if (!(error instanceof Error && 'code' in error && error.code === 'ENOENT')) throw error; }
    mods.push({ id: info.id, version, gameVersionRequirement: info.gameVersion ?? 'unknown' });
  }
  for (const id of enabledIds) {
    const count = mods.filter(mod => mod.id === id).length;
    if (count !== 1) diagnostics.push({ code: count ? 'AMBIGUOUS_MOD' : 'MISSING_ENABLED_MOD', modId: id,
      action: count ? 'Keep exactly one enabled installation of this mod ID.' : 'Restore the enabled mod or disable its ID, then recapture.' });
  }
  artifacts.sort((a,b) => a.path.localeCompare(b.path, 'en') || a.sha256.localeCompare(b.sha256));
  mods.sort((a,b) => a.id.localeCompare(b.id, 'en') || a.version.localeCompare(b.version, 'en'));
  return { schemaVersion: 1, game: { version: gameVersion, versionEvidence: 'contributor-declared; verify against game version display or public Version constant' },
    host: { os: platform(), architecture: arch(), release: release() },
    jvm: { vendor: properties.IMPLEMENTOR, version: properties.JAVA_VERSION,
      runtimeVersion: properties.JAVA_RUNTIME_VERSION ?? properties.JAVA_VERSION,
      os: properties.OS_NAME ?? 'unknown', architecture: properties.OS_ARCH ?? 'unknown',
      flags: safeJvmFlags(vmparams), flagsEvidence: 'configured-vmparams; live process flags unverified',
      excludedFlags: 'properties, agents, paths and arbitrary values omitted; original files hashed' },
    enabledModIds: enabledIds, mods, artifacts, diagnostics,
    compatibility: { status: 'unknown', allowedMode: 'legacy-read-only', reason: 'Inventory is evidence, not a runtime integration certificate.' } };
}
