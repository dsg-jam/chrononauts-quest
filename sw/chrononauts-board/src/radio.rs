use core::str;
use std::{thread::sleep, time::Duration};

use cc1101::Cc1101;
use esp_idf_svc::hal::spi::{SpiDeviceDriver, SpiDriver};

pub struct ChrononautsRadio<'a>(Cc1101<SpiDeviceDriver<'a, SpiDriver<'a>>>);

impl<'a> ChrononautsRadio<'a> {
    pub fn new(cc1101: Cc1101<SpiDeviceDriver<'a, SpiDriver<'a>>>) -> Self {
        ChrononautsRadio(cc1101)
    }

    pub fn init_radio(&mut self) {
        // Reset the radio
        self.0.reset_chip().unwrap();

        sleep(Duration::from_millis(5000));

        // First check if the radio is working
        let (_, version) = self.0.get_hw_info().unwrap();

        if version < 0x14 || version == 0xFF {
            // should exit here
            log::info!(
                "Radio not found - should be >= 0x14 but got 0x{:X}",
                version
            );
        }

        log::info!("Radio found - version 0x{:X}", version);

        self.0.set_idle_state().unwrap();

        // Set the radio to RX mode
        self.init_common_registers();

        self.0.set_idle_state().unwrap();

        self.0.white_data_enable(true).unwrap();
        self.0.crc_enable(true).unwrap();
        self.0.set_packet_length(cc1101::PacketLength::Variable(61)).unwrap();

        self.0.set_channel_number(0).unwrap();
        self.0.set_frequency(433_920_000).unwrap();

        self.0.set_data_rate(4800).unwrap();

        self.0
            .set_register(cc1101::lowlevel::registers::Config::DEVIATN, 0x40)
            .unwrap();

        self.0.set_idle_state().unwrap();
        self.0.set_sync_mode(cc1101::SyncMode::MatchPartialRepeatedCS(0xD391)).unwrap();
        self.0.set_modulation_format(cc1101::ModulationFormat::GaussianFrequencyShiftKeying).unwrap();
        self.0.set_freq_if(1024).unwrap();

        self.0.set_rx_state().unwrap();

        self.0.set_power(cc1101::Power::Power5Dbm).unwrap();

        //self.0.set_address_filter(cc1101::AddressFilter::Disabled).unwrap();
        self.0.set_idle_state().unwrap();

        self.0.set_pqt(4).unwrap();
        self.0.append_status_enable(true).unwrap();

        self.0.set_rx_state().unwrap();
    }

    fn init_common_registers(&mut self) {
        self.0.set_gdo0_cfg(cc1101::Gdo0Cfg::SyncWord).unwrap();

        self.0.set_fifo_threshold(cc1101::FifoThreshold::TX_1_RX_64).unwrap();
        self.0.adc_retention_enable(true).unwrap();

        self.0.set_autocalibration(cc1101::AutoCalibration::FromIdle).unwrap();
        self.0.set_po_timeout(cc1101::PoTimeout::EXPIRE_COUNT_64).unwrap();

        self.0.demodulator_freeze_enable(false).unwrap();

        self.0.set_max_dvga_gain(0x1).unwrap();

        self.0.set_wor_res(3).unwrap();

        self.0.set_fscal3(3).unwrap();
        self.0.vco_core_enable(true).unwrap();
        self.0.set_fscal1(0x00).unwrap();
        self.0.set_fscal0(0x1F).unwrap();
        self.0.set_test2(0x81).unwrap();
        self.0.set_test1(0x35).unwrap();

        self.0.vco_sel_cal_enable(false).unwrap();

    }

    pub fn send_packet(&mut self, msg: &mut [u8]) {
        let mut size = msg.len();

        if size < 1 {
            return;
        }

        if size > 61 {
            size = 61;
        }

        while let Err(e) = self.0.transmit(msg, size as u8) {
            log::info!("Error sending packet: {:?}", e);
            sleep(Duration::from_millis(100));
        }
    }

    pub fn get_packet(&mut self) -> ([u8; 61], u8) {
        let mut buf = [0; 61];
        let mut length = 0u8;
        let ret = self.0.receive(&mut length, &mut buf);
        if ret.is_err() {
            // await for the radio to be ready
            return (buf, 0);
        }
        let ret = ret.unwrap();
        if let Ok(a) = str::from_utf8(&buf) {
            log::info!("Received: {:?}, size {:?}", a, length);
            // from TI app note
            let rssi_dec = ret[0] as i16;
            let rssi_offset = 74;
            let rssi_dbm = if rssi_dec >= 128 {
                ((rssi_dec - 256) / 2) - rssi_offset
            } else {
                (rssi_dec / 2) - rssi_offset
            };
            log::info!("RSSI: {:?}", rssi_dbm);
            log::info!("LQI: {:?}", ret[1] & 0x7F);
        }

        self.0.set_idle_state().unwrap();
        self.0.flush_rx_fifo_buffer().unwrap();
        self.0.set_rx_state().unwrap();
        (buf, length)
    }
}
