//! Full first-case genesis and contiguous catalog replay, not a meta self-hash.
use super::{catalog::*, cursor::Cursor, records::hash, transcripts::{Auth,Stamp},
    observations::{Rows,rows_hash,readonly_public}};
use nf_identity::model::{DeviceProof,MembershipState};
use rusqlite::types::Value;
use std::{collections::BTreeMap,path::Path};
pub struct Signed { pub transcript:Vec<u8>,pub proof:DeviceProof,pub unsigned:Option<Vec<u8>> }
pub struct Operation {
    pub auth:Auth,pub action:Vec<u8>,pub pre:Vec<u8>,pub post:Vec<u8>,
    pub pre_state:Vec<u8>,pub post_state:Vec<u8>,pub effect:Vec<u8>,
}
pub struct Expected {
    pub installation:[u8;16],pub source:Vec<u8>,pub initial:Vec<u8>,pub retired:Vec<u8>,
    pub retirement:Vec<u8>,pub pins:Vec<ExpectedRegistry>,pub registry:BTreeMap<u32,Registry>,
    pub signed:BTreeMap<[u8;32],Signed>,pub operations:BTreeMap<u64,Operation>,
    pub start:u64,pub cutoff:u64,
}
impl Expected {
    pub fn new(installation:[u8;16],source:&[u8],registry:&BTreeMap<u32,Registry>,start:u64,cutoff:u64)->Self{
        Self{installation,source:source.to_vec(),initial:vec![],retired:vec![],retirement:vec![],
            pins:vec![],registry:registry.clone(),signed:BTreeMap::new(),operations:BTreeMap::new(),start,cutoff}
    }
    pub fn signed_owned(&mut self,transcript:&[u8],proof:&DeviceProof,unsigned:&[u8]){
        self.signed(transcript,proof);self.signed.get_mut(&hash(transcript)).unwrap().unsigned=Some(unsigned.to_vec());
    }
    pub fn signed(&mut self,transcript:&[u8],proof:&DeviceProof){
        assert_eq!(proof.challenge,hash(transcript));assert!(self.signed.insert(
            hash(transcript),Signed{transcript:transcript.to_vec(),proof:proof.clone(),unsigned:None}).is_none());
    }
}
fn verify_signed(bytes:&[u8],proof_bytes:&[u8],expected:&Expected,member:&MembershipState){
    let signed=&expected.signed[&hash(bytes)];assert_eq!(signed.transcript,bytes);
    assert_eq!(super::records::proof_bytes(&signed.proof),proof_bytes);
    super::authorize(member,&signed.proof);
}
fn typed_payload(kind:u32,payload:&[u8],changes:&[u8],expected:&Expected,
    member:&MembershipState,after:&State)->Option<Receipt>{
    let mut c=Cursor{bytes:payload,at:0};let mut outcome_row=None;
    match kind{
        1=>{
            let sub=c.byte();assert!([1,2].contains(&sub));
            let t=c.blob(4096);let p=c.blob(300);verify_signed(t,p,expected,member);
            // The expected candidate comes from the actual issued, signed transcript.
            let mut tc=Cursor::new(t,b"NF-OWNER-PROSPECTIVE-1\0",4096);
            assert_eq!(tc.u32(),if sub==1{9}else{10});
            assert_eq!(tc.take(16),expected.installation);
            assert_eq!(tc.take(16),member.scope.universe.as_bytes());
            assert_eq!(tc.take(16),member.scope.history.as_bytes());
            let _catalog=tc.option(72);let candidate=super::transcripts::stamp(&mut tc);
            let claimed_foot=tc.blob(2048);
            assert_eq!(c.blob(197),candidate.bytes);assert_eq!(c.blob(2048),claimed_foot);
            assert_eq!(candidate.installation,after.installation);
            assert_eq!((after.generation,after.term,after.session),
                (candidate.generation,candidate.term,candidate.session));
            let owner=after.owner.unwrap();assert_eq!(owner[..16],candidate.account);
            assert_eq!(owner[16..32],candidate.device);assert_eq!(owner[32..],candidate.incarnation);
            assert_eq!(after.foot.as_ref().unwrap(),claimed_foot);
            assert!(after.pending.is_none());assert_eq!(after.phase,if sub==1{2}else{7});
            assert_eq!(candidate.member,Some(member.revision));
            assert_eq!(candidate.member_digest,hash(&nf_identity::codec::encode_state(member).unwrap()));
            if sub==1{
                assert_eq!((candidate.generation,candidate.term,candidate.session),(1,1,1));
                assert_eq!(after.sequence,0);assert_eq!(claimed_foot,expected.source);
                assert_eq!(after.source,expected.registry[&1].id);
            }else{assert_eq!(changes,super::operations::empty_delta());}
        }
        2|3|4=>{
            let raw=c.blob(16384);let summary=parse_summary(raw);let op=&expected.operations[&summary.sequence];
            assert_eq!(summary.operation,op.auth.operation);assert_eq!(summary.purpose,op.auth.purpose);
            assert_eq!(summary.owner.bytes,op.auth.stamp.bytes);
            assert_eq!(summary.pre,op.pre);assert_eq!(summary.post,op.post);
            assert_eq!(summary.pre_state,hash(&op.pre_state));assert_eq!(summary.post_state,hash(&op.post_state));
            assert_eq!(summary.action,hash(&op.action));assert_eq!(summary.effect,op.effect);
            assert_eq!(summary.artifact,expected.registry[&if op.auth.purpose==8{1}else{3}].id);
            if kind==4{
                let receipt=c.blob(4096);let mut r=Cursor::new(receipt,b"NF-OWNER-RECEIPT-1\0",4096);
                assert_eq!(r.u64(),summary.sequence);assert_eq!(r.take(16),summary.operation);
                assert_eq!(r.u32(),summary.purpose);assert_eq!(r.byte(),if summary.purpose==8{3}else{1});
                assert_eq!(r.blob(2048),op.post);assert_eq!(r.take(32),hash(&op.action));
                let outcome=r.u64();assert!(outcome>0);assert_eq!(r.take(32),[0;32]);r.finish();
                assert_eq!(c.byte(),if summary.purpose==8{2}else{1});
                outcome_row=Some(Receipt{sequence:summary.sequence,operation:summary.operation,
                    bytes:receipt.to_vec(),digest:hash(receipt)});
            }
        }
        5=>match c.byte(){
            1=>{
                let foot=c.blob(2048);assert_eq!(foot,expected.source);assert_eq!(c.byte(),2);let cut=c.u64();
                assert_eq!(changes,super::operations::empty_delta());
                let unsigned=super::operations::quiesce_payload(foot,cut);
                let t=c.blob(4096);let p=c.blob(300);verify_signed(t,p,expected,member);
                assert_eq!(expected.signed[&hash(t)].unsigned.as_ref().unwrap(),&unsigned);
            }
            2=>{
                assert_eq!(c.take(32),rows_hash(&registry_rows(&expected.registry)));
                let unsigned=super::operations::registration_payload(&expected.registry,changes);
                let t=c.blob(4096);let p=c.blob(300);verify_signed(t,p,expected,member);
                assert_eq!(expected.signed[&hash(t)].unsigned.as_ref().unwrap(),&unsigned);
            }
            _=>panic!("catalog registration kind"),
        },
        6=>{
            assert_eq!(c.take(32),hash(&expected.retirement));assert_eq!(c.blob(2048),expected.retired);
            assert_eq!(c.blob(2048),expected.initial);assert_eq!(c.take(32),hash(&expected.registry[&5].manifest));
            let t=c.blob(4096);let p=c.blob(300);verify_signed(t,p,expected,member);
        }
        _=>panic!("bounded first case has no abort or 128-entry compaction"),
    }
    c.finish();outcome_row
}
fn assert_utc(value:u64,expected:&Expected,observed:u64){
    assert!(value>=expected.start&&value<=expected.cutoff&&value<=observed);
}
pub fn replay(path:&Path,member:&MembershipState,expected:&Expected,foot:&[u8],
    epoch:Option<&Stamp>,frontier:(u64,[u8;32],[u8;32])) {
    let c=readonly_public(path);
    let observed=super::fixture::now_ms();assert!(expected.start<=observed&&observed<=expected.cutoff);
    assert!(expected.operations.len()<=3);let mut joined_receipts=BTreeMap::new();
    super::schemas::check(&c,0x4e46_4f43,1,super::schemas::CATALOG_SCHEMA,super::schemas::CATALOG_SCHEMA_DIGEST);
    let n:i64=c.query_row("SELECT count(*) FROM pending_world",[],|r|r.get(0)).unwrap();assert_eq!(n,0);
    let snapshot_count:i64=c.query_row("SELECT count(*) FROM catalog_snapshots",[],|r|r.get(0)).unwrap();assert_eq!(snapshot_count,1);
    let snapshot:(Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>)=c.query_row(
        "SELECT revision,state,digest,head,rows FROM catalog_snapshots",[],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
    assert_eq!(snapshot.0,1u64.to_be_bytes());assert_eq!(hash(&snapshot.1),snapshot.2.as_slice());
    let (seed_registry,seed_receipts)=row_image(&snapshot.4);assert!(seed_receipts.is_empty());
    let genesis=state(&snapshot.1);assert_utc(genesis.utc,expected,observed);
    assert_eq!(genesis.installation,expected.installation);
    assert_eq!(genesis.original,expected.source);assert_eq!(genesis.foot,Some(expected.source.clone()));
    assert_eq!((genesis.phase,genesis.generation,genesis.term,genesis.session,genesis.sequence),(2,1,1,1,0));
    assert!(genesis.pending.is_none()&&genesis.planned.is_none()&&genesis.retirement.is_none());
    assert_eq!(seed_registry.keys().copied().collect::<Vec<_>>(),vec![1,4,5]);
    for (k,r) in &seed_registry{assert_eq!(r,&expected.registry[k]);}
    assert_eq!(map_hashes(&seed_registry,&seed_receipts),(genesis.registry,genesis.receipts));
    let mut registry=BTreeMap::new();let mut receipts=BTreeMap::new();
    let mut number=0u64;let mut digest=[0;32];let mut head=[0;32];let mut current=None;
    let mut kinds=Vec::new();let mut pending=None;
    let mut previous_state=Vec::new();let mut pending_sources=BTreeMap::new();
    let rows=c.prepare("SELECT revision,kind,previous,digest,head,action,state FROM catalog_journal ORDER BY revision LIMIT 33").unwrap()
        .query_map([],|r|Ok((r.get::<_,Vec<u8>>(0)?,r.get::<_,u32>(1)?,r.get::<_,Vec<u8>>(2)?,
            r.get::<_,Vec<u8>>(3)?,r.get::<_,Vec<u8>>(4)?,r.get::<_,Vec<u8>>(5)?,r.get::<_,Vec<u8>>(6)?)))
        .unwrap().collect::<rusqlite::Result<Vec<_>>>().unwrap();
    assert!(!rows.is_empty()&&rows.len()<33);
    for (revision,kind,previous,post_digest,post_head,action,post) in rows{
        let before_revision=number;let before_digest=digest;let before_head=head;
        number+=1;assert_eq!(revision,number.to_be_bytes());assert_eq!(previous,digest);
        assert_eq!(post_digest,hash(&post));let after=state(&post);
        let mut ac=Cursor::new(&action,b"NF-OWNER-CATALOG-ACTION-1\0",131072);
        assert_eq!(ac.u32(),kind);let sequence=ac.u64();let operation:[u8;16]=ac.take(16).try_into().unwrap();
        let payload=ac.blob(131072);let changes=ac.blob(131072);ac.finish();
        assert_utc(after.utc,expected,observed);
        let outcome_row=typed_payload(kind,payload,changes,expected,member,&after);
        assert_fixture_delta(changes,kind,outcome_row.as_ref());
        if let Some(row)=outcome_row{
            assert!(receipts.get(&row.sequence).is_none());
            assert!(joined_receipts.insert(row.sequence,row).is_none());
        }
        if let Some(before)=&current{
            reduce(kind,payload,before,&after,member);
            assert!(after.utc>=before.utc);
        }else{
            assert_eq!(kind,1);assert_eq!(post,snapshot.1);assert_eq!(sequence,0);
            assert_eq!(after,genesis);
        }
        if [2,3,4].contains(&kind){
            let mut p=Cursor{bytes:payload,at:0};let s=parse_summary(p.blob(16384));
            assert_eq!(sequence,s.sequence);assert_eq!(operation,s.operation);
            if kind==4{assert_eq!(s.intended,hash(&post));}
            let op=&expected.operations[&sequence];
            match kind{
                2=>{
                    assert!(pending.is_none());assert!(s.pre_pending.is_none());
                    pending_sources.insert(sequence,(before_revision,before_digest,before_head,previous_state.clone(),
                        vec![(kind,s.clone(),after.utc)]));
                    pending=Some((sequence,s.operation,s.post_pending.unwrap()));
                }
                3=>{let prior=pending.unwrap();assert_eq!((prior.0,prior.1),(sequence,s.operation));
                    assert_eq!(Some(prior.2),s.pre_pending);pending=Some((sequence,s.operation,s.post_pending.unwrap()));
                    pending_sources.get_mut(&sequence).unwrap().4.push((kind,s.clone(),after.utc));
                }
                4=>{let prior=pending.take().unwrap();assert_eq!((prior.0,prior.1),(sequence,s.operation));
                    assert_eq!(Some(prior.2),s.post_pending);assert_eq!(after.foot,Some(op.post.clone()));
                    let receipt=p.blob(4096);let mut r=Cursor::new(receipt,b"NF-OWNER-RECEIPT-1\0",4096);
                    let _sequence=r.u64();let _id=r.take(16);let _purpose=r.u32();let _result=r.byte();
                    let _foot=r.blob(2048);let _action=r.take(32);assert_eq!(r.u64(),number);
                    let source=pending_sources.remove(&sequence).unwrap();
                    assert_eq!(number,source.0+if op.auth.purpose==8{3}else{2});
                    for (step,summary,step_utc) in source.4{
                        assert_eq!(step_utc,after.utc);
                        let body=pending_record(op,source.0,source.1,source.2,&post,summary.artifact,summary.stage);
                        assert_eq!(summary.post_pending,Some(hash(&body)));
                        assert_eq!(summary.intended,hash(&post));
                        if step==3{
                            let prior_body=pending_record(op,source.0,source.1,source.2,&post,summary.artifact,1);
                            assert_eq!(summary.pre_pending,Some(hash(&prior_body)));
                        }
                    }
                    let body=pending_record(op,source.0,source.1,source.2,&post,s.artifact,s.stage);
                    assert_eq!(s.post_pending,Some(hash(&body)));
                    assert_eq!(after.utc,state(&source.3).utc.max(after.utc));
                }
                _=>unreachable!(),
            }
        }
        apply_delta(changes,&mut registry,&mut receipts);
        assert_eq!(receipts,joined_receipts);
        assert_eq!(map_hashes(&registry,&receipts),(after.registry,after.receipts));
        let mut bytes=b"NF-OWNER-CATALOG-JOURNAL-1\0\x01\0".to_vec();
        bytes.extend_from_slice(&head);bytes.extend_from_slice(&number.to_be_bytes());
        bytes.extend_from_slice(&kind.to_le_bytes());bytes.extend_from_slice(&hash(&action));
        bytes.extend_from_slice(&digest);bytes.extend_from_slice(&hash(&post));bytes.extend_from_slice(&after.registry);
        head=hash(&bytes);assert_eq!(post_head,head);
        if number==1{assert_eq!(post_head,snapshot.3);assert_eq!(registry,seed_registry);}
        digest=hash(&post);previous_state=post;current=Some(after);kinds.push(kind);
    }
    assert!(pending.is_none());assert!(pending_sources.is_empty());let current=current.unwrap();
    assert_eq!(&kinds[..7],&[1,5,5,2,3,4,6]);
    // Every later claim is kind1; each real business operation is exactly2 then4.
    assert_eq!(kinds.iter().filter(|&&k|k==2).count(),expected.operations.len());
    assert_eq!(kinds.iter().filter(|&&k|k==4).count(),expected.operations.len());
    assert_eq!(receipts.keys().copied().collect::<Vec<_>>(),
        expected.operations.keys().copied().collect::<Vec<_>>());
    assert_eq!(receipts.len(),expected.operations.len());
    assert_eq!(registry,expected.registry);assert_eq!(current.original,expected.source);
    assert_eq!(current.planned,Some(expected.initial.clone()));assert_eq!(current.retirement,Some(hash(&expected.retirement)));
    assert_eq!(current.foot,Some(foot.to_vec()));assert!(current.pending.is_none());
    assert_eq!(current.source,registry[&1].id);assert_eq!(current.selected,Some(registry[&3].id));
    assert_eq!(current.scope[..16],*member.scope.universe.as_bytes());assert_eq!(current.scope[16..],*member.scope.history.as_bytes());
    assert_eq!(current.sequence,*expected.operations.last_key_value().unwrap().0);
    if let Some(s)=epoch{
        assert_eq!((current.phase,current.generation,current.term,current.session),(7,s.generation,s.term,s.session));
        let owner=current.owner.unwrap();assert_eq!(owner[..16],s.account);assert_eq!(owner[16..32],s.device);
        assert_eq!(owner[32..],s.incarnation);
    }else{assert_eq!(current.phase,6);assert!(current.owner.is_none());}
    let actual:(Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>,Vec<u8>)=c.query_row(
        "SELECT revision,state,digest,head,schema_digest FROM catalog_meta WHERE singleton=1",[],
        |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
    assert_eq!((number,digest,head),frontier);assert_eq!(actual.0,number.to_be_bytes());
    let meta=state(&actual.1);assert_utc(meta.utc,expected,observed);
    assert_eq!(meta,current);assert_eq!(actual.2,digest);assert_eq!(actual.3,head);
    assert_eq!(actual.4,super::schemas::CATALOG_SCHEMA_DIGEST);
    let actual_registry:Rows=BTreeMap::from([("artifact_registry".into(),c.prepare("SELECT * FROM artifact_registry ORDER BY kind LIMIT 6").unwrap()
        .query_map([],|r|(0..6).map(|i|r.get(i)).collect::<rusqlite::Result<Vec<Value>>>()).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap())]);
    let actual_receipts:Rows=BTreeMap::from([("operation_receipts".into(),c.prepare("SELECT * FROM operation_receipts ORDER BY sequence LIMIT 257").unwrap()
        .query_map([],|r|(0..4).map(|i|r.get(i)).collect::<rusqlite::Result<Vec<Value>>>()).unwrap()
        .collect::<rusqlite::Result<Vec<_>>>().unwrap())]);
    assert_eq!(actual_registry,registry_rows(&registry));assert_eq!(actual_receipts,receipt_rows(&receipts));
    assert_eq!(rows_hash(&actual_registry),current.registry);assert_eq!(rows_hash(&actual_receipts),current.receipts);
    verify_manifests(expected,member);
}

fn pending_record(op:&Operation,before_revision:u64,before_digest:[u8;32],before_head:[u8;32],
    intended:&[u8],artifact:[u8;16],stage:u32)->Vec<u8>{
    let mut out=b"NF-OWNER-PENDING-1\0\x01\0".to_vec();
    out.extend_from_slice(&op.auth.sequence.to_be_bytes());out.extend_from_slice(&op.auth.operation);
    out.extend_from_slice(&op.auth.purpose.to_le_bytes());out.extend_from_slice(&op.auth.stamp.bytes);
    out.extend_from_slice(&before_revision.to_be_bytes());out.extend_from_slice(&before_digest);
    out.extend_from_slice(&before_head);out.extend_from_slice(&artifact);
    for bytes in [&op.pre,&op.post,&op.pre_state,&op.post_state,&op.action]{
        super::records::blob(&mut out,bytes);
    }
    super::records::blob(&mut out,intended);super::records::blob(&mut out,&op.effect);
    out.extend_from_slice(&stage.to_le_bytes());assert!(out.len()<=5_242_880);out
}
