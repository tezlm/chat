import { createEffect, createSignal, onCleanup, VoidProps } from "solid-js";
import { Media } from "sdk";
import iconPlay from "../assets/play.png";
import iconPause from "../assets/pause.png";
import iconVolumeLow from "../assets/volume-low.png";
import iconVolumeMedium from "../assets/volume-medium.png";
import iconVolumeHigh from "../assets/volume-high.png";
import iconVolumeMute from "../assets/volume-mute.png";
import iconVolumeMax from "../assets/volume-max.png";
import { formatTime } from "./util.ts";

type MediaProps = VoidProps<{ media: Media }>;

export const AudioView = (props: MediaProps) => {
	// NOTE: not using audio element so i can keep audio alive while scrolling (will impl later)
	const audio = new globalThis.Audio();
	createEffect(() => audio.src = props.media.url);
	onCleanup(() => audio.pause());

	const [duration, setDuration] = createSignal(
		(props.media.duration ?? 0) / 1000,
	);
	const [progress, setProgress] = createSignal(0);
	const [progressPreview, setProgressPreview] = createSignal<null | number>(
		null,
	);
	const [playing, setPlaying] = createSignal(false);
	const [volume, setVolume] = createSignal(1);
	const [muted, setMuted] = createSignal(false);

	audio.ondurationchange = () => setDuration(audio.duration);
	audio.ontimeupdate = () => setProgress(audio.currentTime);
	audio.onplay = () => setPlaying(true);
	audio.onpause = () => setPlaying(false);
	audio.onvolumechange = () => setVolume(audio.volume);

	createEffect(() => audio.muted = muted());

	const togglePlayPause = () => {
		if (playing()) {
			audio.pause();
		} else {
			audio.play();
		}
	};

	const toggleMute = () => {
		if (muted()) {
			setMuted(false);
		} else {
			setMuted(true);
		}
	};

	const handleVolumeWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			audio.volume = Math.max(volume() - .05, 0);
		} else {
			audio.volume = Math.min(volume() + .05, 1);
		}
	};

	const handleScrubWheel = (e: WheelEvent) => {
		e.preventDefault();
		if (e.deltaY > 0) {
			audio.currentTime = Math.max(progress() - 5, 0);
		} else {
			audio.currentTime = Math.min(progress() + 5, duration());
		}
	};

	const handleScrubClick = () => {
		audio.currentTime = progressPreview()!;
	};

	const handleScrubMouseOut = () => {
		setProgressPreview(null);
	};

	const handleScrubMouseMove = (e: MouseEvent) => {
		const target = e.target as HTMLElement;
		const { x, width } = target.getBoundingClientRect();
		const p = ((e.clientX - x) / width) * duration();
		setProgressPreview(p);
		if (e.buttons) audio.currentTime = p;
	};

	const progressWidth = () => `${(progress() / duration()) * 100}%`;
	const progressPreviewWidth = () =>
		progressPreview()
			? `${(progressPreview()! / duration()) * 100}%`
			: undefined;

	const byteFmt = Intl.NumberFormat("en", {
		notation: "compact",
		style: "unit",
		unit: "byte",
		unitDisplay: "narrow",
	});

	const ty = () => props.media.mime.split(";")[0];

	const getVolumeIcon = () => {
		if (muted()) return iconVolumeMute;
		if (volume() === 0) return iconVolumeMute;
		if (volume() < .333) return iconVolumeLow;
		if (volume() < .667) return iconVolumeMedium;
		if (volume() <= 1) return iconVolumeHigh;
		return iconVolumeMax;
	};

	const getVolumeText = () => {
		if (muted()) return "muted";
		return `${Math.round(volume() * 100)}%`;
	};

	return (
		<div class="audio">
			<div
				class="progress"
				onWheel={handleScrubWheel}
				onMouseOut={handleScrubMouseOut}
				onMouseMove={handleScrubMouseMove}
				onMouseDown={handleScrubClick}
				onClick={handleScrubClick}
			>
				<div class="fill" style={{ width: progressWidth() }}></div>
				<div class="preview" style={{ width: progressPreviewWidth() }}></div>
			</div>
			<div class="info">
				<a
					download={props.media.filename}
					title={props.media.filename}
					href={props.media.url}
				>
					{props.media.filename}
				</a>
				<div class="dim">{ty()} - {byteFmt.format(props.media.size)}</div>
			</div>
			<div class="controls">
				<button onClick={togglePlayPause} title={playing() ? "pause" : "play"}>
					<img
						class="icon"
						src={playing() ? iconPause : iconPlay}
						alt={playing() ? "pause" : "play"}
					/>
				</button>
				<button
					onClick={toggleMute}
					title={getVolumeText()}
					onWheel={handleVolumeWheel}
				>
					<img
						class="icon"
						src={getVolumeIcon()}
						alt={getVolumeText()}
					/>
				</button>
				<div class="space"></div>
				<div
					class="time"
					classList={{ preview: progressPreview() !== null }}
					onWheel={handleScrubWheel}
				>
					<span class="progress">
						{formatTime(progressPreview() ?? progress())}
					</span>{" "}
					/ <span class="duration">{formatTime(duration())}</span>
				</div>
			</div>
		</div>
	);
};
