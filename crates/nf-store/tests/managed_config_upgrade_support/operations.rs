//! Exact final-selected unsigned command/input/effect bytes, independent of managed encoders.
use super::{catalog::{Registry,ExpectedRegistry,apply_delta,registry_rows},catalog_replay::Expected,
    cursor::Cursor,fixture::{Case,read_with_cap,MAX_BINARY_FILE},observations::{Rows,rows_hash},
    records::{blob,hash,PROFILE1_SCHEMA,PROFILE4_SCHEMA,composition_bytes},transcripts::Stamp};
use nf_store::migration::Composition;
use std::collections::BTreeMap;
pub const DATA_PROVENANCE:&str="Caller-created managed lifecycle fixture; bytes independently verified";
pub fn empty_delta()->Vec<u8>{
    let mut out=b"NF-OWNER-ROW-DELTA-1\0\x01\0".to_vec();out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());out
}
fn command(purpose:u32)->Vec<u8>{
    let mut out=b"NF-OWNER-CATALOG-COMMAND-1\0\x01\0".to_vec();
    out.extend_from_slice(&purpose.to_le_bytes());out
}
pub fn quiesce_payload(foot:&[u8],cut:u64)->Vec<u8>{
    let mut out=command(11);out.push(1);blob(&mut out,foot);out.push(2);
    out.extend_from_slice(&cut.to_be_bytes());out.extend_from_slice(&hash(&empty_delta()));out
}
pub fn registration_payload(registry:&BTreeMap<u32,Registry>,delta:&[u8])->Vec<u8>{
    let mut out=command(12);out.push(2);out.extend_from_slice(&rows_hash(&registry_rows(registry)));
    out.extend_from_slice(&hash(delta));out
}
pub fn registration_delta(bytes:&[u8],registry:&mut BTreeMap<u32,Registry>){
    let before=registry.clone();let mut receipts=BTreeMap::new();
    apply_delta(bytes,registry,&mut receipts);assert!(receipts.is_empty());
    assert_eq!(before.keys().copied().collect::<Vec<_>>(),vec![1,4,5]);
    assert_eq!(registry.keys().copied().collect::<Vec<_>>(),vec![1,2,3,4,5]);
}
fn interior(bytes:&[u8],purpose:u32)->Vec<u8>{
    let mut c=Cursor::new(bytes,b"NF-OWNER-ADMISSION-1\0",172_002);
    assert_eq!(c.u32(),purpose);let _installation=c.take(16);let _scope=c.take(32);
    let _foot=c.blob(2048);let _composition=c.blob(1923);let _path=c.blob(4096);
    let delta=c.blob(131072);
    if purpose!=9{assert_eq!(delta,empty_delta());}
    let operation=c.blob(32768).to_vec();c.finish();operation
}
fn operation(purpose:u32)->Vec<u8>{
    let mut out=b"NF-OWNER-OPERATION-1\0\x01\0".to_vec();
    out.extend_from_slice(&purpose.to_le_bytes());out
}
pub fn assert_adoption_operation(admission:&[u8],registry:&BTreeMap<u32,Registry>){
    let mut expected=operation(9);expected.extend_from_slice(&registry[&1].id);
    expected.extend_from_slice(&1u32.to_le_bytes());expected.extend_from_slice(&[41;16]);
    expected.extend_from_slice(&[41;32]);expected.extend_from_slice(&1u32.to_le_bytes());
    expected.extend_from_slice(&1u32.to_le_bytes());assert_eq!(interior(admission,9),expected);
}
pub fn assert_claim_operation(admission:&[u8],stamp:&Stamp){
    let mut expected=operation(10);expected.extend_from_slice(&stamp.generation.to_be_bytes());
    expected.extend_from_slice(&stamp.term.to_be_bytes());expected.extend_from_slice(&stamp.session.to_be_bytes());
    expected.extend_from_slice(&stamp.incarnation);assert_eq!(interior(admission,10),expected);
}
pub fn assert_selection_operation(admission:&[u8],expected:&Expected,stamp:&Stamp){
    let mut bytes=operation(13);bytes.extend_from_slice(&expected.registry[&1].id);
    bytes.extend_from_slice(&expected.registry[&3].id);blob(&mut bytes,&expected.retired);
    blob(&mut bytes,&expected.initial);bytes.extend_from_slice(&hash(&expected.retirement));
    bytes.extend_from_slice(&hash(&expected.registry[&5].manifest));
    bytes.extend_from_slice(&stamp.generation.to_be_bytes());bytes.extend_from_slice(&stamp.term.to_be_bytes());
    bytes.extend_from_slice(&stamp.session.to_be_bytes());bytes.extend_from_slice(&stamp.incarnation);
    assert_eq!(interior(admission,13),bytes);
}
pub fn artifact_pins(case:&Case,original:&[u8],from:&Composition,to:&Composition)->Vec<ExpectedRegistry>{
    assert_eq!(from.digest(),hash(&composition_bytes(1,PROFILE1_SCHEMA,2)));
    assert_eq!(to.digest(),hash(&composition_bytes(4,PROFILE4_SCHEMA,9)));
    let target=super::fixture::bounded_read(&case.target());
    let old=read_with_cap(&case.old_binary(),MAX_BINARY_FILE);
    let new=read_with_cap(&case.new_binary(),MAX_BINARY_FILE);
    let mut out=Vec::new();
    for (kind,path,bytes,profile,schema,descriptor) in [
        (1,case.source(),original,1,PROFILE1_SCHEMA,composition_bytes(1,PROFILE1_SCHEMA,2)),
        (2,case.retained_source(),original,1,PROFILE1_SCHEMA,composition_bytes(1,PROFILE1_SCHEMA,2)),
        (3,case.target(),target.as_slice(),4,PROFILE4_SCHEMA,composition_bytes(4,PROFILE4_SCHEMA,9)),
        (4,case.old_binary(),old.as_slice(),0,[0;32],composition_bytes(1,PROFILE1_SCHEMA,2)),
        (5,case.new_binary(),new.as_slice(),0,[0;32],composition_bytes(4,PROFILE4_SCHEMA,9)),
    ]{
        out.push(ExpectedRegistry{kind,path:path.to_str().unwrap().to_string(),
            bytes:bytes.len().try_into().unwrap(),digest:hash(bytes),profile,schema,descriptor});
    }
    out
}
fn checkpoint(foot:&[u8])->Vec<u8>{
    let mut c=Cursor::new(foot,b"NF-OWNER-STORAGE-1\0",2048);
    let _profile=c.u32();let _schema=c.take(32);let _scope=c.take(32);let _r=c.u64();let _e=c.u64();
    let _m=c.option(8);let _core=c.take(32);let _member=c.option(32);
    let _state=c.take(32);let _head=c.take(32);let _lineage=c.option(32);
    let _q=c.u32();let _policy=c.option(17);let _cancel=c.take(32);
    let start=c.at;let _cr=c.u64();let _ce=c.u64();let _cm=c.option(8);
    let _cs=c.take(32);let _ch=c.take(32);let _cq=c.u32();
    let result=foot[start..c.at].to_vec();let _rows=c.take(32);c.finish();result
}
pub fn physical_effect(purpose:u32,pre_profile:u32,post_profile:u32,pre:&Rows,post:&Rows,
    pre_foot:&[u8],post_foot:&[u8])->Vec<u8>{
    let mut out=b"NF-OWNER-EFFECT-1\0\x01\0".to_vec();
    for value in [purpose,pre_profile,post_profile,65_536,65_536]{out.extend_from_slice(&value.to_le_bytes());}
    let names=pre.keys().chain(post.keys()).cloned().collect::<std::collections::BTreeSet<_>>();
    assert_eq!(names.len(),if purpose==8{11}else{15});
    out.extend_from_slice(&u32::try_from(names.len()).unwrap().to_le_bytes());
    for name in names{
        blob(&mut out,name.as_bytes());
        let before=pre.get(&name).cloned().unwrap_or_else(||{
            assert_eq!((purpose,pre_profile,post_profile),(8,1,5));assert_eq!(name,"owner_retirement");vec![]});
        let after=post.get(&name).unwrap();
        out.extend_from_slice(&u64::try_from(before.len()).unwrap().to_be_bytes());
        out.extend_from_slice(&rows_hash(&BTreeMap::from([(name.clone(),before)])));
        out.extend_from_slice(&u64::try_from(after.len()).unwrap().to_be_bytes());
        out.extend_from_slice(&rows_hash(&BTreeMap::from([(name,after.clone())])));
    }
    for profile in [pre_profile,post_profile]{
        for value in [0x4e46_5354,profile,4096]{out.extend_from_slice(&value.to_le_bytes());}
    }
    blob(&mut out,&checkpoint(pre_foot));blob(&mut out,&checkpoint(post_foot));
    assert!(out.len()<=1698);out
}
