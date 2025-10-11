import { listen } from "@tauri-apps/api/event";
import { useEffect, useRef, useState } from "react";
import type { AudioState } from "~/lib/tauri";
import { commands } from "~/lib/tauri";
import { DictatingView } from "./dictating-view";
import { IdleView } from "./idle-view";
import { TranscribingView } from "./transcribing-view";

export function AudioPanel() {
	const [state, setState] = useState<AudioState>({ type: "idle" });
	const [isHovered, setIsHovered] = useState(false);

	// Use ref to always get the latest state in event handlers
	const stateRef = useRef(state);

	// Keep ref in sync with state
	useEffect(() => {
		stateRef.current = state;
	}, [state]);

	// Listen to audio state changes
	useEffect(() => {
		let unlisten: (() => void) | null = null;

		const setupListener = async () => {
			unlisten = await listen<AudioState>("audio-state-changed", (event) => {
				setState(event.payload);
			});
		};

		setupListener();

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	}, []);

	// Handle hover events - only process when in idle state
	useEffect(() => {
		let unlisten: (() => void) | null = null;
		let tooltipTimeout: NodeJS.Timeout | null = null;

		const setupHoverListener = async () => {
			unlisten = await listen<boolean>("audio-panel-hover", (event) => {
				const isHovering = event.payload;
				setIsHovered(isHovering);

				// Use ref to get the current state (avoid closure issues)
				if (stateRef.current.type !== "idle") {
					return;
				}

				if (isHovering) {
					commands.resizeAudioPanel(72, 32).catch(console.error);

					// Clear previous timeout
					if (tooltipTimeout) {
						clearTimeout(tooltipTimeout);
					}

					// Wait for window resize animation before showing tooltip
					tooltipTimeout = setTimeout(() => {
						commands
							.showFeedback("点击开始语音识别", "tooltip", null)
							.catch(console.error);
						tooltipTimeout = null;
					}, 150);
				} else {
					// Clear timeout to avoid showing tooltip after hover leaves
					if (tooltipTimeout) {
						clearTimeout(tooltipTimeout);
						tooltipTimeout = null;
					}

					commands.resizeAudioPanel(40, 8).catch(console.error);
				}
			});
		};

		setupHoverListener();

		return () => {
			if (tooltipTimeout) {
				clearTimeout(tooltipTimeout);
			}
			if (unlisten) {
				unlisten();
			}
		};
	}, []); // Only setup once to avoid re-creating listeners

	return (
		<div className="h-screen w-screen flex items-center justify-center">
			{state.type === "idle" && <IdleView isHovered={isHovered} />}
			{state.type === "dictating" && <DictatingView mode={state.mode} />}
			{state.type === "transcribing" && <TranscribingView />}
		</div>
	);
}
