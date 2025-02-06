use async_trait::async_trait;
use sqlx::{query, query_as, query_scalar, Acquire};
use tracing::info;
use types::{
    PaginationDirection, PaginationQuery, PaginationResponse, RoomMember, RoomMemberPatch,
    RoomMembership,
};

use crate::error::Result;
use crate::gen_paginate;
use crate::types::{DbRoomMember, DbRoomMembership, RoomId, UserId};

use crate::data::DataRoomMember;

use super::{Pagination, Postgres};

#[async_trait]
impl DataRoomMember for Postgres {
    async fn room_member_put(
        &self,
        room_id: RoomId,
        user_id: UserId,
        membership: RoomMembership,
    ) -> Result<()> {
        let membership: DbRoomMembership = membership.into();
        query!(
            r#"
            INSERT INTO room_member (user_id, room_id, membership)
            VALUES ($1, $2, $3)
			ON CONFLICT ON CONSTRAINT room_member_pkey DO UPDATE SET
    			membership = excluded.membership
            "#,
            user_id.into_inner(),
            room_id.into_inner(),
            membership as _
        )
        .execute(&self.pool)
        .await?;
        info!("inserted room member");
        Ok(())
    }

    async fn room_member_delete(&self, room_id: RoomId, user_id: UserId) -> Result<()> {
        query!(
            "DELETE FROM room_member WHERE room_id = $1 AND user_id = $2",
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .execute(&self.pool)
        .await?;
        info!("deleted room member");
        Ok(())
    }

    async fn room_member_list(
        &self,
        room_id: RoomId,
        pagination: PaginationQuery<UserId>,
    ) -> Result<PaginationResponse<RoomMember>> {
        let p: Pagination<_> = pagination.try_into()?;
        gen_paginate!(
            p,
            self.pool,
            query_as!(
                DbRoomMember,
                r#"
            	SELECT room_id, user_id, membership as "membership: _", override_name, override_description, membership_updated_at
                FROM room_member
            	WHERE room_id = $1 AND user_id > $2 AND user_id < $3 AND membership = 'Join'
            	ORDER BY (CASE WHEN $4 = 'f' THEN user_id END), user_id DESC LIMIT $5
                "#,
                room_id.into_inner(),
                p.after.into_inner(),
                p.before.into_inner(),
                p.dir.to_string(),
                (p.limit + 1) as i32
            ),
            query_scalar!(
                "SELECT count(*) FROM room_member WHERE room_member.room_id = $1 AND membership = 'Join'",
                room_id.into_inner()
            )
        )
    }

    async fn room_member_get(&self, room_id: RoomId, user_id: UserId) -> Result<RoomMember> {
        let item = query_as!(
            DbRoomMember,
            r#"
        	SELECT room_id, user_id, membership as "membership: _", override_name, override_description, membership_updated_at
            FROM room_member
            WHERE room_id = $1 AND user_id = $2
        "#,
            room_id.into_inner(),
            user_id.into_inner(),
        )
        .fetch_one(&self.pool)
        .await?;
        Ok(item.into())
    }

    async fn room_member_patch(
        &self,
        room_id: RoomId,
        user_id: UserId,
        patch: RoomMemberPatch,
    ) -> Result<()> {
        query!(
            r#"
            UPDATE room_member
        	SET override_name = $3, override_description = $4
            WHERE room_id = $1 AND user_id = $2 AND membership = 'Join'
        "#,
            room_id.into_inner(),
            user_id.into_inner(),
            patch.override_name,
            patch.override_description,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    async fn room_member_set_membership(
        &self,
        room_id: RoomId,
        user_id: UserId,
        membership: RoomMembership,
    ) -> Result<()> {
        let membership: DbRoomMembership = membership.into();
        query!(
            r#"
            UPDATE room_member
        	SET membership = $3, membership_updated_at = now()
            WHERE room_id = $1 AND user_id = $2
            "#,
            room_id.into_inner(),
            user_id.into_inner(),
            membership as _,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }
}
