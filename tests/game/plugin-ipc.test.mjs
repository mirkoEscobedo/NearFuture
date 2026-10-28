// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp, rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {delimiter, join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
import {ipcClasspath} from './support/ipc-classpath.mjs';

test('actual plugin callback host remains responsive with a missing node and revokes exact bindings across save/load', async t => {
  const game=process.env.STARSECTOR_HOME;
  assert.ok(game,'licensed STARSECTOR_HOME required');
  const directory=await mkdtemp(join(tmpdir(),'nf-plugin-ipc-'));
  t.after(()=>rm(directory,{recursive:true,force:true}));
  const libraries=['starfarer.api.jar','xstream-1.4.10.jar','log4j-1.2.9.jar',
    'lwjgl.jar','lwjgl_util.jar','json.jar','fs.common_obf.jar','fs.sound_obf.jar'];
  const api=libraries.map(name=>join(game,'starsector-core',name)).join(delimiter)+delimiter+ipcClasspath();
  const options={cwd:fileURLToPath(new URL('../../',import.meta.url)),
    encoding:/** @type {const} */('utf8'),timeout:30000,maxBuffer:100000};
  const compiled=spawnSync('javac',['--release','17','-Xlint:all','-Werror','-cp',api,
    '-sourcepath','java/adapter/src/main/java','-d',directory,
    'java/contract/src/main/java/nf/contract/BuildIdentity.java',
    'tests/game/support/CampaignPluginIpcControls.java'],options);
  assert.equal(compiled.status,0,compiled.stderr);
  const result=spawnSync('java',['-Dnf.profiling.callback.enabled=false','-cp',[directory,api].join(delimiter),
    'CampaignPluginIpcControls',directory],options);
  assert.equal(result.status,0,result.stderr);
  assert.equal(result.stdout.trim(),'PLUGIN_MISSING_NODE_FENCES_OK');
});
