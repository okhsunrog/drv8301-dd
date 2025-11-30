#![no_std]
#![no_main]

use defmt::info;
use drv8301_dd::{Drv8301Async, DrvError, OcAdjSet, OcpMode, ShuntAmplifierGain};
use embassy_embedded_hal::shared_bus::asynch::spi::SpiDevice;
use embassy_executor::Spawner;
use embassy_sync::blocking_mutex::raw::NoopRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_time::{Duration, Timer};
use esp_hal::{
    dma::{DmaRxBuf, DmaTxBuf},
    dma_buffers,
    gpio::{Level, Output, OutputConfig},
    interrupt::software::SoftwareInterruptControl,
    spi::{
        Mode,
        master::{Config as SpiConfig, Spi},
    },
    time::Rate,
    timer::timg::TimerGroup,
};
use panic_rtt_target as _;
use rtt_target::rtt_init_defmt;
use static_cell::StaticCell;

esp_bootloader_esp_idf::esp_app_desc!();

type SpiMutex = Mutex<NoopRawMutex, esp_hal::spi::master::SpiDmaBus<'static, esp_hal::Async>>;
static SPI_BUS: StaticCell<SpiMutex> = StaticCell::new();

#[esp_rtos::main]
async fn main(_spawner: Spawner) {
    rtt_init_defmt!();
    info!("Init!");

    let p = esp_hal::init(esp_hal::Config::default());

    let timg0 = TimerGroup::new(p.TIMG0);
    let sw_ints = SoftwareInterruptControl::new(p.SW_INTERRUPT);
    esp_rtos::start(timg0.timer0, sw_ints.software_interrupt0);

    // Configure SPI pins
    let sclk = p.GPIO6;
    let miso = p.GPIO5;
    let mosi = p.GPIO7;
    let cs = p.GPIO4;

    // Create CS pin as output
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
    .with_buffers(dma_rx_buf, dma_tx_buf)
    .into_async();

    // Create shared bus and wrap with SpiDevice
    let spi_bus = SPI_BUS.init(Mutex::new(spi));
    let spi_device = SpiDevice::new(spi_bus, cs_pin);

    init_drv(spi_device).await.unwrap();

    loop {
        info!("Hello world!");
        Timer::after(Duration::from_secs(1)).await;
    }
}

async fn init_drv<SPI, E>(spi: SPI) -> Result<(), DrvError<E>>
where
    SPI: embedded_hal_async::spi::SpiDevice<Error = E>,
    E: core::fmt::Debug,
{
    let mut drv = Drv8301Async::new(spi);

    info!("=== High-Level API Examples ===");

    // Read device ID (high-level API)
    info!("Device ID: {:#x}", drv.get_device_id().await?);

    // Check for faults (high-level API)
    let has_fault = drv.has_fault().await?;
    info!("Has fault: {}", has_fault);

    // Configure overcurrent threshold (high-level API)
    drv.set_oc_threshold(OcAdjSet::Vds250mV).await?;

    // Set overcurrent protection mode (high-level API)
    drv.set_ocp_mode(OcpMode::CurrentLimit).await?;

    // Set 6-PWM mode (high-level API)
    drv.set_pwm_mode(false).await?;

    // Set amplifier gain (high-level API)
    drv.set_shunt_amplifier_gain(ShuntAmplifierGain::Gain20)
        .await?;

    info!("=== Low-Level API Examples ===");

    // Read status register 1 using low-level API
    let status1 = drv.ll.status_register_1().read_async().await?;
    info!(
        "Status1 - Fault: {}, GVDD_UV: {}, OTW: {}",
        status1.fault(),
        status1.gvdd_uv(),
        status1.otw()
    );

    // Read status register 2 using low-level API
    let status2 = drv.ll.status_register_2().read_async().await?;
    info!(
        "Status2 - Device ID: {:#x}, GVDD_OV: {}",
        status2.device_id(),
        status2.gvdd_ov()
    );

    // Read control register 1 using low-level API
    let ctrl1 = drv.ll.control_register_1().read_async().await?;
    info!(
        "Ctrl1 - 3-PWM mode: {}, OC threshold raw: {}",
        ctrl1.pwm_mode(),
        ctrl1.oc_adj_set() as u8
    );

    // Modify control register 2 to enable DC calibration using low-level API
    drv.ll
        .control_register_2()
        .modify_async(|w| {
            w.set_dc_cal_ch1(true);
            w.set_dc_cal_ch2(true);
        })
        .await?;
    info!("DC calibration enabled via LL API");

    info!("DRV8301 configured!");

    Ok(())
}
