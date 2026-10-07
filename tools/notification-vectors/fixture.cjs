'use strict';
const {bytes,cat,key,publicKey} = require('./fields.cjs');
function identity(seed,account,device) {
  const privateKey=key(Buffer.from(seed,'hex')),publicBytes=publicKey(privateKey);
  return {seed,account:bytes(16,account),device:bytes(16,device),key:privateKey,public_key:publicBytes,
    peer:cat(Buffer.from('002408011220','hex'),publicBytes)};
}
function fixture() {
  const selected={body:1024,queue:16384,items:16,rate:8,burst:8,challenges:1};
  return {client:identity('9d61b19deffd5a60ba844af492ec2cc44449c5697b326919703bac031cae7f60',1,2),
    server:identity('4ccd089b28ff96da9db6c346ec114e0f5b8a319f35aba624da8cf6ed4fb8a6fb',3,4),
    session:bytes(16,5),universe:bytes(16,6),history:bytes(16,7),ruleset:bytes(32,8),content:bytes(32,9),
    client_nonce:bytes(32,10),server_nonce:bytes(32,11),request:bytes(16,12),operation:bytes(16,13),
    subscription:bytes(16,21),subscribe_nonce:bytes(32,14),server_subscribe_nonce:bytes(32,15),notice_nonce:bytes(32,22),
    binding:Buffer.from('2788a8743e8aecac5c03ca9402c34979f3f64667df33bf83100c4b5671030a93','hex'),
    membership:12n,minimum_membership:11n,lifetime:10,sequence:1n,offered:selected,server_limits:selected,selected};
}
module.exports={fixture};
