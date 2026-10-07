'use strict';
const {readFileSync,writeFileSync} = require('node:fs');
const {resolve} = require('node:path');
const {corpus} = require('./corpus.cjs');
const {hash} = require('./fields.cjs');
const root=resolve(__dirname,'../..'),data=corpus();
const contract=readFileSync(resolve(root,data.contract.path),'utf8').replace(/\r\n/g,'\n');
if(hash(Buffer.from(contract,'utf8')).toString('hex')!==data.contract.sha256)
  throw Error('frozen receipt contract changed');
const json=JSON.stringify(data,null,2)+'\n';
const fields=['category','name','layer','expectation','hex','sha256'];
const tsv='# Independent NF-PEER-2 receipt data; categories are not production admission evidence\n'+fields.join('\t')+'\n'+
  data.rows.map(row=>fields.map(field=>row[field]).join('\t')).join('\n')+'\n';
const outputs=[['docs/transport/vectors/receipt-v2.json',json],['docs/transport/vectors/receipt-v2.tsv',tsv]];
const args=process.argv.slice(2);
if(args.length>1 || args.length===1 && args[0]!=='--check')throw Error('closed fixture arguments');
for(const [name,text] of outputs) {
  const path=resolve(root,name);
  if(args.length===1) { if(readFileSync(path,'utf8')!==text)throw Error('receipt fixture bytes differ'); }
  else writeFileSync(path,text);
}
process.stdout.write(args.length===1?'RECEIPT_VECTORS_OK\n':'RECEIPT_VECTORS_WRITTEN\n');
