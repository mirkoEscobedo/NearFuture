// @ts-check
import test from 'node:test';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

test('coherently copied phase clocks cannot extend the immutable original caller cap',()=>{
 const f=wrongSceneHistoryFixture({phaseCallerDeadlineDelta:1});
 try{assertCaptureRefused(f);}finally{f.close();}
});
