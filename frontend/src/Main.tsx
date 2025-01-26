import { createEffect, createSignal, For, ParentProps, Show } from "solid-js";
import { Portal } from "solid-js/web";
import { MessageMenu, RoomMenu, ThreadMenu } from "./menu/mod.ts";
import { ChatNav } from "./Nav.tsx";
import { Menu, Modal as ContextModal, useCtx } from "./context.ts";
import { ChatMain } from "./Chat.tsx";
import { RoomHome } from "./Room.tsx";
import { RoomSettings } from "./RoomSettings.tsx";
import { UserSettings } from "./UserSettings.tsx";
import { ClientRectObject, ReferenceElement, shift } from "@floating-ui/dom";
import { useFloating } from "solid-floating-ui";
import { Route, Router } from "@solidjs/router";
import { Home } from "./Home.tsx";

const Title = (props: { children: string }) => {
	createEffect(() => document.title = props.children);
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
		// force solid to track these properties
		ctx?.data.menu?.x;
		ctx?.data.menu?.y;

		setMenuParentRef({
			getBoundingClientRect(): ClientRectObject {
				const { menu } = ctx.data;
				if (!menu) throw new Error("missing menu!");
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
				return <RoomMenu room={menu.room} />;
			}
			case "thread": {
				return <ThreadMenu thread={menu.thread} />;
			}
			case "message": {
				return <MessageMenu message={menu.message} />;
			}
		}
	}

	function getModal(modal: ContextModal) {
		switch (modal.type) {
			case "alert": {
				return (
					<Modal>
						<p>{modal.text}</p>
						<div style="height: 8px"></div>
						<button onClick={() => ctx.dispatch({ do: "modal.close" })}>
							okay!
						</button>
					</Modal>
				);
			}
			case "confirm": {
				return (
					<Modal>
						<p>{modal.text}</p>
						<div style="height: 8px"></div>
						<button
							onClick={() => {
								modal.cont(true);
								ctx.dispatch({ do: "modal.close" });
							}}
						>
							okay!
						</button>&nbsp;
						<button
							onClick={() => {
								modal.cont(false);
								ctx.dispatch({ do: "modal.close" });
							}}
						>
							nevermind...
						</button>
					</Modal>
				);
			}
			case "prompt": {
				return (
					<Modal>
						<p>{modal.text}</p>
						<div style="height: 8px"></div>
						<form
							onSubmit={(e) => {
								e.preventDefault();
								const form = e.target as HTMLFormElement;
								const input = form.elements.namedItem(
									"text",
								) as HTMLInputElement;
								modal.cont(input.value);
								ctx.dispatch({ do: "modal.close" });
							}}
						>
							<input type="text" name="text" autofocus />
							<div style="height: 8px"></div>
							<input type="submit" value="done!"></input>{" "}
							<button
								onClick={() => {
									modal.cont(null);
									ctx.dispatch({ do: "modal.close" });
								}}
							>
								nevermind...
							</button>
						</form>
					</Modal>
				);
			}
		}
	}

	// HACK: wrap in Show since ctx might be null during hmr
	// this router is extremely messy - i'm not sure if i'm going to keep it or if i'll roll my own
	return (
		<>
			<Show when={useCtx()}>
				<Router>
					<Route
						path="/"
						component={() => (
							<>
								<Title>Home</Title>
								<ChatNav />
								<Home />
							</>
						)}
					/>
					<Route
						path="/settings/:page?"
						component={(p) => {
							const user = () => ctx.data.user;
							return (
								<>
									<Title>{user() ? "Settings" : "loading..."}</Title>
									<Show when={user()}>
										<UserSettings user={user()!} page={p.params.page} />
									</Show>
								</>
							);
						}}
					/>
					<Route
						path="/room/:room_id"
						component={(p) => {
							const room = () => ctx.data.rooms[p.params.room_id];
							return (
								<>
									<Title>{room() ? room().name : "loading..."}</Title>
									<ChatNav />
									<Show when={room()}>
										<RoomHome room={room()} />
									</Show>
								</>
							);
						}}
					/>
					<Route
						path="/room/:room_id/settings/:page?"
						component={(p) => {
							const room = () => ctx.data.rooms[p.params.room_id];
							return (
								<>
									<Title>
										{room() ? `${room().name} settings` : "loading..."}
									</Title>
									<ChatNav />
									<Show when={room()}>
										<RoomSettings room={room()} page={p.params.page} />
									</Show>
								</>
							);
						}}
					/>
					<Route
						path="/thread/:thread_id"
						component={(p) => {
							const thread = () => ctx.data.threads[p.params.thread_id];
							const room = () => ctx.data.rooms[thread()?.room_id];

							createEffect(() => {
								if (thread()?.room_id && !ctx.data.rooms[thread()?.room_id]) {
									ctx.dispatch({ do: "fetch.room", room_id: p.params.room_id });
								}
							});

							createEffect(() => {
								if (!ctx.data.threads[p.params.thread_id]) {
									ctx.dispatch({
										do: "fetch.thread",
										thread_id: p.params.thread_id,
									});
								}
							});

							return (
								<>
									<Title>
										{room() && thread()
											? `${thread().name} - ${room().name}`
											: "loading..."}
									</Title>
									<ChatNav />
									<Show when={room() && thread()}>
										<ChatMain room={room()} thread={thread()} />
									</Show>
								</>
							);
						}}
					/>
					<Route
						path="*404"
						component={() => (
							<div style="padding:8px">
								not found
							</div>
						)}
					/>
				</Router>
				<Portal mount={document.getElementById("overlay")!}>
					<For each={ctx.data.modals}>
						{(modal) => getModal(modal)}
					</For>
					<Show when={ctx.data.menu}>
						<div class="contextmenu">
							<div
								ref={setMenuRef}
								class="inner"
								style={{
									translate: `${menuFloating.x}px ${menuFloating.y}px`,
								}}
							>
								{getMenu(ctx.data.menu!)}
							</div>
						</div>
					</Show>
				</Portal>
			</Show>
		</>
	);
};

const Modal = (props: ParentProps) => {
	const ctx = useCtx()!;
	return (
		<div class="modal">
			<div class="bg" onClick={() => ctx.dispatch({ do: "modal.close" })}></div>
			<div class="content">
				<div class="base"></div>
				<div class="inner" role="dialog">
					{props.children}
				</div>
			</div>
		</div>
	);
};
