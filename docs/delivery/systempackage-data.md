# Bounded SystemPackage data APIs

Refs #105, #33, #92. Source PR85 at
`12b05f69b732ad57c9918834bb5def6ed6ca764e` is composed onto dev
`0a531d7e150f0a94dc961abcb7118e81001245be`. Its latest original PR/push
workflows were cancelled overall. Historical focused passes, earlier green
parents, distinct storage/helper failures and unknown cancellation causes are
preserved; qualification requires the actual combined Windows/Linux candidate.

The matching Rust and Java APIs provide four data layers:

- NF-SPT-1 topology codecs retain semantic IDs, raw unsigned revisions,
  descriptive campaign dates and content dependencies. Admission rejects invalid
  references, cycles, noncanonical text and excessive sizes. Explicit selection
  refuses hidden parents/content; required missing content is a diagnostic.
- A universe-scoped placement registry retains bounded canonical rows. Updating
  the same history/owner/system key preserves its conceptual slot. An occupied
  slot request from another system is refused without changing the assignment.
  Admission uses the actual topology validator and hash.
- Bounded publication history owns immutable version records and explicit
  tombstones. Tombstones retain the referenced publication revision/hash;
  historical replay preserves the latest event and original reference across
  republication. Java returns defensive arrays and immutable list snapshots.
- Grid256V1 client presentation projects actual registry entries into declared
  integer domains with checked bounds, nominal footprint fit and owned source
  binding. Epoch identity includes namespace, fixed profile and all five domain
  fields. Coordinates describe a pure data projection.

All source owned-value, boundary, unsigned, conformance and layout cases remain
in the actual Rust package and Java check. Independent fixed topology literals
and hashes remain independent of production encoders. The source's focused
Rust/Java workflow steps are added before the existing public-build checks with
their original five-minute limits where declared. Existing complete Rust,
operator, interoperability, release and profile gates, their twenty-minute job
limits, strict qualification and job-local retained fixture setup remain intact.
Incomplete or cancelled required work is never counted as a pass.

The next consumer slice in #33 is decode -> required-content diagnosis ->
registry admission -> read-only projection. Synthetic fixtures must remain
labelled synthetic; actual campaign extraction needs its own source and
evidence. Rich export, content/privacy selection, authenticated current owner
admission, durable archive/recovery and the consumer itself remain incomplete.
No topology bytes or presentation coordinates grant native installation,
rendering/import, interaction, asset downloading or writer authority. Native
foreign-system creation remains behind G4. Root runtime stays build identity
only, with writes and incomplete capabilities disabled; whole #33 stays open.
