mod accelerometer;
mod led;
mod potentiometer;

pub use accelerometer::ChrononautsAccelerometer;
pub use led::ChrononautsLed;
pub use potentiometer::ChrononautsPotentiometer;

#[derive(Debug, thiserror::Error)]
pub enum PeripheralError {
    #[error(transparent)]
    AccelerometerError(#[from] accelerometer::AccelerometerError),
    #[error(transparent)]
    LedError(#[from] led::LedError),
}
