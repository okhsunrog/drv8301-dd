#![no_std]
#![no_main]

use defmt::info;
use drv8301_dd::{Drv8301, DrvError, OcAdjSet, OcpMode, ShuntAmplifierGain};
use embedded_hal_bus::spi::ExclusiveDevice;
use esp_hal::{
    delay::Delay,
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
};
use panic_rtt_target as _;
use rtt_target::rtt_init_defmt;

esp_bootloader_esp_idf::esp_app_desc!();

#[esp_hal::main]
fn main() -> ! {
    rtt_init_defmt!();
    info!("Init!");

    let p = esp_hal::init(esp_hal::Config::default());

    // Configure SPI pins
    let sclk = p.GPIO6;
    let miso = p.GPIO5;
    let mosi = p.GPIO7;
    let cs = p.GPIO4;

    // Create CS pin as output (directly managed)
    let cs_pin = Output::new(cs, Level::High, OutputConfig::default());

    // Configure DMA buffers
    let (rx_buffer, rx_descriptors, tx_buffer, tx_descriptors) = dma_buffers!(256);
    let dma_rx_buf = DmaRxBuf::new(rx_descriptors, rx_buffer).unwrap();
    let dma_tx_buf = DmaTxBuf::new(tx_descriptors, tx_buffer).unwrap();

    // Configure SPI - DRV8301: CPOL=0, CPHA=1 (Mode 1), max 10MHz
    let spi = Spi::new(
        p.SPI2,
        SpiConfig::default()
            .with_frequency(Rate::from_mhz(1))
            .with_mode(Mode::_1),
    )
    .unwrap()
    .with_sck(sclk)
    .with_miso(miso)
    .with_mosi(mosi)
    .with_dma(p.DMA_CH0)
    .with_buffers(dma_rx_buf, dma_tx_buf);

    let delay = Delay::new();

    // Wrap SpiBus with ExclusiveDevice to get SpiDevice
    let spi_device = ExclusiveDevice::new(spi, cs_pin, delay).unwrap();

    init_drv(spi_device).unwrap();

    let delay = Delay::new();
    loop {
        info!("Hello world!");
        delay.delay_millis(250);
    }
}

fn init_drv<SPI, E>(spi: SPI) -> Result<(), DrvError<E>>
where
    SPI: embedded_hal::spi::SpiDevice<Error = E>,
    E: core::fmt::Debug,
{
    let mut drv = Drv8301::new(spi);

    // Read device ID
    info!("Device ID: {:#x}", drv.get_device_id()?);

    // Check for faults
    let has_fault = drv.has_fault()?;
    info!("Has fault: {}", has_fault);

    // Configure overcurrent threshold
    drv.set_oc_threshold(OcAdjSet::Vds250mV)?;

    // Set overcurrent protection mode
    drv.set_ocp_mode(OcpMode::CurrentLimit)?;

    // Set 6-PWM mode
    drv.set_pwm_mode(false)?;

    // Set amplifier gain
    drv.set_shunt_amplifier_gain(ShuntAmplifierGain::Gain20)?;

    info!("DRV8301 configured!");

    Ok(())
}
