// @ts-check
import {readFileSync,writeFileSync} from 'node:fs';
import {createHash} from 'node:crypto';
import {Client} from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/index.mjs';
import {StdioClientTransport,getDefaultEnvironment} from '../../computer-use-mcp/node_modules/@modelcontextprotocol/client/dist/stdio.mjs';
const [path,expectedHash,...extra]=process.argv.slice(2),bytes=readFileSync(path);
if(extra.length||bytes.length>65536||!/^[a-f0-9]{64}$/.test(expectedHash??'')||createHash('sha256').update(bytes).digest('hex')!==expectedHash)throw Error('EXACT_OBSERVATION_INPUT_REQUIRED');
const p=JSON.parse(bytes.toString('utf8'));
const actual={schemaVersion:1,kind:'NATIVE99_CAPTURE_ADAPTER_OBSERVATION_ONLY_ACTUAL_V1',startMs:Date.now(),calls:/** @type {any[]} */([]),catalog:/** @type {any} */(null),
    serverPid:/** @type {number|null} */(null),serverStderr:'',serverStderrOverflow:false,closeReturned:false,serverPidAbsent:false,nativeInputSent:false,gameLaunched:false,wholeNativeAcceptance:false};
writeFileSync(p.actualPath,JSON.stringify(actual)+'\n',{flag:'wx'});
const transport=new StdioClientTransport({command:p.node,args:[p.entrypoint,p.configuration.path,p.configuration.sha256],
    env:{...getDefaultEnvironment(),...p.environment},cwd:p.workspaceRoot,stderr:'pipe'});
transport.stderr?.on('data',chunk=>{
    if(actual.serverStderrOverflow)return;
    const text=String(chunk),remaining=65536-actual.serverStderr.length;
    actual.serverStderr+=text.slice(0,remaining);
    if(text.length>remaining){
        actual.serverStderrOverflow=true;actual.serverPid??=transport.pid;
        failure??=Error('BOUNDED_PROVIDER_STDERR_REQUIRED');
        void close().catch(error=>{failure??=error;});
    }
});
const client=new Client({name:'native99-capture-adapter-observation-only',version:'1'},
    {capabilities:{},supportedProtocolVersions:['2025-11-25']});
/** @type {any} */let failure;
/** @type {Promise<void>|undefined} */let closing;
function close(){return closing??=client.close();}
try{
    await client.connect(transport);actual.serverPid=transport.pid;
    if(failure)throw failure;
    actual.catalog=await client.listTools();
    for(const name of ['policy_status','list_windows']){
        if(failure)throw failure;
        const record={name,args:{},startMs:Date.now(),raw:/** @type {any} */(null),endMs:/** @type {number|null} */(null)};
        actual.calls.push(record);writeFileSync(p.actualPath,JSON.stringify(actual)+'\n');
        record.raw=await client.callTool({name,arguments:{}});record.endMs=Date.now();
        writeFileSync(p.actualPath,JSON.stringify(actual)+'\n');
        if(record.raw?.isError===true)throw Error('OBSERVATION_TOOL_REFUSED:'+name);
    }
}catch(error){failure??=error;}
finally{
    actual.serverPid??=transport.pid;
    try{await close();actual.closeReturned=true;}catch(error){failure??=error;}
    if(actual.serverPid){try{process.kill(actual.serverPid,0);}catch(error){if(/** @type {NodeJS.ErrnoException} */(error).code==='ESRCH')actual.serverPidAbsent=true;else failure??=error;}}
    const receipt={...actual,endMs:Date.now(),status:!failure&&actual.closeReturned&&actual.serverPidAbsent?'CATALOG_POLICY_WINDOWS_COMPLETED_ONLY':'HOLD'};
    writeFileSync(p.actualPath,JSON.stringify(receipt)+'\n');
}
if(failure)throw failure;
if(!actual.serverPidAbsent)throw Error('OWNED_SERVER_PID_STILL_PRESENT');
console.log(JSON.stringify({serverPid:actual.serverPid,serverPidAbsent:actual.serverPidAbsent,
    toolCalls:actual.calls.map(r=>r.name),catalogCount:actual.catalog?.tools?.length,nativeInputSent:false,
    gameLaunched:false,wholeNativeAcceptance:false,actualPath:p.actualPath}));
