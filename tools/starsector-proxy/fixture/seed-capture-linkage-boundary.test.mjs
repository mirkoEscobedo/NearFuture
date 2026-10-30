// @ts-check
import test from 'node:test';
import {resolve} from 'node:path';
import {mkdirSync} from 'node:fs';
import {captureLinkageHistoryFixture} from './seed-capture-linkage-test-support.mjs';
import {write} from './seed-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

for(const name of ['B1-PLAY-client-geometry.actual.private.json','B1-PLAY-client-geometry-terminal.actual.private.json','B1-PLAY-coordinate-proof.private.json','geometry-owned-B1-PLAY']){
 test('zero geometry history refuses the actual downstream artifact '+name,()=>{
  const f=captureLinkageHistoryFixture();
  try{const path=resolve(f.priorScenario,name);if(name.startsWith('geometry-owned'))mkdirSync(path);else write(path,'{}');assertCaptureRefused(f);}finally{f.close();}
 });
}
for(const [label,change] of /** @type {[string,(v:any)=>void][]} */([
 ['revoked B1 authority',v=>{v.authorized=false;}],['extended B1 watchdog',v=>{v.ownedGameHardTimeoutMs=480001;}],
 ['extra B1 action',v=>{v.maxActions=9;}]
])){
 test('historical launch refuses coherently pinned '+label,()=>{
  const f=captureLinkageHistoryFixture({changePhaseLease:change});try{assertCaptureRefused(f);}finally{f.close();}
 });
}
