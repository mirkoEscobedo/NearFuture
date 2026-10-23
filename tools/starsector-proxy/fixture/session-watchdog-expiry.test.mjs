// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { makeOutput } from '../capture/test-output.mjs';

const repository = resolve(fileURLToPath(new URL('../../../', import.meta.url)));
const powershell = process.env.NF_TEST_POWERSHELL ?? 'pwsh';
const quote = (/** @type {string} */ s) => "'" + s.replaceAll("'", "''") + "'";

test('actual watchdog expiry tail retains INCOMPLETE and exits nonzero without launching a game', () => {
    const output = makeOutput('watchdog-expiry-');
    const source = resolve(repository, 'tools/starsector-proxy/fixture/native-controller.ps1');
    // Execute the exact actual tail after its real loop, with its real receipt functions.
    // No controller initialization, Add-Type, process census, loop, or launcher executes.
    const script = String.raw`$ErrorActionPreference='Stop'
$source=Get-Content -LiteralPath ${quote(source)} -Raw
$tokens=$null;$errors=$null
$tree=[Management.Automation.Language.Parser]::ParseInput($source,[ref]$tokens,[ref]$errors)
if(@($errors).Count){throw 'Controller AST invalid'}
foreach($name in @('Assert-OwnedHookOrdinary','Write-OwnedHookExclusive')){
 $definitions=@($tree.FindAll({param($node) $node-is[Management.Automation.Language.FunctionDefinitionAst]-and$node.Name-ceq$name},$true))
 if($definitions.Count-ne1){throw 'Exact real receipt function required'}
 . ([scriptblock]::Create($definitions[0].Extent.Text))
}
$loops=@($tree.FindAll({param($node) $node-is[Management.Automation.Language.WhileStatementAst]-and$node.Extent.Text.StartsWith('while([DateTime]::UtcNow-lt$hookState.ControllerDeadlineUtc)')},$true))
if($loops.Count-ne1){throw 'Exact real controller loop required'}
$tail=$source.Substring($loops[0].Extent.EndOffset)
$EvidenceRoot=${quote(output)}
$SessionWatchdogMs=7200000
. ([scriptblock]::Create($tail))
`;
    const args = ['-NoProfile', '-NonInteractive', '-Command', script];
    const child = spawnSync(powershell, args, { cwd: repository, encoding: 'utf8', timeout: 15000 });
    writeFileSync(resolve(output, 'expiry-child.actual.private.json'), JSON.stringify({
        scope: 'ACTUAL_EXTRACTED_EXPIRY_TAIL_NO_GAME_NO_TWO_HOUR_WAIT', program: powershell, args,
        status: child.status, signal: child.signal, stdout: child.stdout, stderr: child.stderr,
        error: child.error?.message, evidenceRoot: output,
    }) + '\n', { flag: 'wx' });
    const receipt = JSON.parse(readFileSync(resolve(output, 'controller-watchdog.private.json'), 'utf8'));
    assert.equal(receipt.status, 'INCOMPLETE_AUTOMATION_WATCHDOG');
    assert.equal(receipt.productOutcome, 'NOT_ESTABLISHED');
    assert.equal(receipt.sessionWatchdogMs, 7200000);
    assert.equal(child.signal, null);
    assert.equal(child.status, 2, 'watchdog expiry must never report successful controller completion');
});
