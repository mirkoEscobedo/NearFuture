// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import {analyzeTrace} from '../../tools/performance/analyze.mjs';
import {run} from './support/run.mjs';
import {compareRuns} from '../../tools/performance/compare.mjs';

const metadata=run({startEpochUs:1000,warmupUs:0,durationUs:1000});
const budgets={bridgeP99Us:2000,longFrameUs:500};
const intervals=[
  {atUs:900,durationUs:100}, {atUs:1000,durationUs:300},
  {atUs:1300,durationUs:700}, {atUs:2000,durationUs:200}
].map((value,index)=>({traceId:metadata.traceId,source:'jfr',kind:'presentation_interval',
  threadId:19,frameId:index+1,clockUncertaintyUs:0,...value}));

test('unattested presentation cadence remains separate from whole-frame evidence',()=>{
  const report=analyzeTrace(metadata,intervals,budgets);
  assert.equal(report.frames.count,0);
  assert.deepEqual(report.presentations,{count:2,p50Us:300,p95Us:700,p99Us:700,p999Us:700,boundaryEvidenceSha256:null});
  assert.equal(report.actualGameBaseline,'unmeasured');
});

test('an explicit retained boundary certificate admits only complete presentation intervals as wall frames',()=>{
  const report=analyzeTrace({...metadata,frameBoundaryEvidenceSha256:'f'.repeat(64)},intervals,budgets);
  assert.deepEqual(report.frames,{count:2,p50Us:300,p95Us:700,p99Us:700,p999Us:700,longFrames:1});
  assert.equal(report.presentations.boundaryEvidenceSha256,'f'.repeat(64));
  assert.equal(report.actualGameBaseline,'unmeasured'); // A certificate never turns synthetic input into a game.
});

test('paired comparison refuses different presentation-boundary certificates',()=>{
  const baseline=analyzeTrace({...metadata,frameBoundaryEvidenceSha256:'f'.repeat(64)},intervals,budgets);
  const other=analyzeTrace({...metadata,scenario:'instrumentation-off',instrumentation:'frame',
    frameBoundaryEvidenceSha256:'e'.repeat(64)},intervals,budgets);
  assert.throws(()=>compareRuns([baseline,other]),/COMPARISON_MISMATCH: frameBoundaryEvidenceSha256/);
});

test('presentation endpoint uncertainty cannot exceed its declared run calibration',()=>{
  const outside=intervals.map(value=>({...value,clockUncertaintyUs:1001}));
  assert.throws(()=>analyzeTrace(metadata,outside,budgets),/PRESENTATION_CLOCK_UNCERTAINTY/);
});

test('missing, duplicate or mixed-thread presentation observations refuse rather than forming partial frames',()=>{
  const certified={...metadata,frameBoundaryEvidenceSha256:'f'.repeat(64)};
  for(const values of [intervals.slice(1),[intervals[0],intervals[0],...intervals.slice(1)],
    intervals.map((value,index)=>index===2?{...value,threadId:20}:value)]) {
    assert.throws(()=>analyzeTrace(certified,values,budgets),/PRESENTATION_LOSS/);
  }
  assert.throws(()=>analyzeTrace(certified,[],budgets),/MISSING_PRESENTATION/);
  assert.throws(()=>analyzeTrace(certified,intervals.slice(0,2),budgets),/INCOMPLETE_PRESENTATION_WINDOW/);
  assert.throws(()=>analyzeTrace({...certified,durationUs:1300},intervals,budgets),/INCOMPLETE_PRESENTATION_WINDOW/);
  assert.throws(()=>analyzeTrace(certified,intervals.map(value=>({...value,atUs:value.atUs+4000})),budgets),/INCOMPLETE_PRESENTATION_WINDOW/);
});

test('whole-phase windows keep complete long intervals and exclude crossing tails without clipping',()=>{
  const certified={...metadata,frameBoundaryEvidenceSha256:'f'.repeat(64)};
  const crossing=analyzeTrace({...certified,durationUs:800},intervals,budgets);
  assert.deepEqual(crossing.frames,{count:1,p50Us:300,p95Us:300,p99Us:300,p999Us:300,longFrames:0});
  assert.equal(crossing.quality.excludedPhaseEvents,1);
  const long=[{atUs:900,durationUs:100},{atUs:1000,durationUs:2000000},
    {atUs:2001000,durationUs:300000},{atUs:2301000,durationUs:100}]
    .map((value,index)=>({...intervals[0],frameId:index+1,...value}));
  const full=analyzeTrace({...certified,durationUs:2300000},long,budgets);
  assert.equal(full.frames.count,2);
  assert.equal(full.frames.p99Us,2000000);
  assert.equal(full.frames.longFrames,2);
});
