// @ts-check
import { mkdtemp, mkdir, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, dirname } from 'node:path';
/** @param {import("node:test").TestContext} t @param {Record<string,string>} overrides */
export async function installation(t, overrides = {}) {
  const root = await mkdtemp(join(tmpdir(), 'nf-manifest-'));
  t.after(() => rm(root, { recursive: true, force: true }));
  const files = {
    'starsector-core/starfarer.api.jar': 'abc',
    'starsector-core/starfarer_obf.jar': 'game',
    'starsector-core/data/config/settings.json': '{"devMode":false}',
    'jre/release': 'IMPLEMENTOR="Example JVM"\nJAVA_VERSION="17.0.10"\nJAVA_RUNTIME_VERSION="17.0.10+7"\nOS_ARCH="x86_64"\nOS_NAME="Windows"',
    'vmparams': 'java.exe -Xmx4g -Dprivate.token=secret -Djava.library.path=C:\\private\\native -classpath starfarer.api.jar com.fs.starfarer.StarfarerLauncher',
    'mods/enabled_mods.json': '{"enabledMods":["nexerelin"]}',
    'mods/Nex/mod_info.json': '{# comment\n"id":"nexerelin","version":{"major":0,"minor":12,"patch":"2c"},"jars":["jars/Nex.jar"],}',
    'mods/Nex/jars/Nex.jar': 'abc',
    'mods/Nex/exerelin_config.json': '{"enableStrategicAI":true}',
    ...overrides,
  };
  for (const [path, content] of Object.entries(files)) {
    await mkdir(dirname(join(root, path)), { recursive: true });
    await writeFile(join(root, path), content);
  }
  return root;
}
