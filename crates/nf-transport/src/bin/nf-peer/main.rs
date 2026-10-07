mod config;
use nf_contract::identity::RequestId;
use nf_identity::private_storage::PrivateVault;
use nf_store::Store;
use nf_transport::{
    PeerError,
    node::{PeerServer, PeerServerEvent, request_status},
    query::QueryResult,
    records::RetainedPhase,
};
use std::{io::Write, path::Path, time::Duration};
#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("NF_PEER_ERROR {e:?}");
        std::process::exit(1);
    }
}
async fn run() -> Result<(), PeerError> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.len() != 6 {
        return Err(PeerError::Malformed);
    }
    let mode = &args[0];
    if mode != "serve" && mode != "query" {
        return Err(PeerError::Malformed);
    }
    let vault = PrivateVault::open(Path::new(&args[1]), Path::new(&args[2]))
        .map_err(|_| PeerError::Storage)?;
    let cfg = config::Config::read(Path::new(&args[4]), mode == "query")?;
    let store =
        Store::open_existing(Path::new(&args[3]), cfg.known).map_err(|_| PeerError::Storage)?;
    if mode == "serve" {
        let millis = args[5].parse::<u64>().map_err(|_| PeerError::Malformed)?;
        if !(500..=60000).contains(&millis) {
            return Err(PeerError::Limit);
        }
        let mut server = PeerServer::new(store, &vault, cfg.policy)?;
        let deadline = tokio::time::sleep(Duration::from_millis(millis));
        tokio::pin!(deadline);
        loop {
            tokio::select! {_= &mut deadline=>break,event=server.next()=>if let PeerServerEvent::Ready{control,bulk,..}=event?{println!("NF_PEER_READY {control} {bulk}");std::io::stdout().flush().map_err(|_|PeerError::Storage)?;}}
        }
        println!("NF_PEER_STOPPED");
    } else {
        let (pin, address) = cfg.remote.ok_or(PeerError::Malformed)?;
        let request = RequestId::from_bytes(config::hex(&args[5])?);
        let result = request_status(store, &vault, cfg.policy, pin, address, request).await?;
        let code = match result {
            QueryResult::Status(RetainedPhase::UnknownRequest) => "UNKNOWN_REQUEST",
            QueryResult::Status(RetainedPhase::Pending { .. }) => "PENDING",
            QueryResult::Status(RetainedPhase::Rejected { .. }) => "REJECTED",
            QueryResult::Unsupported(_) => "UNSUPPORTED",
        };
        println!("NF_PEER_STATUS {code}");
    }
    Ok(())
}
