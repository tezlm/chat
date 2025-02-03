import { Pagination, Thread } from "sdk";
import { ReactiveMap } from "@solid-primitives/map";
import {
	batch,
	createEffect,
	createResource,
	Resource,
} from "solid-js";
import { Api } from "../api.tsx";
import { createComputed } from "solid-js";

type Listing<T> = {
	// resource: Resource<Pagination<T>>;
	pagination: Pagination<T> | null;
	// mutate: (value: Pagination<T>) => void;
	// refetch: () => void;
	prom: Promise<unknown> | null;
};

export class Threads {
	api: Api = null as unknown as Api;
	cache = new ReactiveMap<string, Thread>();
	_requests = new Map<string, Promise<Thread>>();
	_cachedListings = new Map<string, Listing<Thread>>();
	_listingMutators = new Set<
		{ room_id: string; mutate: (value: Pagination<Thread>) => void }
	>();

	fetch(thread_id: () => string): Resource<Thread> {
		const [resource, { mutate }] = createResource(thread_id, (thread_id) => {
			const cached = this.cache.get(thread_id);
			if (cached) return cached;
			const existing = this._requests.get(thread_id);
			if (existing) return existing;

			const req = (async () => {
				const { data, error } = await this.api.client.http.GET(
					"/api/v1/thread/{thread_id}",
					{
						params: { path: { thread_id } },
					},
				);
				if (error) throw error;
				this._requests.delete(thread_id);
				this.cache.set(thread_id, data);
				return data;
			})();

			createEffect(() => {
				mutate(this.cache.get(thread_id));
			});

			this._requests.set(thread_id, req);
			return req;
		});

		return resource;
	}

	list(room_id_signal: () => string): Resource<Pagination<Thread>> {
		const paginate = async (pagination?: Pagination<Thread>) => {
			if (pagination && !pagination.has_more) return pagination;

			const { data, error } = await this.api.client.http.GET(
				"/api/v1/room/{room_id}/thread",
				{
					params: {
						path: { room_id: room_id_signal() },
						query: {
							dir: "f",
							limit: 100,
							from: pagination?.items.at(-1)?.id,
						},
					},
				},
			);

			if (error) {
				// TODO: handle unauthenticated
				console.error(error);
				throw error;
			}

			batch(() => {
				for (const item of data.items) {
					this.cache.set(item.id, item);
				}
			});

			return {
				...data,
				items: [...pagination?.items ?? [], ...data.items],
			};
		};

		// const room_id = untrack(room_id_signal);
		// const l = this._cachedControllers.get(room_id);
		// if (l) {
		// 	// NOTE: does this deduplicate refetches?
		// 	l.refetch();
		// 	return l.resource;
		// }

		createComputed(() => {
			// const l = {
			// 	resource: (() => {}) as unknown as Resource<Pagination<Thread>>,
			// 	refetch: () => {},
			// 	mutate: () => {},
			// 	prom: null,
			// 	pagination: null,
			// };
			console.log("set listing");
			this._cachedListings.set(room_id_signal(), {
				prom: null,
				pagination: null,
			});
		});

		const [resource, { mutate }] = createResource(
			room_id_signal,
			async (room_id) => {
				const l = this._cachedListings.get(room_id)!;
				if (l?.prom) {
					await l.prom;
					return l.pagination!;
				}

				const prom = l.pagination ? paginate(l.pagination) : paginate();
				l.prom = prom;
				const res = await prom;
				l!.pagination = res;
				l!.prom = null;

				for (const mut of this._listingMutators) {
					if (mut.room_id === room_id) mut.mutate(res);
				}

				return res!;
			},
		);

		const mut = { room_id: room_id_signal(), mutate };
		this._listingMutators.add(mut);

		createEffect(() => {
			mut.room_id = room_id_signal();
		});

		return resource;
	}
}
