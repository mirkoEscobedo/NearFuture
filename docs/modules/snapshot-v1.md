# Read-only canonical inspection v1

The first probe artifact remains independent. Build this optional adapter with
`cargo build -p nf-sidecar --locked --offline` and enable
capability2 in the private parent bootstrap (3 enables both built-ins). The
same [probe-v1 envelope](probe-v1.md) and authentication transport apply unchanged.

Module2, operation1, API1, capability2 inspects supplied NF-CANON-1 snapshot
bytes with `nf_kernel::decode_snapshot`, then `nf_kernel::state_hash`. The
decoder already checks canonical byte-for-byte re-encoding, references and its
bounded collection budgets. The thin adapter first caps input at65,408 bytes,
smaller than the kernel's general1MiB limit, and checks the cooperative deadline
and cancellation before/between/after calls. The borrowed input is not copied
by the runtime decoder. The kernel remains unchanged, no_std and reusable.

Success payload is34 bytes: digest schema u16BE (=1), followed by the32-byte
canonical state SHA-256. Invalid/noncanonical/trailing/truncated inputs return
status12, with no error payload. Oversized input returns4 at the headless entry
point or rejects the frame before allocation at the negotiated transport bound.
An otherwise successful digest that exceeds a caller's negotiated response
capacity returns a bounded status4 instead of flushing an oversized reply.

`Host::new(session, probe)?.with_snapshot(true)?.dispatch(...)` is the headless
entry point. Both in-process and sidecar callers invoke that exact adapter.
The sidecar includes the selected adapter as a default Cargo feature so required
workspace tests exercise it. `cargo build -p nf-sidecar --no-default-features
--locked --offline` omits it and removes its kernel dependency from nf-modules;
omitting the runtime capability leaves probe service independent and unchanged.
A known disabled module returns7; negotiation requiring its absent capability
returns3. Capabilities grant no ambient filesystem, Store, network or game access.

This is inspection of caller-supplied bytes. It authenticates the local channel,
not the snapshot's game history or provenance. It performs no Store commit,
simulation, native write, history admission or world-authority activation.
