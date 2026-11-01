// @ts-check
import assert from 'node:assert/strict';

/** @typedef {{status:number|null,signal:string|null,pid:number,error?:Error,stdout:string|null|undefined,stderr:string|null|undefined}} PowerShellResult */
/** @typedef {Readonly<{args:readonly string[],status:null,signal:string|null,pid:number,error:Readonly<{name:string,message:string,code:string|null}>|null,stdout:string|null|undefined,stderr:string|null|undefined}>} IncompletePowerShellResult */

/** The observer cannot weaken the original terminal-status assertion.
 * @param {PowerShellResult} result @param {string[]} args
 * @param {(value:IncompletePowerShellResult)=>void} [observeIncomplete] */
export function assertTerminalPowerShellResult(result, args, observeIncomplete) {
    const status = result.status;
    const message = 'Fixture PowerShell requires terminal status; error=' +
        (result.error?.message ?? 'NONE') + '; signal=' + (result.signal ?? 'NONE');
    let captureRefused = false;
    if (status === null && observeIncomplete) {
        try {
            if (![result.stdout, result.stderr].every(value => value === undefined || value === null || typeof value === 'string') ||
                !(result.signal === null || typeof result.signal === 'string')) {
                throw Error('Incomplete result requires original output values');
            }
            const error = result.error ? Object.freeze({ name: result.error.name, message: result.error.message,
                code: 'code' in result.error && typeof result.error.code === 'string' ? result.error.code : null }) : null;
            observeIncomplete(Object.freeze({ args: Object.freeze([...args]), status: null, signal: result.signal,
                pid: result.pid, error, stdout: result.stdout, stderr: result.stderr }));
        } catch { captureRefused = true; }
    }
    assert.notEqual(status, null, message + (captureRefused ? '; failure evidence capture refused' : ''));
}
