#![cfg_attr(not(any(test, feature = "std")), no_std)]
//! # DRV8301 Gate Driver Interface
//!
//! This crate provides a bisync-based driver for the DRV8301 three-phase gate driver IC,
//! built upon the `device-driver` crate for robust, declarative register
//! definitions via a YAML manifest. It supports both asynchronous (`async`)
//! and blocking operation through a unified API, using the [`bisync`](https://docs.rs/bisync) crate
//! for seamless compatibility with both `embedded-hal` and `embedded-hal-async` traits.
//!
//! ## Features
//!
//! *   **Declarative Register Map:** Full device configuration defined in `device.yaml`.
//! *   **Unified Async/Blocking Support:** Write your code once and use it in both async and blocking contexts via bisync.
//! *   **Type-Safe API:** High-level functions for common operations (e.g., setting overcurrent thresholds)
//!     and a generated low-level API (`ll`) for direct register access.
//! *   **Comprehensive Register Coverage:** Supports the full feature set of the DRV8301.
//! *   **`defmt` and `log` Integration:** Optional support for logging and debugging.
//!
//! ## Getting Started
//!
//! To use the driver, instantiate `Drv8301` (blocking) or `Drv8301Async` (async) with your SPI bus implementation:
//!
//! ```rust,no_run
//! # use embedded_hal::spi::SpiDevice;
//! # use drv8301_dd::Drv8301;
//! let spi_device = todo!();
//! let mut drv = Drv8301::new(spi_device);
//!
//! // Check for faults
//! let has_fault = drv.has_fault()?;
//! # Ok::<(), drv8301_dd::DrvError<()>>(())
//! ```
//!
//! For async environments, use `Drv8301Async` (re-exported from the `asynchronous` module):
//!
//! ```rust,no_run
//! # use embedded_hal_async::spi::SpiDevice;
//! # use drv8301_dd::Drv8301Async;
//! let spi_device = todo!();
//! let mut drv = Drv8301Async::new(spi_device);
//!
//! // Check for faults
//! let has_fault = drv.has_fault().await?;
//! # Ok::<(), drv8301_dd::DrvError<()>>(())
//! ```
//!
//! For a detailed register map, please refer to the `device.yaml` file in the
//! [repository](https://github.com/okhsunrog/drv8301-dd).
//!
//! ## Warning!
//!
//! ***Caution!*** This chip controls power FETs that can handle high voltages and currents.
//! Incorrect configuration can potentially damage FETs, motors, or other components.
//! Proceed with care and always consult the DRV8301 datasheet.

#[macro_use]
pub(crate) mod fmt;

use thiserror::Error;

device_driver::create_device!(device_name: DrvLowLevel, manifest: "device.yaml");

#[derive(Debug, Error)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum DrvError<SpiErr> {
    #[error("SPI error")]
    Spi(SpiErr),
    #[error("SPI frame error detected in response")]
    FrameError,
    #[error("Feature or specific mode not supported/implemented: {0}")]
    NotSupported(&'static str),
}

/// Complete fault status from both DRV8301 status registers
///
/// This struct provides a comprehensive view of all fault conditions
/// reported by the DRV8301 gate driver IC.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct FaultStatus {
    /// Master fault indicator (OR of all fault conditions)
    pub fault: bool,
    /// GVDD undervoltage fault (GVDD < ~8V)
    pub gvdd_uv: bool,
    /// GVDD overvoltage fault (GVDD > ~16V)
    pub gvdd_ov: bool,
    /// PVDD undervoltage fault (PVDD < ~5.9V)
    pub pvdd_uv: bool,
    /// Overtemperature shutdown (die temp > ~150°C)
    pub otsd: bool,
    /// Overtemperature warning (die temp > ~130°C)
    pub otw: bool,
    /// Phase A high-side FET overcurrent
    pub fetha_oc: bool,
    /// Phase A low-side FET overcurrent
    pub fetla_oc: bool,
    /// Phase B high-side FET overcurrent
    pub fethb_oc: bool,
    /// Phase B low-side FET overcurrent
    pub fetlb_oc: bool,
    /// Phase C high-side FET overcurrent
    pub fethc_oc: bool,
    /// Phase C low-side FET overcurrent
    pub fetlc_oc: bool,
}

impl FaultStatus {
    /// Returns true if any overcurrent fault is active on any phase
    pub fn has_overcurrent(&self) -> bool {
        self.fetha_oc
            || self.fetla_oc
            || self.fethb_oc
            || self.fetlb_oc
            || self.fethc_oc
            || self.fetlc_oc
    }

    /// Returns true if any thermal condition (warning or shutdown) is active
    pub fn has_thermal(&self) -> bool {
        self.otsd || self.otw
    }

    /// Returns true if any voltage fault (UV or OV) is active
    pub fn has_voltage_fault(&self) -> bool {
        self.gvdd_uv || self.gvdd_ov || self.pvdd_uv
    }

    /// Returns true if no faults are active
    pub fn is_ok(&self) -> bool {
        !self.fault
    }

    /// Returns true if any phase A FET has an overcurrent fault
    pub fn phase_a_overcurrent(&self) -> bool {
        self.fetha_oc || self.fetla_oc
    }

    /// Returns true if any phase B FET has an overcurrent fault
    pub fn phase_b_overcurrent(&self) -> bool {
        self.fethb_oc || self.fetlb_oc
    }

    /// Returns true if any phase C FET has an overcurrent fault
    pub fn phase_c_overcurrent(&self) -> bool {
        self.fethc_oc || self.fetlc_oc
    }
}

pub struct DrvInterface<SpiBus> {
    spi_bus: SpiBus,
}

impl<SpiBus> DrvInterface<SpiBus> {
    pub fn new(spi_bus: SpiBus) -> Self {
        Self { spi_bus }
    }
}

#[path = "."]
mod asynchronous {
    use bisync::asynchronous::*;
    use device_driver::AsyncRegisterInterface as RegisterInterface;
    use embedded_hal_async::spi::SpiDevice;
    mod driver;
    pub use driver::*;
}
pub use asynchronous::Drv8301 as Drv8301Async;

#[path = "."]
mod blocking {
    use bisync::synchronous::*;
    use device_driver::RegisterInterface;
    use embedded_hal::spi::SpiDevice;
    #[allow(clippy::duplicate_mod)]
    mod driver;
    pub use driver::*;
}
pub use blocking::Drv8301;
