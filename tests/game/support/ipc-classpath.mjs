// @ts-check
import {existsSync, readFileSync} from 'node:fs';
import {delimiter} from 'node:path';
/** Explicit Gradle prerequisite: :ipc:exportAuthTestClasspath. No downloads or game deployment. */
export function ipcClasspath() {
  const value = readFileSync(new URL('../../../java/ipc/build/auth-test-classpath.txt', import.meta.url), 'utf8').trim();
  if (value.length === 0 || value.length > 65535) throw new Error('Prepared IPC test classpath required');
  return value.split(delimiter).filter(path => {
    if (existsSync(path)) return true;
    if (/[\\/]ipc[\\/]build[\\/]resources[\\/](main|test)$/.test(path)) return false; // Gradle NO-SOURCE roots.
    throw new Error('Prepared IPC classpath entry is missing');
  }).join(delimiter);
}
