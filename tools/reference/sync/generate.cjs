'use strict';
const {readFileSync,writeFileSync}=require('node:fs');
const {resolve}=require('node:path');
const {corpus}=require('./corpus.cjs');
const {hash}=require('./fields.cjs');
const root=resolve(__dirname,'../../..'),data=corpus(),args=process.argv.slice(2);
if(args.length>1||(args.length===1&&args[0]!=='--check'))throw Error('closed sync fixture arguments');
const contract=readFileSync(resolve(root,data.contract.path),'utf8').replace(/\r\n/g,'\n');
if(hash(Buffer.from(contract)).toString('hex')!==data.contract.sha256)throw Error('frozen sync contract changed');
const fields=['category','name','layer','expectation','hex','sha256','kind','lane','selected_chunk','selected_transfer_frame'];
const outputs=[['docs/transport/sync-vectors/sync-v1.json',JSON.stringify(data,null,2)+'\n'],
  ['docs/transport/sync-vectors/sync-v1.tsv','# Independent NF-SYNC-1 data; no runtime/semantic/authority verdict\n'+fields.join('\t')+'\n'+data.rows.map(r=>fields.map(f=>r[f]??'').join('\t')).join('\n')+'\n']];
for(const[name,text]of outputs){const path=resolve(root,name);if(args.length){if(readFileSync(path,'utf8')!==text)throw Error('sync fixture bytes differ');}else writeFileSync(path,text);}
process.stdout.write(args.length?'SYNC_VECTORS_OK\n':'SYNC_VECTORS_WRITTEN\n');
