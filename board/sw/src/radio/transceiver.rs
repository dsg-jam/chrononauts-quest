use std::{cmp::min, pin::pin, sync::mpsc, thread::sleep, time::Duration};

use cc1101::Cc1101;
use esp_idf_svc::{
    hal::{
        delay,
        spi::{SpiDeviceDriver, SpiDriver},
        task::block_on,
    },
    timer::EspTaskTimerService,
};

use super::{ChrononautsPacket, RadioError};
use crate::{
    consts,
    event::{PacketReceptionEvent, PacketTransmissionEvent},
    utils::ChrononautsId,
    ChrononautsEventLoop,
};

pub struct ChrononautsTransceiver<'a> {
    radio: Cc1101<SpiDeviceDriver<'a, SpiDriver<'a>>>,
    event_loop: ChrononautsEventLoop,
}

impl<'a> ChrononautsTransceiver<'a> {
    pub fn new(
        radio: Cc1101<SpiDeviceDriver<'a, SpiDriver<'a>>>,
        event_loop: ChrononautsEventLoop,
    ) -> Self {
        ChrononautsTransceiver { radio, event_loop }
    }

    fn init(&mut self) -> Result<(), RadioError> {
        // Reset the radio
        self.radio.reset_chip()?;

        sleep(Duration::from_millis(5000));

        // First check if the radio is working
        let (_, version) = self.radio.get_hw_info()?;

        if version < 0x14 || version == 0xFF {
            log::error!(
                "Radio not found - should be >= 0x14 but got 0x{:X}",
                version
            );
            return Err(RadioError::RadioNotFound);
        }

        self.radio.set_idle_state()?;

        self.init_common_registers()?;

        self.radio.set_idle_state()?;

        self.radio.white_data_enable(true)?;
        self.radio.crc_enable(true)?;
        self.radio
            .set_packet_length(cc1101::PacketLength::Variable(
                consts::MAX_PACKET_SIZE as u8,
            ))?;

        self.radio.set_channel_number(0)?;
        self.radio.set_frequency(consts::RADIO_FREQUENCY_HZ)?;

        self.radio.set_data_rate(4800)?;

        self.radio.set_deviation(0xc60000)?;

        self.radio.set_idle_state()?;
        self.radio
            .set_sync_mode(cc1101::SyncMode::MatchPartialRepeatedCS(0xD391))?;
        self.radio
            .set_modulation_format(cc1101::ModulationFormat::GaussianFrequencyShiftKeying)?;

        // Sets the IF frequency (radio MUST be in IDLE state)
        self.radio.set_idle_state()?;
        self.radio.set_freq_if(152_343)?;

        self.radio.set_rx_state()?;

        self.radio.set_power(cc1101::PowerLevel::Power5Dbm)?;

        self.radio.set_idle_state()?;

        self.radio.set_pqt(4)?;
        self.radio.append_status_enable(true)?;

        self.radio.set_rx_state()?;

        Ok(())
    }

    fn init_common_registers(&mut self) -> Result<(), RadioError> {
        // Asserts when sync word has been sent / received, and de-asserts at the end of the packet
        self.radio.set_gdo0_cfg(cc1101::GdoCfg::SYNC_WORD)?;

        // Set Fifo threshold to 1 byte in TX and 64 bytes in RX
        self.radio
            .set_fifo_threshold(cc1101::FifoThreshold::TX_1_RX_64)?;

        // Enable ADC retention mode
        self.radio.adc_retention_enable(true)?;

        // Auto calibration from IDLE to RX/TX
        self.radio
            .set_autocalibration(cc1101::AutoCalibration::FromIdle)?;

        // Wait ~150 us for the crystal oscillator to stabilize (Ripple counter must expire 64 times)
        self.radio
            .set_po_timeout(cc1101::PoTimeout::EXPIRE_COUNT_64)?;

        // Demidulator freeze disabled
        self.radio.demodulator_freeze_enable(false)?;

        // Reduces the maximum allowable DVGA gain. Restricts the use of all gain settings except the highest gain setting
        self.radio
            .set_max_dvga_gain(cc1101::DVGASetting::AllButHighest)?;

        self.radio.set_wor_res(3)?;

        self.radio.set_fscal3(3)?;
        self.radio.vco_core_enable(true)?;
        self.radio.set_fscal1(0x00)?;
        self.radio.set_fscal0(0x1F)?;
        self.radio.set_test2(0x81)?;
        self.radio.set_test1(0x35)?;

        self.radio.vco_sel_cal_enable(false)?;

        Ok(())
    }

    fn send_packet(&mut self, packet: &ChrononautsPacket) -> Result<(), RadioError> {
        let mut packet = postcard::to_vec::<_, { consts::MAX_PACKET_SIZE }>(packet)?;
        let mut size = packet.len();

        if size < 1 {
            return Err(RadioError::EmptyPayload);
        }

        size = min(size, consts::MAX_PACKET_SIZE);

        self.radio.transmit(&mut packet, size as u8)?;

        Ok(())
    }

    fn get_packet(&mut self) -> Result<ChrononautsPacket, RadioError> {
        let mut buf = [0; consts::MAX_PACKET_SIZE];
        let mut length = 0u8;
        let ret = self.radio.receive(&mut length, &mut buf)?;

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

        self.radio.set_idle_state()?;
        self.radio.flush_rx_fifo_buffer()?;
        self.radio.set_rx_state()?;
        let packet = postcard::from_bytes(&buf[..length as usize])?;
        Ok(packet)
    }

    pub fn run(&mut self, chrononauts_id: ChrononautsId) -> Result<(), RadioError> {
        self.init().expect("Radio init failed");
        let (packets_to_send_tx, packets_to_send_rx) = mpsc::channel::<ChrononautsPacket>();

        let _packet_transmission_sub = self
            .event_loop
            .subscribe::<PacketTransmissionEvent, _>(move |event| {
                let PacketTransmissionEvent::Packet(packet) = event;
                log::info!("Packet transmission event: {:?}", packet);
                packets_to_send_tx.send(packet).unwrap();
            })
            .unwrap();

        let timer_service = EspTaskTimerService::new()?;

        block_on(pin!(async move {
            let mut async_timer = timer_service.timer_async()?;
            loop {
                if let Ok(packet) = self.get_packet() {
                    if packet.matches_destination(chrononauts_id.into()) {
                        self.event_loop
                            .post::<PacketReceptionEvent>(
                                &PacketReceptionEvent::Packet(packet),
                                delay::BLOCK,
                            )
                            .unwrap();
                    }
                }

                if let Ok(packet) = packets_to_send_rx.try_recv() {
                    if self.send_packet(&packet).is_err() {
                        log::error!("Failed to send packet");
                    }
                }

                async_timer.after(Duration::from_millis(20)).await?;
            }
        }))
    }
}
