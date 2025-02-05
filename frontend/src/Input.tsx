import { For, Match, Show, Switch } from "solid-js/web";
import { Attachment, useCtx } from "./context.ts";
import { ThreadT } from "./types.ts";
import Editor from "./Editor.tsx";
import { uuidv7 } from "uuidv7";
import { renderAttachment } from "./Message.tsx";
import { useApi } from "./api.tsx";

type InputProps = {
	thread: ThreadT;
};

export function Input(props: InputProps) {
	const ctx = useCtx();
	const api = useApi();
	const reply_id = () => ctx.thread_reply_id.get(props.thread.id);
	const reply = () => api.messages.cache.get(reply_id()!);

	function handleUpload(file: File) {
		console.log(file);
		const local_id = uuidv7();
		ctx.dispatch({
			do: "upload.init",
			file,
			local_id,
			thread_id: props.thread.id,
		});
	}

	function uploadFile(e: InputEvent) {
		const target = e.target! as HTMLInputElement;
		const files = target.files!;
		for (const file of files) {
			handleUpload(file);
		}
	}

	const editor_state = () => ctx.thread_editor_state.get(props.thread.id)!;
	const atts = () => ctx.thread_attachments.get(props.thread.id);

	return (
		<div class="input">
			<Show when={atts()?.length}>
				<ul class="attachments">
					<For each={atts()}>
						{(att) => (
							<li>
								{renderAttachment2(props.thread, att)}
							</li>
						)}
					</For>
				</ul>
			</Show>
			<Show when={reply_id()}>
				<div class="reply">
					<button
						class="cancel"
						onClick={() => ctx.thread_reply_id.delete(props.thread.id)}
					>
						cancel
					</button>
					<div class="info">
						replying to {reply()?.override_name ?? reply()?.author.name}:{" "}
						{reply()?.content}
					</div>
				</div>
			</Show>
			<div class="text">
				<label class="upload">
					upload file
					<input
						multiple
						type="file"
						onInput={uploadFile}
						value="upload file"
					/>
				</label>
				<Show when={editor_state()}>
					<Editor
						state={editor_state()}
						onUpload={handleUpload}
						placeholder="send a message..."
					/>
				</Show>
			</div>
		</div>
	);
}

function renderAttachment2(thread: ThreadT, att: Attachment) {
	const ctx = useCtx();

	function renderInfo(att: Attachment) {
		if (att.status === "uploading") {
			if (att.progress === att.file.size) {
				return `processing...`;
			} else {
				const percent = ((att.progress / att.file.size) * 100).toFixed(2);
				return `uploading (${percent}%)`;
			}
		} else {
			return renderAttachment(att.media);
		}
	}

	function removeAttachment(local_id: string) {
		const atts = ctx.thread_attachments.get(thread.id);
		if (!atts) return;
		ctx.thread_attachments.set(
			thread.id,
			atts.filter((i) => i.local_id !== local_id),
		);
	}

	return (
		<>
			<div>
				{renderInfo(att)}
			</div>
			<button onClick={() => removeAttachment(att.local_id)}>
				cancel/remove
			</button>
			<Switch>
				<Match when={att.status === "uploading" && att.paused}>
					<button
						onClick={() =>
							ctx.dispatch({ do: "upload.resume", local_id: att.local_id })}
					>
						resume
					</button>
				</Match>
				<Match when={att.status === "uploading"}>
					<button
						onClick={() =>
							ctx.dispatch({ do: "upload.pause", local_id: att.local_id })}
					>
						pause
					</button>
				</Match>
			</Switch>
		</>
	);
}
