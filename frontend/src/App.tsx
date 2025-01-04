import { Component, JSX, Show, untrack } from "solid-js";
import { createEffect, createSignal, For, onCleanup } from "solid-js";
import { Dynamic } from "solid-js/web";
import { ChatMain, ChatNav } from "./Chat.tsx";
import { Client, Room, Thread } from "sdk";
import { chatctx } from "./context.ts";

const BASE_URL = "http://localhost:8000";
// const TOKEN = "0a11b93f-ff19-4c56-9bd2-d25bede776de";
const TOKEN = "abcdefg";

const App: Component = () => {
	const [hash, setHash] = createSignal(location.hash.slice(1));
	const [title, setTitle] = createSignal(document.title);
	const [isReady, setIsReady] = createSignal(false);
	const [roomId, setRoomId] = createSignal<string | undefined>(
		"01942ef7-3f8b-7537-80f2-f821870cdd8f"
	);
	const [threadId, setThreadId] = createSignal<string | undefined>(
		"01942ef7-5bb2-7b36-b6de-e0b62387e3f8"
	);

	const [room, setRoom] = createSignal<Room>();
	const [thread, setThread] = createSignal<Thread>();
	const [rooms, setRooms] = createSignal<Array<Room>>([]);
	const [threads, setThreads] = createSignal<Array<Thread>>([]);

	const client = new Client(TOKEN, BASE_URL);
	client.events.on("ready", () => setIsReady(true));
	client.events.on("close", () => setIsReady(false));
	client.events.on("update", () => {
		console.log("update");
		setRooms([...client.rooms.values()]);
		setThreads([...client.threads.values()]);
		roomId() && setRoom(client.rooms.get(roomId()!));
		threadId() && setThread(client.threads.get(threadId()!));
	});
	globalThis.client = client;
	client.connect();

	createEffect(async () => {
		if (roomId() && !client.rooms.has(roomId()!)) await client.fetchRoom(roomId()!);
		if (roomId()) setRoom(client.rooms.get(roomId()!));
	});
	
	createEffect(async () => {
		if (threadId() && !client.threads.has(threadId()!)) await client.fetchThread(threadId()!);
		if (threadId()) setThread(client.threads.get(threadId()!));
	});
	
	createEffect(async () => {
		await (roomId() && client.temp_fetchThreadsInRoom(roomId()!));
		console.log("fetch threads", [...client.threads.values()])
		setThreads([...client.threads.values()]);
	});

	const handleHashChange = () => setHash(location.hash.slice(1));
	globalThis.addEventListener("hashchange", handleHashChange);
	onCleanup(() => {
		globalThis.removeEventListener("hashchange", handleHashChange);
	});
	createEffect(() => document.title = title());
	createEffect(() => location.hash = hash());
	// createEffect(() => setTitle(parts.get(hash())?.title ?? "unknown"));

	return (
		<div id="root" class="flex h-screen font-sans">
			<chatctx.Provider value={{ client, roomId, threadId, thread, room, setRoomId, setThreadId }}>
				<ChatNav rooms={rooms()} threads={threads()} />
				<Show when={thread()} fallback={<div>thread not found...</div>}>
					<ChatMain thread={thread()!} />
				</Show>
			</chatctx.Provider>
		</div>
	);
};

export default App;
