use std::pin::Pin;

pub struct TransactionError(pub String);

impl TransactionError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self(msg.into())
    }
}

#[async_trait::async_trait]
pub trait TransactionManager<T> {
    async fn execute<Out: Send + Sync + 'static>(
        &self,
        item: TransactionItem<T, Result<Out, TransactionError>>,
    ) -> Result<Out, TransactionError>;
    async fn execute_without_transaction<Out: Send + Sync + 'static>(
        &self,
        item: TransactionItem<T, Result<Out, TransactionError>>,
    ) -> Result<Out, TransactionError>;
}

pub struct TransactionItem<T, Out> {
    item: TransactionFn<T, Out>,
}

impl<T, Out> TransactionItem<T, Out> {
    pub fn new(item: TransactionFn<T, Out>) -> Self {
        Self { item }
    }
}

impl<T: Send + Sync + 'static, Out: Send + Sync + 'static> TransactionItem<T, Out> {
    pub async fn execute(self, ctx: &T) -> Out {
        (self.item)(ctx).await
    }
    pub fn constant(val: Out) -> TransactionItem<T, Out> {
        TransactionItem::new(Box::new(move |_ctx| Box::pin(async move { val })))
    }
}

impl<T: Send + Sync + 'static, Out: Send + Sync + 'static, Err: Send + Sync + 'static>
    TransactionItem<T, Result<Out, Err>>
{
    pub fn try_then<
        NewOut: Send + Sync + 'static,
        F: Fn(Out) -> Result<TransactionItem<T, Result<NewOut, Err>>, Err> + Send + Sync + 'static,
    >(
        self,
        f: F,
    ) -> TransactionItem<T, Result<NewOut, Err>> {
        TransactionItem::new(Box::new(move |ctx| {
            let item = (self.item)(ctx);
            Box::pin(async move {
                let intermediate = item.await?;
                (f(intermediate)?.item)(ctx).await
            })
        }))
    }
    pub fn try_parallel(
        items: impl IntoIterator<Item = TransactionItem<T, Result<Out, Err>>>,
    ) -> TransactionItem<T, Result<Vec<Out>, Err>> {
        let items = items.into_iter().collect::<Vec<_>>();
        TransactionItem::new(Box::new(move |ctx| {
            let items = items
                .into_iter()
                .map(|item| (item.item)(ctx))
                .collect::<Vec<_>>();
            Box::pin(async move {
                let results = futures::future::join_all(items).await;
                results.into_iter().collect::<Result<Vec<_>, _>>()
            })
        }))
    }
    pub fn map_value<
        NewOut: Send + Sync + 'static,
        F: Fn(Out) -> NewOut + Send + Sync + 'static,
    >(
        self,
        f: F,
    ) -> TransactionItem<T, Result<NewOut, Err>> {
        TransactionItem::new(Box::new(move |ctx| {
            let item = (self.item)(ctx);
            Box::pin(async move {
                let intermediate = item.await;
                match intermediate {
                    Ok(out) => Ok(f(out)),
                    Err(e) => Err(e),
                }
            })
        }))
    }
}

pub type TransactionFn<T, Out> =
    Box<dyn for<'a> FnOnce(&'a T) -> Pin<Box<dyn Future<Output = Out> + Send + 'a>> + Send>;
