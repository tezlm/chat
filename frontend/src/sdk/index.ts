import { UUID, uuidv7 } from "uuidv7";
import EventEmitter from "event-emitter";

// HACK: https://github.com/andywer/typed-emitter/issues/39
import TypedEventEmitter, { EventMap } from "typed-emitter";
type TypedEmitter<T extends EventMap> = TypedEventEmitter.default<T>;

export type MessageId = string;
export type ThreadId = string;
export type RoomId = string;

export class Room {
  constructor(
    public readonly client: Client,
    public readonly id: string,
    public readonly data: any,
  ) {}

  async fetch(): Promise<Room> {
    const data = await this.client.http("GET", `/api/v1/rooms/${this.id}`);
    const room = new Room(this.client, this.id, data);
    this.client.rooms.set(this.id, room);
    return room;
  }  
}

export class Thread {
  public readonly timelines: TimelineSet;
  
  constructor(
    public readonly client: Client,
    public readonly id: string,
    public readonly data: any,
    timelines?: TimelineSet,
  ) {
    this.timelines = timelines ?? new TimelineSet(client, this.id);
  }
  
  async fetch(): Promise<Thread> {
    const data = await this.client.http("GET", `/api/v1/threads/${this.id}`);
    const thread = new Thread(this.client, this.id, data, this.timelines);
    this.client.threads.set(this.id, thread);
    return thread;
  }

  async send(content: object) {
		const nonce = uuidv7();
		const msg = new Message(this.client, this.id, `local-${uuidv7()}`, {
      ...content,
			nonce,
      is_local: true,
    });
    this.timelines.live.messages.push(msg);
    this.timelines.live.events.emit("append", [msg]);
		await this.client.http("POST", `/api/v1/threads/${this.id}/messages`, {
			...content,
			nonce,
		});
  }
}

export class Message {
  constructor(
    public readonly client: Client,
    public readonly thread_id: string,
    public readonly id: string,
    public readonly data: any,
  ) {}

  async fetch(): Promise<Message> {
    const data = await this.client.http("GET", `/api/v1/threads/${this.thread_id}/messages/${this.id}`);
    return new Message(this.client, this.thread_id, this.id, data);
  }
}

export class TimelineSet {  
  public live: Timeline;
  public timelines = new Set<Timeline>();
  public map = new Map<MessageId, Timeline>();
  
  constructor(
    public readonly client: Client,
    public readonly thread_id: ThreadId,
  ) {
    this.live = new Timeline(client, thread_id, [], false, true);
    this.timelines.add(this.live);
  }

  // async fetch(at: EventId | "start" | "end", limit = 50): Promise<Timeline> {
  // }
}

type TimelineEvents = {
  prepend: (messages: Array<Message>) => void,
  append: (messages: Array<Message>) => void,
}

export class Timeline {
  public events: TypedEmitter<TimelineEvents> = new EventEmitter();
  
  constructor(
    public readonly client: Client,
    public readonly thread_id: ThreadId,
    public messages: Array<Message> = [],
    public isAtBeginning = false,
    public isAtEnd = false,
  ) {}

  public async paginate(dir: "f" | "b", limit: number = 50): Promise<Timeline> {
    if (dir === "b" && this.isAtBeginning) return this;
    if (dir === "f" && this.isAtEnd) return this;

    const url = new URL(`/api/v1/threads/${this.thread_id}/messages`, this.client.baseUrl);
    url.searchParams.set("limit", limit.toString());
    if (dir === "f") {
      const after = this.messages[0]?.id ?? "00000000-0000-0000-0000-000000000000";
      url.searchParams.set("after", after);
    } else {
      const before = this.messages.at(-1)?.id ?? "ffffffff-ffff-ffff-ffff-ffffffffffff";
      url.searchParams.set("before", before);
    }
    
		const data = await this.client.httpDirect("GET", url.href);
		const batch = data.items.map((i: any) => new Message(this.client, i.thread_id, i.id, i));
		if (dir === "f") {
		  this.messages.push(...batch);
  		this.isAtEnd = !data.has_more;
  		this.events.emit("prepend", batch);
		} else {
		  this.messages.unshift(...batch);
  		this.isAtBeginning = !data.has_more;
  		this.events.emit("append", batch);
		}
		// TODO: merge timelines
		return this;
		
    //   const events = data.chunk.map(intoEvent(room));
    //   if (data.prev_batch) {
    //     this.prevBatch = data.prev_batch;
    //   } else {
    //     this.isAtBeginning = true;
    //   }
    //   this._eventList.unshift(...events);
    //   for (const event of events) {
    //     const existing = this.timelineSet.timelineMap.get(event.id);
    //     if (existing) {
    //       // TODO
    //       // const merged = merge(this.timelineSet, this);
    //       // if (merged !== this) {
    //         // this.emit("timelineReplace", merged);
    //         // return 0;
    //       // }
    //     } else {
    //       this.timelineSet.timelineMap.set(event.id, this);
    //       room.events.set(event.id, event);
    //     }
    //   }
    //   this.emit("timelineUpdate", events, true);
    //   return events.length;
  }
}

// FIXME: this has some kind of bug
// function merge(timelines: ThreadTimelineSet, tl: ThreadTimeline): ThreadTimeline {
//   // [_, _, 2, 3, 4, 5, _, _, _] (current)
//   // [_, _, _, 3, 4, 5, 6, 7, 8] (other1)
//   // [0, 1, 2, 3, 4, _, _, _, _] (other2)
//   // [0, 1, 2, 3, 4, 5, 6, 7, 8] (other3)
//   // [_, _, 2, 3, 4, 5, 6, 7, 8] (other4)
//   // event = 3, thisIdx = 3, otherIdx1 = 0
//   // event = 2, thisIdx = 0, otherIdx2 = 2
//   // event = 2, thisIdx = 0, otherIdx3 = 2
//   // event = 2, thisIdx = 0, otherIdx4 = 0

//   const events = tl._eventList;

//   for (let idx = 0; idx < events.length; idx++) {
//     const event = events[idx];
//     const other = timelines.timelineMap.get(event.id);
//     if (!other) continue;
//     if (tl === other) continue;

//     other._eventList.unshift(...events.slice(0, idx));
//     other.isAtBeginning = tl.isAtBeginning;

//     for (const event of events.slice(0, idx)) {
//       timelines.timelineMap.set(event.id, other);
//     }
//     timelines.timelines.delete(tl);
//     tl.emit("timelineReplace", other);
//     tl = other;
//   }

//   // do it again because a thread can overlap 2 threads (before and after)
//   // maybe this works?
//   for (let idx = 0; idx < events.length; idx++) {
//     const event = events[idx];
//     const other = timelines.timelineMap.get(event.id);
//     if (!other) continue;
//     if (tl === other) continue;
    
//     other._eventList.unshift(...events.slice(0, idx));
//     other.isAtBeginning = tl.isAtBeginning;

//     for (const event of events.slice(0, idx)) {
//       timelines.timelineMap.set(event.id, other);
//     }
//     timelines.timelines.delete(tl);
//     tl.emit("timelineReplace", other);
//     tl = other;
//   }
  
//   return tl;
// }

// abstract class TimelineSet {
//   abstract timelines: Set<Timeline>;
//   abstract timelineMap: Map<EventId, Timeline>;
  
//   merge(events: Array<Event>): Timeline | null {
//     for (const event of events) {
//       const tl = this.timelineMap.get(event.id);
//       if (!tl) continue;
//       return tl;
//     }
//     return null;
//   }
// }

// export class ThreadTimelineSet extends TimelineSet {
//   // This is the one live timeline
//   public client = this.thread.room.client;
//   public live: ThreadTimeline;
//   timelines: Set<ThreadTimeline> = new Set();
//   timelineMap: Map<EventId, ThreadTimeline> = new Map();
  
//   constructor(public thread: Thread) {
//     super();
//     this.live = new ThreadTimeline(this, thread);
//     this.live.isAtEnd = true;
//     this.timelines.add(this.live);
//   }

//   public async fetch(at: EventId | "start" | "end", limit = 50): Promise<ThreadTimeline> {
//     if (at === "end") {
//       const fetchCount = limit - this.live.getEvents().length;
//       if (fetchCount > 0) await this.live.paginate("b", fetchCount);
//       const tl = this.live;
//       const realTimeline = merge(this, tl);
//       if (realTimeline !== tl) {
//         this.timelines.delete(tl);
//         this.live = realTimeline;
//       }
//       return realTimeline ?? tl;
//     } else if (at === "start") {
//       const existing = [...this.timelines].find(i => i.isAtBeginning);
//       if (existing) {
//         const fetchCount = limit - existing.getEvents().length;
//         if (fetchCount > 0) await existing.paginate("f", fetchCount);
//         return existing;
//       } else {
//         const tl = new ThreadTimeline(this, this.thread);
//         await tl.paginate("f", limit);
//         const realTimeline = merge(this, tl);
//         if (realTimeline !== tl) this.timelines.add(realTimeline);
//         return realTimeline;
//       }
//     } else {
//       // TODO: respect limit?
//       // TODO: merge threads
//       const existing = this.timelineMap.get(at);
//       if (existing) {
//         const fetchCount = limit - existing.getEvents().length;
//         if (fetchCount > 0) await existing.paginate("f", fetchCount);
//         return existing;
//       } else {
//         const tl = new ThreadTimeline(this, this.thread);
//         const context = await this.client.net.fetchContext(this.thread.room.id, at, 0);
//         const event = intoEvent(this.thread.room)(context.event);
//         tl._eventList = [event];
//         tl.prevBatch = context.start;
//         tl.nextBatch = context.end;
//         this.timelineMap.set(event.id, tl);
//         this.timelines.add(tl);
//         return tl;
//       }
//     }
//   }
// }

type ClientEvents = {
  ready: () => void,
  close: () => void,
  update: () => void,
}

export class Client {
	private ws: WebSocket | undefined;
	public rooms = new Map<string, Room>();
	public threads = new Map<string, Thread>();
	public user: any = null;
	public events: TypedEmitter<ClientEvents> = new EventEmitter();

	constructor(
		private token: string,
		public baseUrl: string,
	) {}

	connect() {
		this.ws = new WebSocket(`${this.baseUrl}/api/v1/sync`);

		this.ws.onopen = () => {
			console.log("opened");
			this.ws!.send(JSON.stringify({ type: "hello", token: this.token }));
		};

		this.ws.onclose = () => {
			console.log("closed");
			this.events.emit("close");
		};

		this.ws.onmessage = (ev) => {
			const msg = JSON.parse(ev.data);
			console.log("recv", msg);

  		if (msg.type === "ping") {
  			this.ws!.send(JSON.stringify({ type: "pong" }));
  		} else if (msg.type === "ready") {
  		  this.user = msg.user;
  			this.events.emit("ready");
    	} else if (msg.type === "upsert.room") {
    	  this.rooms.set(msg.room.id, new Room(this, msg.room.id, msg.room));
    	} else if (msg.type === "upsert.thread") {
    	  const existing = this.threads.get(msg.thread.id);
    	  this.threads.set(msg.thread.id, new Thread(this, msg.thread.id, msg.thread, existing?.timelines));
    	} else if (msg.type === "upsert.message") {
    	  const { thread_id } = msg.message;
    	  if (!this.threads.has(thread_id)) this.threads.set(thread_id, new Thread(this, thread_id, {}));
    	  const message = new Message(this, thread_id, msg.message.id, msg.message);
    	  const thread = this.threads.get(thread_id)!;
    	  const { live } = thread.timelines;
    	  const messages = [...live.messages.filter(i => i.data.nonce !== message.data.nonce), message];
    	  live.messages = messages;
    	} else {
    		console.warn("unknown message type", msg.type);
    		return;
    	}
			this.events.emit("update");
		};
	}

	http(
		method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
		url: string,
		body?: any,
	) {
		console.log(`${method} ${url}`);
		return this.httpDirect(method, `${this.baseUrl}${url}`, body);
	}
	
	async httpDirect(
		method: "GET" | "POST" | "PUT" | "PATCH" | "DELETE",
		url: string,
		body?: any,
	) {
		console.log(`${method} ${url}`);
		const req = await fetch(url, {
			method,
			headers: {
				"authorization": this.token,
				"content-type": "application/json",
			},
			body: body ? JSON.stringify(body) : undefined,
		});
		if (!req.ok) {
			throw new Error(`request failed (${req.status}): ${await req.text()}`);
		}
		return req.json();
	}

  async fetchRoom(id: string): Promise<Room> {
    const existing = this.rooms.get(id);
    if (existing) return existing.fetch();
    const data = await this.http("GET", `/api/v1/rooms/${id}`);
    const room = new Room(this, id, data);
    this.rooms.set(id, room);
		this.events.emit("update");
    return room;
  }
  
  async fetchThread(id: string): Promise<Thread> {
    const existing = this.threads.get(id);
    if (existing) return existing.fetch();
    const data = await this.http("GET", `/api/v1/threads/${id}`);
    const thread = new Thread(this, id, data);
    this.threads.set(id, thread);
		this.events.emit("update");
    return thread;
  }
}

export function getTimestampFromUUID(uuid: string): Date {
	const bytes = UUID.parse(uuid).bytes;
	const timestamp = bytes.slice(0, 6).reduce(
		(acc: number, e: number) => acc * 256 + e,
		0,
	);
	return new Date(timestamp);
}

export async function *createPagination(client: Client, path: string, after?: string) {
	const url = new URL(path, client.baseUrl);
	url.searchParams.set("limit", "5");
	if (after) url.searchParams.set("after", after);
	while (true) {
	  const batch = await client.httpDirect("GET", url.href);
	  for (const item of batch.items) yield item;
	  if (!batch.has_more) break;
	  console.log(batch.items)
		url.searchParams.set("after", batch.items.at(-1).id);
	}
}
