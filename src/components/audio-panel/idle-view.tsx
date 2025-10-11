import { AnimatePresence, motion } from "motion/react";
import { useStartDictating } from "~/hooks/use-audio-flow";
import { cn } from "~/lib/utils";

interface IdleViewProps {
	isHovered: boolean;
}

export function IdleView({ isHovered }: IdleViewProps) {
	const { mutate: startDicating, isPending } = useStartDictating();

	return (
		<motion.button
			type="button"
			disabled={isPending}
			onClick={() => startDicating()}
			initial
			layout
			className={cn(
				"bg-background border border-foreground/40 rounded-full shadow-sm flex items-center justify-center w-full h-full",
				isHovered && "border-border",
			)}
		>
			<AnimatePresence>
				{isHovered && (
					<motion.div className="flex items-center gap-1">
						{Array.from({ length: 9 }, (_, i) => (
							<div
								key={`dot-${i}`}
								className="w-0.5 h-0.5 bg-border rounded-ful"
							/>
						))}
					</motion.div>
				)}
			</AnimatePresence>
		</motion.button>
	);
}
