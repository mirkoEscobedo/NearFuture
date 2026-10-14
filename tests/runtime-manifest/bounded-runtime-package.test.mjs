// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { inspect } from 'node:util';
import { pathToFileURL } from 'node:url';
import { CaseClock } from './support/bounded-runtime/clock.mjs';
import { RuntimeCase } from './support/bounded-runtime-custody.mjs';
import { sha256 } from './support/bounded-runtime/io.mjs';
import { installFixture, verifyCheckout, CANARIES } from './support/bounded-runtime/fixture.mjs';
import { getterProjection, immutableValues } from './support/bounded-runtime/projection.mjs';
import { GOLDEN, DECLARATION } from './support/bounded-runtime/literals.mjs';

test('bounded_runtime_package_observer_reads_registered_public_bytes_and_keeps_declarations_unverified', async t => {
  const required = process.env.NF_RUNTIME_OBSERVER_REQUIRE_CASE === '1';
  const supplied = process.env.NF_RUNTIME_OBSERVER_EVIDENCE_ROOT;
  if (!required && !supplied) {
    t.skip('unqualified ordinary discovery: no explicit caller-owned evidence parent');
    return;
  }
  const clock = new CaseClock();
  /** @type {RuntimeCase | undefined} */
  let retained;
  try {
    if (!supplied) throw new Error('MISSING_CALLER_RUNTIME_EVIDENCE: setup, not behavior RED');
    // The real checkout API is imported first: absent module is intended RED.
    const publicURL = new URL('../../tools/runtime-manifest/observe-runtime-package.mjs',
      import.meta.url);
    const currentAPI = await clock.guard(() => import(publicURL.href));
    const names = ['RuntimeObservationError', 'observeRuntimePackage',
      'revalidateRuntimePackage'];
    assert.deepEqual(Object.keys(currentAPI).sort(), [...names].sort());
    for (const name of names) assert.equal(typeof currentAPI[name], 'function');

    const evidence = await RuntimeCase.create(clock);
    retained = evidence;
    const checkout = new URL('../../', import.meta.url);
    const originals = await installFixture(evidence, checkout);
    await evidence.put('control/golden.exact.json', Buffer.from(GOLDEN));
    await evidence.put('control/declaration.exact.json', Buffer.from(DECLARATION));
    assert.equal(Buffer.byteLength(GOLDEN), 1603);
    assert.equal(sha256(Buffer.from(GOLDEN)),
      'd3a36708434a1fcc0174930a624440da642d6deee4aa7c9360a76465f7675e6a');
    assert.equal(Buffer.byteLength(DECLARATION), 217);
    assert.equal(sha256(Buffer.from(DECLARATION)),
      'b9e9bb7927a78386de57ed358b9481ccae555be4dc61caba8281aa1fd7d03eec');
    const before = await evidence.snapshot('package');
    assert.equal(Object.values(before).filter(x => x.kind === 'file').length, 10);
    await evidence.record('capture/package.before.json', evidence.summary(before));

    // Only the two byte-identical real product modules execute, in one namespace.
    const fixtureURL = pathToFileURL(evidence.path(
      'package/tools/runtime-manifest/observe-runtime-package.mjs'));
    const fixtureAPI = await clock.guard(() => import(fixtureURL.href));
    assert.deepEqual(Object.keys(fixtureAPI).sort(), [...names].sort());
    await verifyCheckout(evidence, checkout, originals);
    const observation = await clock.guard(() =>
      fixtureAPI.observeRuntimePackage(clock.deadline, DECLARATION));
    assert.equal(observation.modules().length, 1);
    assert.equal(observation.artifacts().length, 3);
    assert.equal(observation.declaredRuntime().jvmFlags().length, 2);
    assert.equal(observation.limitations().length, 10);
    for (const value of immutableValues(observation))
      assert.ok(Object.isFrozen(value), 'closed DTO and every returned array frozen');
    const projected = getterProjection(observation);
    assert.equal(projected, GOLDEN);
    assert.equal(inspect(observation),
      'RuntimePackageObservationV1 { data_only: true, live_runtime: unavailable, certification: unknown }');
    assert.equal(observation.certification(), 'unknown');
    assert.equal(observation.liveRuntime(), 'unavailable');
    assert.equal(observation.embeddedSourceRevision(), null);
    assert.ok(!projected.includes(evidence.root));
    for (const [, canary] of CANARIES) {
      const markers = canary.match(/FAKE_[A-Z_]+/g) ?? [];
      for (const marker of markers) assert.ok(!projected.includes(marker));
    }
    await evidence.put('capture/runtime-getters.exact.json', Buffer.from(projected));
    await clock.guard(() =>
      fixtureAPI.revalidateRuntimePackage(observation, clock.deadline));
    assert.equal(getterProjection(observation), GOLDEN);
    await verifyCheckout(evidence, checkout, originals);
    const after = await evidence.snapshot('package');
    assert.deepEqual(after, before, 'entire observed package names, bytes and available metadata');
    await evidence.record('capture/package.after.json', evidence.summary(after));
    await evidence.record('capture/outcome.json', {
      version: 1, result: 'observed_registered_public_bytes_declarations_unverified',
      fullRuntimeQualified: false, fullACLQualified: false,
      liveRuntime: 'unavailable', certification: 'unknown',
      sourceCopyIdentityChecked: true, singleFixtureModuleBrand: true,
      noCaseChildren: 'source_audit_only', harnessDescendantZero: 'unverified',
      clock: 'one_absolute_20s_including_setup_capture_revalidation_cleanup',
    });
    const whole = await evidence.snapshot('', true);
    await evidence.record('capture/final-tree.json', {
      excludesThisReceipt: true, records: evidence.summary(whole),
    }, true);
    clock.check(true);
  } catch (error) {
    if (retained) {
      try {
        await retained.record('capture/failure.json', {
          version: 1, result: 'failed_no_qualification',
          originalDiagnostic: 'retained_in_complete_hosted_Node_test_log',
          fullACLQualified: false, harnessDescendantZero: 'unverified',
        }, true);
        const partial = await retained.snapshot('', true);
        await retained.record('capture/failure-tree.json', {
          excludesThisReceipt: true, records: retained.summary(partial),
        }, true);
      } catch {
        t.diagnostic('INSUFFICIENT_EVIDENCE: capture incomplete; owned originals retained');
      }
    }
    throw error;
  }
});
