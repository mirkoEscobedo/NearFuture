// @ts-check
import { loadCaptureConfiguration,captureFileCommand } from './capture-adapter.mjs';
const [mode,path,sha,input,...extra]=process.argv.slice(2);
if(extra.length || !input || Buffer.byteLength(input)>262144) throw Error('BOUNDED_CAPTURE_FILE_COMMAND_REQUIRED');
const configuration=loadCaptureConfiguration(path,sha);
console.log(JSON.stringify(captureFileCommand(mode,configuration,JSON.parse(input))));
