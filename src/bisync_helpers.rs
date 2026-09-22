// bisync glue for the device-driver 2.x register operations.
//
// The high-level methods are written once with `#[bisync]` and call these
// helpers, which exist in an `#[only_sync]` and an `#[only_async]` flavor so the
// right (`read`/`read_async`, `modify`/`modify_async`) method is selected per
// build. `RegisterInterface` here is the per-flavor alias brought in by the
// enclosing module (sync `RegisterInterface` or `AsyncRegisterInterface`).
//
// Register operations are consumed by their read/write/modify methods, so the
// helpers take them by value.

#[allow(dead_code)]
#[only_sync]
fn read_internal<'a, B, RegisterFs, Access>(
    op: RegisterOperation<'a, B, RegisterFs, u8, Access, ()>,
) -> Result<RegisterFs, <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability,
{
    op.read()
}

#[allow(dead_code)]
#[only_async]
async fn read_internal<'a, B, RegisterFs, Access>(
    op: RegisterOperation<'a, B, RegisterFs, u8, Access, ()>,
) -> Result<RegisterFs, <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability,
{
    op.read_async().await
}

#[allow(dead_code)]
#[only_sync]
fn modify_internal<'a, B, RegisterFs, Access>(
    op: RegisterOperation<'a, B, RegisterFs, u8, Access, ()>,
    f: impl FnOnce(&mut RegisterFs),
) -> Result<(), <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability + WriteCapability,
{
    op.modify(f)
}

#[allow(dead_code)]
#[only_async]
async fn modify_internal<'a, B, RegisterFs, Access>(
    op: RegisterOperation<'a, B, RegisterFs, u8, Access, ()>,
    f: impl FnOnce(&mut RegisterFs),
) -> Result<(), <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability + WriteCapability,
{
    op.modify_async(f).await
}
