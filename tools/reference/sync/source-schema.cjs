'use strict';
const {readFileSync}=require('node:fs');
const {resolve}=require('node:path');
const {hash}=require('./fields.cjs');
const PROVENANCE={path:'crates/nf-store/src/schema.rs',source_lf_sha256:'b5a569b470920aa47712f4e525e9056720c22ae8ca24914b072797846435b40d',literal_lf_sha256:'7e06491204d7afa6935f76f769cb3fbf981c8b7c9c65100673ce3e3201236ebb',literal_bytes:2859};
function sourceSchema(){
  const root=resolve(__dirname,'../../..'),source=readFileSync(resolve(root,PROVENANCE.path),'utf8').replace(/\r\n/g,'\n');
  if(hash(Buffer.from(source)).toString('hex')!==PROVENANCE.source_lf_sha256)throw Error('accepted Store1 schema source changed');
  const literal=/pub\(crate\) const SCHEMA: &str = r#"([\s\S]*?)"#;/.exec(source);
  if(!literal)throw Error('closed Store1 literal absent');
  const raw=Buffer.from(literal[1]);
  if(raw.length!==PROVENANCE.literal_bytes||hash(raw).toString('hex')!==PROVENANCE.literal_lf_sha256)throw Error('Store1 literal provenance changed');
  return {digest:hash(raw),provenance:{...PROVENANCE}};
}
module.exports={sourceSchema,PROVENANCE};
