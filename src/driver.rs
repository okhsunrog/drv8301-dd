use super::{RegisterInterface, SpiDevice, bisync, only_async, only_sync};
use crate::{DrvError, DrvInterface, DrvLowLevel, FaultStatus};
use crate::{GateCurrent, OcAdjSet, OcpMode, OctwMode, ShuntAmplifierGain};

#[bisync]
impl<SpiBus, E> RegisterInterface for DrvInterface<SpiBus>
where
    SpiBus: SpiDevice<Error = E>,
    E: core::fmt::Debug,
{
    type AddressType = u8;
    type Error = DrvError<E>;

    async fn read_register(
        &mut self,
        address: u8,
        _size_bits: u32,
        data: &mut [u8],
    ) -> Result<(), Self::Error> {
        // Build read command: bit 15 = 1 (read), bits 14:11 = address, bits 10:0 = don't care
        let cmd: u16 = 0x8000 | ((address as u16 & 0x0F) << 11);
        let cmd_bytes = cmd.to_be_bytes();

        // First transaction: send read command
        let mut response_bytes = [0u8; 2];
        self.spi_bus
            .transfer(&mut response_bytes, &cmd_bytes)
            .await
            .map_err(DrvError::Spi)?;

        // Second transaction: send same command to get actual data (N+1 timing)
        let mut read_response = [0u8; 2];
        self.spi_bus
            .transfer(&mut read_response, &cmd_bytes)
            .await
            .map_err(DrvError::Spi)?;

        let response = u16::from_be_bytes(read_response);

        // Check for frame error (bit 15 = 1 in response)
        if (response & 0x8000) != 0 {
            return Err(DrvError::FrameError);
        }

        // Extract 11-bit data and store in output buffer (big-endian)
        let reg_data = response & 0x07FF;
        if data.len() >= 2 {
            data[0] = (reg_data >> 8) as u8;
            data[1] = reg_data as u8;
        }

        Ok(())
    }

    async fn write_register(
        &mut self,
        address: u8,
        _size_bits: u32,
        data: &[u8],
    ) -> Result<(), Self::Error> {
        // Extract 11-bit data from buffer (big-endian)
        let reg_data = if data.len() >= 2 {
            ((data[0] as u16) << 8) | (data[1] as u16)
        } else if data.len() == 1 {
            data[0] as u16
        } else {
            0
        };

        // Build write command: bit 15 = 0 (write), bits 14:11 = address, bits 10:0 = data
        let cmd: u16 = ((address as u16 & 0x0F) << 11) | (reg_data & 0x07FF);
        let cmd_bytes = cmd.to_be_bytes();

        // Execute write transaction
        let mut response_bytes = [0u8; 2];
        self.spi_bus
            .transfer(&mut response_bytes, &cmd_bytes)
            .await
            .map_err(DrvError::Spi)?;

        Ok(())
    }
}

pub struct Drv8301<
    SpiImpl: RegisterInterface<AddressType = u8, Error = DrvError<SpiBusErr>>,
    SpiBusErr: core::fmt::Debug = <SpiImpl as RegisterInterface>::Error,
> {
    pub ll: DrvLowLevel<SpiImpl>,
    _marker: core::marker::PhantomData<SpiBusErr>,
}

impl<SpiBus, E> Drv8301<DrvInterface<SpiBus>, E>
where
    SpiBus: SpiDevice<Error = E>,
    E: core::fmt::Debug,
{
    pub fn new(spi: SpiBus) -> Self {
        Self {
            ll: DrvLowLevel::new(DrvInterface::new(spi)),
            _marker: core::marker::PhantomData,
        }
    }
}

pub trait CurrentDrvDriverInterface<E>:
    RegisterInterface<AddressType = u8, Error = DrvError<E>>
{
}

impl<T, E> CurrentDrvDriverInterface<E> for T
where
    T: RegisterInterface<AddressType = u8, Error = DrvError<E>>,
    E: core::fmt::Debug,
{
}

include!("bisync_helpers.rs");

impl<SpiImpl, SpiBusErr> Drv8301<SpiImpl, SpiBusErr>
where
    SpiImpl: CurrentDrvDriverInterface<SpiBusErr>,
    SpiBusErr: core::fmt::Debug,
{
    /// Check if any fault condition is active
    #[bisync]
    pub async fn has_fault(&mut self) -> Result<bool, DrvError<SpiBusErr>> {
        let mut op = self.ll.status_register_1();
        let status = read_internal(&mut op).await?;
        Ok(status.fault())
    }

    /// Get device ID from Status Register 2
    #[bisync]
    pub async fn get_device_id(&mut self) -> Result<u8, DrvError<SpiBusErr>> {
        let mut op = self.ll.status_register_2();
        let status = read_internal(&mut op).await?;
        Ok(status.device_id())
    }

    /// Get complete fault status from both status registers
    ///
    /// Returns a [`FaultStatus`] struct containing all fault flags from the DRV8301.
    /// This includes voltage faults, thermal conditions, and per-phase overcurrent status.
    ///
    /// # Example
    /// ```rust,no_run
    /// # use drv8301_dd::Drv8301;
    /// # let spi = todo!();
    /// # let mut drv = Drv8301::new(spi);
    /// let status = drv.get_fault_status()?;
    /// if status.has_overcurrent() {
    ///     // Handle overcurrent condition
    /// }
    /// if status.phase_a_overcurrent() {
    ///     // Phase A specific handling
    /// }
    /// # Ok::<(), drv8301_dd::DrvError<()>>(())
    /// ```
    #[bisync]
    pub async fn get_fault_status(&mut self) -> Result<FaultStatus, DrvError<SpiBusErr>> {
        let mut op1 = self.ll.status_register_1();
        let status1 = read_internal(&mut op1).await?;

        let mut op2 = self.ll.status_register_2();
        let status2 = read_internal(&mut op2).await?;

        Ok(FaultStatus {
            fault: status1.fault(),
            gvdd_uv: status1.gvdd_uv(),
            gvdd_ov: status2.gvdd_ov(),
            pvdd_uv: status1.pvdd_uv(),
            otsd: status1.otsd(),
            otw: status1.otw(),
            fetha_oc: status1.fetha_oc(),
            fetla_oc: status1.fetla_oc(),
            fethb_oc: status1.fethb_oc(),
            fetlb_oc: status1.fetlb_oc(),
            fethc_oc: status1.fethc_oc(),
            fetlc_oc: status1.fetlc_oc(),
        })
    }

    /// Set the overcurrent (VDS) threshold
    #[bisync]
    pub async fn set_oc_threshold(
        &mut self,
        threshold: OcAdjSet,
    ) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_1();
        modify_internal(&mut op, |r| r.set_oc_adj_set(threshold)).await
    }

    /// Set the overcurrent protection mode
    #[bisync]
    pub async fn set_ocp_mode(&mut self, mode: OcpMode) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_1();
        modify_internal(&mut op, |r| r.set_ocp_mode(mode)).await
    }

    /// Set PWM mode (6-PWM or 3-PWM)
    #[bisync]
    pub async fn set_pwm_mode(&mut self, three_pwm: bool) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_1();
        modify_internal(&mut op, |r| r.set_pwm_mode(three_pwm)).await
    }

    /// Reset gate driver faults
    #[bisync]
    pub async fn reset_gate_faults(&mut self) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_1();
        modify_internal(&mut op, |r| r.set_gate_reset(true)).await
    }

    /// Set the peak gate drive current
    #[bisync]
    pub async fn set_gate_current(
        &mut self,
        current: GateCurrent,
    ) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_1();
        modify_internal(&mut op, |r| r.set_gate_current(current)).await
    }

    /// Set the current shunt amplifier gain
    #[bisync]
    pub async fn set_shunt_amplifier_gain(
        &mut self,
        gain: ShuntAmplifierGain,
    ) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_2();
        modify_internal(&mut op, |r| r.set_gain(gain)).await
    }

    /// Set the nOCTW pin reporting mode
    #[bisync]
    pub async fn set_octw_mode(&mut self, mode: OctwMode) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_2();
        modify_internal(&mut op, |r| r.set_octw_mode(mode)).await
    }

    /// Enable or disable DC calibration mode for shunt amplifier channel 1
    #[bisync]
    pub async fn set_dc_cal_ch1(&mut self, enable: bool) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_2();
        modify_internal(&mut op, |r| r.set_dc_cal_ch1(enable)).await
    }

    /// Enable or disable DC calibration mode for shunt amplifier channel 2
    #[bisync]
    pub async fn set_dc_cal_ch2(&mut self, enable: bool) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_2();
        modify_internal(&mut op, |r| r.set_dc_cal_ch2(enable)).await
    }

    /// Set overcurrent off-time control mode
    #[bisync]
    pub async fn set_oc_toff(&mut self, off_time_control: bool) -> Result<(), DrvError<SpiBusErr>> {
        let mut op = self.ll.control_register_2();
        modify_internal(&mut op, |r| r.set_oc_toff(off_time_control)).await
    }
}
