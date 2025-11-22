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
		<div className="flex h-screen w-screen items-center justify-center px-1">
			<MovingLabel
				duration={3500}
				borderRadius="1.75rem"
				className="bg-background border border-border w-full h-8 rounded-full shadow-sm flex items-center justify-center"
				containerClassName="w-[120px] h-8.5"
			>
				<div className="flex items-center gap-2 text-xs font-medium text-foreground/40">
					<span className="shimmer shimmer-speed-200">{label}</span>
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
