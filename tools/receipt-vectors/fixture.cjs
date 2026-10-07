'use strict';
const {bytes,cat,key,publicKey,hash} = require('./fields.cjs');
const {binding,intent} = require('./profiles.cjs');
const PUBLIC_CLIENT_SEED = '9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60';
const PUBLIC_SERVER_SEED = '4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb';
function identity(seed,account,device) {
  const privateKey=key(Buffer.from(seed,'hex')),publicBytes=publicKey(privateKey);
  return {seed,account:bytes(16,account),device:bytes(16,device),key:privateKey,public_key:publicBytes,
    peer:cat(Buffer.from('002408011220','hex'),publicBytes)};
}
function fixture() {
  const limit={control_frame:4096,bulk_frame:9216,chunk:8192,control_queue:65536,bulk_queue:131072,control_items:16,bulk_items:4,challenges:8};
  const c={client:identity(PUBLIC_CLIENT_SEED,1,2),server:identity(PUBLIC_SERVER_SEED,3,4),
    session:bytes(16,5),universe:bytes(16,6),history:bytes(16,7),ruleset:bytes(32,8),content:bytes(32,9),
    client_nonce:bytes(32,10),server_nonce:bytes(32,11),request:bytes(16,12),operation:bytes(16,13),
    operation_nonce:bytes(32,14),server_operation_nonce:bytes(32,15),job:bytes(16,16),provider:bytes(16,17),
    component:bytes(16,18),provider_aggregate:bytes(16,19),market:bytes(16,20),operation_kind:3,
    membership:12n,minimum_membership:11n,minimum_event:90n,minimum_store:190n,source_event:100n,source_store:200n,
    required:1,optional:0x80000000,available:1,selected:1,offered:limit,server_limits:limit,selected_limits:limit};
  c.intent=intent(c);c.payload_digest=hash(c.intent);c.binding=hash(binding(c));
  return c;
}
module.exports = {fixture,PUBLIC_CLIENT_SEED,PUBLIC_SERVER_SEED};
