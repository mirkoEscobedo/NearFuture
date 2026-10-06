// @ts-check
import { spawnSync } from 'node:child_process';

const mode = process.argv[2];
if (!['synthetic', 'game'].includes(mode ?? '')) {
    console.error('Usage: node tools/build/java.mjs <synthetic|game>');
    process.exit(2);
}
const task = mode === 'synthetic' ? 'syntheticCheck' : 'gameAdapter';
// Windows shell text is fixed. Local game paths are read by Gradle from STARSECTOR_HOME.
const command = process.platform === 'win32' ? 'cmd.exe' : 'sh';
const args = process.platform === 'win32'
    ? ['/d', '/s', '/c', `java\\gradlew.bat -p java ${task} --no-daemon`]
    : ['./java/gradlew', '-p', 'java', task, '--no-daemon'];
const result = spawnSync(command, args, { stdio: 'inherit', timeout: 300_000 });
if (result.error) {
    console.error(`Java build unavailable: ${result.error.message}`);
}
process.exit(result.status ?? 1);
