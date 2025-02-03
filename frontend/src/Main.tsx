import { createEffect, createSignal, For, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { MessageMenu, RoomMenu, ThreadMenu } from "./menu/mod.ts";
import { ChatNav } from "./Nav.tsx";
import { Menu, useCtx } from "./context.ts";
import { ChatMain } from "./Chat.tsx";
import { RoomHome, RoomMembers } from "./Room.tsx";
import { RoomSettings } from "./RoomSettings.tsx";
import { UserSettings } from "./UserSettings.tsx";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { Route, Router, RouteSectionProps } from "@solidjs/router";
import { Home } from "./Home.tsx";
import { getModal } from "./modal/mod.tsx";
import { useApi } from "./api.tsx";
import { flags } from "./flags.ts";

const Title = (props: { title: string }) => {
	createEffect(() => document.title = props.title);
	return undefined;
};

export const Main = () => {
	const ctx = useCtx();

	const [menuParentRef, setMenuParentRef] = createSignal<ReferenceElement>();
	const [menuRef, setMenuRef] = createSignal<HTMLElement>();
	const menuFloating = useFloating(menuParentRef, menuRef, {
		middleware: [shift({ mainAxis: true, crossAxis: true, padding: 8 })],
		placement: "right-start",
	});

	createEffect(() => {
		ctx.menu();
		
		setMenuParentRef({
			getBoundingClientRect(): ClientRectObject {
				const menu = ctx.menu();
				if (!menu) return {};
				return {
					x: menu.x,
					y: menu.y,
					left: menu.x,
					top: menu.y,
					right: menu.x,
					bottom: menu.y,
					width: 0,
					height: 0,
				};
			},
		});
	});

	function getMenu(menu: Menu) {
		switch (menu.type) {
			case "room": {
				return <RoomMenu room_id={menu.room_id} />;
			}
			case "thread": {
				return <ThreadMenu thread_id={menu.thread_id} />;
			}
			case "message": {
				return <MessageMenu thread_id={menu.thread_id} message_id={menu.message_id} />;
			}
		}
	}

	// HACK: wrap in Show since ctx might be null during hmr
	// this router is extremely messy - i'm not sure if i'm going to keep it or if i'll roll my own
	return (
		<>
			<Show when={useCtx()}>
				<Router>
					<Route path="/" component={RouteHome} />
					<Route path="/settings/:page?" component={RouteSettings} />
					<Route path="/room/:room_id" component={RouteRoom} />
					<Route
						path="/room/:room_id/settings/:page?"
						component={RouteRoomSettings}
					/>
					<Route path="/thread/:thread_id" component={RouteThread} />
					<Route path="*404" component={RouteNotFound} />
				</Router>
				<Portal mount={document.getElementById("overlay")!}>
					<For each={ctx.data.modals}>
						{(modal) => getModal(modal)}
					</For>
					<Show when={ctx.menu()}>
						<div class="contextmenu">
							<div
								ref={setMenuRef}
								class="inner"
								style={{
									translate: `${menuFloating.x}px ${menuFloating.y}px`,
								}}
							>
								{getMenu(ctx.menu()!)}
							</div>
						</div>
					</Show>
				</Portal>
			</Show>
		</>
	);
};

const RouteHome = () => {
	return (
		<>
			<Title title="Home" />
			<ChatNav />
			<Home />
		</>
	);
};

function RouteSettings(p: RouteSectionProps) {
	const api = useApi();
	const user = () => api.users.cache.get("@self");
	return (
		<>
			<Title title={user() ? "Settings" : "loading..."} />
			<Show when={user()}>
				<UserSettings user={user()!} page={p.params.page} />
			</Show>
		</>
	);
}

function RouteRoom(p: RouteSectionProps) {
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	return (
		<>
			<Title title={room() ? room()!.name : "loading..."} />
			<ChatNav />
			<Show when={room()}>
				<RoomHome room={room()!} />
				<Show when={flags.has("room_member_list")}>
					<RoomMembers room={room()!} />
				</Show>
			</Show>
		</>
	);
}

function RouteRoomSettings(p: RouteSectionProps) {
	const api = useApi();
	const room = api.rooms.fetch(() => p.params.room_id);
	const title = () => room() ? `${room()!.name} settings` : "loading...";
	return (
		<>
			<Title title={title()} />
			<ChatNav />
			<Show when={room()}>
				<RoomSettings room={room()!} page={p.params.page} />
			</Show>
		</>
	);
}

function RouteThread(p: RouteSectionProps) {
	const api = useApi();
	const thread = api.threads.fetch(() => p.params.thread_id);
	const room = api.rooms.fetch(() => thread()?.room_id!);

	return (
		<>
			<Show when={room()} fallback={<Title title="loading..." />}>
				<Title title={`${thread()!.name} - ${room()!.name}`} />
			</Show>
			<ChatNav />
			<Show when={room()}>
				<ChatMain room={room()!} thread={thread()!} />
			</Show>
		</>
	);
}

function RouteNotFound() {
	return (
		<div style="padding:8px">
			not found
		</div>
	);
}
