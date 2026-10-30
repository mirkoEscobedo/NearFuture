// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';

test('wrong-scene failure history cannot substitute for completed seed provenance in either real consumer',async()=>{
 const source=await modernSeedFixture(),failed=wrongSceneHistoryFixture();
 try{
  writeFileSync(source.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
  json(source.closeoutPath,failed.closeout);source.manifest.provenance.closeout=filePin(source.closeoutPath);source.refresh();
  assert.throws(()=>inspectSeedCreationCustody(source.source.scenario,filePin(source.closeoutPath).sha256.toLowerCase()));
  const staged=source.target.stage();assert.equal(staged.error,undefined);assert.notEqual(staged.status,null);assert.notEqual(staged.status,0);
  assert.match(staged.stderr,/closed seed|closeout/i);assert.equal(existsSync(resolve(source.target.target,'seed-stage-claim.private.json')),false);
 }finally{failed.close();source.close();}
});
