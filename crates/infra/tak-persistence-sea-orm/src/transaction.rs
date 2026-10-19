use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, ExecResult,
    QueryResult, Statement, TransactionTrait,
};
use tak_server_app::ports::transaction::{TransactionError, TransactionItem, TransactionManager};

use crate::create_db_pool;

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
    async fn execute<Out: Send + Sync + 'static>(
        &self,
        item: TransactionItem<TransactionContext, Result<Out, TransactionError>>,
    ) -> Result<Out, TransactionError> {
        let tr = self.db.begin().await.map_err(|e| {
            tracing::error!("Failed to begin transaction: {:?}", e);
            TransactionError::new(format!("Failed to begin transaction: {:?}", e))
        })?;
        let ctx = TransactionContext::Transaction(tr);
        let res = item.execute(&ctx).await;
        let tr = match ctx {
            TransactionContext::Transaction(tr) => tr,
            _ => unreachable!(),
        };
        match res {
            Ok(out) => {
                tr.commit().await.unwrap();
                Ok(out)
            }
            Err(err) => {
                tr.rollback().await.unwrap();
                Err(err)
            }
        }
    }
}
