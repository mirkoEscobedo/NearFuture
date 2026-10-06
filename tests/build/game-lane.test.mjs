// @ts-check
import test from 'node:test';
import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';

/** @param {string} [gameDir] */
function runUnavailableGameLane(gameDir = '') {
    const env = { ...process.env };
    delete env.STARSECTOR_HOME;
    if (gameDir) env.STARSECTOR_HOME = gameDir;
    const result = spawnSync(process.execPath, ['tools/build/java.mjs', 'game'], {
        env, encoding: 'utf8', timeout: 300_000,
    });
    assert.equal(result.error, undefined);
    assert.notEqual(result.status, 0, 'Unavailable integration must fail');
    assert.match(result.stdout + result.stderr, /Game integration unavailable:/);
    assert.match(result.stdout + result.stderr, /starfarer\.api\.jar/);
    assert.match(result.stdout + result.stderr, /[Nn]o game libraries are downloaded/);
}

test('game adapter requires an explicit licensed installation', () => {
    runUnavailableGameLane();
});

test('game adapter reports missing API libraries in an unavailable installation', () => {
    runUnavailableGameLane('.tmp/nonexistent-starsector-installation');
});
