use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, ExecResult,
    QueryResult, Statement, TransactionTrait,
};
use tak_server_app::ports::{RepoError, transaction::TransactionManager};

use crate::create_db_pool;

#[derive(Clone)]
pub enum TransactionContext {
    Transaction(DatabaseTransaction),
    Connection(DatabaseConnection),
}

#[async_trait::async_trait]
impl ConnectionTrait for TransactionContext {
    fn get_database_backend(&self) -> DbBackend {
        match self {
            TransactionContext::Transaction(tr) => tr.get_database_backend(),
            TransactionContext::Connection(conn) => conn.get_database_backend(),
        }
    }
    async fn execute_raw(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        match self {
            TransactionContext::Transaction(tr) => tr.execute_raw(stmt).await,
            TransactionContext::Connection(conn) => conn.execute_raw(stmt).await,
        }
    }
    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        match self {
            TransactionContext::Transaction(tr) => tr.execute_unprepared(sql).await,
            TransactionContext::Connection(conn) => conn.execute_unprepared(sql).await,
        }
    }
    async fn query_one_raw(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        match self {
            TransactionContext::Transaction(tr) => tr.query_one_raw(stmt).await,
            TransactionContext::Connection(conn) => conn.query_one_raw(stmt).await,
        }
    }
    async fn query_all_raw(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        match self {
            TransactionContext::Transaction(tr) => tr.query_all_raw(stmt).await,
            TransactionContext::Connection(conn) => conn.query_all_raw(stmt).await,
        }
    }
}

pub struct TransactionManagerImpl {
    db: DatabaseConnection,
}

impl TransactionManagerImpl {
    pub async fn new() -> Self {
        let db = create_db_pool().await;
        Self { db }
    }
}

#[async_trait::async_trait]
impl TransactionManager<TransactionContext> for TransactionManagerImpl {
    fn get_context(&self) -> TransactionContext {
        TransactionContext::Connection(self.db.clone())
    }
    async fn in_transaction<F, Fut, R, E>(&self, f: F) -> Result<Result<R, E>, RepoError>
    where
        F: FnOnce(TransactionContext) -> Fut + Send,
        R: Send,
        E: Send,
        Fut: std::future::Future<Output = Result<R, E>> + Send,
    {
        let tr = self
            .db
            .begin()
            .await
            .map_err(|e| RepoError::StorageError(e.to_string()))?;
        let tr = TransactionContext::Transaction(tr);
        match f(tr).await {
            Ok(result) => {
                let TransactionContext::Transaction(tr) = tr else {
                    unreachable!("TransactionContext should be a Transaction here");
                };
                tr.commit()
                    .await
                    .map_err(|e| RepoError::StorageError(e.to_string()))?;
                Ok(Ok(result))
            }
            Err(e) => {
                let TransactionContext::Transaction(tr) = tr else {
                    unreachable!("TransactionContext should be a Transaction here");
                };
                tr.rollback()
                    .await
                    .map_err(|e| RepoError::StorageError(e.to_string()))?;
                Ok(Err(e))
            }
        }
    }
}
