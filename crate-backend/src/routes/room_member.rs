use std::sync::Arc;

use axum::extract::{Path, Query};
use axum::response::IntoResponse;
use axum::{extract::State, Json};
use common::v1::types::util::Diff;
use common::v1::types::{
    MessageSync, PaginationQuery, PaginationResponse, Permission, RoomId, RoomMember,
    RoomMemberPatch, RoomMemberPut, RoomMembership, UserId,
};
use http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};
use utoipa_axum::{router::OpenApiRouter, routes};
use validator::Validate;

use crate::types::UserIdReq;
use crate::ServerState;

use super::util::{Auth, HeaderReason};
use crate::error::{Error, Result};

/// Room member list
#[utoipa::path(
    get,
    path = "/room/{room_id}/member",
    params(
        PaginationQuery<UserId>,
        ("room_id" = RoomId, description = "Room id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = PaginationResponse<RoomMember>, description = "success"),
    )
)]
async fn room_member_list(
    Path(room_id): Path<RoomId>,
    Query(paginate): Query<PaginationQuery<UserId>>,
    Auth(user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let d = s.data();
    let perms = s.services().perms.for_room(user_id, room_id).await?;
    perms.ensure_view()?;
    let res = d.room_member_list(room_id, paginate).await?;
    Ok(Json(res))
}

/// Room member get
#[utoipa::path(
    get,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
    )
)]
async fn room_member_get(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    let res = d.room_member_get(room_id, target_user_id).await?;
    // TODO: return `Ban`s
    if !matches!(res.membership, RoomMembership::Join { .. }) {
        Err(Error::NotFound)
    } else {
        Ok(Json(res))
    }
}

/// Room member add
///
/// Only `Puppet` users can be added to rooms (via MemberBridge permission)
#[utoipa::path(
    put,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn room_member_add(
    Path((_room_id, _target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(_auth_user_id): Auth,
    State(_s): State<Arc<ServerState>>,
    HeaderReason(_reason): HeaderReason,
    Json(_json): Json<RoomMemberPut>,
) -> Result<Json<()>> {
    Err(Error::Unimplemented)
}

/// Room member update
#[utoipa::path(
    patch,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = OK, body = RoomMember, description = "success"),
        (status = NOT_MODIFIED, description = "not modified"),
    )
)]
async fn room_member_update(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    State(s): State<Arc<ServerState>>,
    HeaderReason(reason): HeaderReason,
    Json(json): Json<RoomMemberPatch>,
) -> Result<impl IntoResponse> {
    json.validate()?;
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberManage)?;
    }

    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    if !json.changes(&start) {
        return Err(Error::NotModified);
    }
    d.room_member_patch(room_id, target_user_id, json).await?;
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        reason,
        MessageSync::UpsertRoomMember {
            member: res.clone(),
        },
    )
    .await?;
    Ok(Json(res).into_response())
}

#[derive(Debug, Default, Serialize, Deserialize, ToSchema, IntoParams, Validate)]
struct LeaveQuery {
    /// when leaving a room, allow this room to be found with ?include=Removed
    #[serde(default)]
    soft: bool,
    // /// don't send any leave messages?
    // // wasn't planning on doing it for rooms anyways, maybe threads though?
    // #[serde(default)]
    // silent: bool,
}

/// Room member delete (kick/leave)
#[utoipa::path(
    delete,
    path = "/room/{room_id}/member/{user_id}",
    params(
        ("room_id" = RoomId, description = "Room id"),
        ("user_id" = String, description = "User id"),
    ),
    tags = ["room_member"],
    responses(
        (status = NO_CONTENT, description = "success"),
    )
)]
async fn room_member_delete(
    Path((room_id, target_user_id)): Path<(RoomId, UserIdReq)>,
    Auth(auth_user_id): Auth,
    HeaderReason(reason): HeaderReason,
    Query(_q): Query<LeaveQuery>,
    State(s): State<Arc<ServerState>>,
) -> Result<impl IntoResponse> {
    let target_user_id = match target_user_id {
        UserIdReq::UserSelf => auth_user_id,
        UserIdReq::UserId(id) => id,
    };
    let d = s.data();
    let perms = s.services().perms.for_room(auth_user_id, room_id).await?;
    perms.ensure_view()?;
    if target_user_id != auth_user_id {
        perms.ensure(Permission::MemberKick)?;
    }
    let start = d.room_member_get(room_id, target_user_id).await?;
    if !matches!(start.membership, RoomMembership::Join { .. }) {
        return Err(Error::NotFound);
    }
    d.room_member_set_membership(room_id, target_user_id, RoomMembership::Leave {})
        .await?;
    s.services()
        .perms
        .invalidate_room(target_user_id, room_id)
        .await;
    s.services().perms.invalidate_is_mutual(target_user_id);
    let res = d.room_member_get(room_id, target_user_id).await?;
    s.broadcast_room(
        room_id,
        auth_user_id,
        reason,
        MessageSync::UpsertRoomMember { member: res },
    )
    .await?;
    Ok(StatusCode::NO_CONTENT)
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(room_member_list))
        .routes(routes!(room_member_get))
        .routes(routes!(room_member_add))
        .routes(routes!(room_member_update))
        .routes(routes!(room_member_delete))
}
