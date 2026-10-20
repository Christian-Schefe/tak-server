pub struct RepoError(pub String);

impl RepoError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

#[async_trait::async_trait]
pub trait TransactionManager<RpCtx> {
    fn simple(&self) -> RpCtx;
    async fn transaction(&self) -> Result<RpCtx, RepoError>;
    async fn commit(&self, ctx: RpCtx) -> Result<(), RepoError>;
    async fn rollback(&self, ctx: RpCtx) -> Result<(), RepoError>;
}
