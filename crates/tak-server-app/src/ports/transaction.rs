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
}

impl<
    T: Send + Sync + 'static,
    LeftOut: Send + Sync + 'static,
    RightOut: Send + Sync + 'static,
    Err: Send + Sync + 'static,
> TransactionItem<T, Result<(LeftOut, RightOut), Err>>
{
    pub fn try_pair(
        left: TransactionItem<T, Result<LeftOut, Err>>,
        right: TransactionItem<T, Result<RightOut, Err>>,
    ) -> TransactionItem<T, Result<(LeftOut, RightOut), Err>> {
        TransactionItem::new(
            Box::new(move |ctx| {
                let left_item = (left.item)(ctx);
                let right_item = (right.item)(ctx);
                Box::pin(async move {
                    let (left_result, right_result) =
                        futures::future::join(left_item, right_item).await;
                    Ok((left_result?, right_result?))
                })
            }),
            true,
        )
    }
}

pub type TransactionFn<T, Out> =
    Box<dyn for<'a> FnOnce(&'a T) -> Pin<Box<dyn Future<Output = Out> + Send + 'a>> + Send>;
