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
}

pub struct TransactionItem<T, Out> {
    item: TransactionFn<T, Out>,
    needs_transaction: bool,
}

impl<T, Out> TransactionItem<T, Out> {
    pub fn simple(item: TransactionFn<T, Out>) -> Self {
        Self {
            item,
            needs_transaction: false,
        }
    }
    pub fn new(item: TransactionFn<T, Out>, needs_transaction: bool) -> Self {
        Self {
            item,
            needs_transaction,
        }
    }
}

impl<T: Send + Sync + 'static, Out: Send + Sync + 'static> TransactionItem<T, Out> {
    pub async fn execute(self, ctx: &T) -> Out {
        (self.item)(ctx).await
    }
    pub fn constant(val: Out) -> TransactionItem<T, Out> {
        TransactionItem::new(Box::new(move |_ctx| Box::pin(async move { val })), false)
    }
}

impl<T: Send + Sync + 'static, Out: Send + Sync + 'static, Err: Send + Sync + 'static>
    TransactionItem<T, Result<Out, Err>>
{
    pub fn try_then_ignore<
        NewOut: Send + Sync + 'static,
        F: Fn(&Out) -> TransactionItem<T, Result<NewOut, Err>> + Send + Sync + 'static,
    >(
        self,
        f: F,
    ) -> TransactionItem<T, Result<Out, Err>> {
        TransactionItem::new(
            Box::new(move |ctx| {
                let item = (self.item)(ctx);
                Box::pin(async move {
                    let first = item.await?;
                    let next = f(&first);
                    (next.item)(ctx).await?;
                    Ok(first)
                })
            }),
            true,
        )
    }
    pub fn try_then<
        NewOut: Send + Sync + 'static,
        F: Fn(Out) -> TransactionItem<T, Result<NewOut, Err>> + Send + Sync + 'static,
    >(
        self,
        f: F,
    ) -> TransactionItem<T, Result<NewOut, Err>> {
        TransactionItem::new(
            Box::new(move |ctx| {
                let item = (self.item)(ctx);
                Box::pin(async move {
                    let next = f(item.await?);
                    (next.item)(ctx).await
                })
            }),
            true,
        )
    }
    pub fn try_parallel(
        items: impl IntoIterator<Item = TransactionItem<T, Result<Out, Err>>>,
    ) -> TransactionItem<T, Result<Vec<Out>, Err>> {
        let items = items.into_iter().collect::<Vec<_>>();
        TransactionItem::new(
            Box::new(move |ctx| {
                let items = items
                    .into_iter()
                    .map(|item| (item.item)(ctx))
                    .collect::<Vec<_>>();
                Box::pin(async move {
                    let results = futures::future::join_all(items).await;
                    results.into_iter().collect::<Result<Vec<_>, _>>()
                })
            }),
            true,
        )
    }
    pub fn map_value<
        NewOut: Send + Sync + 'static,
        F: FnOnce(Out) -> NewOut + Send + Sync + 'static,
    >(
        self,
        f: F,
    ) -> TransactionItem<T, Result<NewOut, Err>> {
        TransactionItem::new(
            Box::new(move |ctx| {
                let item = (self.item)(ctx);
                Box::pin(async move {
                    match item.await {
                        Ok(out) => Ok(f(out)),
                        Err(e) => Err(e),
                    }
                })
            }),
            self.needs_transaction,
        )
    }
    pub fn flat_map_value<
        NewOut: Send + Sync + 'static,
        F: FnOnce(Out) -> Result<NewOut, Err> + Send + Sync + 'static,
    >(
        self,
        f: F,
    ) -> TransactionItem<T, Result<NewOut, Err>> {
        TransactionItem::new(
            Box::new(move |ctx| {
                let item = (self.item)(ctx);
                Box::pin(async move {
                    match item.await {
                        Ok(out) => f(out),
                        Err(e) => Err(e),
                    }
                })
            }),
            self.needs_transaction,
        )
    }
    pub fn try_simultaneous<NewOut: Send + Sync + 'static>(
        self,
        item: TransactionItem<T, Result<NewOut, Err>>,
    ) -> TransactionItem<T, Result<(Out, NewOut), Err>> {
        TransactionItem::new(
            Box::new(move |ctx| {
                let first_item = (self.item)(ctx);
                let second_item = (item.item)(ctx);
                Box::pin(async move { Ok((first_item.await?, second_item.await?)) })
            }),
            true,
        )
    }
}

pub type TransactionFn<T, Out> =
    Box<dyn for<'a> FnOnce(&'a T) -> Pin<Box<dyn Future<Output = Out> + Send + 'a>> + Send>;
