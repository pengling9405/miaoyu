import ExclamationCircleIcon from "@heroicons/react/24/outline/ExclamationCircleIcon";
import { createFileRoute } from "@tanstack/react-router";
import { listen } from "@tauri-apps/api/event";
import { AnimatePresence, motion } from "motion/react";
import { useEffect, useState } from "react";
import type { ShowNotification } from "~/lib/tauri";
import { cn } from "~/lib/utils";

export const Route = createFileRoute("/notification")({
	component: NotificationComponent,
});

function NotificationComponent() {
	const [notification, setNotification] = useState<ShowNotification | null>(
		null,
	);

	useEffect(() => {
		let unlisten: (() => void) | null = null;

		const setupListener = async () => {
			unlisten = await listen<ShowNotification>(
				"show-notification",
				(event) => {
					const payload = event.payload;
					setNotification(payload);

					// 根据通知类型设置不同的显示时长（与后端保持一致）
					const duration = payload.type === "error" ? 3000 : 2500;

					setTimeout(() => {
						setNotification(null);
					}, duration);
				},
			);
		};

		setupListener();

		return () => {
			if (unlisten) {
				unlisten();
			}
		};
	}, []);

	return (
		<div className="h-full w-full flex items-end justify-center bg-transparent pointer-events-none">
			<AnimatePresence>
				{notification && (
					<motion.div
						initial={{ opacity: 0, y: 16 }}
						animate={{ opacity: 1, y: 0 }}
						exit={{ opacity: 0, y: 16 }}
						transition={{ duration: 0.25, ease: "easeInOut" }}
						className="pointer-events-auto px-4 py-2 rounded-2xl text-sm shadow-2xl whitespace-pre-wrap bg-background/95 border border-border text-foreground flex items-center justify-center gap-1.5"
					>
						<ExclamationCircleIcon
							className={cn(
								"size-4",
								notification.type === "error"
									? "text-destructive"
									: "text-violet-600",
							)}
						/>
						{notification.message}
					</motion.div>
				)}
			</AnimatePresence>
		</div>
	);
}
