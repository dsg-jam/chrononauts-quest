mod accelerometer;

pub use accelerometer::ChrononautsAccelerometer;

#[derive(Debug, thiserror::Error)]
pub enum PeripheralError {
    #[error(transparent)]
    AccelerometerError(#[from] accelerometer::AccelerometerError),
}
