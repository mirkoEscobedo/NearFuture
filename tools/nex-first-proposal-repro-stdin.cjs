'use strict';
// This zero-argument adapter is only for the independently reviewed public synthetic test fixture.
// The Synthetic marker alone is not a privacy classification. No wire-supplied path is accepted.
if (process.argv.length !== 2) throw new Error('NO_ADAPTER_ARGUMENTS');
const fs = require('node:fs');
const { persistSyntheticReproduction } = require('./nex-first-proposal-repro.cjs');
// A fixed cap is allocated before input admission; never read an unbounded stream into memory.
const buffer = Buffer.alloc(65537);
let length = 0;
while (length < buffer.length) {
    const count = fs.readSync(0, buffer, length, buffer.length - length, null);
    if (count === 0) break;
    length += count;
}
if (length > 65536) throw new Error('BOUNDED_PUBLIC_FIXTURE_REQUIRED');
const result = persistSyntheticReproduction(buffer.subarray(0, length), { reviewedPublicSynthetic: true });
process.stdout.write(JSON.stringify(result) + '\n');
