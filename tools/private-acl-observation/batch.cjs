"use strict";
const fs = require("fs"), cp = require("child_process"), path = require("path"), crypto = require("crypto");
if (process.argv.length !== 2 || process.platform !== "win32" || process.versions.node !== "24.11.0") throw Error("WINDOWS_ZERO_ARGS");
const workspace = path.resolve(__dirname, "../.."), output = path.join(workspace, ".tmp/private-acl-observation");
const fixture = path.join(output, "fixture"), resultPath = path.join(output, "observation.json");
const sha = p => crypto.createHash("sha256").update(fs.readFileSync(p)).digest("hex");
const canonicalScriptSha = p => crypto.createHash("sha256").update(fs.readFileSync(p,"utf8").replaceAll("\r\n","\n")).digest("hex");
let rawBefore;
const sourcePins = [
 ["crates/nf-identity/src/private-acl.ps1", "278da1aa88d8c0082c02ba8d8279953a1e67158bffea131c1e9c16f1d6fcfd92"],
 ["crates/nf-identity/src/private_diagnostics/access.rs", "f4da957a6fd15f558c27526db40f94b602dce3f496aa7656bda99c8d3ddffdd2"],
 ["crates/nf-identity/src/owned_process.rs", "e9baf15da9016b619353afdfa014beff075d14e748b4acd5091604fe2441fbbc"]
];
const tokens = ["DEBUGGER_SETUP_BEGIN","DEBUGGER_SETUP_READY","IMPORT_BEGIN","IMPORT_RETURN","GETITEM_BEGIN","GETITEM_RETURN","SID_BEGIN","SID_RETURN","DIRECTORY_ACL_NEW","SETOWNER_BEGIN","SETOWNER_RETURN","SETACL_BEGIN","SETACL_RETURN","GETACL_BEGIN","GETACL_RETURN","OWNER_CHECK_BEGIN","OWNER_CHECK_RETURN","RULES_BEGIN","RULES_RETURN","ACCEPT_EXIT"].map(t => "NF_ACL_DIAG|" + t);
function guard() {
 for (const [p,h] of sourcePins) {
  const absolute=path.join(workspace,p);
  const actual=p.endsWith(".ps1")?canonicalScriptSha(absolute):sha(absolute);
  if(actual!==h)throw Error("SOURCE_PIN");
  if(rawBefore&&sha(absolute)!==rawBefore.get(p))throw Error("RAW_SOURCE_CHANGED");
 }
 for (const p of [workspace,path.dirname(output),output]) {
  const s=fs.lstatSync(p); if(!s.isDirectory()||s.isSymbolicLink()) throw Error("OUTPUT_NO_REPARSE");
 }
}
async function main() {
 guard();
 rawBefore=new Map(sourcePins.map(([p])=>[p,sha(path.join(workspace,p))]));
 if (fs.existsSync(fixture) || fs.existsSync(resultPath)) throw Error("FRESH_OUTPUT_REQUIRED");
 fs.mkdirSync(fixture);
 const r={schema:1,status:"UNQUALIFIED",node:process.versions.node,arch:process.arch,helperDeadlineMs:5000,stdoutCap:4096,stderrCap:4096,
  markers:[],wrapperErrors:[],stdoutBytes:0,stderrBytes:0,spawned:false,reaped:false,exit:null,signal:null,reason:null,
  productionSources:sourcePins.map(([p,h])=>({path:p,canonicalSha256:h,rawSha256:rawBefore.get(p)})),hostedCauseQualified:false};
 const processStart=process.hrtime.bigint(); let childStart=null,timer,pending=Buffer.alloc(0);
 const child=cp.spawn("C:/Windows/System32/WindowsPowerShell/v1.0/powershell.exe",
 ["-NoLogo","-NoProfile","-NonInteractive","-ExecutionPolicy","Bypass","-File",path.join(__dirname,"observe.ps1")],
 {cwd:workspace,windowsHide:true,stdio:["ignore","pipe","pipe"]});
 await new Promise(resolve=>{
  function stop(reason){if(!r.reason){r.reason=reason;child.kill();}}
  child.once("spawn",()=>{r.spawned=true;childStart=process.hrtime.bigint();timer=setTimeout(()=>stop("Original5sDeadline"),5000);});
  child.once("error",e=>{r.reason="Spawn:"+ (e.code||"Unknown");});
  child.stdout.on("data",b=>{
   r.stdoutBytes+=b.length;if(r.stdoutBytes>4096){stop("StdoutCap4096");return;}
   pending=Buffer.concat([pending,b]);
   for(;;){
    const n=pending.indexOf(10);if(n<0)break;
    let line=pending.subarray(0,n).toString("ascii");pending=pending.subarray(n+1);if(line.endsWith("\r"))line=line.slice(0,-1);
    const error=/^NF_ACL_ERROR\|(PlatformPin|PlatformImport|WrapperCommands|SourcePin|FixtureExists|FixtureMetadata|DebuggerSetup|HelperInvocation)\|([A-Za-z0-9_.]{1,128})\|([A-Za-z0-9_.,-]{1,128})$/.exec(line);
    if(error){r.wrapperErrors.push({phase:error[1],exceptionType:error[2],fullyQualifiedErrorId:error[3]});continue;}
    if(!tokens.includes(line)){stop("UnexpectedStdoutToken");continue;}
    r.markers.push({token:line,arrivalMs:Number(process.hrtime.bigint()-childStart)/1000000});
   }
  });
  child.stderr.on("data",b=>{r.stderrBytes+=b.length;stop(r.stderrBytes>4096?"StderrCap4096":"UnexpectedStderr");});
  child.once("exit",(code,signal)=>{clearTimeout(timer);r.exit=code;r.signal=signal;});
  child.once("close",()=>{clearTimeout(timer);r.reaped=true;resolve();});
 });
 r.elapsedMs=Number(process.hrtime.bigint()-(childStart||processStart))/1000000;
 if(pending.length)r.reason ||= "IncompleteMarkerLine";
 r.lastPhase=r.markers.at(-1)||null;
 const all=JSON.stringify(r.markers.map(m=>m.token))===JSON.stringify(tokens);
 r.status=r.spawned&&r.reaped&&!r.reason&&!r.signal&&r.exit===0&&all?"HOSTED_PLUMBING_PASS":"STOP_FIRST_OBSERVED_FAILURE";
 try{guard();r.sourceGuardAfter=true;}catch{r.sourceGuardAfter=false;r.status="SOURCE_CUSTODY_FAILURE";}
 fs.writeFileSync(resultPath,JSON.stringify(r,null,2)+"\n",{flag:"wx"});
 console.log(JSON.stringify(r));
 if(r.status!=="HOSTED_PLUMBING_PASS")process.exitCode=1;
}
main().catch(()=>{console.log(JSON.stringify({status:"SETUP_FAILURE",hostedCauseQualified:false}));process.exitCode=1;});
