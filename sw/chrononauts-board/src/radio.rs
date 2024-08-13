use core::{fmt, str};
use std::{
    cmp::min, fmt::{Display, Formatter}, thread::sleep, time::Duration
};

use cc1101::{Cc1101, Error};
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver, SpiError};

// Maximum packet size is 61 bytes (64 - 3 bytes for length and RSSI/LQI)
const MAX_PACKET_SIZE: usize = 61;

const RADIO_FREQUENCY_HZ: u64 = 433_920_000;


#[derive(Debug, thiserror::Error)]
pub enum RadioError {
    EmptyPayload,
    RadioNotFound,
    #[error(transparent)]
    SpiError(#[from] Error<SpiError>),
}

impl Display for RadioError {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            RadioError::EmptyPayload => write!(f, "Empty payload"),
            RadioError::RadioNotFound => write!(f, "Radio not found"),
            RadioError::SpiError(e) => write!(f, "SPI error: {}", e),
        }
    }
}

pub struct ChrononautsRadio<'a>(Cc1101<SpiDeviceDriver<'a, SpiDriver<'a>>>);

impl<'a> ChrononautsRadio<'a> {
    pub fn new(cc1101: Cc1101<SpiDeviceDriver<'a, SpiDriver<'a>>>) -> Self {
        ChrononautsRadio(cc1101)
    }

    pub fn init_radio(&mut self) -> Result<(), RadioError> {
        // Reset the radio
        self.0.reset_chip()?;

        sleep(Duration::from_millis(5000));

        // First check if the radio is working
        let (_, version) = self.0.get_hw_info()?;

        if version < 0x14 || version == 0xFF {
            log::info!(
                "Radio not found - should be >= 0x14 but got 0x{:X}",
                version
            );
            return Err(RadioError::RadioNotFound);
        }

        log::info!("Radio found - version 0x{:X}", version);

        self.0.set_idle_state()?;

        self.init_common_registers()?;

        self.0.set_idle_state()?;

        self.0.white_data_enable(true)?;
        self.0.crc_enable(true)?;
        self.0
            .set_packet_length(cc1101::PacketLength::Variable(MAX_PACKET_SIZE as u8))?;

        self.0.set_channel_number(0)?;
        self.0.set_frequency(RADIO_FREQUENCY_HZ)?;

        self.0.set_data_rate(4800)?;

        self.0.set_deviation(0xc60000)?;

        self.0.set_idle_state()?;
        self.0
            .set_sync_mode(cc1101::SyncMode::MatchPartialRepeatedCS(0xD391))?;
        self.0
            .set_modulation_format(cc1101::ModulationFormat::GaussianFrequencyShiftKeying)?;
        
        // Sets the IF frequency (radio MUST be in IDLE state)
        self.0.set_idle_state()?;
        self.0.set_freq_if(152_343)?;

        self.0.set_rx_state()?;

        self.0.set_power(cc1101::Power::Power5Dbm)?;

        self.0.set_idle_state()?;

        self.0.set_pqt(4)?;
        self.0.append_status_enable(true)?;

        self.0.set_rx_state()?;

        Ok(())
    }

    fn init_common_registers(&mut self) -> Result<(), RadioError> {
        // Asserts when sync word has been sent / received, and de-asserts at the end of the packet
        self.0.set_gdo0_cfg(cc1101::Gdo0Cfg::SyncWord)?;

        // Set Fifo threshold to 1 byte in TX and 64 bytes in RX
        self.0
            .set_fifo_threshold(cc1101::FifoThreshold::TX_1_RX_64)?;
        
        // Enable ADC retention mode
        self.0.adc_retention_enable(true)?;

        // Auto calibration from IDLE to RX/TX
        self.0
            .set_autocalibration(cc1101::AutoCalibration::FromIdle)?;
        
        // Wait ~150 us for the crystal oscillator to stabilize (Ripple counter must expire 64 times)
        self.0.set_po_timeout(cc1101::PoTimeout::EXPIRE_COUNT_64)?;

        // Demidulator freeze disabled
        self.0.demodulator_freeze_enable(false)?;

        // Reduces the maximum allowable DVGA gain. Restricts the use of all gain settings except the highest gain setting
        self.0.set_max_dvga_gain(cc1101::DVGASetting::AllButHighest)?;

        self.0.set_wor_res(3)?;

        self.0.set_fscal3(3)?;
        self.0.vco_core_enable(true)?;
        self.0.set_fscal1(0x00)?;
        self.0.set_fscal0(0x1F)?;
        self.0.set_test2(0x81)?;
        self.0.set_test1(0x35)?;

        self.0.vco_sel_cal_enable(false)?;

        Ok(())
    }

    pub fn send_packet(&mut self, msg: &mut [u8]) -> Result<(), RadioError> {
        let mut size = msg.len();

        if size < 1 {
            return Err(RadioError::EmptyPayload);
        }

        size = min(size, MAX_PACKET_SIZE);

        self.0.transmit(msg, size as u8)?;

        Ok(())
    }

    pub fn get_packet(&mut self) -> Result<([u8; MAX_PACKET_SIZE], u8), RadioError> {
        let mut buf = [0; MAX_PACKET_SIZE];
        let mut length = 0u8;
        let ret = self.0.receive(&mut length, &mut buf)?;
        if let Ok(payload) = str::from_utf8(&buf) {
            // from TI app note
            let rssi_dec = ret[0] as i16;
            let rssi_offset = 74;
            let rssi_dbm = if rssi_dec >= 128 {
                ((rssi_dec - 256) / 2) - rssi_offset
            } else {
                (rssi_dec / 2) - rssi_offset
            };

            log::info!("Packet received: {:?}, size {:?}", payload, length);
            log::info!("RSSI: {:?}", rssi_dbm);
            log::info!("LQI: {:?}", ret[1] & 0x7F);
        }

        self.0.set_idle_state()?;
        self.0.flush_rx_fifo_buffer()?;
        self.0.set_rx_state()?;
        Ok((buf, length))
    }
}
