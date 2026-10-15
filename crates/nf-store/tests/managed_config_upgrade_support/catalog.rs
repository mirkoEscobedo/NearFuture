//! Fixture-specific closed catalog row-image/delta replay, independent of managed encoders.
use super::{cursor::Cursor, observations::{readonly_public, Rows, rows_hash},
    records::{hash, proof_bytes}, transcripts::{stamp, Stamp}};
use nf_identity::model::{DeviceProof, MembershipState};
use rusqlite::types::Value;
use std::{collections::BTreeMap, path::Path};
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct Registry { pub kind:u32,pub id:[u8;16],pub path:String,pub bytes:u64,
    pub digest:[u8;32],pub manifest:Vec<u8> }
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct Receipt { pub sequence:u64,pub operation:[u8;16],pub bytes:Vec<u8>,pub digest:[u8;32] }
fn registry(c:&mut Cursor<'_>)->Registry {
    Registry { kind:c.u32(),id:c.take(16).try_into().unwrap(),
        path:String::from_utf8(c.blob(4096).to_vec()).unwrap(),bytes:c.u64(),
        digest:c.take(32).try_into().unwrap(),manifest:c.blob(8192).to_vec() }
}
fn receipt(c:&mut Cursor<'_>)->Receipt {
    let r=Receipt{sequence:c.u64(),operation:c.take(16).try_into().unwrap(),
        bytes:c.blob(4096).to_vec(),digest:c.take(32).try_into().unwrap()};
    assert_eq!(hash(&r.bytes),r.digest);r
}
fn opt_registry(c:&mut Cursor<'_>)->Option<Registry>{
    match c.byte(){0=>None,1=>Some(registry(c)),_=>panic!("registry Option")}
}
fn opt_receipt(c:&mut Cursor<'_>)->Option<Receipt>{
    match c.byte(){0=>None,1=>Some(receipt(c)),_=>panic!("receipt Option")}
}
pub fn row_image(bytes:&[u8])->(BTreeMap<u32,Registry>,BTreeMap<u64,Receipt>){
    let mut c=Cursor::new(bytes,b"NF-OWNER-CATALOG-ROWS-1\0",1_179_648);
    let n=c.u32();assert!(n<=5);let mut rs=BTreeMap::new();
    let mut previous=0;
    for _ in 0..n {let r=registry(&mut c);assert!((1..=5).contains(&r.kind));
        assert!(r.kind>previous);previous=r.kind;assert!(rs.insert(r.kind,r).is_none());}
    let n=c.u32();assert!(n<=256);let mut receipts=BTreeMap::new();
    let mut previous=None;
    for _ in 0..n {let r=receipt(&mut c);assert!(r.sequence>0);
        assert!(previous.is_none_or(|p|p<r.sequence));previous=Some(r.sequence);
        assert!(receipts.insert(r.sequence,r).is_none());}
    c.finish();(rs,receipts)
}
pub fn apply_delta(bytes:&[u8],rs:&mut BTreeMap<u32,Registry>,receipts:&mut BTreeMap<u64,Receipt>){
    let mut c=Cursor::new(bytes,b"NF-OWNER-ROW-DELTA-1\0",131_072);
    let n=c.u32();assert!(n<=5);let mut previous=0;
    for _ in 0..n {
        let key=c.u32();assert!(key>previous);previous=key;
        let before=opt_registry(&mut c);let after=opt_registry(&mut c);
        assert_eq!(rs.get(&key),before.as_ref());
        // Selected registry is append-only; no existing artifact replacement/deletion.
        assert!(before.is_none());let r=after.unwrap();assert_eq!(r.kind,key);
        assert!(rs.insert(key,r).is_none());
    }
    let n=c.u32();assert!(n<=2);let mut previous=None;
    for _ in 0..n {
        let key=c.u64();assert!(previous.is_none_or(|p|p<key));previous=Some(key);
        let before=opt_receipt(&mut c);let after=opt_receipt(&mut c);
        assert_eq!(receipts.get(&key),before.as_ref());
        if let Some(r)=after {assert_eq!(r.sequence,key);assert!(before.is_none());receipts.insert(key,r);}
        else {assert_eq!(Some(&key),receipts.first_key_value().map(|(k,_)|k));receipts.remove(&key);}
    }
    assert!(receipts.len()<=256);c.finish();
}
// This fixture resolves at most three operations: no selected 257th-row eviction applies.
pub fn assert_fixture_delta(bytes:&[u8],kind:u32,expected:Option<&Receipt>){
    let mut c=Cursor::new(bytes,b"NF-OWNER-ROW-DELTA-1\0",131_072);
    let n=c.u32();assert!(n<=5);
    if kind==4{assert_eq!(n,0);}
    let mut previous=0;
    for _ in 0..n{
        let key=c.u32();assert!((1..=5).contains(&key)&&key>previous);previous=key;
        let _before=opt_registry(&mut c);let _after=opt_registry(&mut c);
    }
    let n=c.u32();
    if let Some(row)=expected{
        assert_eq!(kind,4);assert_eq!(n,1);assert_eq!(c.u64(),row.sequence);
        assert!(opt_receipt(&mut c).is_none());
        assert_eq!(opt_receipt(&mut c).as_ref(),Some(row));
    }else{assert_ne!(kind,4);assert_eq!(n,0);}
    c.finish();
}
#[derive(Clone,Debug,Eq,PartialEq)]
pub struct State {
    pub installation:[u8;16],pub scope:[u8;32],pub phase:u32,pub generation:u64,
    pub term:u64,pub session:u64,pub owner:Option<[u8;48]>,pub sequence:u64,
    pub selected:Option<[u8;16]>,pub source:[u8;16],pub foot:Option<Vec<u8>>,
    pub original:Vec<u8>,pub planned:Option<Vec<u8>>,pub retirement:Option<[u8;32]>,
    pub pending:Option<[u8;32]>,pub registry:[u8;32],pub receipts:[u8;32], pub utc:u64,
}
fn opt_blob(c:&mut Cursor<'_>,cap:usize)->Option<Vec<u8>>{
    match c.byte(){0=>None,1=>Some(c.blob(cap).to_vec()),_=>panic!("blob Option")}
}
pub fn state(bytes:&[u8])->State{
    let mut c=Cursor::new(bytes,b"NF-OWNER-CATALOG-STATE-1\0",16_384);
    let installation=c.take(16).try_into().unwrap();let scope=c.take(32).try_into().unwrap();
    let phase=c.u32();assert!((1..=8).contains(&phase));
    let generation=c.u64();let term=c.u64();let session=c.u64();
    let owner=c.option(48).map(|v|v.try_into().unwrap());
    let sequence=c.u64();let selected=c.option(16).map(|v|v.try_into().unwrap());
    let source=c.take(16).try_into().unwrap();let foot=opt_blob(&mut c,2048);
    let original=c.blob(2048).to_vec();let planned=opt_blob(&mut c,2048);
    let retirement=c.option(32).map(|v|v.try_into().unwrap());
    let pending=c.option(32).map(|v|v.try_into().unwrap());
    // Source policy is explicitly retained, although this first drained fixture never has one.
    assert!(c.option(17).is_none());let utc=c.u64();
    let registry=c.take(32).try_into().unwrap();let receipts=c.take(32).try_into().unwrap();
    c.finish();State{installation,scope,phase,generation,term,session,owner,sequence,selected,
        source,foot,original,planned,retirement,pending,registry,receipts,utc}
}
pub fn map_hashes(rs:&BTreeMap<u32,Registry>,receipts:&BTreeMap<u64,Receipt>)->([u8;32],[u8;32]){
    let registry=rs.values().map(|r|vec![
        Value::Integer(i64::from(r.kind)),Value::Blob(r.id.to_vec()),
        Value::Blob(r.path.as_bytes().to_vec()),Value::Blob(r.bytes.to_be_bytes().to_vec()),
        Value::Blob(r.digest.to_vec()),Value::Blob(r.manifest.clone()),
    ]).collect();
    let receipt=receipts.values().map(|r|vec![
        Value::Blob(r.sequence.to_be_bytes().to_vec()),Value::Blob(r.operation.to_vec()),
        Value::Blob(r.bytes.clone()),Value::Blob(r.digest.to_vec()),
    ]).collect();
    (rows_hash(&BTreeMap::from([("artifact_registry".to_string(),registry)])),
        rows_hash(&BTreeMap::from([("operation_receipts".to_string(),receipt)])))
}
fn proof(bytes:&[u8],member:&MembershipState)->DeviceProof{
    let mut c=Cursor{bytes,at:0};
    let p=DeviceProof{scope:nf_identity::model::Scope{
        universe:nf_contract::identity::UniverseId::from_bytes(c.take(16).try_into().unwrap()),
        history:nf_contract::identity::HistoryId::from_bytes(c.take(16).try_into().unwrap())},
        account:nf_contract::identity::AccountId::from_bytes(c.take(16).try_into().unwrap()),
        device:nf_contract::identity::DeviceId::from_bytes(c.take(16).try_into().unwrap()),
        frontier:c.u64(),peer:c.blob(128).to_vec(),challenge:c.take(32).try_into().unwrap(),
        signature:c.take(64).try_into().unwrap()};
    c.finish();assert_eq!(proof_bytes(&p),bytes);
    member.authorize(&p,&p.peer,&p.challenge,member.revision,
        nf_identity::model::ProtectedOperation::Administration).unwrap();p
}
#[derive(Clone)]
pub struct Summary {
    pub sequence:u64,pub operation:[u8;16],pub purpose:u32,pub owner:Stamp,pub artifact:[u8;16],
    pub pre_pending:Option<[u8;32]>,pub post_pending:Option<[u8;32]>,pub stage:u32,
    pub pre:Vec<u8>,pub post:Vec<u8>,pub pre_state:[u8;32],pub post_state:[u8;32],
    pub action:[u8;32],pub effect:Vec<u8>,pub intended:[u8;32],
}
pub fn parse_summary(bytes:&[u8])->Summary{
    let mut c=Cursor::new(bytes,b"NF-OWNER-PENDING-SUMMARY-1\0",16_384);
    let sequence=c.u64();let operation=c.take(16).try_into().unwrap();let purpose=c.u32();
    assert!([1,2,3,5,6,7,8].contains(&purpose));
    let owner=stamp(&mut c);let artifact=c.take(16).try_into().unwrap();
    let pre_pending=c.option(32).map(|v|v.try_into().unwrap());
    let post_pending=c.option(32).map(|v|v.try_into().unwrap());
    let stage=c.u32();assert!([1,2].contains(&stage));
    let pre=c.blob(2048).to_vec();let post=c.blob(2048).to_vec();
    let pre_state=c.take(32).try_into().unwrap();let post_state=c.take(32).try_into().unwrap();let action=c.take(32).try_into().unwrap();
    let effect=c.blob(8192).to_vec();assert!(effect.starts_with(b"NF-OWNER-EFFECT-1\0\x01\0"));
    let intended=c.take(32).try_into().unwrap();c.finish();
    Summary{sequence,operation,purpose,owner,artifact,pre_pending,post_pending,stage,pre,post,
        pre_state,post_state,action,effect,intended}
}
pub fn reduce(kind:u32,payload:&[u8],before:&State,after:&State,member:&MembershipState){
    assert_eq!(before.installation,after.installation);assert_eq!(before.scope,after.scope);
    assert_eq!(before.source,after.source);assert_eq!(before.original,after.original);
    if kind!=6{assert_eq!(before.generation,after.generation);}
    if kind!=1{assert_eq!((before.term,before.session),(after.term,after.session));}
    assert!(after.sequence>=before.sequence);
    let mut c=Cursor{bytes:payload,at:0};
    match kind{
        1=>{
            let sub=c.byte();assert!([1,2].contains(&sub));
            let transcript=c.blob(4096);let signed=proof(c.blob(300),member);
            assert_eq!(signed.challenge,hash(transcript));
            let post_stamp=c.blob(197);let mut sc=Cursor{bytes:post_stamp,at:0};
            let post=stamp(&mut sc);sc.finish();
            assert_eq!(post.installation,after.installation);
            assert_eq!((post.generation,post.term,post.session),(after.generation,after.term,after.session));
            assert_eq!(after.owner.unwrap()[..16],post.account);
            assert_eq!(after.owner.unwrap()[16..32],post.device);
            assert_eq!(after.owner.unwrap()[32..],post.incarnation);
            assert_eq!(c.blob(2048),after.foot.as_ref().unwrap());
            assert_eq!(after.sequence,before.sequence);
            assert!(after.pending.is_none());assert!([2,7].contains(&after.phase));
            assert_eq!(after.foot,before.foot);assert_eq!(after.selected,before.selected);
            if sub==2{assert_eq!(after.term,before.term+1);assert_eq!(after.session,before.session+1);}
        }
        2=>{
            let s=parse_summary(c.blob(16384));assert_eq!(s.stage,1);
            assert_eq!(s.sequence,before.sequence+1);assert_eq!(after.sequence,s.sequence);
            assert_eq!(before.foot.as_ref().unwrap(),&s.pre);
            assert_eq!(after.foot,before.foot);assert!(before.pending.is_none());
            assert_eq!(after.pending,s.post_pending);assert_eq!(s.owner.term,before.term);
            assert_eq!(after.owner,before.owner);assert_eq!(after.phase,if s.purpose==8{4}else{before.phase});
            assert_ne!(s.intended,[0;32]);
        }
        3=>{
            let s=parse_summary(c.blob(16384));assert_eq!(s.purpose,8);assert_eq!(s.stage,2);
            assert_eq!(before.pending,s.pre_pending);assert_eq!(after.pending,s.post_pending);
            assert_eq!(s.sequence,before.sequence);assert_eq!(after.sequence,before.sequence);
            assert_eq!(after.foot,before.foot);assert_eq!(after.phase,4);assert_eq!(after.owner,before.owner);
        }
        4=>{
            let s=parse_summary(c.blob(16384));let result=c.blob(4096);
            assert!(result.starts_with(b"NF-OWNER-RECEIPT-1\0\x01\0"));
            let disposition=c.byte();assert!([1,2].contains(&disposition));
            assert_eq!(before.pending,s.post_pending);assert!(after.pending.is_none());
            assert_eq!(after.foot.as_ref().unwrap(),&s.post);assert_eq!(after.sequence,s.sequence);
            if s.purpose==8{assert_eq!(after.phase,5);assert!(after.owner.is_none());}
            else{assert_eq!(after.phase,before.phase);assert_eq!(after.owner,before.owner);assert_eq!(disposition,1);}
            assert_eq!(after.selected,before.selected);
        }
        5=>{
            match c.byte(){
                1=>{
                    assert_eq!(c.blob(2048),before.foot.as_ref().unwrap());assert_eq!(c.byte(),2);
                    let _cut=c.u64();let transcript=c.blob(4096);let signed=proof(c.blob(300),member);
                    assert_eq!(signed.challenge,hash(transcript));assert_eq!((before.phase,after.phase),(2,3));
                }
                2=>{
                    let _new_rows=c.take(32);let transcript=c.blob(4096);let signed=proof(c.blob(300),member);
                    assert_eq!(signed.challenge,hash(transcript));assert_eq!((before.phase,after.phase),(3,3));
                }
                _=>panic!("catalog registration subtag"),
            }
            assert_eq!(after.foot,before.foot);assert_eq!(after.sequence,before.sequence);
            assert_eq!(after.owner,before.owner);assert_eq!(after.selected,before.selected);
        }
        6=>{
            let retirement=c.take(32);assert_eq!(Some(retirement),before.retirement.as_ref().map(|v|v.as_slice()));
            assert_eq!(c.blob(2048),before.foot.as_ref().unwrap());
            assert_eq!(c.blob(2048),after.foot.as_ref().unwrap());let _binary=c.take(32);
            let transcript=c.blob(4096);let signed=proof(c.blob(300),member);
            assert_eq!(signed.challenge,hash(transcript));assert_eq!((before.phase,after.phase),(5,6));
            assert_eq!(after.generation,before.generation+1);assert!(after.owner.is_none());
            assert_eq!(after.sequence,before.sequence);assert!(after.pending.is_none());
        }
        8=>{
            let _prior_revision=c.u64();let _prior_state=c.take(32);let _prior_head=c.take(32);
            let _prior_rows=c.take(32);assert!(before.pending.is_none());assert!(after.pending.is_none());
            assert_eq!(before.foot,after.foot);assert_eq!(before.sequence,after.sequence);
        }
        _=>panic!("first positive fixture must not have abort/unsupported catalog kind"),
    }
    c.finish();
}
pub struct ExpectedRegistry {pub kind:u32,pub path:String,pub bytes:u64,pub digest:[u8;32],
    pub profile:u32,pub schema:[u8;32],pub descriptor:Vec<u8>}

pub fn registry_rows(rs:&BTreeMap<u32,Registry>)->Rows{
    BTreeMap::from([("artifact_registry".into(),rs.values().map(|r|vec![
        Value::Integer(i64::from(r.kind)),Value::Blob(r.id.to_vec()),Value::Blob(r.path.as_bytes().to_vec()),
        Value::Blob(r.bytes.to_be_bytes().to_vec()),Value::Blob(r.digest.to_vec()),Value::Blob(r.manifest.clone())]).collect())])
}
pub fn receipt_rows(rs:&BTreeMap<u64,Receipt>)->Rows{
    BTreeMap::from([("operation_receipts".into(),rs.values().map(|r|vec![
        Value::Blob(r.sequence.to_be_bytes().to_vec()),Value::Blob(r.operation.to_vec()),
        Value::Blob(r.bytes.clone()),Value::Blob(r.digest.to_vec())]).collect())])
}
pub fn admission_delta(bytes:&[u8],rs:&mut BTreeMap<u32,Registry>){
    let mut c=Cursor::new(bytes,b"NF-OWNER-ADMISSION-1\0",180000);
    let _purpose=c.u32();let _installation=c.take(16);let _scope=c.take(32);
    let _foot=c.blob(2048);let _composition=c.blob(1923);let _path=c.blob(4096);
    let changes=c.blob(131072);let _operation=c.blob(32768);c.finish();
    let mut receipts=BTreeMap::new();apply_delta(changes,rs,&mut receipts);assert!(receipts.is_empty());
}
pub fn verify_manifests(expected:&super::catalog_replay::Expected,member:&MembershipState){
    assert_eq!(expected.registry.len(),5);assert_eq!(expected.pins.len(),5);
    for pin in &expected.pins{
        let r=&expected.registry[&pin.kind];assert_eq!(r.path,pin.path);assert_eq!(r.bytes,pin.bytes);assert_eq!(r.digest,pin.digest);
        assert_ne!(r.id,[0;16]);
        let mut m=Cursor::new(&r.manifest,b"NF-OWNER-ARTIFACT-1\0",8192);
        assert_eq!(m.take(16),expected.installation);assert_eq!(m.take(16),member.scope.universe.as_bytes());
        assert_eq!(m.take(16),member.scope.history.as_bytes());assert_eq!(m.u32(),pin.kind);
        assert_eq!(m.take(16),r.id);assert_eq!(m.blob(4096),pin.path.as_bytes());assert_eq!(m.u64(),pin.bytes);
        assert_eq!(m.take(32),pin.digest);assert_eq!(m.u32(),pin.profile);assert_eq!(m.take(32),pin.schema);
        assert_eq!(m.blob(2048),if pin.kind>=4{
            b"Running test executable; operational old/new node compatibility NOT QUALIFIED".as_slice()
        }else{super::operations::DATA_PROVENANCE.as_bytes()});
        assert_eq!(m.blob(1923),pin.descriptor);m.finish();
    }
}
