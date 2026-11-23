// Composição do Button do shadcn/ui, adaptada ao tema local.
import { Slot } from "@radix-ui/react-slot";
import { cva } from "class-variance-authority";
import type { ComponentProps } from "react";
const variants = cva("btn", {
  variants: { variant: { default: "primary", outline: "outline" } },
  defaultVariants: { variant: "default" },
});
export function Button({
  asChild = false,
  variant = "default",
  className = "",
  ...props
}: ComponentProps<"button"> & {
  asChild?: boolean;
  variant?: "default" | "outline";
}) {
  const Component = asChild ? Slot : "button";
  return (
    <Component
      data-slot="button"
      data-variant={variant}
      className={variants({ variant, className })}
      {...props}
    />
  );
}
