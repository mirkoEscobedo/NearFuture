#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/scratch.rs"]
mod scratch;
use nf_contract::identity::RequestId;
use nf_transport::{
    auth::ServerPin,
    bulk::BulkResult,
    node::{PeerServer, PeerServerEvent, query_and_verify_bulk},
    query::QueryResult,
    records::{PeerLimits, RetainedPhase},
    session::SessionPolicy,
};
use sha2::{Digest, Sha256};
#[tokio::test(flavor = "current_thread")]
async fn sole_store_owner_drives_separate_physical_bulk_progress_and_control_query() {
    tokio::time::timeout(std::time::Duration::from_secs(15),async{let tmp=scratch::Scratch::new();let c=owned_community::OwnedCommunity::new(&tmp.0);let policy=SessionPolicy{scope:c.state.scope,ruleset:[4;32],content:[5;32],limits:PeerLimits::default(),minimum_membership:1};let pin=ServerPin{peer:libp2p::PeerId::from_bytes(&c.server.public.peer).unwrap(),account:c.server.public.account,device:c.server.public.device,minimum_membership:1};let mut server=PeerServer::new(c.server_store,&c.server_vault,policy).unwrap();let(control,bulk)=loop{if let PeerServerEvent::Ready{control,bulk,..}=server.next().await.unwrap(){break(control,bulk);}};assert_ne!(control,bulk);let bytes=vec![7u8;65536];let digest:[u8;32]=Sha256::digest(&bytes).into();let pair=query_and_verify_bulk(c.client_store,&c.client_vault,policy,pin,control,bulk,RequestId::from_bytes([9;16]),&bytes);tokio::pin!(pair);loop{tokio::select!{result=&mut pair=>{let(q,b)=result.unwrap();assert_eq!(q,QueryResult::Status(RetainedPhase::UnknownRequest));assert!(matches!(b,BulkResult::Verified{total:65536,digest:d,..}if d==digest));break;},e=server.next()=>{e.unwrap();}}}}).await.expect("owned two-lane deadline");
}
