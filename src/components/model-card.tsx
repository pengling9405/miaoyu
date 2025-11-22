import { type ReactNode } from "react";
import {
  Field,
  FieldContent,
  FieldDescription,
  FieldLabel,
  FieldTitle,
} from "~/components/ui/field";
import { RadioGroupItem } from "~/components/animate-ui/components/radix/radio-group";
import { cn } from "~/lib/utils";

interface ModelCardProps {
  id: string;
  value?: string;
  icon: ReactNode;
  iconWrapperClassName?: string;
  title: ReactNode;
  description: ReactNode;
  descriptionWrapperClassName?: string;
  radioGroupItemClassName?: string;
  radioDisabled?: boolean;
  contentClassName?: string;
  children?: ReactNode;
  hideRadio?: boolean;
}

export function ModelCard({
  id,
  value = id,
  icon,
  iconWrapperClassName,
  title,
  description,
  descriptionWrapperClassName,
  radioGroupItemClassName,
  radioDisabled,
  contentClassName,
  children,
  hideRadio,
}: ModelCardProps) {
  return (
    <FieldLabel htmlFor={id}>
      <Field orientation="horizontal" className="relative">
        <FieldContent
          className={cn("flex flex-row gap-4 items-center", contentClassName)}
        >
          <div
            className={cn(
              "size-14 rounded-xl flex items-center justify-center",
              iconWrapperClassName,
            )}
          >
            {icon}
          </div>
          <div className="flex flex-col w-full gap-1">
            <FieldTitle>{title}</FieldTitle>
            <FieldDescription
              className={cn(
                "flex flex-1 h-8 pt-px items-center justify-between",
                descriptionWrapperClassName,
              )}
            >
              {description}
            </FieldDescription>
          </div>
        </FieldContent>
        {!hideRadio && (
          <RadioGroupItem
            className={cn("absolute right-4", radioGroupItemClassName)}
            value={value}
            id={id}
            disabled={radioDisabled}
          />
        )}
      </Field>
      {children}
    </FieldLabel>
  );
}
