// @ts-check
import assert from 'node:assert/strict';
import {readFileSync,readdirSync} from 'node:fs';
import {basename} from 'node:path';
import test from 'node:test';

const workflow=readFileSync(new URL('../../.github/workflows/synthetic.yml',import.meta.url),'utf8');
const provenance='seed-initial-focus-provenance.test.mjs';
/** @param {string} name */
function job(name){
    const marker='\n  '+name+':',start=workflow.indexOf(marker);
    assert.ok(start>=0,'Required workflow job is absent: '+name);
    const rest=workflow.slice(start+1),next=rest.search(/\n  [a-z][\w-]*:/);
    return next<0?rest:rest.slice(0,next);
}

test('the required Windows groups retain every fixture file exactly once',()=>{
    const main=job('public-build'),operator=job('operator-public');
    const excluded=main.match(/\$fixtureTests = @\(Get-ChildItem tools\/starsector-proxy\/fixture\/\*\.test\.mjs -File \| Where-Object Name -ne '([^']+)'/);
    assert.ok(excluded,'The main fixture partition must declare its one isolated file');
    const isolated=[...operator.matchAll(/node --test --test-concurrency=1 (tools\/starsector-proxy\/fixture\/[^\s]+\.test\.mjs)/g)].map(m=>basename(m[1]));
    assert.deepEqual(isolated,[provenance]);
    assert.equal(excluded[1],provenance);
    const all=readdirSync(new URL('../../tools/starsector-proxy/fixture/',import.meta.url)).filter(n=>n.endsWith('.test.mjs')).sort();
    const selected=[...all.filter(n=>n!==excluded[1]),...isolated].sort();
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
