// bisync glue for the device-driver 2.x register operations.
//
// The high-level methods are written once with `#[bisync]` and call these
// helpers, which exist in an `#[only_sync]` and an `#[only_async]` flavor so the
// right (`read`/`read_async`, `modify`/`modify_async`) method is selected per
// build. `RegisterInterface` here is the per-flavor alias brought in by the
// enclosing module (sync `RegisterInterface` or `AsyncRegisterInterface`).

#[allow(dead_code)]
#[only_sync]
fn read_internal<'a, B, RegisterFs, Access, Repeat>(
    op: &mut RegisterOperation<'a, B, RegisterFs, u8, Access, Repeat>,
) -> Result<RegisterFs, <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability,
    Repeat: device_driver::NotRepeating,
{
    op.read()
}

#[allow(dead_code)]
#[only_async]
async fn read_internal<'a, B, RegisterFs, Access, Repeat>(
    op: &mut RegisterOperation<'a, B, RegisterFs, u8, Access, Repeat>,
) -> Result<RegisterFs, <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability,
    Repeat: device_driver::NotRepeating,
{
    op.read_async().await
}

#[allow(dead_code)]
#[only_sync]
fn modify_internal<'a, B, RegisterFs, Access, Repeat>(
    op: &mut RegisterOperation<'a, B, RegisterFs, u8, Access, Repeat>,
    f: impl FnOnce(&mut RegisterFs),
) -> Result<(), <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability + WriteCapability,
    Repeat: device_driver::NotRepeating,
{
    op.modify(f)
}

#[allow(dead_code)]
#[only_async]
async fn modify_internal<'a, B, RegisterFs, Access, Repeat>(
    op: &mut RegisterOperation<'a, B, RegisterFs, u8, Access, Repeat>,
    f: impl FnOnce(&mut RegisterFs),
) -> Result<(), <B::Interface as RegisterInterfaceBase>::Error>
where
    B: Block,
    B::Interface: RegisterInterface + RegisterInterfaceBase<AddressType = u8>,
    RegisterFs: Fieldset,
    Access: ReadCapability + WriteCapability,
    Repeat: device_driver::NotRepeating,
{
    op.modify_async(f).await
}
