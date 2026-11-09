// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {mkdtemp,rm} from 'node:fs/promises';
import {tmpdir} from 'node:os';
import {join} from 'node:path';
import {spawnSync} from 'node:child_process';
import {sanitizeEvent} from '../../tools/performance/contract.mjs';

test('presentation export uses preceding monotonic intervals and preserves long samples and endpoint calibration',async t=>{
  const directory=await mkdtemp(join(tmpdir(),'nf-presentation-export-'));
  t.after(()=>rm(directory,{recursive:true,force:true}));
  const options={encoding:/** @type {const} */('utf8'),timeout:30000,maxBuffer:100000};
  const compiled=spawnSync('javac',['--release','17','-Xlint:all','-Werror','-d',directory,
    'tools/performance/jfr/ExportRecording.java','tests/performance/support/JfrPresentationCapture.java'],options);
  assert.equal(compiled.status,0,compiled.stderr);
  const trace='0123456789abcdef0123456789abcdef',file=join(directory,'presentation.jfr');
  const captured=spawnSync('java',['-cp',directory,'JfrPresentationCapture',file,trace,'valid'],options);
  assert.equal(captured.status,0,captured.stderr);
  const endpoints=JSON.parse(captured.stdout);
  const exported=spawnSync('java',['-cp',directory,'ExportRecording',file,trace],options);
  assert.equal(exported.status,0,exported.stderr);
  const records=exported.stdout.trim().split('\n').filter(Boolean).map(line=>sanitizeEvent(JSON.parse(line)));
  assert.equal(records.length,2);
  assert.deepEqual(records.map(value=>value.kind),['presentation_interval','presentation_interval']);
  assert.deepEqual(records.map(value=>value.durationUs),[12345,3000000]);
  assert.deepEqual(records.map(value=>value.clockUncertaintyUs),[3,2]); // Ceil marker jitter plus microsecond quantization.
  for(let index=0;index<records.length;index++) {
    const event=records[index];
    assert.equal(event.atUs+Number(event.durationUs),endpoints[index]);
    assert.equal(event.frameId,index+1);
    assert.ok(Number(event.threadId)>0);
  }
  assert.ok(!exported.stdout.includes('private-marker-secret'));
  assert.ok(!exported.stdout.includes(directory));
  for(const mode of ['zero','negative-uncertainty','probe-rejected','data-loss']) {
    await t.test(`refuse ${mode} and require callers to discard partial export`,()=>{
      const invalid=join(directory,`${mode}.jfr`);
      const capture=spawnSync('java',['-cp',directory,'JfrPresentationCapture',invalid,trace,mode],options);
      assert.equal(capture.status,0,capture.stderr);
      const refused=spawnSync('java',['-cp',directory,'ExportRecording',invalid,trace],options);
      assert.notEqual(refused.status,0);
      assert.match(refused.stderr,/JFR_EXPORT_UNAVAILABLE/);
      assert.ok(!refused.stderr.includes('private-marker-secret'));
    });
  }
});
