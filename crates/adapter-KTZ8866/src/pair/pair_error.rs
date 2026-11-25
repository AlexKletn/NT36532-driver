use ktz8866_driver::DriverError;

#[derive(Debug, PartialEq, Eq)]
pub enum PairError<L, B> {
    LedOnly(DriverError<L>),
    LedAndBias(DriverError<B>),
    BiasEnable {
        source: DriverError<B>,
        rollback: Option<DriverError<B>>,
    },
    LedOnlyConfiguration {
        source: DriverError<L>,
        rollback: Option<DriverError<B>>,
    },
}
