use async_trait::async_trait;
use sqlx::{query, query_file_as, query_file_scalar, query_scalar, Acquire};
use tracing::info;
use uuid::Uuid;

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{
    DbMessage, DbMessageType, Message, MessageCreate, MessageId, MessageVerId, PaginationDirection,
    PaginationQuery, PaginationResponse, ThreadId,
};

use crate::data::DataMessage;

use super::{Pagination, Postgres};

#[async_trait]
impl DataMessage for Postgres {
    async fn message_create(&self, create: MessageCreate) -> Result<MessageId> {
        let message_id = Uuid::now_v7();
        let atts: Vec<Uuid> = create
            .attachment_ids
            .iter()
            .map(|i| i.into_inner())
            .collect();
        let message_type: DbMessageType = create.message_type.into();
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, attachments)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, $10)
        "#, message_id, create.thread_id.into_inner(), message_id, create.content, create.metadata, create.reply_id.map(|i| i.into_inner()), create.author_id.into_inner(), message_type as _, create.override_name, &atts)
        .execute(&self.pool)
        .await?;
        info!("insert message");
        Ok(message_id.into())
    }

    async fn message_update(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
        create: MessageCreate,
    ) -> Result<MessageVerId> {
        let ver_id = Uuid::now_v7();
        let atts: Vec<Uuid> = create
            .attachment_ids
            .iter()
            .map(|i| i.into_inner())
            .collect();
        let message_type: DbMessageType = create.message_type.into();
        query!(r#"
    	    INSERT INTO message (id, thread_id, version_id, ordering, content, metadata, reply_id, author_id, type, override_name, attachments)
    	    VALUES ($1, $2, $3, (SELECT coalesce(max(ordering), 0) FROM message WHERE thread_id = $2), $4, $5, $6, $7, $8, $9, $10)
        "#,
            message_id.into_inner(),
            create.thread_id.into_inner(),
            ver_id,
            create.content,
            create.metadata,
            create.reply_id.map(|i| i.into_inner()),
            create.author_id.into_inner(),
            message_type as _,
            create.override_name,
            &atts,
        )
        .execute(&self.pool)
        .await?;
        Ok(ver_id.into())
    }

    async fn message_get(&self, thread_id: ThreadId, id: MessageId) -> Result<Message> {
        let row = query_file_as!(
            DbMessage,
            "sql/message_get.sql",
            thread_id.into_inner(),
            id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn message_list(
        &self,
        thread_id: ThreadId,
        pagination: PaginationQuery<MessageId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                r"sql/message_paginate.sql",
                thread_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_file_scalar!("sql/message_count.sql", thread_id.into_inner())
        )
    }

    async fn message_delete(&self, _thread_id: ThreadId, message_id: MessageId) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        query!(
            "UPDATE message SET deleted_at = $2 WHERE id = $1",
            message_id.into_inner(),
            now
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_version_get(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Result<Message> {
        let row = query_file_as!(
            DbMessage,
            "sql/message_version_get.sql",
            thread_id.into_inner(),
            message_id.into_inner(),
            version_id.into_inner()
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(row.into())
    }

    async fn message_version_delete(
        &self,
        _thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    ) -> Result<()> {
        let now = time::OffsetDateTime::now_utc().unix_timestamp();
        query!(
            "UPDATE message SET deleted_at = $3 WHERE id = $1 AND version_id = $2",
            message_id.into_inner(),
            version_id.into_inner(),
            now
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn message_version_list(
        &self,
        thread_id: ThreadId,
        message_id: MessageId,
        pagination: PaginationQuery<MessageVerId>,
    ) -> Result<PaginationResponse<Message>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_file_as!(
                DbMessage,
                "sql/message_version_paginate.sql",
                thread_id.into_inner(),
                message_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                r"select count(*) from message where thread_id = $1 and id = $2",
                thread_id.into_inner(),
                message_id.into_inner(),
            )
        )
    }
}
