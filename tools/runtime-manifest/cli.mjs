// @ts-check
import { parseArgs } from 'node:util';
import { writeFile } from 'node:fs/promises';
import { captureInstallation } from './capture.mjs';
try {
  const { values } = parseArgs({ options: {
    'game-dir': { type: 'string' }, 'game-version': { type: 'string' },
    output: { type: 'string' }, help: { type: 'boolean' },
  } });
  if (values.help) {
    process.stdout.write('Usage: node tools/runtime-manifest/cli.mjs --game-dir <installation> [--game-version <build>] [--output <json>]\nReads configured runtime and active mod hashes without launching the game. Unknown configurations remain legacy/read-only.\n');
  } else {
    const manifest = await captureInstallation({ gameDir: values['game-dir'], gameVersion: values['game-version'] });
    const json = JSON.stringify(manifest, null, 2) + '\n';
    if (values.output) await writeFile(values.output, json);
    else process.stdout.write(json);
  }
} catch (error) {
  // Node/OS errors may include absolute private paths: only capture's explicit diagnostics are public.
  const message = error instanceof Error && /^[A-Z_]+: /.test(error.message)
    ? error.message : 'CAPTURE_FAILED: check arguments, file permissions and output directory; rerun --help.';
  process.stderr.write(JSON.stringify({ schemaVersion: 1, error: message }) + '\n');
  process.exitCode = 1;
}
