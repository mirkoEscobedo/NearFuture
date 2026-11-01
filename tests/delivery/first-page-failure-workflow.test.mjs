// @ts-check
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
const workflow = readFileSync(new URL('../../.github/workflows/synthetic.yml', import.meta.url), 'utf8');
const fixture = workflow.match(/^  windows-fixtures:\r?\n([\s\S]*?)(?=^  [a-z][\w-]*:)/m)?.[1] ?? '';

test('failed fixture jobs retain only the dedicated caller-owned synthetic evidence root', () => {
    assert.match(fixture, /near-future-first-page-failures-/);
    assert.match(fixture, /owned-public-first-page-child-evidence/);
    assert.match(fixture, /NF_FIRST_PAGE_CHILD_EVIDENCE_PARENT/);
    assert.match(fixture, /if:.*failure\(\).*NF_FIRST_PAGE_CHILD_EVIDENCE_PARENT/);
    assert.match(fixture, /uses: actions\/upload-artifact@ea165f8d65b6e75b540449e92b4886f43607fa02/);
    assert.match(fixture, /path: \$\{\{ env.NF_FIRST_PAGE_CHILD_EVIDENCE_PARENT \}\}/);
    assert.match(fixture, /if-no-files-found: error/);
    assert.doesNotMatch(fixture, /path:.*(?:\.tmp|\*\*)/);
    assert.doesNotMatch(fixture, /continue-on-error/);
});
