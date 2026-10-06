import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const cli = fileURLToPath(new URL('../../tools/runtime-manifest/cli.mjs', import.meta.url));
test('missing installation is a nonzero actionable private-path-free CLI result', () => {
  const result = spawnSync(process.execPath, [cli, '--game-dir', 'Z:/private/no-game'], { encoding: 'utf8', timeout: 10000 });
  assert.equal(result.status, 1);
  assert.equal(result.stdout, '');
  assert.match(JSON.parse(result.stderr).error, /^MISSING_INSTALLATION:/);
  assert.ok(!result.stderr.includes('Z:/private'));
});
