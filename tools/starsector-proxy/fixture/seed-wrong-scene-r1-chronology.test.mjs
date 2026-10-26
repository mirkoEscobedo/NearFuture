// @ts-check
import test from 'node:test';
import {wrongSceneHistoryFixture} from './seed-wrong-scene-test-support.mjs';
import {assertCaptureRefused} from './seed-capture-stop-test-support.mjs';

/** @type {[string,Parameters<typeof wrongSceneHistoryFixture>[0]][]} */
const cases=[
 ['B1 inspection after E1 start',{b1InspectedOffsetMs:17000}],
 ['B1 closure observed after E1 start',{b1ClosedObservedOffsetMs:17000}],
 ['node born after readiness',{e1NodeBirthOffsetMs:16050,e1HelperBirthOffsetMs:16051}],
 ['helper born after readiness',{e1HelperBirthOffsetMs:16050}],
 ['game predates its controller',{gameBirthOffsetMs:50}],
 ['game born after Play acknowledgement',{gameBirthOffsetMs:2000}],
];
for(const [name,options]of cases)test('both readers refuse coherently pinned '+name,()=>{
 const f=wrongSceneHistoryFixture(options);try{assertCaptureRefused(f);}finally{f.close();}
});
