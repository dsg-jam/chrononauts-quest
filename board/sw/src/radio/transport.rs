//! Chrononauts Reliable Transport Protocol.
//!
//! # Transportation
//!
//! This module contains the transportation layer for the Chrononauts board.
//! It is responsible for sending and receiving RELIABLE (to some extend) messages between TWO boards.
//! Please see the dedicated blog post [here](https://blog.chrononauts.quest/PATH-TO-BLOGPOST) for more information.
//!
//! ## Protocol
//! The protocol is window-based and uses a sliding window to keep track of sent and received messages.
//! The window size is fixed and can be configured in the `consts` module.
//! Each received message is acknowledged by the receiver via cumulative acknowledgements.
//! Because the medium inbetween sender and receive is unreliable, the sender will retransmit messages until they are acknowledged using a timeout mechanism.
//! The timeout is fixed and can be configured in the `consts` module. See Figure 1 for an overview of the protocol.
//!
//! [Figure 1](https://blog.chrononauts.quest/PATH-TO-FIGURE)
//! Figure 1: Overview of the Chrononauts Reliable Transport Protocol
//!

use std::{
    collections::VecDeque,
    pin::pin,
    sync::mpsc,
    time::{Duration, Instant},
};

use esp_idf_svc::{
    hal::{delay, task::block_on},
    sys::EspError,
    timer::EspTaskTimerService,
};

use crate::{
    communication::{ChrononautsMessage, ChrononautsPacket},
    consts,
    event::{MainEvent, MessageTransmissionEvent, PacketReceptionEvent, PacketTransmissionEvent},
    utils::ChrononautsId,
    ChrononautsEventLoop,
};

#[derive(Debug, thiserror::Error)]
pub enum TransportError {
    #[error(transparent)]
    EspError(#[from] EspError),
    #[error("Duplicate packet")]
    DuplicatePacket,
}

pub struct ChrononautsTransport {
    event_loop: ChrononautsEventLoop,
    sender: Sender,
    receiver: Receiver,
    source: ChrononautsId,
    destination: ChrononautsId,
    next_sequence: u8,
}

impl ChrononautsTransport {
    /// Create a new transport.
    pub fn new(event_loop: ChrononautsEventLoop, chrononauts_id: ChrononautsId) -> Self {
        Self {
            sender: Sender::new(event_loop.clone()),
            receiver: Receiver::new(event_loop.clone()),
            event_loop,
            source: chrononauts_id,
            destination: chrononauts_id.other(),
            next_sequence: 0,
        }
    }

    pub fn enqueue_message(&mut self, message: ChrononautsMessage) -> Result<(), TransportError> {
        // Create a new packet
        let package = ChrononautsPacket::from_message(
            self.source.into(),
            self.destination.into(),
            self.next_sequence,
            message,
        );

        self.next_sequence = self.next_sequence.wrapping_add(1);

        self.sender.enqueue_packet(package)?;
        Ok(())
    }

    pub fn handle_reception(
        &mut self,
        package: ChrononautsPacket,
    ) -> Result<Option<ChrononautsMessage>, TransportError> {
        if package.is_ack() {
            self.sender.handle_ack(package)?;
        } else {
            return self.receiver.handle_reception(package);
        }
        Ok(None)
    }

    pub fn handle_send(&mut self) -> Result<(), TransportError> {
        self.sender.handle_send()?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<(), TransportError> {
        let (packets_to_process_tx, packets_to_process_rx) = mpsc::channel::<ChrononautsPacket>();
        let (messages_to_process_tx, messages_to_process_rx) =
            mpsc::channel::<ChrononautsMessage>();

        let _packet_reception_sub = self
            .event_loop
            .subscribe::<PacketReceptionEvent, _>(move |event| {
                let PacketReceptionEvent::Packet(packet) = event;
                packets_to_process_tx.send(packet).unwrap();
            })
            .unwrap();

        let _message_transmission_sub = self
            .event_loop
            .subscribe::<MessageTransmissionEvent, _>(move |event| {
                let MessageTransmissionEvent::Message(message) = event;
                messages_to_process_tx.send(message).unwrap();
            })
            .unwrap();

        let timer_service = EspTaskTimerService::new()?;

        block_on(pin!(async move {
            let mut async_timer = timer_service.timer_async()?;

            loop {
                if let Ok(packet) = packets_to_process_rx.try_recv() {
                    if let Ok(Some(message)) = self.handle_reception(packet) {
                        self.event_loop
                            .post::<MainEvent>(&MainEvent::MessageReceived(message), delay::BLOCK)
                            .unwrap();
                    }
                }
                if let Ok(message) = messages_to_process_rx.try_recv() {
                    self.enqueue_message(message).unwrap();
                }

                self.handle_send()?;

                async_timer.after(Duration::from_millis(100)).await?;
            }
        }))
    }
}

/// Sender
pub struct Sender {
    queue: VecDeque<ChrononautsPacket>,
    window: VecDeque<ChrononautsPacket>,
    event_loop: ChrononautsEventLoop,
    timeout: Instant,
}

impl Sender {
    /// Create a new sender.
    pub fn new(event_loop: ChrononautsEventLoop) -> Self {
        Self {
            queue: VecDeque::with_capacity(consts::WINDOW_SIZE),
            window: VecDeque::with_capacity(consts::WINDOW_SIZE),
            timeout: Instant::now(),
            event_loop,
        }
    }

    pub fn enqueue_packet(&mut self, package: ChrononautsPacket) -> Result<(), TransportError> {
        self.queue.push_back(package);
        Ok(())
    }

    pub fn handle_send(&mut self) -> Result<(), TransportError> {
        // Check if the queue is empty
        if self.window.is_empty() && self.queue.is_empty() {
            return Ok(());
        }

        // Send next packets until the window is full
        while self.window.len() < consts::WINDOW_SIZE {
            // Check if the queue is empty
            if self.queue.is_empty() {
                break;
            }

            // Get the next package
            let packet = self.queue.pop_front().unwrap();

            // Send the package via event loop
            self.event_loop
                .post::<PacketTransmissionEvent>(
                    &PacketTransmissionEvent::Packet(packet),
                    delay::BLOCK,
                )
                .unwrap();

            // Add the package to the window
            self.window.push_back(packet);

            // Reset the timeout
            self.timeout = Instant::now();
        }

        // Check if the timeout has been reached
        if !self.window.is_empty()
            && self.timeout.elapsed().as_millis() >= consts::TIMEOUT_MSEC as u128
        {
            // Resend the first package
            let packet = self.window.front().unwrap();
            log::info!(
                "[TIMEOUT] Timeout reached - resending packet sequence {}",
                packet.get_sequence()
            );

            // Send the packet via event loop

            self.event_loop
                .post::<PacketTransmissionEvent>(
                    &PacketTransmissionEvent::Packet(*packet),
                    delay::BLOCK,
                )
                .unwrap();

            // Reset the timeout
            self.timeout = Instant::now();

            return Ok(());
        }

        Ok(())
    }

    fn handle_ack(&mut self, packet: ChrononautsPacket) -> Result<(), TransportError> {
        // Check if the window is empty
        if self.window.is_empty() {
            return Ok(());
        }

        // Check if the ACK is for the first packet in the window
        if self.window.front().unwrap().matches_sequence(&packet) {
            // Remove the first packet from the window
            self.window.pop_front();

            // Reset the timeout
            self.timeout = Instant::now();
        }

        Ok(())
    }
}

/// Receiver
pub struct Receiver {
    //window: VecDeque<ChrononautsPacket>,
    event_loop: ChrononautsEventLoop,
    last_received_sequence: Option<u8>,
}

impl Receiver {
    /// Create a new receiver.
    pub fn new(event_loop: ChrononautsEventLoop) -> Self {
        Self {
            //window: VecDeque::with_capacity(consts::WINDOW_SIZE),
            event_loop,
            last_received_sequence: None,
        }
    }

    /// Handle the reception of a packet.
    /// We assume each message is always contained in exactly one packet, thus we can accept packets in any order.
    /// The only thing we need to check is if the packet is already in the window.
    pub fn handle_reception(
        &mut self,
        packet: ChrononautsPacket,
    ) -> Result<Option<ChrononautsMessage>, TransportError> {
        // Check if the packet is in order
        let received_sequence = packet.get_sequence();

        /*
        let window_start = self.next_expected_sequence;
        let window_end = self.next_expected_sequence.wrapping_add(consts::WINDOW_SIZE as u8 - 1);
        if window_start <= window_end {
            if received_sequence < window_start || received_sequence > window_end {
                return Err(TransportError::InvalidSequenceNumber);
            }
        } else {
            if received_sequence < window_start && received_sequence > window_end {
                return Err(TransportError::InvalidSequenceNumber);
            }
        }
        */

        // Insert the packet into the window at the correct position
        // let position = (received_sequence.wrapping_sub(self.next_expected_sequence)) as usize;

        // This should never happen and thus it SHALL panic
        // assert!(position < consts::WINDOW_SIZE);

        /*
        // Check if packet is already in the window
        if self.window.get(position).is_some() {
            return Ok(());
        }

        if self.window.len() < consts::WINDOW_SIZE {
            self.window.push_back(packet);
        } else {
            self.window.insert(position, packet);
        }
        */

        // Increment the current sequence number
        // self.next_expected_sequence = self.next_expected_sequence.wrapping_add(1);

        // Send ACK for the received packet
        let ack_packet = ChrononautsPacket::new_ack_from(&packet);

        // Send the ACK via event loop
        self.event_loop
            .post::<PacketTransmissionEvent>(
                &PacketTransmissionEvent::Packet(ack_packet),
                delay::BLOCK,
            )
            .unwrap();

        // Check if the packet is a duplicate
        if self.last_received_sequence.is_some()
            && received_sequence == self.last_received_sequence.unwrap()
        {
            return Ok(None);
        }

        Ok(packet.get_payload())
    }
}
