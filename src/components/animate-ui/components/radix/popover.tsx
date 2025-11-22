import {
	PopoverClose as PopoverClosePrimitive,
	type PopoverCloseProps as PopoverClosePrimitiveProps,
	PopoverContent as PopoverContentPrimitive,
	type PopoverContentProps as PopoverContentPrimitiveProps,
	PopoverPortal as PopoverPortalPrimitive,
	Popover as PopoverPrimitive,
	type PopoverProps as PopoverPrimitiveProps,
	PopoverTrigger as PopoverTriggerPrimitive,
	type PopoverTriggerProps as PopoverTriggerPrimitiveProps,
} from "~/components/animate-ui/primitives/radix/popover";
import { cn } from "~/lib/utils";

type PopoverProps = PopoverPrimitiveProps;

function Popover(props: PopoverProps) {
	return <PopoverPrimitive {...props} />;
}

type PopoverTriggerProps = PopoverTriggerPrimitiveProps;

function PopoverTrigger(props: PopoverTriggerProps) {
	return <PopoverTriggerPrimitive {...props} />;
}

type PopoverContentProps = PopoverContentPrimitiveProps;

function PopoverContent({
	className,
	align = "center",
	sideOffset = 4,
	...props
}: PopoverContentProps) {
	return (
		<PopoverPortalPrimitive>
			<PopoverContentPrimitive
				align={align}
				sideOffset={sideOffset}
				className={cn(
					"bg-popover text-popover-foreground z-50 w-72 origin-(--radix-popover-content-transform-origin) rounded-md border p-4 shadow-md outline-hidden",
					className,
				)}
				{...props}
			/>
		</PopoverPortalPrimitive>
	);
}

type PopoverCloseProps = PopoverClosePrimitiveProps;

function PopoverClose(props: PopoverCloseProps) {
	return <PopoverClosePrimitive {...props} />;
}

export {
	Popover,
	PopoverTrigger,
	PopoverContent,
	PopoverClose,
	type PopoverProps,
	type PopoverTriggerProps,
	type PopoverContentProps,
	type PopoverCloseProps,
};
