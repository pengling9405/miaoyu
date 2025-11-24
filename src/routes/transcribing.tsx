import { createFileRoute } from "@tanstack/react-router";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import { useEffect, useState } from "react";
import { MovingLabel } from "~/components/moving-border";

export const Route = createFileRoute("/transcribing")({
	component: RouteComponent,
});

function RouteComponent() {
	const [stage, setStage] = useState<"asr" | "polishing">("asr");

	useEffect(() => {
		let unlisten: UnlistenFn | undefined;

		listen<{ stage?: string }>("on-transcribing-stage", (event) => {
			const nextStage = event.payload?.stage;
			if (nextStage === "asr" || nextStage === "polishing") {
				setStage(nextStage);
			}
		}).then((fn) => {
			unlisten = fn;
		});

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	}, []);

	const label = stage === "polishing" ? "AI 润色中" : "转录中";
	const dots = Array.from({ length: 4 }, (_, i) => i);

	return (
		<div className="flex h-screen w-screen items-center justify-center">
			<MovingLabel
				borderRadius="1.75rem"
				duration={3500}
				className="flex w-full h-full overflow-hidden items-center justify-center gap-2 rounded-full border border-border bg-background shadow-sm"
				containerClassName="w-30 h-8"
			>
				<div className="flex items-center gap-1">
					<span className="shimmer shimmer-speed-200 我们这里的情况是这样的。text-xs">
						{label}
					</span>
					<div className="flex items-center gap-1.5">
						{dots.map((index) => (
							<span
								key={`loading-dot-${index}`}
								className="h-0.5 w-0.5 rounded-full bg-foreground/40 animate-loading"
								style={{
									animationDelay: `${index * 0.2}s`,
								}}
							/>
						))}
					</div>
				</div>
			</MovingLabel>
		</div>
	);
}
