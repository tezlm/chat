use async_trait::async_trait;
use sqlx::{query_as, query_scalar, Acquire};
use uuid::Uuid;

use crate::error::{Error, Result};
use crate::types::{PaginationDirection, PaginationQuery, PaginationResponse, Session, SessionId, UserId};

use crate::data::DataSession;

use super::{Pagination, Postgres};

#[async_trait]
impl DataSession for Postgres {
    async fn session_create(&self, user_id: UserId, name: Option<String>) -> Result<Session> {
        let session_id = Uuid::now_v7();
        let mut conn = self.pool.acquire().await?;
        let token = Uuid::new_v4(); // TODO: is this secure enough
        let session = query_as!(
            Session,
            r#"
            INSERT INTO session (id, user_id, token, status, name)
            VALUES ($1, $2, $3, 'Unauthorized', $4)
            RETURNING id, user_id, token, status as "status: _", name"#,
            session_id,
            user_id.into_inner(),
            token.to_string(),
            name,
        )
        .fetch_one(&mut *conn)
        .await?;
        Ok(session)
    }

    async fn session_get(&self, id: SessionId) -> Result<Session> {
        let mut conn = self.pool.acquire().await?;
        let session = query_as!(
            Session,
            r#"SELECT id, user_id, token, status as "status: _", name FROM session WHERE id = $1"#,
            id.into_inner()
        )
        .fetch_one(&mut *conn)
        .await
        .map_err(|err| match err.into() {
            Error::NotFound => Error::MissingAuth,
            other => other,
        })?;
        Ok(session)
    }

    async fn session_get_by_token(&self, token: &str) -> Result<Session> {
        let mut conn = self.pool.acquire().await?;
        let session = query_as!(Session, r#"SELECT id, user_id, token, status as "status: _", name FROM session WHERE token = $1"#, token)
            .fetch_one(&mut *conn)
            .await
            .map_err(|err| match err.into() {
                Error::NotFound => Error::MissingAuth,
                other => other,
            })?;
        Ok(session)
    }

    async fn session_list(
        &self,
        user_id: UserId,
        pagination: PaginationQuery<SessionId>,
    ) -> Result<PaginationResponse<Session>> {
        let mut conn = self.pool.acquire().await?;
        let mut tx = conn.begin().await?;
        let p: Pagination<_> = pagination.try_into()?;
        let items = query_as!(Session, r#"
        	SELECT id, user_id, token, status as "status: _", name FROM session
        	WHERE user_id = $1 AND id > $2 AND id < $3
        	ORDER BY (CASE WHEN $4 = 'f' THEN id END), id DESC LIMIT $5
        	"#,
            user_id.into_inner(),
            p.after.into_inner(),
            p.before.into_inner(),
            p.dir.to_string(),
            (p.limit + 1) as i32)
            .fetch_all(&mut *tx)
            .await?;
        let total = query_scalar!(
            "SELECT count(*) FROM session WHERE user_id = $1",
            user_id.into_inner()
        )
        .fetch_one(&mut *tx)
        .await?;
        tx.rollback().await?;
        let has_more = items.len() > p.limit as usize;
        let mut items: Vec<_> = items.into_iter().take(p.limit as usize).collect();
        if p.dir == PaginationDirection::B {
            items.reverse();
        }
        Ok(PaginationResponse {
            items,
            total: total.unwrap_or(0) as u64,
            has_more,
        })
    }

    async fn session_delete(&self, session_id: SessionId) -> Result<()> {
        let mut conn = self.pool.acquire().await?;
        query_as!(Session, r#"DELETE FROM session WHERE id = $1"#, session_id.into_inner())
            .execute(&mut *conn)
            .await?;
        Ok(())
    }
}
