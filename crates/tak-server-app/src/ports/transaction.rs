use crate::ports::RepoError;

#[async_trait::async_trait]
pub trait TransactionManager<T> {
    async fn execute<R>(&self, item: impl TransactionItem<T, R>) -> Result<R, RepoError>;
}

#[async_trait::async_trait]
pub trait TransactionItem<T, Out> {
    async fn execute(&self, ctx: &T) -> Result<Out, RepoError>;
}

pub struct TransactionSequence<T, Intermediate, Out> {
    pub first: Box<dyn TransactionItem<T, Intermediate> + Send + Sync>,
    pub second: Box<dyn TransactionItem<T, Out> + Send + Sync>,
}

#[async_trait::async_trait]
impl<T: Send + Sync, Intermediate: Send + Sync, Out: Send + Sync> TransactionItem<T, ()>
    for TransactionSequence<T, Intermediate, Out>
{
    async fn execute(&self, ctx: &T) -> Result<(), RepoError> {
        let intermediate = self.first.execute(ctx).await?;
        self.second.execute(ctx).await?;
        Ok(())
    }
}
