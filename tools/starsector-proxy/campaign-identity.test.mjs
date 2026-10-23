// @ts-check
import assert from 'node:assert/strict';
import test from 'node:test';
import { readFileSync, existsSync } from 'node:fs';
import { resolve } from 'node:path';
import { exitFixture } from './campaign-exit-test-fixture.mjs';
import { sha } from './campaign-test-fixture.mjs';
import { write } from './batch-test-fixture.mjs';
import { readBatchAuthorization } from './batch-authorization.mjs';

for (const fault of ['changed-birth', 'changed-executable']) test('E2 refuses ' + fault + ' under the same PID before claim, transfer or effect', async () => {
    const names = ['COMPUTER_USE_REQUIRE_APPROVAL', 'COMPUTER_USE_APPROVAL_TOKEN', 'COMPUTER_USE_AUDIT_LOG'];
    const previous = names.map(name => process.env[name]);
    process.env.COMPUTER_USE_REQUIRE_APPROVAL = 'true'; delete process.env.COMPUTER_USE_APPROVAL_TOKEN;
    process.env.COMPUTER_USE_AUDIT_LOG = 'false';
    const f = exitFixture();
    try {
        const completed = await f.completeEscape(), exit = f.stageExit(completed);
        const originalClaim = resolve(f.scenario, 'guard-E1-consumed.private.json');
        const originalOwnedSha256 = sha(f.auth.ownedReceiptPath), originalClaimSha256 = sha(originalClaim);
        const originalMenuSha256 = sha(exit.menuPath), historyBefore = readFileSync(f.historyPath);
        const auth = JSON.parse(readFileSync(exit.source, 'utf8'));
        const owned = JSON.parse(readFileSync(auth.ownedReceiptPath, 'utf8'));
        assert.equal(owned.processes[0].pid, 101);
        if (fault === 'changed-birth') {
            owned.processes[0].creationTime = new Date(Date.parse(owned.processes[0].creationTime) + 1).toISOString();
        } else owned.processes[0].executable = resolve(f.scenario, 'installation/jre/bin/replacement-java.exe');
        write(auth.ownedReceiptPath, owned); auth.ownedReceiptSha256 = sha(auth.ownedReceiptPath); write(exit.source, auth);
        assert.equal(sha(f.source), completed.originalE1SourceSha256);
        assert.equal(sha(f.auth.ownedReceiptPath), originalOwnedSha256);
        assert.equal(sha(originalClaim), originalClaimSha256); assert.equal(sha(exit.menuPath), originalMenuSha256);
        assert.throws(() => readBatchAuthorization(exit.source, { root: f.root, runID: exit.runID }), /initial owned identity/i);
        assert.ok(readFileSync(f.historyPath).equals(historyBefore));
        assert.equal(existsSync(resolve(f.scenario, 'guard-E2-consumed.private.json')), false);
        assert.deepEqual(f.keys, [['escape', 1]]); assert.deepEqual(f.moves, []); assert.deepEqual(f.clicks, []);
        assert.equal(f.history().history.reservedActions, 57); assert.equal(f.history().history.sentActions, 55);
    } finally {
        await f.close();
        names.forEach((name, i) => { if (previous[i] === undefined) delete process.env[name]; else process.env[name] = previous[i]; });
    }
});
