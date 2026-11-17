use crate::errors::{AppResult, InternalError};

#[async_trait::async_trait]
pub trait TransactionManager<RpCtx> {
    fn simple(&self) -> RpCtx;
    async fn transaction(&self) -> AppResult<RpCtx, (InternalError,)>;
    async fn commit(&self, ctx: RpCtx) -> AppResult<(), (InternalError,)>;
    async fn rollback(&self, ctx: RpCtx) -> AppResult<(), (InternalError,)>;
}
