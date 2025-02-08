// TODO: this file is getting big and should probably be split and refactored
// i'm copypasting stuff for now, but will refactor out abstractions later

import {
	Accessor,
	batch,
	Component,
	createContext,
	createSignal,
	ParentProps,
	Resource,
	useContext,
} from "solid-js";
import { ReactiveMap } from "@solid-primitives/map";
import {
	Client,
	Invite,
	Media,
	Message,
	MessageCreate,
	MessageReady,
	MessageSync,
	Pagination,
	Role,
	Room,
	RoomMember,
	Session,
	Thread,
	User,
} from "sdk";
import { Emitter } from "@solid-primitives/event-bus";
import {
	MessageListAnchor,
	MessageRange,
	MessageRanges,
	Messages,
} from "./api/messages.ts";
import { Rooms } from "./api/rooms.ts";
import { Threads } from "./api/threads.ts";
import { Users } from "./api/users.ts";
import { Invites } from "./api/invite.ts";
import { RoomMembers } from "./api/room_members.ts";
import { Roles } from "./api/roles.ts";

export type Json =
	| number
	| string
	| boolean
	| Array<Json>
	| { [k in string]: Json };

const ApiContext = createContext<Api>();

export function useApi() {
	return useContext(ApiContext)!;
}

export function createApi(
	client: Client,
	temp_events: Emitter<{
		sync: MessageSync;
		ready: MessageReady;
	}>,
) {
	const [session, setSession] = createSignal<Session | null>(null);

	const rooms = new Rooms();
	const threads = new Threads();
	const invites = new Invites();
	const roles = new Roles();
	const room_members = new RoomMembers();
	const users = new Users();
	const messages = new Messages();
	const typing = new ReactiveMap<string, Set<string>>();
	const typing_timeout = new Map<string, Map<string, number>>();

	temp_events.on("sync", (msg) => {
		if (msg.type === "UpsertRoom") {
			const { room } = msg;
			rooms.cache.set(room.id, room);
			if (rooms._cachedListing?.pagination) {
				const l = rooms._cachedListing;
				const p = l.pagination!;
				const idx = p.items.findIndex((i) => i.id === room.id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, room),
					});
				} else if (p.items.length === 0 || room.id > p.items[0].id) {
					l.mutate({
						...p,
						items: [...p.items, room],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "UpsertThread") {
			const { thread } = msg;
			threads.cache.set(thread.id, thread);
			const l = threads._cachedListings.get(thread.room_id);
			if (l?.pagination) {
				const p = l.pagination;
				const idx = p.items.findIndex((i) => i.id === thread.id);
				if (idx !== -1) {
					for (const mut of threads._listingMutators) {
						if (mut.room_id === thread.room_id) {
							mut.mutate({
								...p,
								items: p.items.toSpliced(idx, 1, thread),
							});
						}
					}
				} else if (p.items.length === 0 || thread.id > p.items[0].id) {
					for (const mut of threads._listingMutators) {
						if (mut.room_id === thread.room_id) {
							mut.mutate({
								...p,
								items: [...p.items, thread],
								total: p.total + 1,
							});
						}
					}
				}
			}
		} else if (msg.type === "UpsertUser") {
			users.cache.set(msg.user.id, msg.user);
			if (msg.user.id === users.cache.get("@self")?.id) {
				users.cache.set("@self", msg.user);
			}
		} else if (msg.type === "UpsertSession") {
			if (msg.session?.id === session()?.id) {
				setSession(session);
			}
		} else if (msg.type === "UpsertRoomMember") {
			const m = msg.member;
			const c = room_members.cache.get(m.room_id);
			if (c) {
				c.set(m.user_id, m);
			} else {
				room_members.cache.set(m.room_id, new ReactiveMap());
				room_members.cache.get(m.room_id)!.set(m.user_id, m);
			}
			const l = room_members._cachedListings.get(m.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.user_id === m.user_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, m),
					});
				} else {
					l.mutate({
						...p,
						items: [...p.items, m],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "UpsertMessage") {
			const m = msg.message;
			const r = messages.cacheRanges.get(m.thread_id);
			if (r) {
				if (m.nonce) {
					// local echo
					const idx = r.live.items.findIndex((i) => i.nonce === m.nonce);
					if (idx !== -1) {
						r.live.items.splice(idx, 1);
					}
				} else if (m.version_id !== m.id) {
					// edits
					const idx = r.live.items.findIndex((i) => i.id === m.id);
					if (idx !== -1) {
						r.live.items.splice(idx, 1);
					}
				}
				r.live.items.push(m);
				batch(() => {
					messages.cache.set(m.id, m);
					messages._updateMutators(r, m.thread_id);
				});
			}
			const t = api.threads.cache.get(m.thread_id);
			if (t) {
				api.threads.cache.set(m.thread_id, {
					...t,
					message_count: t.message_count + (m.id === m.version_id ? 1 : 0),
					last_version_id: m.version_id,
				});
			}

			{
				const t = typing.get(m.thread_id);
				if (t) {
					t.delete(m.author.id);
					typing.set(m.thread_id, new Set(t));
					clearTimeout(typing_timeout.get(m.thread_id)!.get(m.author.id));
				}
			}
		} else if (msg.type === "DeleteMessage") {
			batch(() => {
				const { message_id, thread_id } = msg;
				const ranges = messages.cacheRanges.get(thread_id);
				const r = ranges?.find(message_id);
				if (ranges && r) {
					const idx = r.items.findIndex((i) => i.id === message_id);
					if (idx !== -1) {
						r.items.splice(idx, 1);
					}
					messages.cache.delete(thread_id);
					messages._updateMutators(ranges, thread_id);
				}
				const t = api.threads.cache.get(msg.thread_id);
				if (t) {
					api.threads.cache.set(msg.thread_id, {
						...t,
						message_count: t.message_count - 1,
						last_version_id: ranges?.live.end ?? t.last_version_id,
					});
				}
			});
		} else if (msg.type === "UpsertInvite") {
			const { invite } = msg;
			invites.cache.set(invite.code, invite);
			console.log("upsert", invites);
			if (invite.target.type === "Room") {
				const room_id = invite.target.room.id;
				const l = invites._cachedListings.get(room_id);
				if (l?.pagination) {
					const p = l.resource.latest;
					if (p) {
						const idx = p.items.findIndex((i) => i.code === invite.code);
						if (idx !== -1) {
							l.mutate({
								...p,
								items: p.items.toSpliced(idx, 1, invite),
							});
						} else {
							l.mutate({
								...p,
								items: [...p.items, invite],
								total: p.total + 1,
							});
						}
					}
				}
			}
		} else if (msg.type === "DeleteInvite") {
			const invite = invites.cache.get(msg.code);
			console.log("delete invite", invite);
			if (invite) {
				if (invite.target.type === "Room") {
					const room_id = invite.target.room.id;
					const l = invites._cachedListings.get(room_id);
					console.log(l);
					if (l?.pagination) {
						const p = l.resource.latest;
						if (p) {
							const idx = p.items.findIndex((i) => i.code === invite.code);
							console.log("splice", idx);
							if (idx !== -1) {
								l.mutate({
									...p,
									items: p.items.toSpliced(idx, 1),
								});
							}
						}
					}
				}
			}
			invites.cache.delete(msg.code);
		} else if (msg.type === "UpsertRole") {
			const r = msg.role;
			roles.cache.set(r.id, r);
			const l = roles._cachedListings.get(r.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === r.id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1, r),
					});
				} else {
					l.mutate({
						...p,
						items: [...p.items, r],
						total: p.total + 1,
					});
				}
			}
		} else if (msg.type === "DeleteRole") {
			roles.cache.delete(msg.role_id);
			const l = roles._cachedListings.get(msg.room_id);
			if (l?.resource.latest) {
				const p = l.resource.latest;
				const idx = p.items.findIndex((i) => i.id === msg.role_id);
				if (idx !== -1) {
					l.mutate({
						...p,
						items: p.items.toSpliced(idx, 1),
						total: p.total - 1,
					});
				}
			}
		} else if (msg.type === "Typing") {
			const { thread_id, user_id, until } = msg;
			const t = typing.get(thread_id) ?? new Set();
			typing.set(thread_id, new Set([...t, user_id]));

			const timeout = setTimeout(() => {
				console.log("remove typing");
				const t = typing.get(thread_id)!;
				t.delete(user_id);
				typing.set(thread_id, new Set(t));
			}, Date.parse(until) - Date.now());

			const tt = typing_timeout.get(thread_id);
			if (tt) {
				const tu = tt.get(user_id);
				if (tu) clearTimeout(tu);
				tt.set(user_id, timeout);
			} else {
				const tt = new Map();
				tt.set(user_id, timeout);
				typing_timeout.set(thread_id, tt);
			}
		} else {
			console.warn(`unknown event ${msg.type}`, msg);
		}
	});

	temp_events.on("ready", (msg) => {
		if (msg.user) {
			users.cache.set("@self", msg.user);
			users.cache.set(msg.user.id, msg.user);
		}
		setSession(msg.session);
	});

	async function tempCreateSession() {
		const res = await client.http.POST("/api/v1/session", {
			body: {},
		});
		if (!res.data) {
			console.error("failed to init session", res.response);
			throw new Error("failed to init session");
		}
		const session = res.data;
		localStorage.setItem("token", session.token);
		setSession(session);
		client.start(session.token);
	}

	// FIXME: make reactive again
	const api: Api = {
		rooms,
		threads,
		invites,
		roles,
		room_members,
		users,
		messages,
		session,
		typing,
		tempCreateSession,
		client: client,
		Provider(props: ParentProps) {
			return (
				<ApiContext.Provider value={api}>
					{props.children}
				</ApiContext.Provider>
			);
		},
	};

	messages.api = api;
	rooms.api = api;
	threads.api = api;
	roles.api = api;
	room_members.api = api;
	invites.api = api;
	users.api = api;

	console.log("provider created", api);
	return api;
}

type MessageSendReq = Omit<MessageCreate, "nonce"> & {
	attachments: Array<Media>;
};

export type Api = {
	rooms: {
		fetch: (room_id: () => string) => Resource<Room>;
		list: () => Resource<Pagination<Room>>;
		cache: ReactiveMap<string, Room>;
	};
	threads: {
		fetch: (thread_id: () => string) => Resource<Thread>;
		list: (room_id: () => string) => Resource<Pagination<Thread>>;
		cache: ReactiveMap<string, Thread>;
	};
	invites: {
		fetch: (invite_code: () => string) => Resource<Invite>;
		list: (room_id: () => string) => Resource<Pagination<Invite>>;
		cache: ReactiveMap<string, Invite>;
	};
	roles: {
		fetch: (room_id: () => string, role_id: () => string) => Resource<Role>;
		list: (room_id: () => string) => Resource<Pagination<Role>>;
		cache: ReactiveMap<string, Role>;
	};
	room_members: {
		fetch: (
			room_id: () => string,
			user_id: () => string,
		) => Resource<RoomMember>;
		list: (room_id: () => string) => Resource<Pagination<RoomMember>>;
		cache: ReactiveMap<string, ReactiveMap<string, RoomMember>>;
	};
	users: {
		fetch: (user_id: () => string) => Resource<User>;
		cache: ReactiveMap<string, User>;
	};
	messages: {
		send: (
			thread_id: string,
			message: MessageSendReq,
		) => Promise<Message>;
		list: (
			thread_id: () => string,
			anchor: () => MessageListAnchor,
		) => Resource<MessageRange>;
		fetch: (
			thread_id: () => string,
			message_id: () => string,
		) => Resource<Message>;
		cache: ReactiveMap<string, Message>;
		cacheRanges: Map<string, MessageRanges>;
	};
	session: Accessor<Session | null>;
	typing: ReactiveMap<string, Set<string>>;
	tempCreateSession: () => void;
	client: Client;
	Provider: Component<ParentProps>;
};

export type Listing<T> = {
	resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	mutate: (value: Pagination<T>) => void;
	refetch: () => void;
	prom: Promise<unknown> | null;
};
