// @ts-check
import { loadCaptureConfiguration,serveCaptureStdio } from './capture-adapter.mjs';
const [path,sha,...extra]=process.argv.slice(2);
if(extra.length || !path || !sha) throw Error('PINNED_CAPTURE_CONFIGURATION_ARGUMENTS_REQUIRED');
const handle=serveCaptureStdio({configuration:loadCaptureConfiguration(path,sha)});
console.error('[computer-use-mcp] Server running with exact caller-intent original capture custody');
const shutdown=async()=>{await handle.close();};
process.once('SIGINT',()=>{void shutdown().finally(()=>process.exit(0));});
process.once('SIGTERM',()=>{void shutdown().finally(()=>process.exit(0));});
