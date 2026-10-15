//! Whole expected rows and Feet come from public legacy codecs plus literal overlay fields.
use super::{observations::{Rows, value, rows_hash}, records::{hash, journal_head, PROFILE4_SCHEMA}};
use nf_store::KnownFrontiers;
use rusqlite::types::Value;
fn b(bytes: impl AsRef<[u8]>) -> Value { Value::Blob(bytes.as_ref().to_vec()) }
pub struct WorldOracle { pub rows: Rows, pub head: [u8;32], pub state: Vec<u8>, pub lineage:[u8;32] }
impl WorldOracle {
    pub fn initial(base:&Rows, known:KnownFrontiers, state:&[u8], action:&[u8],
        lineage:&[u8], before:[u8;32], head:[u8;32]) -> Self {
        let mut rows=base.clone(); let digest=hash(state); let line=hash(lineage);
        let r=known.store_revision.to_be_bytes(); let e=known.event_sequence.0.to_be_bytes();
        let m=known.membership_revision.unwrap().to_be_bytes(); let q=65_536u32.to_le_bytes();
        rows.insert("active_journal".into(),vec![vec![b(r),Value::Integer(4),b(before),b(digest),b(head),b(action),b(state)]]);
        rows.insert("active_events".into(),vec![]);
        rows.insert("active_snapshots".into(),vec![vec![b(r),b(e),b(m),b(q),b(state),b(digest),b(head),b(line)]]);
        rows.insert("migration_lineage".into(),vec![vec![Value::Integer(1),b(lineage),b(line)]]);
        rows.insert("active_meta".into(),vec![vec![Value::Integer(1),b(r),b(e),b(m),b(q),b(state),b(digest),
            b(head),b(PROFILE4_SCHEMA),b(line),b(r),b(digest),b(head)]]);
        Self{rows,head,state:state.to_vec(),lineage:line}
    }
    pub fn append(&mut self, known:KnownFrontiers, kind:u32, action:&[u8], state:&[u8],
        batch:Option<&nf_kernel::CommittedBatch>) {
        let before=hash(&self.state);let digest=hash(state);
        let head=journal_head(self.head,known.store_revision,kind,action,before,digest,self.lineage);
        self.rows.get_mut("active_journal").unwrap().push(vec![b(known.store_revision.to_be_bytes()),
            Value::Integer(i64::from(kind)),b(before),b(digest),b(head),b(action),b(state)]);
        let meta=&mut self.rows.get_mut("active_meta").unwrap()[0];
        meta[1]=b(known.store_revision.to_be_bytes());meta[2]=b(known.event_sequence.0.to_be_bytes());
        meta[3]=b(known.membership_revision.unwrap().to_be_bytes());meta[5]=b(state);meta[6]=b(digest);meta[7]=b(head);
        if let Some(batch)=batch {
            let bytes=nf_kernel::encode_batch(batch).unwrap();
            self.rows.get_mut("active_events").unwrap().push(vec![b(batch.sequence.0.to_be_bytes()),
                b(known.store_revision.to_be_bytes()),b(&bytes),b(hash(&bytes))]);
        }
        self.head=head;self.state=state.to_vec();
    }
    pub fn foot(&self) -> Vec<u8> {
        let base=&self.rows["history_meta"][0]; let meta=&self.rows["active_meta"][0];
        let cp=&self.rows["active_snapshots"][0];
        let mut c=super::cursor::Cursor::new(&self.state,b"NF-ACTIVE-STATE-1\0",1_884_160);
        let core=c.blob(1_048_576);assert_eq!(c.byte(),1);let member=c.blob(262_144);
        let quota=c.u32();let policy=c.option(17);assert_eq!(c.u32(),0);c.finish();
        let mut out=b"NF-OWNER-STORAGE-1\0\x01\0".to_vec();
        out.extend_from_slice(&4u32.to_le_bytes());out.extend_from_slice(&PROFILE4_SCHEMA);
        out.extend_from_slice(value(base,1));out.extend_from_slice(value(base,2));
        out.extend_from_slice(value(meta,1));out.extend_from_slice(value(meta,2));
        out.push(1);out.extend_from_slice(value(meta,3));out.extend_from_slice(&hash(core));
        out.push(1);out.extend_from_slice(&hash(member));out.extend_from_slice(&hash(&self.state));
        out.extend_from_slice(&self.head);out.push(1);out.extend_from_slice(&self.lineage);
        out.extend_from_slice(&quota.to_le_bytes());
        match policy {None=>out.push(0),Some(p)=>{out.push(1);out.extend_from_slice(p);}}
        out.extend_from_slice(&hash(&0u32.to_le_bytes()));
        out.extend_from_slice(value(cp,0));out.extend_from_slice(value(cp,1));
        out.push(1);out.extend_from_slice(value(cp,2));out.extend_from_slice(value(cp,5));
        out.extend_from_slice(value(cp,6));out.extend_from_slice(value(cp,3));
        out.extend_from_slice(&rows_hash(&self.rows));out
    }
    pub fn observe(&self,path:&std::path::Path) {
        assert_eq!(super::observations::tables(&super::observations::readonly_public(path),true),self.rows);
        assert_eq!(super::observations::active_foot(path),self.foot());
    }
}
