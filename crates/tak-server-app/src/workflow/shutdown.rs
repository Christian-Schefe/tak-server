#[async_trait::async_trait]
pub trait ShutdownWorkflow {
    async fn shutdown(&self);
}

pub struct ShutdownWorkflowImpl {}

impl ShutdownWorkflowImpl {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl ShutdownWorkflow for ShutdownWorkflowImpl {
    async fn shutdown(&self) {
        //TODO: Send message to players that the server is shutting down
    }
}
