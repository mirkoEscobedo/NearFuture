// @ts-check
import { pathToFileURL } from 'node:url';
import { resolve } from 'node:path';
import { buildCaptureRuntime } from './build-runtime.mjs';
const runtime = buildCaptureRuntime();
export const {createComputerUseServer,serveComputerUseStdio}=await import(pathToFileURL(resolve(runtime,'dist/server.js')).href);
