'use strict';
const {bytes,hash}=require('./fields.cjs');
const {identity,CLIENT_PUBLIC_RFC_SEED,SERVER_PUBLIC_RFC_SEED}=require('./identity.cjs');
const {sourceSchema}=require('./source-schema.cjs');
function fixture(){
  const source=sourceSchema(),limits={control_frame:1024,transfer_frame:9216,chunk_bytes:8192,control_queue_bytes:16384,transfer_queue_bytes:32768,control_items:4,transfer_items:2,pending_challenges:1};
  const document=bytes(391,0x28);
  return {client:identity(CLIENT_PUBLIC_RFC_SEED,1,2),server:identity(SERVER_PUBLIC_RFC_SEED,3,4),
    session:bytes(16,5),sessions:{1:bytes(16,5),2:bytes(16,29)},universe:bytes(16,6),history:bytes(16,7),ruleset:bytes(32,8),content:bytes(32,9),
    client_nonce:bytes(32,10),server_nonce:bytes(32,11),required:1,optional:0,available:1,selected:1,
    offered:{...limits},server_limits:{...limits},selected_limits:{...limits},membership:12n,membership_digest:bytes(32,13),
    implementation:bytes(32,0x30),schema:source.digest,source_provenance:source.provenance,
    request:bytes(16,14),export_id:bytes(16,15),transfer_id:bytes(16,16),
    sync_client_nonce:bytes(32,17),sync_server_nonce:bytes(32,18),document_client_nonce:bytes(32,30),document_server_nonce:bytes(32,31),completion_nonce:bytes(32,19),
    minimum_event:90n,minimum_store:190n,minimum_membership:11n,
    base:{store:199n,event:99n,world:bytes(32,20),state:bytes(32,21),head:bytes(32,22)},
    target:{store:200n,event:100n,world:bytes(32,23),state:bytes(32,24),head:bytes(32,25)},
    current:{store:201n,event:100n,world:bytes(32,23),state:bytes(32,26),head:bytes(32,27)},
    manifest_digest:bytes(32,28),manifest_length:620,document,document_digest:hash(document),
    document_count:1,total_data:BigInt(document.length),activation_generation:2n};
}
module.exports={fixture};
