import { createResource, For, Show, useContext } from "solid-js";
import { MemberT, Pagination, RoomT, ThreadT } from "./types.ts";
import { chatctx } from "./context.ts";
import { Message } from "./Messages.tsx";
import { getTimestampFromUUID } from "sdk";

const CLASS_BUTTON = "px-1 bg-bg3 hover:bg-bg4 my-0.5";

export const RoomHome = (props: { room: RoomT }) => {
  const ctx = useContext(chatctx)!;
	const room_id = props.room.id;
	
	async function createThread(room_id: string) {
  	const name = await ctx.dispatch({ do: "modal.prompt", text: "name?" });
		ctx.client.http("POST", `/api/v1/rooms/${room_id}/threads`, {
			name
		});
	}
	
	async function leaveRoom(room_id: string) {
  	if (!await ctx.dispatch({ do: "modal.confirm", text: "are you sure you want to leave?" })) return;
		ctx.client.http("DELETE", `/api/v1/rooms/${room_id}/members/@self`);
	}
	
  // const [threads, { refetch: fetchThreads }] = createResource<Pagination<ThreadT> & { room_id: string }, string>(() => props.room.id, async (room_id, { value }) => {
  // 	if (value?.room_id !== room_id) value = undefined;
  // 	if (value?.has_more === false) return value;
  // 	const lastId = value?.items.at(-1)?.id ?? "00000000-0000-0000-0000-000000000000";
  // 	const batch = await ctx.client.http("GET", `/api/v1/rooms/${room_id}/threads?dir=f&from=${lastId}&limit=100`);
  // 	return {
  // 		...batch,
  // 		items: [...value?.items ?? [], ...batch.items],
  // 		room_id,
  // 	};
  // });
	
  // <div class="date"><Time ts={props.thread.baseEvent.originTs} /></div>
  // TODO: use actual links instead of css styled divs
	return (
		<div class="room-home">
			<h2>{props.room.name}</h2>
			<p>{props.room.description}</p>
			<button onClick={() => createThread(room_id)}>create thread</button><br />
			<button onClick={() => leaveRoom(room_id)}>leave room</button><br />
			<button onClick={() => ctx.dispatch({ do: "setView", to: { view: "room-settings", room: props.room }})}>settings</button><br />
			<br />
			<ul>
	    	<For each={Object.values(ctx.data.threads).filter((i) => i.room_id === props.room.id)}>{thread => (
	      	<li>
	      	<article class="thread">
		      	<header
			      	onClick={() => ctx.dispatch({ do: "setView", to: { view: "thread", room: props.room, thread }})}
		      	>
			        <div class="flex items-center gap-[8px] leading-none">
			          <div class="bg-bg4 h-[16px] w-[16px] rounded-full"></div>
			          <div class="truncate text-lg flex-1">{thread.name}</div>
			          <div class="text-fg2">Created at {getTimestampFromUUID(thread.id).toDateString()}</div>
			        </div>
			        <div class="self-start mt-[8px] text-fg2 cursor-pointer hover:text-fg1 hover:underline" onClick={() => ctx.dispatch({ do: "setView", to: { view: "thread", room: props.room, thread }})}>
			          {thread.message_count} messages &bull; last msg {getTimestampFromUUID(thread.last_version_id ?? thread.id).toDateString()}
		          	<Show when={thread.description}>
		          		<br />
				          {thread.description}
	          		</Show>
			        </div>
		      	</header>
	  	      <Show when={true}>
			        <div class="preview">
			          <For each={[]}>
			            {(ev) => <Message message={ev} />}
			          </For>
			          <details>
				          <summary>json data</summary>
				          <pre>
						      	{JSON.stringify(thread, null, 4)}
				          </pre>
			          </details>
			        </div>
			      </Show>
			      <Show when={false}>
			        <footer>message.remaining</footer>
			      </Show>
	      	</article>
	    		</li>
	    	)}</For>
			</ul>
		</div>
	);
			// <button class={CLASS_BUTTON} onClick={() => fetchThreads()}>load more</button><br />
}
