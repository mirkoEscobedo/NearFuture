# Opaque resource scope privacy contract

Positive shared root and nested syntax:

```rust
use nf_contract::canonical::replica_budget::*;
let limits = ReplicaDecodeLimits::new(2, 4, 8).unwrap();
let copied = limits.with_scope::<_, (), _>(|scope| {
    scope.charge_copied(2).map_err(ScopedError::Budget)?;
    scope.nested(|child| {
        child.charge_entries(1).map_err(ScopedError::Budget)?;
        Ok(())
    })?;
    Ok(scope.usage().copied_bytes)
}).unwrap();
assert_eq!(copied, 2);
```

Private core reset is inaccessible:

```compile_fail
use nf_contract::canonical::replica_budget::*;
ReplicaDecodeLimits::new(2, 4, 8).unwrap().with_scope::<_, (), _>(|scope| {
    scope.core.usage.set(ReplicaUsage { entries: 0, copied_bytes: 0, depth: 1 });
    Ok(())
});
```

No reset method:

```compile_fail
use nf_contract::canonical::replica_budget::*;
ReplicaDecodeLimits::new(2, 4, 8).unwrap().with_scope::<_, (), _>(|scope| {
    scope.reset();
    Ok(())
});
```

A shared scope cannot be replaced:

```compile_fail
use nf_contract::canonical::replica_budget::*;
ReplicaDecodeLimits::new(2, 4, 8).unwrap().with_scope::<_, (), _>(|scope| {
    core::mem::replace(scope, scope);
    Ok(())
});
```

No Clone or Default implementation:

```compile_fail
use nf_contract::canonical::replica_budget::*;
ReplicaDecodeLimits::new(2, 4, 8).unwrap().with_scope::<_, (), _>(|scope| {
    let owned: ReplicaDecodeScope<'_> = scope.clone();
    Ok(owned)
});
```

```compile_fail
use nf_contract::canonical::replica_budget::*;
let forged = ReplicaDecodeScope::default();
```

The root cannot escape HRTB invocation:

```compile_fail
use nf_contract::canonical::replica_budget::*;
let escaped = ReplicaDecodeLimits::new(2, 4, 8).unwrap()
    .with_scope::<_, (), _>(|scope| Ok(scope));
```

Independent invariant brands cannot splice work:

```compile_fail
use core::marker::PhantomData;
use nf_contract::canonical::replica_budget::*;
struct Part<'id>(PhantomData<fn(&'id mut ()) -> &'id mut ()>);
fn part<'id>(_: &ReplicaDecodeScope<'id>) -> Part<'id> { Part(PhantomData) }
fn use_part<'id>(_: Part<'id>, _: &ReplicaDecodeScope<'id>) {}
let limits = ReplicaDecodeLimits::new(2, 4, 8).unwrap();
limits.with_scope::<_, (), _>(|outer| {
    limits.with_scope::<_, (), _>(|inner| {
        use_part(part(outer), inner);
        Ok(())
    })
});
```

Budget failure has no semantic conversion:

```compile_fail
use nf_contract::canonical::replica_budget::*;
fn rejected(scope: &ReplicaDecodeScope<'_>) -> Result<(), &'static str> {
    scope.charge_entries(1)?;
    Ok(())
}
```
