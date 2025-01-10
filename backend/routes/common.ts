import { createRoute, RouteConfig, z } from "npm:@hono/zod-openapi";
import { Uint } from "../types.ts";

export const common = {
	429: {
		description: "ratelimited",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	400: {
		description: "bad request",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	401: {
		description: "auth required",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	403: {
		description: "missing permissions",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
					// permissions: Permissions,
				}),
			},
		},
	},
	404: {
		description: "not found",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	409: {
		description: "conflict",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	413: {
		description: "content too large",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	415: {
		description: "unsupported media type",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	500: {
		description: "internal error",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
	501: {
		description: "todo",
		content: {
			"application/json": {
				schema: z.object({
					error: z.string(),
				}),
			},
		},
	},
};

type PaginationConfig = Omit<RouteConfig, "responses"> & {
	pagination: {
		id: z.ZodString,
		ty: z.AnyZodObject
	},
	query?: z.AnyZodObject,
}

export const createPagination = ({ pagination, query, ...config }: PaginationConfig) => createRoute({
	...config,
	request: {
		...config.request,
		query: z.object({
			from: pagination.id.optional(),
			to: pagination.id.optional(),
			dir: z.enum(["f", "b"]),
			limit: z.string().default("10").transform((i) => parseInt(i, 10)).pipe(
				Uint.min(1).max(100),
			),
		}).merge(query ?? z.object({})),
	},
	responses: {
		...common,
		200: {
			description: "success",
			content: {
				"application/json": {
					schema: z.object({
						items: pagination.ty.array(),
						total: Uint,
						has_more: z.boolean(),
					}),
				},
			},
		},
	},
});
