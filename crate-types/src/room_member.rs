use serde::{Deserialize, Serialize};

#[cfg(feature = "utoipa")]
use utoipa::ToSchema;

use super::{RoleId, RoomId, UserId};

use crate::util::{some_option, Diff};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMember {
    pub user_id: UserId,
    pub room_id: RoomId,

    #[serde(flatten)]
    pub membership: RoomMembership,

    /// When this member's membership last changed (joined, left, was kicked, or banned).
    #[serde(
        serialize_with = "time::serde::rfc3339::serialize",
        deserialize_with = "time::serde::rfc3339::deserialize"
    )]
    pub membership_updated_at: time::OffsetDateTime,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMemberPut {
    pub override_name: Option<String>,
    pub override_description: Option<String>,
    // pub override_avatar: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
pub struct RoomMemberPatch {
    #[serde(default, deserialize_with = "some_option")]
    pub override_name: Option<Option<String>>,

    #[serde(default, deserialize_with = "some_option")]
    pub override_description: Option<Option<String>>,
    // #[serde(default, deserialize_with = "some_option")]
    // pub override_avatar: Option<Option<String>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "utoipa", derive(ToSchema))]
#[serde(tag = "membership")]
pub enum RoomMembership {
    /// joined
    Join {
        override_name: Option<String>,
        override_description: Option<String>,
        // override_avatar: z.string().url().or(z.literal("")),
        roles: Vec<RoleId>,
    },

    /// kicked or left, can rejoin with an invite. todo: can still view messages up until then
    Leave {
        // TODO: keep roles on leave?
        // TODO: copy kick/ban reason here
        // /// user supplied reason why this user was banned
        // reason: Option<String>,
        // /// which user caused the kick, or None if the user left themselves
        // user_id: Option<UserId>,
    },

    /// banned. todo: can still view messages up until they were banned
    Ban {
        // /// user supplied reason why this user was banned
        // reason: Option<String>,
        // /// which user caused the ban
        // user_id: Option<UserId>,
    },
}

impl Diff<RoomMember> for RoomMemberPatch {
    fn changes(&self, other: &RoomMember) -> bool {
        match &other.membership {
            RoomMembership::Join {
                override_name,
                override_description,
                roles: _,
            } => {
                self.override_name.changes(override_name)
                    || self.override_description.changes(override_description)
            }
            _ => false,
        }
    }
}
