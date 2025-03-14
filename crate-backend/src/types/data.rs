use common::v1::types::{
    thread::chat::{ThreadTypeChatPrivate, ThreadTypeChatPublic},
    util::Time,
    MediaId, MessageId, MessageType, MessageVerId, Permission, Role, RoleId, RoleVerId, Room,
    RoomId, RoomMembership, RoomType, Session, SessionId, SessionStatus, SessionToken, Thread,
    ThreadId, ThreadMembership, ThreadPrivate, ThreadPublic, ThreadState, ThreadVerId,
    ThreadVisibility, UserId, UserType,
};
use serde::Deserialize;
use utoipa::ToSchema;
use uuid::Uuid;

#[derive(Deserialize)]
pub struct DbRoom {
    pub id: Uuid,
    pub version_id: Uuid,
    pub name: String,
    pub description: Option<String>,
}

pub struct DbUserCreate {
    pub parent_id: Option<UserId>,
    pub name: String,
    pub description: Option<String>,
    pub user_type: UserType,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "membership")]
pub enum DbMembership {
    Join,
    Leave,
    Ban,
}

impl From<DbRoom> for Room {
    fn from(row: DbRoom) -> Self {
        #[allow(deprecated)]
        Room {
            id: row.id.into(),
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            room_type: RoomType::Default,

            // FIXME: add to db, calculate
            visibility: Default::default(),
            member_count: Default::default(),
            online_count: Default::default(),
            thread_count: Default::default(),
            default_order: Default::default(),
            default_layout: Default::default(),
            tags_available: Default::default(),
            tags_applied: Default::default(),
            languages: Default::default(),
            views: Default::default(),
            system_messages: Default::default(),
        }
    }
}

#[derive(Deserialize)]
pub struct DbThread {
    pub id: ThreadId,
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub version_id: ThreadVerId,
    pub name: String,
    pub description: Option<String>,
    pub last_version_id: MessageVerId,
    pub last_read_id: Option<Uuid>,
    pub message_count: i64,
    pub is_unread: bool,
    pub state: DbThreadState,
}

pub struct DbThreadCreate {
    pub room_id: RoomId,
    pub creator_id: UserId,
    pub name: String,
    pub description: Option<String>,
}

// #[sqlx(type_name = "thread_type")]
// pub enum ThreadType {
// 	Default,
// }

impl From<DbThread> for Thread {
    fn from(row: DbThread) -> Self {
        Thread {
            id: row.id,
            room_id: row.room_id,
            creator_id: row.creator_id,
            version_id: row.version_id,
            name: row.name,
            description: row.description,
            state: match row.state {
                DbThreadState::Pinned => todo!(),
                DbThreadState::Active => ThreadState::Active,
                DbThreadState::Temporary => ThreadState::Temporary,
                DbThreadState::Archived => ThreadState::Archived,
                DbThreadState::Deleted => ThreadState::Deleted,
            },
            visibility: ThreadVisibility::Room,
            info: ThreadPublic::Chat(ThreadTypeChatPublic {
                last_version_id: row.last_version_id,
                message_count: row.message_count.try_into().expect("count is negative?"),
            }),
            private: Some(ThreadPrivate::Chat(ThreadTypeChatPrivate {
                is_unread: row.is_unread,
                last_read_id: row.last_read_id.map(Into::into),
                // FIXME: add field to db schema
                mention_count: 0,
                notifications: Default::default(),
            })),

            // FIXME: add field to db schema
            state_updated_at: row.id.try_into().unwrap(),
            // FIXME: add field to db schema or calculate
            member_count: 0,
            // FIXME: calculate field
            online_count: 0,
            tags: Default::default(),
            is_locked: Default::default(),
            is_announcement: Default::default(),
            reactions: Default::default(),
        }
    }
}

pub struct DbSession {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub token: SessionToken,
    pub status: DbSessionStatus,
    pub name: Option<String>,
}

#[derive(sqlx::Type)]
#[sqlx(type_name = "session_status")]
pub enum DbSessionStatus {
    Unauthorized,
    Authorized,
    Sudo,
}

impl From<DbSession> for Session {
    fn from(row: DbSession) -> Self {
        Session {
            id: row.id.into(),
            status: match row.status {
                DbSessionStatus::Unauthorized => SessionStatus::Unauthorized,
                DbSessionStatus::Authorized => SessionStatus::Authorized {
                    user_id: row.user_id.expect("invalid data in db!").into(),
                },
                DbSessionStatus::Sudo => SessionStatus::Sudo {
                    user_id: row.user_id.expect("invalid data in db!").into(),
                    expires_at: Time::now_utc(),
                },
            },
            name: row.name,
        }
    }
}

impl From<SessionStatus> for DbSessionStatus {
    fn from(value: SessionStatus) -> Self {
        match value {
            SessionStatus::Unauthorized => DbSessionStatus::Unauthorized,
            SessionStatus::Authorized { .. } => DbSessionStatus::Authorized,
            SessionStatus::Sudo { .. } => DbSessionStatus::Sudo,
        }
    }
}

pub struct DbRole {
    pub id: RoleId,
    pub version_id: RoleVerId,
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<DbPermission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

impl From<DbRole> for Role {
    fn from(row: DbRole) -> Self {
        Role {
            id: row.id,
            version_id: row.version_id,
            room_id: row.room_id,
            name: row.name,
            description: row.description,
            permissions: row.permissions.into_iter().map(Into::into).collect(),
            is_self_applicable: row.is_self_applicable,
            is_mentionable: row.is_mentionable,
            is_default: row.is_default,
        }
    }
}

pub struct DbRoleCreate {
    pub room_id: RoomId,
    pub name: String,
    pub description: Option<String>,
    pub permissions: Vec<Permission>,
    pub is_self_applicable: bool,
    pub is_mentionable: bool,
    pub is_default: bool,
}

pub struct DbMessageCreate {
    pub message_type: MessageType,
    pub thread_id: ThreadId,
    pub attachment_ids: Vec<MediaId>,
    pub author_id: UserId,
}

// TODO: move to types
impl DbMessageCreate {
    pub fn content(&self) -> Option<String> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.content.clone(),
            MessageType::ThreadUpdate(_patch) => Some("(thread update)".to_owned()),
            _ => None,
        }
    }

    pub fn metadata(&self) -> Option<serde_json::Value> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.metadata.clone(),
            MessageType::ThreadUpdate(patch) => Some(serde_json::to_value(patch).ok()?),
            _ => None,
        }
    }

    pub fn reply_id(&self) -> Option<MessageId> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.reply_id,
            _ => None,
        }
    }

    pub fn override_name(&self) -> Option<String> {
        match &self.message_type {
            MessageType::DefaultMarkdown(msg) => msg.override_name.clone(),
            _ => None,
        }
    }
}

macro_rules! impl_perms {
    ($($e:ident,)*) => {
        #[derive(sqlx::Type, PartialEq, Eq)]
        #[sqlx(type_name = "permission")]
        pub enum DbPermission {
            $($e,)*
        }

        impl From<DbPermission> for Permission {
            fn from(value: DbPermission) -> Self {
                match value {
                    $(DbPermission::$e => Permission::$e,)*
                }
            }
        }

        impl From<Permission> for DbPermission {
            fn from(value: Permission) -> Self {
                match value {
                    $(Permission::$e => DbPermission::$e,)*
                }
            }
        }
    }
}

// surely there's a better way without copypasta
impl_perms!(
    Admin,
    BotsAdd,
    BotsManage,
    EmojiAdd,
    EmojiManage,
    EmojiUseExternal,
    InviteCreate,
    InviteManage,
    MemberBan,
    MemberBanManage,
    MemberBridge,
    MemberKick,
    MemberManage,
    MessageCreate,
    MessageDelete,
    MessageEdit,
    MessageEmbeds,
    MessageMassMention,
    MessageAttachments,
    MessageMove,
    MessagePin,
    ReactionAdd,
    ReactionClear,
    ProfileAvatar,
    ProfileOverride,
    RoleApply,
    RoleManage,
    RoomManage,
    ServerAdmin,
    ServerMetrics,
    ServerOversee,
    ServerReports,
    TagApply,
    TagManage,
    ThreadArchive,
    ThreadCreateChat,
    ThreadCreateDocument,
    ThreadCreateEvent,
    ThreadCreateForumLinear,
    ThreadCreateForumTree,
    ThreadCreateTable,
    ThreadCreateVoice,
    ThreadCreatePublic,
    ThreadCreatePrivate,
    ThreadDelete,
    ThreadEdit,
    ThreadForward,
    ThreadLock,
    ThreadPin,
    ThreadPublish,
    UserDms,
    UserProfile,
    UserSessions,
    UserStatus,
    View,
    ViewAuditLog,
    VoiceConnect,
    VoiceDeafen,
    VoiceDisconnect,
    VoiceMove,
    VoiceMute,
    VoicePriority,
    VoiceSpeak,
    VoiceVideo,
);

impl From<RoomMembership> for DbMembership {
    fn from(value: RoomMembership) -> Self {
        match value {
            RoomMembership::Join { .. } => DbMembership::Join,
            RoomMembership::Ban {} => DbMembership::Ban,
            RoomMembership::Leave {} => DbMembership::Leave,
        }
    }
}

impl From<ThreadMembership> for DbMembership {
    fn from(value: ThreadMembership) -> Self {
        match value {
            ThreadMembership::Join { .. } => DbMembership::Join,
            ThreadMembership::Ban {} => DbMembership::Ban,
            ThreadMembership::Leave {} => DbMembership::Leave,
        }
    }
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum UserIdReq {
    #[serde(deserialize_with = "const_self")]
    UserSelf,
    UserId(UserId),
}

#[derive(Debug, Deserialize, ToSchema)]
#[serde(untagged)]
pub enum SessionIdReq {
    #[serde(deserialize_with = "const_self")]
    SessionSelf,
    // #[serde(deserialize_with = "const_all")]
    // SessionAll,
    SessionId(SessionId),
}

fn const_self<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
where
    D: serde::Deserializer<'de>,
{
    #[derive(Deserialize)]
    enum Helper {
        #[serde(rename = "@self")]
        Variant,
    }

    Helper::deserialize(deserializer).map(|_| ())
}

// fn const_all<'de, D>(deserializer: D) -> std::result::Result<(), D::Error>
// where
//     D: serde::Deserializer<'de>,
// {
//     #[derive(Deserialize)]
//     enum Helper {
//         #[serde(rename = "@all")]
//         Variant,
//     }

//     Helper::deserialize(deserializer).map(|_| ())
// }

pub struct DbInvite {
    pub code: String,
    pub target_type: String,
    pub target_id: Uuid,
    pub creator_id: Uuid,
    pub max_uses: Option<i32>,
    pub uses: i32,
    pub created_at: time::PrimitiveDateTime,
    pub expires_at: Option<time::PrimitiveDateTime>,
}

#[derive(Deserialize)]
pub struct RoleDeleteQuery {
    #[serde(default)]
    pub force: bool,
}

#[derive(Deserialize, sqlx::Type)]
#[sqlx(type_name = "thread_state")]
pub enum DbThreadState {
    Pinned,
    Active,
    Temporary,
    Archived,
    Deleted,
}

#[derive(sqlx::Type, PartialEq, Eq)]
#[sqlx(type_name = "media_link_type")]
pub enum MediaLinkType {
    Message,
    MessageVersion,
    AvatarUser,
    // FIXME(#230): link media to embeds
    // Embed,
}

// TODO: surely there's a better way than manually managing media links/references
pub struct MediaLink {
    pub media_id: MediaId,
    pub target_id: Uuid,
    pub link_type: MediaLinkType,
}
