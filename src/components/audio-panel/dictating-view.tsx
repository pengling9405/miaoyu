import type { DictatingMode } from "~/lib/tauri";
import { CloseButton } from "./close-button";
import { StopButton } from "./stop-button";
import { Timer } from "./timer";
import { AudioWave } from "./wave";

interface DictatingViewProps {
	mode: DictatingMode;
}

export function DictatingView({ mode }: DictatingViewProps) {
	const showButtons = mode === "normal";

	return (
		<div className="bg-background border border-border h-full w-full rounded-full shadow-sm flex items-center justify-center gap-2">
			{showButtons && <CloseButton />}
			<AudioWave />
			<Timer />
			{showButtons && <StopButton />}
		</div>
	);
}
