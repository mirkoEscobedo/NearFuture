#[path = "support/owned_community.rs"]
mod owned_community;
#[path = "support/scratch.rs"]
mod scratch;
use nf_contract::identity::RequestId;
use nf_transport::{
    auth::ServerPin,
    node::{PeerServer, PeerServerEvent, request_status},
    query::QueryResult,
    records::{PeerLimits, RetainedPhase},
    session::SessionPolicy,
};
use owned_community::OwnedCommunity;
#[tokio::test(flavor = "current_thread")]
async fn owned_foreground_peer_drivers_authenticate_and_query_actual_store_over_noise() {
    tokio::time::timeout(std::time::Duration::from_secs(10),async{let tmp=scratch::Scratch::new();let c=OwnedCommunity::new(&tmp.0);let policy=SessionPolicy{scope:c.state.scope,ruleset:[4;32],content:[5;32],limits:PeerLimits::default(),minimum_membership:1};let pin=ServerPin{peer:libp2p::PeerId::from_bytes(&c.server.public.peer).unwrap(),account:c.server.public.account,device:c.server.public.device,minimum_membership:1};let mut server=PeerServer::new(c.server_store,&c.server_vault,policy).unwrap();let addr=loop{if let PeerServerEvent::Ready{control,..}=server.next().await.unwrap(){break control;}};let client=request_status(c.client_store,&c.client_vault,policy,pin,addr,RequestId::from_bytes([9;16]));tokio::pin!(client);loop{tokio::select!{result=&mut client=>{assert_eq!(result.unwrap(),QueryResult::Status(RetainedPhase::UnknownRequest));break;},e=server.next()=>{e.unwrap();}}}}).await.expect("owned peer driver deadline");
}
