use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{
    Invite, InviteCode, Message, MessageId, MessageVerId, Role, RoleId, Room, RoomId, RoomMember,
    Session, SessionId, Thread, ThreadId, User, UserId, SessionToken,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageClient {
    /// initial message
    Hello {
        token: SessionToken,

        #[serde(flatten)]
        resume: Option<SyncResume>,
    },

    /// heartbeat
    Pong,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct SyncResume {
    pub conn: String,
    pub seq: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct MessageEnvelope {
    #[serde(flatten)]
    pub payload: MessagePayload,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "op")]
pub enum MessagePayload {
    /// heartbeat
    Ping,

    /// data to keep local copy of state in sync with server
    Sync { data: MessageSync, seq: u64 },

    /// some kind of error
    Error { error: String },

    /// successfully connected
    Ready { user: User, conn: String, seq: u64 },

    /// successfully reconnected
    Resumed,

    /// client needs to disconnect and reconnect
    Reconnect { can_resume: bool },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "type")]
pub enum MessageSync {
    UpsertRoom {
        room: Room,
    },
    UpsertThread {
        thread: Thread,
    },
    UpsertMessage {
        message: Message,
    },
    UpsertUser {
        user: User,
    },
    UpsertMember {
        member: RoomMember,
    },
    UpsertSession {
        session: Session,
    },
    UpsertRole {
        role: Role,
    },
    UpsertInvite {
        invite: Invite,
    },
    DeleteMessage {
        thread_id: ThreadId,
        message_id: MessageId,
    },
    DeleteMessageVersion {
        thread_id: ThreadId,
        message_id: MessageId,
        version_id: MessageVerId,
    },
    DeleteUser {
        id: UserId,
    },
    DeleteSession {
        id: SessionId,
    },
    DeleteRole {
        room_id: RoomId,
        role_id: RoleId,
    },
    DeleteMember {
        room_id: RoomId,
        user_id: UserId,
    },
    DeleteInvite {
        code: InviteCode,
    },
    Webhook {
        hook_id: Uuid,
        data: serde_json::Value,
    },
}

// /// messages specific to a user
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageUser {}

// /// messages specific to a thread
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageThread {}

// /// messages specific to a room
// #[derive(Debug, PartialEq, Eq, ToSchema, Serialize, Deserialize)]
// #[serde(tag = "type")]
// enum MessageRoom {}
