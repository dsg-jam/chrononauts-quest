use std::{pin::pin, time::Duration};

use backend_api::labyrinth::Direction;
use esp_idf_svc::{
    hal::{delay::BLOCK, i2c::I2cDriver, task::block_on},
    timer::EspTaskTimerService,
};

use crate::{
    consts::{
        ACCEL_FETCH_INTERVAL_MS, ACCEL_I2C_ADDR, ACCEL_ORIENTATION_REG, ACCEL_WHO_AM_I_REG,
        ACCEL_WHO_AM_I_VAL,
    },
    event::MainEvent,
    ChrononautsEventLoop,
};

#[derive(Debug, thiserror::Error)]
pub enum AccelerometerError {
    #[error(transparent)]
    I2cError(#[from] esp_idf_svc::hal::i2c::I2cError),
    #[error(transparent)]
    EspError(#[from] esp_idf_svc::sys::EspError),
    #[error("Invalid WHO AM I")]
    InvalidWhoAmI,
    #[error("Invalid orientation")]
    InvalidOrientation,
}

pub struct ChrononautsAccelerometer {
    i2c_driver: I2cDriver<'static>,
    event_loop: ChrononautsEventLoop,
}

impl ChrononautsAccelerometer {
    pub fn new(i2c_driver: I2cDriver<'static>, event_loop: ChrononautsEventLoop) -> Self {
        Self {
            i2c_driver,
            event_loop,
        }
    }

    pub fn check_availability(&mut self) -> Result<(), AccelerometerError> {
        let mut buf = [0; 1];
        self.i2c_driver
            .write_read(ACCEL_I2C_ADDR, &[ACCEL_WHO_AM_I_REG], &mut buf, BLOCK)?;
        if buf[0] != ACCEL_WHO_AM_I_VAL {
            return Err(AccelerometerError::InvalidWhoAmI);
        }
        Ok(())
    }

    /// Read the direction of the accelerometer and returns the equivalent `Direction` enum
    fn read_direction(&mut self) -> Result<Direction, AccelerometerError> {
        let mut data_buf = [0; 1];
        let reg_buf = [ACCEL_ORIENTATION_REG; 1];

        self.i2c_driver
            .write_read(ACCEL_I2C_ADDR, &reg_buf, &mut data_buf, BLOCK)?;
        let orientation_xy = (data_buf[0] >> 4) & 0b11;
        let direction = match orientation_xy {
            0b00 => Direction::Down,
            0b01 => Direction::Right,
            0b10 => Direction::Up,
            0b11 => Direction::Left,
            _ => return Err(AccelerometerError::InvalidOrientation),
        };

        Ok(direction)
    }

    pub fn run(&mut self) -> Result<(), AccelerometerError> {
        self.check_availability()?;

        let timer_service = EspTaskTimerService::new()?;
        block_on(pin!(async move {
            let mut async_timer = timer_service.timer_async()?;
            let mut last_direction = self.read_direction()?;
            self.event_loop.post::<MainEvent>(
                &MainEvent::AccelerometerDirectionChanged(last_direction),
                BLOCK,
            )?;
            loop {
                async_timer
                    .after(Duration::from_millis(ACCEL_FETCH_INTERVAL_MS))
                    .await
                    .unwrap();
                let direction = self.read_direction().unwrap();
                if direction == last_direction {
                    continue;
                }
                last_direction = direction;
                self.event_loop.post::<MainEvent>(
                    &MainEvent::AccelerometerDirectionChanged(direction),
                    BLOCK,
                )?;
            }
        }))
    }
}
