import XMarkIcon from "@heroicons/react/24/solid/XMarkIcon";
import { useRef } from "react";

import { Button } from "~/components/ui/button";
import { useCancelDictating } from "~/hooks/use-audio-flow";
import { commands } from "~/lib/tauri";
import { cn } from "~/lib/utils";

type CloseButtonProps = Omit<React.ComponentProps<"button">, "onClick">;

export function CloseButton({
	className,
	disabled,
	...props
}: CloseButtonProps) {
	const { mutate, isPending } = useCancelDictating();
	const isDisabled = Boolean(disabled) || isPending;
	const buttonRef = useRef<HTMLButtonElement>(null);

	const handleMouseEnter = () => {
		if (!isDisabled && buttonRef.current) {
			const rect = buttonRef.current.getBoundingClientRect();
			const offsetX = rect.left + rect.width / 2;
			void commands.showFeedback("取消", "tooltip", offsetX);
		}
	};

	return (
		<Button
			{...props}
			ref={buttonRef}
			variant="secondary"
			size="icon"
			onClick={() => mutate()}
			disabled={isDisabled}
			onMouseEnter={handleMouseEnter}
			className={cn("rounded-full ", className)}
		>
			<XMarkIcon className="size-2.5 text-muted-foreground" />
		</Button>
	);
}
