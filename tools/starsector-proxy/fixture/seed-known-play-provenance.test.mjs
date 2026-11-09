// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {existsSync,writeFileSync} from 'node:fs';
import {resolve} from 'node:path';
import {modernSeedFixture} from './seed-modern-test-support.mjs';
import {knownPlayHistoryFixture} from './seed-known-play-history-test-support.mjs';
import {json,filePin} from './seed-test-support.mjs';
import {inspectSeedCreationCustody} from '../seed-creation-custody.mjs';

test('known Play failure cannot substitute for completed seed provenance in either consumer',async()=>{
 const source=await modernSeedFixture(),failed=knownPlayHistoryFixture();
 try{
  writeFileSync(source.commonPath,'{"campaignHelpPopupsOptionChecked": true}');
  json(source.closeoutPath,failed.closeout);source.manifest.provenance.closeout=filePin(source.closeoutPath);source.refresh();
  assert.throws(()=>inspectSeedCreationCustody(source.source.scenario,filePin(source.closeoutPath).sha256.toLowerCase()));
  const result=source.target.stage();assert.notEqual(result.status,0);assert.match(result.stderr,/closed seed|closeout/i);
  assert.equal(existsSync(resolve(source.target.target,'seed-stage-claim.private.json')),false);
 }finally{failed.close();source.close();}
});
