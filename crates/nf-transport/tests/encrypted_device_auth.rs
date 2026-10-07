#[path = "support/community.rs"]
mod community;
use community::Community;
use futures::StreamExt;
use libp2p::{
    request_response::{Event, Message},
    swarm::SwarmEvent,
};
use nf_identity::model::Roles;
use nf_transport::{
    auth::ServerPin,
    network::{PeerBehaviourEvent, build_lane_swarm},
    records::{Lane, PeerBody, PeerLimits},
    session::{ClientSession, ServerSession, SessionPolicy},
};
#[tokio::test(flavor = "current_thread")]
async fn actual_noise_connection_ids_drive_mutual_device_auth_on_separate_physical_lanes() {
    for lane in [Lane::Control, Lane::Bulk] {
        tokio::time::timeout(std::time::Duration::from_secs(5),async{
 let c=Community::new(Roles::PLAYER);let cp=c.client_noise.public().to_peer_id();let sp=c.server_noise.public().to_peer_id();let policy=SessionPolicy{scope:c.state.scope,ruleset:[4;32],content:[5;32],limits:PeerLimits::default(),minimum_membership:1};let pin=ServerPin{peer:sp,account:c.server.account,device:c.server.device,minimum_membership:1};let(mut client_auth,hello)=ClientSession::begin(c.client.clone(),cp,pin,lane,policy).unwrap();let mut server_auth=None;
 let mut client=build_lane_swarm(c.client_noise,lane).unwrap();let mut server=build_lane_swarm(c.server_noise,lane).unwrap();server.listen_on("/ip4/127.0.0.1/tcp/0".parse().unwrap()).unwrap();let addr=loop{if let SwarmEvent::NewListenAddr{address,..}=server.select_next_some().await{break address;}};client.dial(addr.with(libp2p::multiaddr::Protocol::P2p(sp))).unwrap();let mut sent=false;
 loop{tokio::select!{
 e=client.select_next_some()=>match e{SwarmEvent::ConnectionEstablished{peer_id,..}=>{assert_eq!(peer_id,sp);if !sent{client.behaviour_mut().messages.send_request(&sp,hello.clone());sent=true;}},SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::Message{peer,connection_id,message:Message::Response{response,..}}))=>match response.body{PeerBody::ServerHello{..}=>{let proof=client_auth.server_hello(response,peer,connection_id,&c.state,&c.client_device).unwrap();client.behaviour_mut().messages.send_request(&sp,proof);},PeerBody::Finished(_)=>{client_auth.finished(response,peer,connection_id,&c.state).unwrap();assert!(client_auth.active());assert!(server_auth.as_ref().is_some_and(ServerSession::active));break;},_=>panic!("unexpected body")},_=>{}},
 e=server.select_next_some()=>if let SwarmEvent::Behaviour(PeerBehaviourEvent::Messages(Event::Message{peer,connection_id,message:Message::Request{request,channel,..}}))=e{assert_eq!(peer,cp);let reply=match request.body{PeerBody::Hello{..}=>{let(auth,reply)=ServerSession::begin(request,peer,connection_id,c.server.clone(),sp,lane,policy,&c.state,&c.server_device).unwrap();server_auth=Some(auth);reply},PeerBody::ClientProof(_)=>server_auth.as_mut().unwrap().client_proof(request,peer,connection_id,&c.state,&c.server_device).unwrap(),_=>panic!("unexpected request")};server.behaviour_mut().messages.send_response(channel,reply).unwrap();}
 }}
 }).await.expect("mutual auth deadline");
    }
}
