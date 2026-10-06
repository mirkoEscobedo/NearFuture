use sha2::{Digest, Sha256};
pub struct Packet(pub Vec<u8>);
impl Packet {
    pub fn new(kind: u16) -> Self {
        let mut p = Self(b"NF-CANON-1\0".to_vec());
        p.0.extend(7u16.to_le_bytes());
        p.0.extend(kind.to_le_bytes());
        p.0.extend(1u16.to_le_bytes());
        p
    }
    pub fn id(&mut self, n: u8) {
        self.0.extend([n; 16]);
    }
    pub fn ordinal(&mut self, n: u32) {
        self.0.extend(n.to_le_bytes());
        self.0.extend([0; 12]);
    }
    pub fn digest(&mut self, n: u8) {
        self.0.extend([n; 32]);
    }
    pub fn u32(&mut self, n: u32) {
        self.0.extend(n.to_le_bytes());
    }
    pub fn u64(&mut self, n: u64) {
        self.0.extend(n.to_le_bytes());
    }
    pub fn bytes(&mut self, b: &[u8]) {
        self.u32(b.len() as u32);
        self.0.extend_from_slice(b);
    }
}
pub fn large_snapshot() -> Vec<u8> {
    let mut s = Packet::new(1);
    s.id(1);
    s.id(2);
    s.digest(3);
    s.digest(4);
    s.u64(2048);
    s.u64(2048);
    for _ in 0..3 {
        s.u32(0);
    }
    s.u32(32);
    for p in 1..=32 {
        s.id(p);
        s.id(p + 100);
        s.u64(0);
        s.u64(0);
    }
    s.u32(32);
    for p in 1..=32 {
        s.id(p);
        s.digest(p);
        s.u32(1);
        s.u32(1);
        s.u32(2);
        s.u32(3);
        for n in [1, 3, 4] {
            s.u32(n);
        }
        s.u32(2);
        for n in [3, 4] {
            s.u32(n);
        }
        s.u32(1);
        s.u32(3);
        s.u32(64);
        for a in 0..64 {
            s.id(a);
        }
        s.u32(2);
    }
    s.u32(32);
    for p in 1..=32 {
        s.id(p + 100);
        s.id(p);
        s.u64(1);
        s.u64(0);
        s.digest(4);
    }
    s.u32(32);
    for p in 1..=32 {
        s.id(p + 100);
        s.u64(0);
    }
    s.u32(4096);
    for n in 0..4096 {
        s.ordinal(n);
        s.ordinal(n);
        s.u32(11);
    }
    s.0
}
pub fn excessive_frontier(truncated: bool) -> Vec<u8> {
    frontier(64, truncated, true)
}
pub fn bounded_frontier() -> Vec<u8> {
    frontier(61, false, false)
}
fn frontier(count: u8, truncated: bool, malformed: bool) -> Vec<u8> {
    let snapshot = large_snapshot();
    let mut f = Packet::new(3);
    f.bytes(&snapshot);
    f.0.extend(Sha256::digest(&snapshot));
    f.u64(1);
    f.u64(1);
    f.u32(u32::from(count));
    for n in 1..=count {
        let mut i = Packet::new(4);
        for value in [n, n, n, 90, 1, 2, 1] {
            i.id(value);
        }
        i.u32(160);
        if truncated && n == 62 {
            f.bytes(&i.0);
            break;
        }
        for a in 0..160 {
            i.ordinal(a);
            i.u64(0);
        }
        i.u32(if malformed && n == 64 { u32::MAX } else { 3 });
        i.id(200);
        i.u64(0);
        f.bytes(&i.0);
    }
    f.0
}
pub fn excessive_combined_entities() -> Vec<u8> {
    let mut s = Packet::new(1);
    s.id(1);
    s.id(2);
    s.digest(3);
    s.digest(4);
    s.u64(0);
    s.u64(0);
    s.u32(128);
    for n in 0..128 {
        s.id(n);
        s.id(n);
    }
    s.u32(1);
    s.0
}
