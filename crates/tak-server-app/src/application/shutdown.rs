#[async_trait::async_trait]
pub trait ShutdownUseCase {
    async fn shutdown(&self);
}

pub struct ShutdownUseCaseImpl {}

impl ShutdownUseCaseImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl ShutdownUseCase for ShutdownUseCaseImpl {
    async fn shutdown(&self) {
        //TODO: Send message to players that the server is shutting down
    }
}
