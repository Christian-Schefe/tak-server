pub trait DisconnectTimeoutRunner {}

pub struct DisconnectTimeoutRunnerImpl {}

impl DisconnectTimeoutRunner for DisconnectTimeoutRunnerImpl {}

impl DisconnectTimeoutRunnerImpl {
    pub fn new() -> Self {
        Self {}
    }
}
