import { OpenAPIHono } from "@hono/zod-openapi";
import {
	DmGet,
	DmInitialize,
	RoomAck,
	RoomCreate,
	RoomGet,
	RoomList,
	RoomUpdate,
} from "./def.ts";
import { withAuth } from "../auth.ts";
import { broadcast, HonoEnv, data } from "globals";
import { uuidv7 } from "uuidv7";

export default function setup(app: OpenAPIHono<HonoEnv>) {
	app.openapi(withAuth(RoomCreate), async (c) => {
		const roomReq = await c.req.json();
		const room = await data.roomInsert(uuidv7(), roomReq.name, roomReq.description);
		broadcast({ type: "upsert.room", room });
		return c.json(room, 201);
	});

	app.openapi(withAuth(RoomList), async (c) => {
		const rooms = await data.roomList({
			limit: parseInt(c.req.query("limit") ?? "10", 10),
			from: c.req.query("from"),
			to: c.req.query("to"),
			dir: c.req.query("dir") as "f" | "b",
		});
		return c.json(rooms, 200);
	});

	app.openapi(withAuth(RoomUpdate), async (c) => {
		const patch = await c.req.json();
		const room_id = c.req.param("room_id");
		const room = await data.roomUpdate(room_id, patch.name, patch.description);
		if (!room) return c.json({ error: "not found" }, 404);
		broadcast({ type: "upsert.room", room });
		return c.json(room, 200);
	});

	app.openapi(withAuth(RoomGet), async (c) => {
		const room_id = c.req.param("room_id");
		const room = await data.roomSelect(room_id);
		if (!room) return c.json({ error: "not found" }, 404);
		return c.json(room, 200);
	});
}
