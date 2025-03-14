import type { Pagination, RoomMember } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createEffect,
	createResource,
	type Resource,
	untrack,
} from "solid-js";
import type { Api, Listing } from "../api.tsx";

export class RoomMembers {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, ReactiveMap<string, RoomMember>>();
	_requests = new Map<string, Map<string, Promise<RoomMember>>>();
	_cachedListings = new Map<string, Listing<RoomMember>>();

	fetch(room_id: () => string, user_id: () => string): Resource<RoomMember> {
		const query = () => ({
			room_id: room_id(),
			user_id: user_id(),
		});

		const [resource, { mutate }] = createResource(
			query,
			({ room_id, user_id }) => {
				const cached = this.cache.get(room_id)?.get(user_id);
				if (cached) return cached;
				const existing = this._requests.get(room_id)?.get(user_id);
				if (existing) return existing;

				const req = (async () => {
					const { data, error } = await this.api.client.http.GET(
						"/api/v1/room/{room_id}/member/{user_id}",
						{
							params: { path: { room_id, user_id } },
						},
					);
					// HACK: handle 404s
					type ErrorResp = { error: string } | undefined;
					if ((error as ErrorResp)?.error === "not found") {
						const placeholder: RoomMember = {
							membership: "Leave",
							room_id,
							user_id,
							membership_updated_at: new Date().toISOString(),
						};
						return placeholder;
					}
					if (error) throw error;
					this._requests.get(room_id)?.delete(user_id);
					if (!this.cache.has(room_id)) {
						this.cache.set(room_id, new ReactiveMap());
					}
					this.cache.get(room_id)!.set(user_id, data);
					return data;
				})();

				if (!this._requests.has(room_id)) {
					this._requests.set(room_id, new Map());
				}
				this._requests.get(room_id)!.set(user_id, req);
				return req;
			},
		);

		createEffect(() => {
			const member = this.cache.get(room_id())?.get(user_id());
			if (member) mutate(member);
		});

		return resource;
	}

	list(room_id_sig: () => string): Resource<Pagination<RoomMember>> {
		const room_id = untrack(room_id_sig);

		const paginate = async (pagination?: Pagination<RoomMember>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/member",
				{
					params: {
						path: { room_id: room_id_sig() },
						query: {
							dir: "f",
							limit: 100,
							from: pagination?.items.at(-1)?.user_id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			const room_id = room_id_sig();
			let cache = this.cache.get(room_id);
			if (!cache) {
				cache = new ReactiveMap();
				this.cache.set(room_id, cache);
			}

			batch(() => {
				for (const item of data.items) {
					cache.set(item.user_id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		const l = this._cachedListings.get(room_id);
		if (l) {
			if (!l.prom) l.refetch();
			return l.resource;
		}

		const l2 = {
			resource: (() => {}) as unknown as Resource<Pagination<RoomMember>>,
			refetch: () => {},
			mutate: () => {},
			prom: null,
			pagination: null,
		};
		this._cachedListings.set(room_id, l2);

		const [resource, { refetch, mutate }] = createResource(
			room_id_sig,
			async (room_id) => {
				let l = this._cachedListings.get(room_id)!;
				if (!l) {
					l = {
						resource: (() => {}) as unknown as Resource<Pagination<RoomMember>>,
						refetch: () => {},
						mutate: () => {},
						prom: null,
						pagination: null,
					};
					this._cachedListings.set(room_id, l);
				}
				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;
				return res!;
			},
		);

		l2.resource = resource;
		l2.refetch = refetch;
		l2.mutate = mutate;

		return resource;
	}
}
