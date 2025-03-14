import { Client } from "./client.ts";
import type { Media } from "./types.ts";

export type UploadOptions = {
	file: File;
	client: Client;
	onProgress: (progress: number) => void;
	onFail: (error: Error) => void;
	onComplete: (media: Media) => void;
	onPause: () => void;
	onResume: () => void;
};

export type Upload = {
	media_id: string;
	pause(): void;
	resume(): void;
	abort(): void;
};

export async function createUpload(opts: UploadOptions): Promise<Upload> {
	const { data, error } = await opts.client.http.POST("/api/v1/media", {
		body: {
			filename: opts.file.name,
			size: opts.file.size,
		},
	});

	if (error) {
		opts.onFail(error);
		throw new Error(error);
	}

	const { upload_url, media_id } = data;
	let offset = 0;
	let currentOffset = 0;
	let xhr: XMLHttpRequest;

	async function resumeUpload() {
		// make sure to cancel the currently in flight upload, in case resume is called multiple times
		xhr?.abort();

		const res = await fetch(upload_url!, {
			method: "HEAD",
			headers: {
				"authorization": `Bearer ${opts.client.opts.token}`,
			},
		});
		if (res.ok) {
			offset = parseInt(res.headers.get("upload-offset")!, 10);
			currentOffset = offset;
			attemptUpload();
		} else {
			opts.onFail(
				new Error(`upload probe failed: ${await res.text() ?? res.statusText}`),
			);
		}
	}

	function attemptUpload() {
		xhr = new XMLHttpRequest();

		xhr.upload.onprogress = (ev) => {
			offset = ev.loaded + currentOffset;
			opts.onProgress(offset / opts.file.size);
		};

		xhr.onload = () => {
			if (xhr.status === 200) {
				const media = JSON.parse(xhr.responseText);
				opts.onComplete(media);
			} else if (xhr.status === 204) {
				opts.onFail(new Error("upload failed: incomplete file"));
			} else {
				opts.onFail(new Error(`upload failed: ${xhr.responseText}`));
			}
		};

		xhr.onabort = () => {
			console.log("upload manually aborted");
		};

		xhr.onerror = () => {
			console.log("upload failed, retrying in 1s...");
			setTimeout(resumeUpload, 1000);
		};

		xhr.open("PATCH", upload_url!);
		// TODO: handle missing token
		xhr.setRequestHeader("authorization", `Bearer ${opts.client.opts.token}`);
		xhr.setRequestHeader("upload-offset", offset.toString());
		xhr.send(opts.file.slice(offset));
	}

	function pause() {
		xhr?.abort();
		opts.onPause();
	}

	let started = false;
	function resume() {
		// save a roundtrip
		if (started) {
			resumeUpload();
		} else {
			attemptUpload();
			started = true;
		}

		opts.onResume();
	}

	async function abort() {
		xhr?.abort();
		await opts.client.http.DELETE("/api/v1/media/{media_id}", {
			params: {
				path: { media_id },
			},
		});
	}

	resume();
	return { media_id, pause, resume, abort };
}
