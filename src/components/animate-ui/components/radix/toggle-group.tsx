import type { VariantProps } from "class-variance-authority";
import { toggleVariants } from "~/components/animate-ui/components/radix/toggle";
import {
	ToggleGroupHighlightItem as ToggleGroupHighlightItemPrimitive,
	ToggleGroupHighlight as ToggleGroupHighlightPrimitive,
	ToggleGroupItem as ToggleGroupItemPrimitive,
	type ToggleGroupItemProps as ToggleGroupItemPrimitiveProps,
	ToggleGroup as ToggleGroupPrimitive,
	type ToggleGroupProps as ToggleGroupPrimitiveProps,
	useToggleGroup as useToggleGroupPrimitive,
} from "~/components/animate-ui/primitives/radix/toggle-group";
import { getStrictContext } from "~/lib/get-strict-context";
import { cn } from "~/lib/utils";

const [ToggleGroupProvider, useToggleGroup] =
	getStrictContext<VariantProps<typeof toggleVariants>>("ToggleGroupContext");

type ToggleGroupProps = ToggleGroupPrimitiveProps &
	VariantProps<typeof toggleVariants>;

function ToggleGroup({
	className,
	variant,
	size,
	children,
	...props
}: ToggleGroupProps) {
	return (
		<ToggleGroupPrimitive
			data-variant={variant}
			data-size={size}
			className={cn(
				"group/toggle-group flex gap-0.5 w-fit items-center rounded-lg data-[variant=outline]:shadow-xs data-[variant=outline]:border data-[variant=outline]:p-0.5",
				className,
			)}
			{...props}
		>
			<ToggleGroupProvider value={{ variant, size }}>
				{props.type === "single" ? (
					<ToggleGroupHighlightPrimitive className="bg-accent rounded-md">
						{children}
					</ToggleGroupHighlightPrimitive>
				) : (
					children
				)}
			</ToggleGroupProvider>
		</ToggleGroupPrimitive>
	);
}

type ToggleGroupItemProps = ToggleGroupItemPrimitiveProps &
	VariantProps<typeof toggleVariants>;

function ToggleGroupItem({
	className,
	children,
	variant,
	size,
	...props
}: ToggleGroupItemProps) {
	const { variant: contextVariant, size: contextSize } = useToggleGroup();
	const { type } = useToggleGroupPrimitive();

	return (
		<ToggleGroupHighlightItemPrimitive
			value={props.value}
			className={cn(type === "multiple" && "bg-accent rounded-md")}
		>
			<ToggleGroupItemPrimitive
				data-variant={contextVariant || variant}
				data-size={contextSize || size}
				className={cn(
					toggleVariants({
						variant: contextVariant || variant,
						size: contextSize || size,
					}),
					"min-w-0 border-0 flex-1 shrink-0 shadow-none rounded-md focus:z-10 focus-visible:z-10",
					className,
				)}
				{...props}
			>
				{children}
			</ToggleGroupItemPrimitive>
		</ToggleGroupHighlightItemPrimitive>
	);
}

export {
	ToggleGroup,
	ToggleGroupItem,
	type ToggleGroupProps,
	type ToggleGroupItemProps,
};
