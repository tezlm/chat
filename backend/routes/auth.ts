import { Context, Next } from "npm:hono";
import { data, HonoEnv } from "globals";
import { RouteConfig } from "npm:@hono/zod-openapi";

type AuthOptions = {
	strict: boolean;
};

export const auth =
	(opts: AuthOptions) => async (c: Context<HonoEnv>, next: Next) => {
		const authToken = c.req.header("authorization");
		if (!authToken) return c.json({ error: "Missing authorization token" }, 401);
		const session = await data.sessionSelectByToken(authToken);
		if (!session) return c.json({ error: "Invalid or expired token" }, 401);
		// if (opts.strict && session.level as number < 1) {
		// 	return c.json({ error: "Unauthorized" }, 403);
		// }
		c.set("user_id", session.user_id);
		c.set("session_id", session.id);
		// c.set("session_level", session.level);
		await next();
	};

export function withAuth<T extends RouteConfig>(
	route: T,
	opts: AuthOptions = { strict: true },
) {
	const m = route.middleware;
	const middleware = [...Array.isArray(m) ? m : m ? [m] : [], auth(opts)];
	return { ...route, middleware } as T;
}
// import { Context, Next } from "npm:hono";
// import { data, HonoEnv } from "globals";
// import { RouteConfig } from "npm:@hono/zod-openapi";

// type AuthOptions = {
// 	strict: boolean;
// };

// export const auth =
// 	(opts: AuthOptions) => async (c: Context<HonoEnv>, next: Next) => {
// 		const auth = c.req.header("authorization");
// 		if (!auth) return c.json({ error: "Missing authorization token" }, 401);
// 		const row = db.prepareQuery("SELECT * FROM sessions WHERE token = ?")
// 			.firstEntry([auth]);
// 		if (!row) return c.json({ error: "Invalid or expired token" }, 401);
// 		if (opts.strict && row.level as number < 1) {
// 			return c.json({ error: "Unauthorized" }, 403);
// 		}
// 		c.set("user_id", row.user_id as string);
// 		c.set("session_id", row.session_id as string);
// 		c.set("session_level", row.level as number);
// 		await next();
// 	};

// export function withAuth<T extends RouteConfig>(
// 	route: T,
// 	opts: AuthOptions = { strict: true },
// ) {
// 	const m = route.middleware;
// 	const middleware = [...Array.isArray(m) ? m : m ? [m] : [], auth(opts)];
// 	return { ...route, middleware } as T;
// }
