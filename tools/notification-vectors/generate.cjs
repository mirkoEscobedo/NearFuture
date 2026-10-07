'use strict';
const {readFileSync,writeFileSync} = require('node:fs');
const {resolve} = require('node:path');
const {corpus} = require('./corpus.cjs');
const {hash} = require('./fields.cjs');
const root=resolve(__dirname,'../..'),data=corpus();
const contract=readFileSync(resolve(root,data.contract.path),'utf8').replace(/\r\n/g,'\n');
if(hash(Buffer.from(contract,'utf8')).toString('hex')!==data.contract.sha256)throw Error('frozen notification contract changed');
const fields=['category','name','layer','expectation','hex','sha256'];
const outputs=[['docs/transport/vectors/notify-v1.json',JSON.stringify(data,null,2)+'\n'],
  ['docs/transport/vectors/notify-v1.tsv','# Independent NF-NOTIFY-1 data; categories are not production admission evidence\n'+fields.join('\t')+'\n'+
    data.rows.map(row=>fields.map(field=>row[field]).join('\t')).join('\n')+'\n']];
const args=process.argv.slice(2);
if(args.length>1 || args.length===1 && args[0]!=='--check')throw Error('closed fixture arguments');
for(const [name,text]of outputs) {
  const path=resolve(root,name);
  if(args.length===1) { if(readFileSync(path,'utf8')!==text)throw Error('notification fixture bytes differ'); }
  else writeFileSync(path,text);
}
process.stdout.write(args.length===1?'NOTIFICATION_VECTORS_OK\n':'NOTIFICATION_VECTORS_WRITTEN\n');
