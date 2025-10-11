import ExclamationCircleIcon from "@heroicons/react/24/outline/ExclamationCircleIcon";
import { createFileRoute } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import type { ShowFeedback } from "~/lib/tauri";
import { cn } from "~/lib/utils";

export const Route = createFileRoute("/feedback")({
	component: FeedbackComponent,
});

function FeedbackComponent() {
	const [feedback, setFeedback] = useState<ShowFeedback | null>(null);

	useEffect(() => {
		let unlisten: (() => void) | null = null;

		const setupListener = async () => {
			unlisten = await listen<ShowFeedback>("show-feedback", (event) => {
				const payload = event.payload;
				setFeedback(payload);

				// 根据反馈类型设置不同的显示时长（与后端保持一致）
				const duration =
					payload.type === "tooltip"
						? 1200
						: payload.type === "toast"
							? 2500
							: 3000;

				setTimeout(() => {
					setFeedback(null);
				}, duration);
			});
		};

		setupListener();

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	}, []);

	return (
		<div className="h-screen w-screen flex items-center justify-center bg-transparent">
			<AnimatePresence>
				{feedback && (
					<motion.div
						initial={{ opacity: 0 }}
						animate={{ opacity: 1 }}
						exit={{ opacity: 0 }}
						transition={{ duration: 0.3, ease: "easeInOut" }}
						className="px-4 py-2 rounded-full text-xs shadow-lg whitespace-nowrap bg-background border border-border text-foreground flex items-center justify-center gap-1.5"
					>
						{feedback.type !== "tooltip" && (
							<ExclamationCircleIcon
								className={cn(
									"size-3.5",
									feedback.type === "error" && "text-destructive",
									feedback.type === "toast" && "text-violet-600",
								)}
							/>
						)}
						{feedback.message}
					</motion.div>
				)}
			</AnimatePresence>
		</div>
	);
}
