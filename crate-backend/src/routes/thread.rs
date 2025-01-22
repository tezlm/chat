use std::sync::Arc;

use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    types::{
        MessageCreate, MessageServer, MessageType, MessageVerId, PaginationQuery,
        PaginationResponse, Permission, RoomId, Thread, ThreadCreate, ThreadCreateRequest,
        ThreadId, ThreadPatch,
    },
    ServerState,
};

use super::util::Auth;
use crate::error::Result;

/// Create a thread
#[utoipa::path(
    post,
    // path = "/thread",
    path = "/room/{room_id}/thread",
    params(("room_id", description = "Room id")),
    tags = ["thread"],
    responses(
        (status = CREATED, description = "Create thread success", body = Thread),
    )
)]
async fn thread_create(
    Path((room_id,)): Path<(RoomId,)>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<ThreadCreateRequest>,
) -> Result<(StatusCode, Json<Thread>)> {
    let data = s.data();
    let user_id = session.user_id;
    let perms = data.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::ThreadCreate)?;
    let thread_id = data
        .thread_create(ThreadCreate {
            room_id,
            creator_id: user_id,
            name: json.name,
            description: json.description,
            is_closed: json.is_closed.unwrap_or(false),
            is_locked: json.is_locked.unwrap_or(false),
            is_pinned: json.is_pinned.unwrap_or(false),
        })
        .await?;
    data.message_create(MessageCreate {
        thread_id,
        content: Some("(thread create)".to_string()),
        attachment_ids: vec![],
        author_id: user_id,
        message_type: MessageType::ThreadUpdate,
        metadata: None,
        reply_id: None,
        override_name: None,
    })
    .await?;
    let thread = data.thread_get(thread_id, user_id).await?;
    s.sushi.send(MessageServer::UpsertThread {
        thread: thread.clone(),
    })?;
    Ok((StatusCode::CREATED, Json(thread)))
}

/// Get a thread
#[utoipa::path(
    get,
    path = "/thread/{thread_id}",
    params(("thread_id", description = "Thread id")),
    tags = ["thread"],
    responses(
        (status = OK, description = "Get thread success", body = Thread),
    )
)]
async fn thread_get(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<(StatusCode, Json<Thread>)> {
    let data = s.data();
    let user_id = session.user_id;
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let thread = data.thread_get(thread_id, user_id).await?;
    Ok((StatusCode::OK, Json(thread)))
}

/// List threads in a room
#[utoipa::path(
    get,
    path = "/room/{room_id}/thread",
    params(("id", description = "Room id")),
    tags = ["thread"],
    responses(
        (status = OK, description = "List room threads success"),
    )
)]
async fn thread_list(
    Path((room_id,)): Path<(RoomId,)>,
    Query(q): Query<PaginationQuery<ThreadId>>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
) -> Result<Json<PaginationResponse<Thread>>> {
    let user_id = session.user_id;
    let data = s.data();
    let perms = data.permission_room_get(user_id, room_id).await?;
    perms.ensure_view()?;
    let res = data.thread_list(user_id, room_id, q).await?;
    Ok(Json(res))
}

/// edit a thread
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, description = "edit message success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn thread_update(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<ThreadPatch>,
) -> Result<Json<Thread>> {
    let user_id = session.user_id;
    let data = s.data();
    let mut perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let thread = data.thread_get(thread_id, user_id).await?;
    if thread.creator_id == user_id {
        perms.add(Permission::RoomManage);
    }
    perms.ensure(Permission::RoomManage)?;
    data.thread_update(thread_id, user_id, json).await?;
    let thread = data.thread_get(thread_id, user_id).await?;
    s.sushi.send(MessageServer::UpsertThread {
        thread: thread.clone(),
    })?;
    Ok(Json(thread))
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AckReq {
    version_id: MessageVerId,
}

#[derive(Debug, Serialize, Deserialize, ToSchema)]
struct AckRes {
    version_id: MessageVerId,
}

/// ack thread
///
/// mark a thread as read (or unread)
#[utoipa::path(
    put,
    path = "/thread/{thread_id}/ack",
    params(
        ("thread_id", description = "Thread id"),
    ),
    tags = ["thread"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn thread_ack(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(session): Auth,
    State(s): State<Arc<ServerState>>,
    Json(json): Json<AckReq>,
) -> Result<Json<AckRes>> {
    let user_id = session.user_id;
    let data = s.data();
    let version_id = json.version_id;
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    data.unread_put(user_id, thread_id, version_id).await?;
    Ok(Json(AckRes { version_id }))
}

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .routes(routes!(thread_create))
        .routes(routes!(thread_get))
        .routes(routes!(thread_list))
        .routes(routes!(thread_update))
        .routes(routes!(thread_ack))
}
