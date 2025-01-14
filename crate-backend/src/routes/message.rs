use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    Json,
};
use utoipa_axum::{router::OpenApiRouter, routes};

use crate::{
    error::Error,
    types::{
        MediaLinkType, Message, MessageCreate, MessageCreateRequest, MessageId, MessagePatch,
        MessageServer, MessageType, MessageVerId, PaginationQuery, PaginationResponse, Permission,
        ThreadId,
    },
    ServerState,
};

use super::util::Auth;
use crate::error::Result;

/// Create a message
#[utoipa::path(
    post,
    path = "/thread/{thread_id}/message",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses(
        (status = CREATED, description = "Create message success", body = Message),
    )
)]
async fn message_create(
    Path((thread_id,)): Path<(ThreadId,)>,
    Auth(session): Auth,
    State(s): State<ServerState>,
    Json(json): Json<MessageCreateRequest>,
) -> Result<(StatusCode, Json<Message>)> {
    let data = s.data();
    let user_id = session.user_id;
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    perms.ensure(Permission::MessageCreate)?;
    if !json.attachments.is_empty() {
        perms.ensure(Permission::MessageFilesEmbeds)?;
    }
    // TODO: everyone can set override_name, but it's meant to be temporary so its probably fine
    if json.content.is_none() && json.attachments.is_empty() {
        return Err(Error::BadStatic(
            "at least one of content, attachments, or embeds must be defined",
        ));
    }
    let attachment_ids: Vec<_> = json.attachments.into_iter().map(|r| r.id).collect();
    for id in &attachment_ids {
        let existing = data.media_link_select(*id).await?;
        if !existing.is_empty() {
            return Err(Error::BadStatic("cant reuse media"));
        }
    }
    let message_id = data
        .message_create(MessageCreate {
            thread_id,
            content: json.content,
            attachment_ids: attachment_ids.clone(),
            author_id: user_id,
            message_type: MessageType::Default,
            metadata: json.metadata,
            reply_id: json.reply_id,
            override_name: json.override_name,
        })
        .await?;
    let message_uuid = message_id.into_inner();
    for id in &attachment_ids {
        data.media_link_insert(*id, message_uuid, MediaLinkType::Message)
            .await?;
        data.media_link_insert(*id, message_uuid, MediaLinkType::MessageVersion)
            .await?;
    }
    let mut message = data.message_get(thread_id, message_id).await?;
    for media in &mut message.attachments {
        media.url = s.presign(media.id).await?;
    }
    s.sushi.send(MessageServer::UpsertMessage {
        message: message.clone(),
    })?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// List messages in a thread
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message",
    params(("thread_id", description = "Thread id")),
    tags = ["message"],
    responses(
        (status = OK, description = "List thread messages success"),
    )
)]
async fn message_list(
    Path((thread_id,)): Path<(ThreadId,)>,
    Query(q): Query<PaginationQuery<MessageId>>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<PaginationResponse<Message>>> {
    let user_id = session.user_id;
    let data = s.data();
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut res = data.message_list(thread_id, q).await?;
    for message in &mut res.items {
        for media in &mut message.attachments {
            media.url = s.presign(media.id).await?;
        }
    }
    Ok(Json(res))
}

/// Get a message
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, description = "List thread messages success"),
    )
)]
async fn message_get(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<Message>> {
    let user_id = session.user_id;
    let data = s.data();
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut message = data.message_get(thread_id, message_id).await?;
    for media in &mut message.attachments {
        media.url = s.presign(media.id).await?;
    }
    Ok(Json(message))
}

/// edit a message
#[utoipa::path(
    patch,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, description = "edit message success"),
        (status = NOT_MODIFIED, description = "no change"),
    )
)]
async fn message_edit(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(session): Auth,
    State(s): State<ServerState>,
    Json(json): Json<MessagePatch>,
) -> Result<(StatusCode, Json<Message>)> {
    let data = s.data();
    let user_id = session.user_id;
    let mut perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data.message_get(thread_id, message_id).await?;
    if message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant edit that message"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageEdit)?;
    if json.content.is_none() && json.attachments.as_ref().is_some_and(|a| a.is_empty()) {
        return Err(Error::BadStatic(
            "at least one of content, attachments, or embeds must be defined",
        ));
    }
    if !json.attachments.as_ref().is_some_and(|a| !a.is_empty()) {
        perms.ensure(Permission::MessageFilesEmbeds)?;
    }
    if json.wont_change(&message) {
        return Ok((StatusCode::NOT_MODIFIED, Json(message)));
    }
    let attachment_ids: Vec<_> = json
        .attachments
        .map(|ats| ats.into_iter().map(|r| r.id).collect())
        .unwrap_or_else(|| {
            message
                .attachments
                .into_iter()
                .map(|media| media.id)
                .collect()
        });
    for id in &attachment_ids {
        let existing = data.media_link_select(*id).await?;
        let has_link = existing.iter().any(|i| {
            i.link_type == MediaLinkType::Message && i.target_id == message_id.into_inner()
        });
        if !has_link {
            return Err(Error::BadStatic("cant reuse media"));
        }
    }
    let version_id = data
        .message_update(
            thread_id,
            message_id,
            MessageCreate {
                thread_id,
                content: json.content.unwrap_or(message.content),
                attachment_ids: attachment_ids.clone(),
                author_id: user_id,
                message_type: MessageType::Default,
                metadata: json.metadata.unwrap_or(message.metadata),
                reply_id: json.reply_id.unwrap_or(message.reply_id),
                override_name: json.override_name.unwrap_or(message.override_name),
            },
        )
        .await?;
    let version_uuid = version_id.into_inner();
    for id in &attachment_ids {
        data.media_link_insert(*id, version_uuid, MediaLinkType::MessageVersion)
            .await?;
    }
    let mut message = data
        .message_version_get(thread_id, message_id, version_id)
        .await?;
    for media in &mut message.attachments {
        media.url = s.presign(media.id).await?;
    }
    s.sushi.send(MessageServer::UpsertMessage {
        message: message.clone(),
    })?;
    Ok((StatusCode::CREATED, Json(message)))
}

/// delete a message
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = NO_CONTENT, description = "delete message success"),
    )
)]
async fn message_delete(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<StatusCode> {
    let data = s.data();
    let user_id = session.user_id;
    let mut perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data.message_get(thread_id, message_id).await?;
    if message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete that message"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageEdit);
    }
    perms.ensure(Permission::MessageDelete)?;
    data.message_delete(thread_id, message_id).await?;
    data.media_link_delete_all(message_id.into_inner()).await?;
    s.sushi.send(MessageServer::DeleteMessage {
        thread_id,
        message_id,
    })?;
    Ok(StatusCode::NO_CONTENT)
}

/// list message versions
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/version",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id")
    ),
    tags = ["message"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn message_version_list(
    Path((thread_id, message_id)): Path<(ThreadId, MessageId)>,
    Query(q): Query<PaginationQuery<MessageVerId>>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<PaginationResponse<Message>>> {
    let data = s.data();
    let user_id = session.user_id;
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut res = data.message_version_list(thread_id, message_id, q).await?;
    for message in &mut res.items {
        for media in &mut message.attachments {
            media.url = s.presign(media.id).await?;
        }
    }
    Ok(Json(res))
}

/// get message version
#[utoipa::path(
    get,
    path = "/thread/{thread_id}/message/{message_id}/version/{version_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("version_id", description = "Version id"),
    ),
    tags = ["message"],
    responses(
        (status = OK, description = "success"),
    )
)]
async fn message_version_get(
    Path((thread_id, message_id, version_id)): Path<(ThreadId, MessageId, MessageVerId)>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<Message>> {
    let user_id = session.user_id;
    let data = s.data();
    let perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let mut message = data
        .message_version_get(thread_id, message_id, version_id)
        .await?;
    for media in &mut message.attachments {
        media.url = s.presign(media.id).await?;
    }
    Ok(Json(message))
}

/// delete message version
#[utoipa::path(
    delete,
    path = "/thread/{thread_id}/message/{message_id}/version/{version_id}",
    params(
        ("thread_id", description = "Thread id"),
        ("message_id", description = "Message id"),
        ("version_id", description = "Version id"),
    ),
    tags = ["message"],
    responses(
        (status = NO_CONTENT, description = "delete message success"),
    )
)]
async fn message_version_delete(
    Path((thread_id, message_id, version_id)): Path<(ThreadId, MessageId, MessageVerId)>,
    Auth(session): Auth,
    State(s): State<ServerState>,
) -> Result<Json<()>> {
    let user_id = session.user_id;
    let data = s.data();
    let mut perms = data.permission_thread_get(user_id, thread_id).await?;
    perms.ensure_view()?;
    let message = data
        .message_version_get(thread_id, message_id, version_id)
        .await?;
    if !message.message_type.is_deletable() {
        return Err(Error::BadStatic("cant delete this message type"));
    }
    if message.author.id == user_id {
        perms.add(Permission::MessageDelete);
    }
    perms.ensure(Permission::MessageDelete)?;
    data.message_version_delete(thread_id, message_id, version_id)
        .await?;
    Ok(Json(()))
}

pub fn routes() -> OpenApiRouter<ServerState> {
    OpenApiRouter::new()
        .routes(routes!(message_create))
        .routes(routes!(message_get))
        .routes(routes!(message_list))
        .routes(routes!(message_edit))
        .routes(routes!(message_delete))
        .routes(routes!(message_version_list))
        .routes(routes!(message_version_get))
        .routes(routes!(message_version_delete))
}
