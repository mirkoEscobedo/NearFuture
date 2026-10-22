// @ts-check
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';

const workflow = readFileSync(new URL('../../.github/workflows/synthetic.yml', import.meta.url), 'utf8');
const rust = workflow.match(/^  public-rust:\r?\n([\s\S]*?)(?=^  [a-z][\w-]*:)/m)?.[1] ?? '';

test('required Rust execution covers all three suites on both platforms without target filters', () => {
    assert.match(rust, /os: \[ubuntu-latest, windows-latest\]/);
    assert.match(rust, /suite: \[store, peers, remainder\]/);
    assert.match(rust, /fail-fast: false/);
    assert.match(rust, /timeout-minutes: 20/);
    assert.match(rust, /run: node tools\/delivery\/rust-partition\.mjs --suite \$\{\{ matrix\.suite \}\}/);
    assert.doesNotMatch(rust, /(?:^|\s)--(?:all-targets|test|lib|bin|doc|exclude|ignore|ignored)(?=\s|=|$)/m);
});

test('partitioning retains required groups, strict aggregate, and restore custody', () => {
    assert.match(workflow, /needs: \[public-build, public-rust, windows-interop, release-build, profile-smoke, operator-public\]/);
    assert.match(workflow, /headless-qualification:\r?\n[^\n]*\r?\n    if: always\(\)/);
    assert.match(rust, /NF_ISOLATED_RESTORE_REFUSAL_EVIDENCE_PARENT/);
    assert.match(rust, /Retain caller-owned full-suite restore refusal originals/);
    assert.match(workflow, /NF_SHADOW_NODE/g);
});
