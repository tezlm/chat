import { OpenAPIHono, z } from "@hono/zod-openapi";
import { data, events, HonoEnv } from "globals";
import { SyncInit } from "./def.ts";
import { upgradeWebSocket } from "npm:hono/deno";
import { MessageClient, MessageServer } from "../../types/sync.ts";
import { uuidv7 } from "uuidv7";
import { fetchDataAndPermissions } from "../auth.ts";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(SyncInit, async (c, next) => {
		const id = uuidv7();
		let ws: WebSocket;
		let state: "closed" | "unauth" | "auth" = "closed";
		let heartbeatTimeout: number;
		let closeTimeout: number;
		let user_id: string;

		function send(msg: z.infer<typeof MessageServer>) {
			if (state === "closed") {
				throw new Error("tried to send message to closed websocket");
			}
			console.log(`send websocket ${id}`, msg);
			ws.send(JSON.stringify(msg));
		}

		function rescheduleHeartbeat() {
			clearTimeout(heartbeatTimeout);
			clearTimeout(closeTimeout);
			heartbeatTimeout = setTimeout(() => {
				if (state === "closed") return;
				ws.send(JSON.stringify({ type: "ping" }));
			}, 1000 * 30);
			closeTimeout = setTimeout(() => {
				if (state === "closed") return;
				ws.close();
			}, 1000 * 45);
		}

		async function handle(msg: z.infer<typeof MessageServer>) {
			if (state === "closed") return;
			// if (state === "unauth") return;
			// TODO: handle deletes
			// TODO: make this more efficient/less awful
			if (msg.type === "upsert.session" && msg.session.user_id === user_id) {
				ws.send(JSON.stringify(msg));
			} else {
				const { permissions: perms } = await fetchDataAndPermissions({
					user_id_self: user_id,
					message_id: msg.type === "upsert.message" ? msg.message.id :  msg.type === "delete.message" ? msg.id : undefined,
					room_id: msg.type === "upsert.room" ? msg.room.id : msg.type === "upsert.member" ? msg.member.room_id : undefined,
					thread_id: msg.type === "upsert.thread" ? msg.thread.id : msg.type === "upsert.message" ? msg.message.thread_id : undefined,
				});
				console.log(perms, msg)
				if (!perms.has("View")) return;
				ws.send(JSON.stringify(msg));
			}
		}

		async function handleHello(token: string, _last_id?: string) {
			const session = await data.sessionSelectByToken(token);
			if (!session) return c.json({ error: "Invalid or expired token" }, 401);
			// if (row.level as number < 1) return c.json({ error: "Unauthorized" }, 403);
			user_id = session.user_id;
			const user = await data.userSelect(user_id);
			if (!user) {
				throw new Error("user doesn't exist, but session does...!?");
			}
			state = "auth";
			send({ type: "ready", user });
		}

		const middle = upgradeWebSocket(() => ({
			onOpen(ev) {
				console.log(`opened websocket ${id}`);
				ws = ev.target as WebSocket;
				state = "unauth";
				events.on("sushi", handle);
				rescheduleHeartbeat();
			},
			onClose() {
				console.log(`closed websocket ${id}`);
				events.off("sushi", handle);
				state = "closed";
			},
			onMessage(event, ws) {
				try {
					console.log(`recv websocket ${id}`, event.data);
					const msg = MessageClient.parse(JSON.parse(event.data as string));
					// console.log(`recv websocket ${id}`, msg);
					if (msg.type === "hello") {
						if (state === "auth") {
							send({ type: "error", error: "already authenticated" });
							return;
						}
						handleHello(msg.token, msg.last_id);
					} else if (msg.type === "pong") {
						rescheduleHeartbeat();
					}
				} catch (err) {
					console.log(`websocket error ${id}`, err);
					ws.close();
					state = "closed";
				}
			},
		}));

		const r = await middle(c, next);
		return r ?? c.text("error", 500);
	});
}
