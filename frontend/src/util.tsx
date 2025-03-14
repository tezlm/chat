import { getTimestampFromUUID, type Message } from "sdk";
import { useCtx } from "./context";

export function createWeaklyMemoized<T extends object, U>(
	fn: (_: T) => U,
): (_: T) => U {
	const cache = new WeakMap();
	return (t: T) => {
		const cached = cache.get(t);
		if (cached) return cached;
		const ran = fn(t);
		cache.set(t, ran);
		return ran;
	};
}

export const getMsgTs = createWeaklyMemoized((m: Message) =>
	getTimestampFromUUID(m.id)
);

export const Copyable = (props: { children: string }) => {
	const ctx = useCtx();
	const copy = () => {
		navigator.clipboard.writeText(props.children);
		ctx.dispatch({ do: "modal.alert", text: "copied!" });
	};

	return <code onClick={copy}>{props.children}</code>;
};
