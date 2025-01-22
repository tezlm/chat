import type { components } from "./schema.d.ts";

export type Room = components["schemas"]["Room"];
export type Thread = components["schemas"]["Thread"];
export type User = components["schemas"]["User"];
export type Message = components["schemas"]["Message"];
export type Role = components["schemas"]["Role"];
// export type Invite = components["schemas"]["Invite"];
export type Session = components["schemas"]["Session"];
export type RoomMember = components["schemas"]["RoomMember"];

export type Invite = { code: string };

export type MessageServer =
	| { type: "Ping" }
	| { type: "Ready"; user: User }
	| { type: "Error"; error: string }
	| { type: "UpsertRoom"; room: Room }
	| { type: "UpsertThread"; thread: Thread }
	| { type: "UpsertMessage"; message: Message }
	| { type: "UpsertUser"; user: User }
	| { type: "UpsertMember"; member: RoomMember }
	| { type: "UpsertSession"; session: Session }
	| { type: "UpsertRole"; role: Role }
	| { type: "UpsertInvite"; invite: Invite }
	| { type: "DeleteMessage"; thread_id: string; message_id: string }
	| {
		type: "DeleteMessageVersion";
		thread_id: string;
		message_id: string;
		version_id: string;
	}
	| { type: "DeleteUser"; id: string }
	| { type: "DeleteSession"; id: string }
	| { type: "DeleteRole"; room_id: string; role_id: string }
	| { type: "DeleteMember"; room_id: string; user_id: string }
	| { type: "DeleteInvite"; code: string }
	| { type: "Webhook"; hook_id: string; data: unknown };
