import { getTimestampFromUUID, type Message, type Thread } from "sdk";
import { type MessageT, MessageType } from "./types.ts";
import { createSignal, For, Match, onMount, Show, Switch } from "solid-js";
import { marked } from "marked";
import sanitizeHtml from "sanitize-html";
import { useApi } from "./api.tsx";
import { useCtx } from "./context.ts";
import {
	AudioView,
	FileView,
	ImageView,
	TextView,
	VideoView,
} from "./media/mod.tsx";
import { flags } from "./flags.ts";
import { byteFmt, getUrl, type MediaProps } from "./media/util.tsx";
import { Time } from "./Time.tsx";
import { createTooltip, tooltip } from "./Tooltip.tsx";
import { UserView } from "./User.tsx";
import { UrlEmbedView } from "./UrlEmbed.tsx";

type MessageProps = {
	message: MessageT;
};

type MessageTextProps = {
	message: MessageT;
};

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	transformTags: {
		del: "s",
	},
};

const md = marked.use({
	breaks: true,
	gfm: true,
});

const contentToHtml = new WeakMap();

function MessageText(props: MessageTextProps) {
	function getHtml(): string {
		const cached = contentToHtml.get(props.message);
		if (cached) return cached;
		// console.count("render_html");
		const html = sanitizeHtml(
			md.parse(props.message.content!) as string,
			sanitizeHtmlOptions,
		).trim();
		contentToHtml.set(props.message, html);
		return html;
	}

	return (
		<div class="body markdown" classList={{ local: props.message.is_local }}>
			<span innerHTML={getHtml()}></span>
			<Show when={props.message.id !== props.message.version_id}>
				<span class="edited">(edited)</span>
			</Show>
		</div>
	);
}

export function MessageView(props: MessageProps) {
	const api = useApi();
	const thread = api.threads.fetch(() => props.message.thread_id);

	function getComponent() {
		const date =
			/^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(
					props.message.id,
				)
				? getTimestampFromUUID(props.message.id)
				: new Date();
		if (props.message.type === MessageType.ThreadUpdate) {
			const updates = [];
			const listFormatter = new Intl.ListFormat();
			const patch = props.message.patch as any;
			if (patch) {
				if (patch.name) updates.push(`set name to ${patch.name}`);
				if (patch.description) {
					updates.push(
						patch.description ? `set description to ${patch.description}` : "",
					);
				}
				if (patch.state) {
					updates.push(`set state to ${patch.state}`);
				}
			} else {
				console.warn("missing patch", props.message);
			}
			return (
				<>
					<span></span>
					<div class="content">
						<span class="body">
							<Author message={props.message} thread={thread()} />{" "}
							updated the thread:{" "}
							{listFormatter.format(updates) || "did nothing"}
						</span>
					</div>
					<div class="time">
						<Time date={date} animGroup="message-ts" />
					</div>
				</>
			);
		} else {
			const [arrow_width, set_arrow_width] = createSignal(0);
			const set_w = (e: HTMLElement) => {
				onMount(() => {
					set_arrow_width(
						e.querySelector(".user")!.getBoundingClientRect().width,
					);
				});
			};

			return (
				<article
					class="message menu-message"
					data-message-id={props.message.id}
				>
					<Show when={props.message.reply_id}>
						<ReplyView
							thread_id={props.message.thread_id}
							reply_id={props.message.reply_id!}
							arrow_width={arrow_width()}
						/>
					</Show>
					<div class="author-wrap">
						<div
							class="author sticky menu-user"
							classList={{ "override-name": !!props.message.override_name }}
							ref={set_w}
						>
							<Author message={props.message} thread={thread()} />
						</div>
					</div>
					<div class="content">
						<Show when={props.message.content}>
							<MessageText message={props.message} />
						</Show>
						<Show when={props.message.attachments?.length}>
							<ul class="attachments">
								<For each={props.message.attachments}>
									{(att) => <AttachmentView media={att} />}
								</For>
							</ul>
						</Show>
						<Show when={props.message.embeds?.length}>
							<ul class="embeds">
								<For each={props.message.embeds}>
									{(embed) => <UrlEmbedView embed={embed} />}
								</For>
							</ul>
						</Show>
					</div>
					<Time date={date} animGroup="message-ts" />
				</article>
			);
		}
	}

	return <>{getComponent()}</>;
}

type ReplyProps = {
	thread_id: string;
	reply_id: string;
	arrow_width?: number;
};

function ReplyView(props: ReplyProps) {
	const ctx = useCtx();
	const api = useApi();
	const reply = api.messages.fetch(() => props.thread_id, () => props.reply_id);
	const thread = api.threads.fetch(() => props.thread_id);

	const content = () => {
		const r = reply();
		if (!r) return;
		return r.content ?? `${r.attachments.length} attachment(s)`;
	};

	const scrollToReply = () => {
		// if (!props.reply) return;
		ctx.thread_anchor.set(props.thread_id, {
			type: "context",
			limit: 50, // TODO: calc dynamically
			message_id: props.reply_id,
		});
		ctx.thread_highlight.set(props.thread_id, props.reply_id);
	};

	return (
		<>
			<div class="reply">
				<div class="arrow">
					<svg
						viewBox="0 0 100 100"
						preserveAspectRatio="none"
						style={{ width: `${props.arrow_width}px` }}
					>
						<path
							vector-effect="non-scaling-stroke"
							shape-rendering="crispEdges"
							// M = move to x y
							// L = line to x y
							d="M 50 100 L 50 50 L 100 50"
						/>
					</svg>
				</div>
				<div class="content" onClick={scrollToReply}>
					<Show when={!reply.loading} fallback="loading...">
						<Show
							when={reply() && thread()}
							fallback={<span class="author"></span>}
						>
							<Author message={reply()!} thread={thread()!} />
						</Show>
						{content()}
					</Show>
				</div>
			</div>
		</>
	);
}

export function AttachmentView(props: MediaProps) {
	const b = () => props.media.source.mime.split("/")[0];
	const ty = () => props.media.source.mime.split(";")[0];
	if (b() === "image") {
		return (
			<li>
				<ImageView media={props.media} />
				<a download={props.media.filename} href={getUrl(props.media.source)}>
					download {props.media.filename}
				</a>
				<div class="dim">
					{ty()} - {byteFmt.format(props.media.source.size)}
				</div>
			</li>
		);
	} else if (b() === "video") {
		return (
			<li class="raw">
				<VideoView media={props.media} />
			</li>
		);
	} else if (b() === "audio") {
		return (
			<li class="raw">
				<AudioView media={props.media} />
			</li>
		);
	} else if (
		b() === "text" || /^application\/json\b/.test(props.media.source.mime)
	) {
		return (
			<li>
				<TextView media={props.media} />
			</li>
		);
	} else {
		return (
			<li>
				<FileView media={props.media} />
			</li>
		);
	}
}

function Author(props: { message: Message; thread?: Thread }) {
	const api = useApi();
	const room_member = props.thread
		? api.room_members.fetch(
			() => props.thread!.room_id,
			() => props.message.author_id,
		)
		: () => null;
	const thread_member = api.thread_members.fetch(
		() => props.message.thread_id,
		() => props.message.author_id,
	);
	const user = api.users.fetch(() => props.message.author_id);

	function name() {
		let name = props.message.override_name;
		const tm = thread_member();
		if (tm?.membership === "Join") name ??= tm.override_name;

		const rm = room_member?.();
		if (rm?.membership === "Join") name ??= rm.override_name;

		const us = user();
		name ??= us?.name;

		return name;
	}

	const { content } = createTooltip({
		// animGroup: "users",
		placement: "right-start",
		interactive: true,
		tip: () => (
			<UserView
				user={user()}
				room_member={room_member()}
				thread_member={thread_member()}
			/>
		),
	});

	return (
		<span
			class="user"
			classList={{ "override-name": !!props.message.override_name }}
			data-user-id={props.message.author_id}
			use:content
		>
			{name()}
		</span>
	);
}
