import StopIcon from "@heroicons/react/24/solid/StopIcon";
import { useRef } from "react";
import { Button } from "~/components/ui/button";
import { useStopDictating } from "~/hooks/use-audio-flow";
import { commands } from "~/lib/tauri";
import { cn } from "~/lib/utils";

type Props = React.ComponentProps<"button">;

export function StopButton({ className, ...props }: Props) {
	const { mutate, isPending } = useStopDictating();
	const buttonRef = useRef<HTMLButtonElement>(null);

	const handleMouseEnter = () => {
		if (!isPending && buttonRef.current) {
			const rect = buttonRef.current.getBoundingClientRect();
			const offsetX = rect.left + rect.width / 2;
			void commands.showFeedback("结束", "tooltip", offsetX);
		}
	};

	return (
		<Button
			{...props}
			ref={buttonRef}
			size="icon"
			disabled={isPending}
			onClick={() => mutate()}
			onMouseEnter={handleMouseEnter}
			className={cn("rounded-full", className)}
		>
			<StopIcon className="size-2.5 bg-background rounded-xs" />
		</Button>
	);
}
