import { Component, onCleanup } from "solid-js";
import { ChatCtx, chatctx, Data, defaultData } from "./context.ts";
import { createStore } from "solid-js/store";
import { Main } from "./Main.tsx";
import { createDispatcher } from "./dispatch.ts";
import { createClient } from "sdk";

const BASE_URL = localStorage.getItem("base_url") ??
	"https://chat.celery.eu.org";

// TODO: refactor bootstrap code?
const App: Component = () => {
	const TOKEN = localStorage.getItem("token")!;
	const client = createClient({
		baseUrl: BASE_URL,
		onState(state) {
			console.log({ state });
		},
		onSync(msg) {
			console.log("recv", msg);
			ctx.dispatch({
				do: "server",
				msg,
			});
		},
		onReady(msg) {
			ctx.dispatch({ do: "server.ready", msg });
		},
	});

	const [data, update] = createStore<Data>(defaultData);

	const ctx: ChatCtx = {
		client,
		data,
		dispatch: () => {
			throw new Error("oh no!");
		},
	};
	const dispatch = createDispatcher(ctx, update);
	ctx.dispatch = dispatch;

	if (TOKEN) {
		client.start(TOKEN);
		ctx.dispatch({ do: "init" });
	} else {
		ctx.dispatch({ do: "server.init_session" });
	}

	onCleanup(() => client.stop());

	const handleClick = () => {
		dispatch({ do: "menu", menu: null });
	};

	const handleKeypress = (e: KeyboardEvent) => {
		if (e.key === "Escape") dispatch({ do: "modal.close" });
	};

	const handleMouseMove = (e: MouseEvent) => {
		dispatch({ do: "window.mouse_move", e });
	};

	globalThis.addEventListener("click", handleClick);
	globalThis.addEventListener("keydown", handleKeypress);
	globalThis.addEventListener("mousemove", handleMouseMove);
	onCleanup(() => {
		globalThis.removeEventListener("click", handleClick);
		globalThis.removeEventListener("keydown", handleKeypress);
		globalThis.removeEventListener("mousemove", handleMouseMove);
	});

	return (
		<div id="root">
			<chatctx.Provider value={{ client, data, dispatch }}>
				<Main />
			</chatctx.Provider>
		</div>
	);
};

export default App;
