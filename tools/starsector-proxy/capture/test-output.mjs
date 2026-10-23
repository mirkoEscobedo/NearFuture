// @ts-check
import { mkdirSync,mkdtempSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath,pathToFileURL } from 'node:url';
export const testOutputRoot=fileURLToPath(new URL('../../../.tmp/starsector-proxy/capture-tests/',import.meta.url));
mkdirSync(testOutputRoot,{recursive:true});
/** @param {string} prefix */export function makeOutput(prefix){return mkdtempSync(resolve(testOutputRoot,prefix));}
export function projectionInputRoot(){const value=process.env.NF_CAPTURE_PROJECTION_FIXTURE_ROOT;if(!value)throw Error('EXPLICIT_PINNED_PROJECTION_TEST_INPUT_REQUIRED');return pathToFileURL(resolve(value)+'/');}
