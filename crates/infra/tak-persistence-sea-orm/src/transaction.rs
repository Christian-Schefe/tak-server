use sea_orm::{
    ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr, ExecResult,
    QueryResult, Statement, TransactionTrait,
};
use tak_server_app::ports::transaction::{RepoError, TransactionManager};

use crate::create_db_pool;

pub enum RepositoryContext {
    Transaction(DatabaseTransaction),
    Connection(DatabaseConnection),
}

#[async_trait::async_trait]
impl ConnectionTrait for RepositoryContext {
    fn get_database_backend(&self) -> DbBackend {
        match self {
            RepositoryContext::Transaction(tr) => tr.get_database_backend(),
            RepositoryContext::Connection(conn) => conn.get_database_backend(),
        }
    }
    async fn execute_raw(&self, stmt: Statement) -> Result<ExecResult, DbErr> {
        match self {
            RepositoryContext::Transaction(tr) => tr.execute_raw(stmt).await,
            RepositoryContext::Connection(conn) => conn.execute_raw(stmt).await,
        }
    }
    async fn execute_unprepared(&self, sql: &str) -> Result<ExecResult, DbErr> {
        match self {
            RepositoryContext::Transaction(tr) => tr.execute_unprepared(sql).await,
            RepositoryContext::Connection(conn) => conn.execute_unprepared(sql).await,
        }
    }
    async fn query_one_raw(&self, stmt: Statement) -> Result<Option<QueryResult>, DbErr> {
        match self {
            RepositoryContext::Transaction(tr) => tr.query_one_raw(stmt).await,
            RepositoryContext::Connection(conn) => conn.query_one_raw(stmt).await,
        }
    }
    async fn query_all_raw(&self, stmt: Statement) -> Result<Vec<QueryResult>, DbErr> {
        match self {
            RepositoryContext::Transaction(tr) => tr.query_all_raw(stmt).await,
            RepositoryContext::Connection(conn) => conn.query_all_raw(stmt).await,
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
impl TransactionManager<RepositoryContext> for TransactionManagerImpl {
    fn simple(&self) -> RepositoryContext {
        RepositoryContext::Connection(self.db.clone())
    }
    async fn transaction(&self) -> Result<RepositoryContext, RepoError> {
        match self.db.begin().await {
            Ok(tr) => Ok(RepositoryContext::Transaction(tr)),
            Err(e) => Err(RepoError::new(format!(
                "Failed to start transaction: {:?}",
                e
            ))),
        }
    }
    async fn commit(&self, ctx: RepositoryContext) -> Result<(), RepoError> {
        match ctx {
            RepositoryContext::Transaction(tr) => match tr.commit().await {
                Ok(_) => Ok(()),
                Err(e) => Err(RepoError::new(format!(
                    "Failed to commit transaction: {:?}",
                    e
                ))),
            },
            RepositoryContext::Connection(_) => Err(RepoError::new(
                "Cannot commit a non-transaction context".to_string(),
            )),
        }
    }
    async fn rollback(&self, ctx: RepositoryContext) -> Result<(), RepoError> {
        match ctx {
            RepositoryContext::Transaction(tr) => match tr.rollback().await {
                Ok(_) => Ok(()),
                Err(e) => Err(RepoError::new(format!(
                    "Failed to rollback transaction: {:?}",
                    e
                ))),
            },
            RepositoryContext::Connection(_) => Err(RepoError::new(
                "Cannot rollback a non-transaction context".to_string(),
            )),
        }
    }
}
