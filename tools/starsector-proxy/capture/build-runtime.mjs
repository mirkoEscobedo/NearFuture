// @ts-check
import { createHash } from 'node:crypto';
import { readFileSync, existsSync, lstatSync, readdirSync, mkdirSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
const installed = fileURLToPath(new URL('../../computer-use-mcp/node_modules/@zavora-ai/computer-use-mcp/', import.meta.url));
export const captureRuntimeDirectory = fileURLToPath(new URL('../../computer-use-mcp/node_modules/@zavora-ai/near-future-capture-runtime-7.1.0/', import.meta.url));
const sha = (/** @type {Buffer|string} */ b) => createHash('sha256').update(b).digest('hex');
/** @param {string} path */
function ordinary(path) {
 for (let p=resolve(path);;p=dirname(p)) {
  if(lstatSync(p).isSymbolicLink()) throw Error('ORDINARY_CAPTURE_RUNTIME_REQUIRED');
  if(dirname(p)===p) break;
 }
}
/** @param {string} root */
function names(root) {
 ordinary(root);
 return readdirSync(root,{recursive:true,withFileTypes:true}).flatMap(entry=>{
  if(entry.isSymbolicLink()) throw Error('ORDINARY_CAPTURE_RUNTIME_REQUIRED');
  return entry.isFile() ? [resolve(entry.parentPath,entry.name).slice(resolve(root).length+1).replaceAll('\\','/')] : [];
 }).sort();
}
/** Build only the pinned, qualified vendor variant. Installed package is read-only.
 * Cache contents are generated dependencies, never source or approval authority. */
export function buildCaptureRuntime() {
 if(process.platform!=='win32'||process.arch!=='x64'||process.env.COMPUTER_USE_NATIVE_PATH)
  throw Error('QUALIFIED_WIN32_X64_DEFAULT_NATIVE_RUNTIME_REQUIRED');
 const manifest=JSON.parse(readFileSync(new URL('vendor-pins.json',import.meta.url),'utf8'));
 const patches=JSON.parse(readFileSync(new URL('vendor-patch.json',import.meta.url),'utf8'));
 const files=manifest.files;
 if(manifest.version!=='7.1.0'||sha(JSON.stringify(patches))!==manifest.patchSha256)
  throw Error('EXACT_CAPTURE_VENDOR_PATCH_REQUIRED');
 const dist=names(resolve(installed,'dist')).map(name=>'dist/'+name);
 if(JSON.stringify(dist)!==JSON.stringify(files.filter((/** @type {any} */ f)=>f.path.startsWith('dist/')).map((/** @type {any} */ f)=>f.path).sort()))
  throw Error('EXACT_INSTALLED_CAPTURE_DIST_REQUIRED');
 const outputs=new Map();
 for(const row of files){
  if(!/^(?:dist\/[A-Za-z0-9_.\/-]+|package.json|LICENSE|computer-use-napi.win32-x64.node)$/.test(row.path)||row.path.split('/').includes('..'))
   throw Error('CLOSED_CAPTURE_RUNTIME_PATH_REQUIRED');
  const path=resolve(installed,row.path);ordinary(path);
  let bytes=readFileSync(path);
  if(bytes.length!==row.bytes||sha(bytes)!==row.sha256)throw Error('INSTALLED_CAPTURE_RUNTIME_CHANGED');
  const patch=patches.find((/** @type {any} */ p)=>p.path===row.path);
  if(patch){
   let text=bytes.toString('utf8');
   for(const edit of patch.edits){
    if(!edit.before||text.split(edit.before).length!==2)throw Error('UNIQUE_CAPTURE_PATCH_ANCHOR_REQUIRED');
    text=text.replace(edit.before,edit.after);
   }
   bytes=Buffer.from(text);
   if(sha(bytes)!==patch.qualifiedSha256)throw Error('QUALIFIED_CAPTURE_PATCH_BYTES_REQUIRED');
  }
  outputs.set(row.path,bytes);
 }
 if(existsSync(captureRuntimeDirectory)){
  ordinary(captureRuntimeDirectory);
  if(JSON.stringify(names(captureRuntimeDirectory))!==JSON.stringify([...outputs.keys()].sort()))throw Error('CAPTURE_CACHE_INVENTORY_CHANGED');
  for(const [path,bytes] of outputs)if(sha(readFileSync(resolve(captureRuntimeDirectory,path)))!==sha(bytes))throw Error('CAPTURE_CACHE_BYTES_CHANGED');
  return captureRuntimeDirectory;
 }
 ordinary(dirname(captureRuntimeDirectory));
 mkdirSync(captureRuntimeDirectory);
 for(const [path,bytes] of outputs){
  const target=resolve(captureRuntimeDirectory,path);
  mkdirSync(dirname(target),{recursive:true});ordinary(dirname(target));
  writeFileSync(target,bytes,{flag:'wx'});
 }
 return captureRuntimeDirectory;
}
