use futures::StreamExt;
use libp2p::{
    request_response::{Event, Message},
    swarm::SwarmEvent,
};
use nf_contract::identity::{AccountId, DeviceId, HistoryId, UniverseId};
use nf_identity::model::Scope;
use nf_transport::{
    network::{PeerBehaviourEvent, build_lane_swarm},
    records::{Lane, PeerBody, PeerContext, PeerLimits, PeerRecord},
};
#[tokio::test(flavor = "current_thread")]
async fn actual_noise_tcp_pair_roundtrips_closed_records_on_owned_loopback() {
    tokio::time::timeout(std::time::Duration::from_secs(5),async{
 let ck=libp2p::identity::Keypair::generate_ed25519();let sk=libp2p::identity::Keypair::generate_ed25519();let cp=ck.public().to_peer_id();let sp=sk.public().to_peer_id();
 let mut client=build_lane_swarm(ck,Lane::Control).unwrap();let mut server=build_lane_swarm(sk,Lane::Control).unwrap();server.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();let addr=loop{if let SwarmEvent::NewListenAddr{address,..}=server.select_next_some().await{break address;}};client.dial(addr.with(libp2p::multiaddr::Protocol::P2p(sp))).unwrap();
 let record=PeerRecord{context:PeerContext{session:[0;16],scope:Scope{universe:UniverseId::from_bytes([1;16]),history:HistoryId::from_bytes([2;16])},ruleset:[3;32],content:[4;32]},body:PeerBody::Hello{account:AccountId::from_bytes([5;16]),device:DeviceId::from_bytes([6;16]),nonce:[7;32],required:1,optional:2,offered:PeerLimits::default()}};
 let mut sent=false;loop{tokio::select!{
 e=client.select_next_some()=>match e{SwarmEvent::ConnectionEstablished{peer_id,..}=>{assert_eq!(peer_id,sp);if !sent{client.behaviour_mut().messages.send_request(&sp,record.clone());sent=true;}},SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::Message{peer,message:Message::Response{response,..},..}))=>{assert_eq!(peer,sp);assert_eq!(response,record);break;},_=>{}},
 e=server.select_next_some()=>if let SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::Message{peer,message:Message::Request{request,channel,..},..}))=e{assert_eq!(peer,cp);assert_eq!(request,record);server.behaviour_mut().messages.send_response(channel,request).unwrap();}
 }}
 }).await.expect("owned encrypted pair deadline");
}
