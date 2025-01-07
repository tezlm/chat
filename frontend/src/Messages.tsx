// import { Tooltip } from "./Atoms.tsx";
import { getTimestampFromUUID } from "sdk";
import {
	createEffect,
	createSignal,
	lazy,
	Match,
	onMount,
	ParentProps,
	Show,
	Switch,
} from "solid-js";
import { TimelineItemT } from "./list.tsx";
import { MessageT, MessageType } from "./types.ts";
import { marked } from "marked";
// @ts-types="npm:@types/sanitize-html@^2.13.0"
import sanitizeHtml from "npm:sanitize-html";
import { useCtx } from "./context.ts";

const Tooltip = (props: ParentProps<{ tip: any, attrs: any }>) => props.children;

const sanitizeHtmlOptions: sanitizeHtml.IOptions = {
	transformTags: {
		del: "s"
	}
}

type UserProps = {
	name: string;
};

const User = (props: UserProps) => {
	return (
		<div>
			<h3>{props.name}</h3>
			<p>some info here</p>
			<p>more stuff</p>
			<p>click to view full profile</p>
		</div>
	);
};

const WRAPPER_CSS = "group grid grid-cols-[128px_auto_max-content] px-[8px] hover:bg-bg1/30";
const BODY_CSS = "mx-[8px] overflow-hidden markdown";

type MessageProps = {
	message: MessageT;
};

const md = marked.use({
	breaks: true,
	gfm: true,
});

export const Message = (props: MessageProps) => {
	const ctx = useCtx();
	let bodyEl: HTMLSpanElement;

	// createEffect(async () => {
	// 	props.message; // make it react
	// 	// FIXME: flash of unhighlighted code on update
	// 	const hljs = await import("highlight.js");
	// 	for (const code of bodyEl.querySelectorAll("code[class*=language-]")) {
	// 		hljs.default.highlightElement(code);
	// 	}
	// });

	function Reply(props: { reply: MessageT }) {
		const name = props.reply.override_name ?? props.reply.author.name;
		
		return (
			<>
				<div class="mb-[-4px] text-xs text-fg5 text-right">{"\u21B1"}</div>
				<div class="mb-[-4px] text-xs text-fg4 mx-[8px]">
					<span class="text-fg5">{name}: </span>
					{props.reply.content}
				</div>
				<div class="mb-[-4px]">
				</div>
			</>
		)
	}

	function getComponent() {
		const date = /^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(props.message.id) ? getTimestampFromUUID(props.message.id) : new Date();
		const authorName = props.message.override_name ?? props.message.author.name;
		if (props.message.type === MessageType.ThreadUpdate) {
			const updates = [];
			const listFormatter = new Intl.ListFormat();
			const patch = props.message.metadata;
			if (patch.name) updates.push(`set name to ${patch.name}`);
			if (patch.description) updates.push(patch.description ? `set description to ${patch.description}` : "");
			if (patch.is_locked) updates.push(patch.is_locked ? "locked thread" : "unlocked thread");
			if (patch.is_closed) updates.push(patch.is_closed  ? "closed thread" : "unarchived thread");
			return (
				<div class={WRAPPER_CSS}>
					<span class="text-fg4 text-right"></span>
					<span class={BODY_CSS} ref={bodyEl!}>
						<span class="hover:underline cursor-pointer">{authorName}</span>
						{" "}updated the thread: {listFormatter.format(updates) || "did nothing"}
					</span>
					<span class="invisible group-hover:visible text-fg4">
						{(/^[a-z0-9]{8}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{4}-[a-z0-9]{12}$/.test(props.message.id) ? getTimestampFromUUID(props.message.id) : new Date).toDateString()}
					</span>
				</div>
			)
		} else {
			// console.log(md.parse(props.message.content!));
			return (
				<div class={props.message.reply_id ? `${WRAPPER_CSS} grid-rows-[auto_auto]` : WRAPPER_CSS}>
					<Show when={props.message.reply_id && ctx.data.messages[props.message.reply_id!]}>
						<Reply reply={ctx.data.messages[props.message.reply_id!]} />
					</Show>
					<span
						class="hover:underline cursor-pointer truncate text-right"
						classList={{
							"text-[#9ca9db]": !!props.message.override_name,
							"text-fg4": !props.message.override_name,
					}}>
						<Tooltip
							tip={() => <User name={authorName} />}
							attrs={{ class: "" }}
						>
							{authorName}
						</Tooltip>
					</span>
					<Show when={props.message.content}>
						<span class={BODY_CSS} ref={bodyEl!} innerHTML={sanitizeHtml(md.parse(props.message.content!) as string, sanitizeHtmlOptions).trim()}></span>
					</Show>
					<span class="invisible group-hover:visible text-fg4">{date.toDateString()}</span>
				</div>
			)
		}
	}

	return <>{getComponent()}</>;
};

function getTimelineItem(item: TimelineItemT) {
	switch(item.type) {
		case "message": {
			// unread: item.message.unread,
			// "bg-[#67dc8222]": item.message.mention,
			// "shadow-arst": item.message.mention || item.message.unread,
			// "shadow-[#67dc82]": item.message.mention,
			// "shadow-[#3fa9c9]": item.message.unread,
			// "text-fg4": item.message.is_local,
			return (
				<li data-message-id={item.message.id}>
					<Message message={item.message} />
				</li>
			);
		}
		case "info": {
					// <header class="sticky top-[0] px-[144px] bg-bg3 mb-[8px] border-b-[1px] border-b-sep mt-[-8px]">
					// <header class="shadow-foo shadow-[#0009] bg-bg1 p-2 text-cente">
					// 	<p>more info here</p>
					// </header>
			return (
				<li class="contents">
					<header class="px-[144px] bg-bg3 mb-4">
						<h1 class="text-xl">header</h1>
						<p>more info here</p>
					</header>
				</li>
			)
		}
		case "spacer": {
			return <li class="flex-1"><div class="h-[800px]"></div></li>
		}
		case "spacer-mini": {
			return <li><div class="h-8"></div></li>
		}
	}
}

export const TimelineItem = (props: { item: TimelineItemT }) => {
	return (<>{getTimelineItem(props.item)}</>);

	// <Match when={props.item.type === "unread-marker" && false}>
	// 	<li class="text-[#3fa9c9] shadow-arst shadow-[#3fa9c9] shadow-[#3fa9c922]">
	// 		<div class="grid grid-cols-[128px_auto_max-content] px-[8px]">
	// 			<span class="sender">-----</span>
	// 			<span class="body">new messages</span>
	// 		</div>
	// 	</li>
	// </Match>
	// <Match when={props.item.type === "unread-marker"}>
	// 	<li classList={{ unreadMarker2: true }}>
	// 		<hr />
	// 		<span>unread messages</span>
	// 		<hr />
	// 	</li>
	// </Match>
	// <Match when={props.item.type === "time-split" && false}>
	// 	<li
	// 		classList={{
	// 			message: true,
	// 			timeSplit: true,
	// 		}}
	// 	>
	// 		<div class="grid grid-cols-[128px_auto_max-content] px-[8px]">
	// 			<span class="sender">-----</span>
	// 			<span class="body">
	// 				time changed to{" "}
	// 				<time>{new Date(props.msg.origin_ts).toDateString()}</time>
	// 			</span>
	// 		</div>
	// 	</li>
	// </Match>
	// <Match when={props.item.type === "time-split"}>
	// 	<li
	// 		classList={{
	// 			timeSplit2: true,
	// 		}}
	// 	>
	// 		<hr />
	// 		<time>{new Date(props.msg.origin_ts).toDateString()}</time>
	// 		<hr />
	// 	</li>
	// </Match>
}

// export const Messages = (props: MessagesProps) => {
// 	return (
// 		<>
// 			<ul class="flex flex-col justify-end py-[8px]" classList={{ "notime": props.notime }}>
// 				{props.messages.map((i) => <TimelineItem msg={i} />)}
// 			</ul>
// 		</>
// 	);
// };

// {props.message.type === "message_html"
// 	? (
// 		<span
// 			class={BODY_CSS}
// 			ref={bodyEl!}
// 			innerHTML={props.message.body}
// 		>
// 		</span>
// 	)
// 	: <span class={BODY_CSS} ref={bodyEl!}>{props.message.body}</span>}
