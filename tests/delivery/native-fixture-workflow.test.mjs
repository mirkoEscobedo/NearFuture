// @ts-check
import assert from 'node:assert/strict';
import {readFileSync,readdirSync} from 'node:fs';
import {basename} from 'node:path';
import test from 'node:test';

const workflow=readFileSync(new URL('../../.github/workflows/synthetic.yml',import.meta.url),'utf8');
const isolatedFiles=[
    'seed-initial-focus-provenance.test.mjs',
    'seed-provider-budget-json.test.mjs',
    'seed-provider-budget-launch.test.mjs',
    'seed-provider-budget-provenance.test.mjs',
    'seed-provider-budget-refusals.test.mjs',
    'seed-provider-budget-runtime.test.mjs',
    'seed-pre-continue-history.test.mjs',
    'seed-pre-continue-history-refusals.test.mjs',
    'seed-pre-continue-review-r1.test.mjs',
];
/** @param {string} name */
function job(name){
    const marker='\n  '+name+':',start=workflow.indexOf(marker);
    assert.ok(start>=0,'Required workflow job is absent: '+name);
    const rest=workflow.slice(start+1),next=rest.search(/\n  [a-z][\w-]*:/);
    return next<0?rest:rest.slice(0,next);
}

test('the required Windows groups retain every fixture file exactly once',()=>{
    const main=job('public-build'),operator=job('operator-public');
    const declaration=main.match(/\$isolatedFixtureTests = @\(([^)]+)\)/);
    assert.ok(declaration,'The main fixture partition must declare its closed isolated set');
    const excluded=[...declaration[1].matchAll(/'([^']+)'/g)].map(m=>m[1]);
    const command=operator.match(/node --test --test-concurrency=1 ([^\r\n]+)/);
    assert.ok(command,'The operator must execute its isolated set serially');
    const isolated=command[1].trim().split(/\s+/).map(path=>{
        assert.match(path,/^tools\/starsector-proxy\/fixture\/[^/]+\.test\.mjs$/);
        return basename(path);
    });
    assert.deepEqual(excluded,isolatedFiles);
    assert.deepEqual(isolated,isolatedFiles);
    assert.match(main,/\$fixtureTests = @\(Get-ChildItem tools\/starsector-proxy\/fixture\/\*\.test\.mjs -File \| Where-Object \{ \$_.Name -notin \$isolatedFixtureTests \}/);
    const all=readdirSync(new URL('../../tools/starsector-proxy/fixture/',import.meta.url)).filter(n=>n.endsWith('.test.mjs')).sort();
    const selected=[...all.filter(n=>!excluded.includes(n)),...isolated].sort();
    assert.deepEqual(selected,all,'No fixture may disappear or execute twice');
    assert.match(main,/node --test @fixtureTests/);
    assert.match(main,/node --test @captureTests/);
    assert.match(job('headless-qualification'),/needs: \[[^\n]*operator-public[^\n]*\]/);
});

test('isolated provenance retains pinned setup and the existing deadlines',()=>{
    const operator=job('operator-public'),marker='      - name: Qualify initial-focus provenance without other fixture workers';
    const start=operator.indexOf(marker);
    assert.ok(start>=0,'The required Windows operator job must execute the isolated file');
    const step=operator.slice(start);
    assert.match(step,/if: runner.os == 'Windows'/);
    assert.match(step,/npm ci --prefix tools\/computer-use-mcp --ignore-scripts --bin-links=false --no-audit --no-fund/);
    assert.match(step,/\$env:NF_TEST_POWERSHELL = \(Get-Command pwsh/);
    assert.match(step,/\$env:NF_CAPTURE_TEST_POWERSHELL = \$env:NF_TEST_POWERSHELL/);
    assert.match(step,/buildCaptureRuntime\(\)/);
    assert.match(step,/\$PSNativeCommandUseErrorActionPreference = \$true/);
    assert.match(operator,/timeout-minutes: 20/);
    assert.match(job('public-build'),/matrix.os == 'windows-latest' && 30 \|\| 20/);
    const helper=readFileSync(new URL('../../tools/starsector-proxy/fixture/seed-test-support.mjs',import.meta.url),'utf8');
    assert.match(helper,/timeout:15000\b/);
});
