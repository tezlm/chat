use std::sync::Arc;

use utoipa_axum::router::OpenApiRouter;

use crate::ServerState;

mod auth;
mod invite;
mod media;
mod message;
mod role;
mod room;
mod room_member;
mod search;
mod session;
mod sync;
mod thread;
mod thread_member;
mod user;
mod util;

pub fn routes() -> OpenApiRouter<Arc<ServerState>> {
    OpenApiRouter::new()
        .merge(auth::routes())
        .merge(invite::routes())
        .merge(media::routes())
        .merge(message::routes())
        .merge(role::routes())
        .merge(room::routes())
        .merge(room_member::routes())
        .merge(search::routes())
        .merge(session::routes())
        .merge(sync::routes())
        .merge(thread::routes())
        .merge(thread_member::routes())
        .merge(user::routes())
}
