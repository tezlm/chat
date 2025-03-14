import { For, from, Show } from "solid-js";
import { useCtx } from "./context.ts";
import { A } from "@solidjs/router";
import { useApi } from "./api.tsx";
import type { Room, Thread } from "sdk";

export const ChatNav = () => {
	const ctx = useCtx();
	const api = useApi();
	const state = from(ctx.client.state);

	const rooms = api.rooms.list();

	return (
		<nav id="nav">
			<ul>
				<li>
					<A href="/" end>home</A>
				</li>
				<For each={rooms()?.items}>
					{(room) => <ItemRoom room={room} />}
				</For>
			</ul>
			<div style="flex:1"></div>
			<div style="margin: 8px">
				state: {state()}
			</div>
		</nav>
	);
};

const ItemRoom = (props: { room: Room }) => {
	const api = useApi();

	// TODO: send self room member in api? this works for now though
	const shouldShow = () => {
		const user_id = api.users.cache.get("@self")?.id;
		const c = api.room_members.cache.get(props.room.id);
		const m = c?.get(user_id!);
		if (m && m.membership !== "Join") return false;
		return true;
	};

	return (
		<Show when={shouldShow()}>
			<li>
				<A
					class="menu-room"
					data-room-id={props.room.id}
					href={`/room/${props.room.id}`}
				>
					{props.room.name}
				</A>
				<Show when={true}>
					<ul>
						<li>
							<A
								class="menu-room"
								href={`/room/${props.room.id}`}
								data-room-id={props.room.id}
							>
								home
							</A>
						</li>
						<For
							each={[
								...api.threads.cache.values().filter((i) =>
									i.room_id === props.room.id && i.state !== "Deleted"
								),
							]}
						>
							{(thread) => <ItemThread thread={thread} />}
						</For>
					</ul>
				</Show>
			</li>
		</Show>
	);
};

const ItemThread = (props: { thread: Thread }) => {
	return (
		<li>
			<A
				href={`/thread/${props.thread.id}`}
				class="menu-thread"
				classList={{
					"closed": props.thread.state === "Archived",
					"unread": props.thread.is_unread,
				}}
				data-thread-id={props.thread.id}
			>
				{props.thread.name}
			</A>
		</li>
	);
};
