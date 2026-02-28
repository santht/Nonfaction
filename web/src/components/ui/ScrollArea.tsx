'use client';

import * as RadixScrollArea from '@radix-ui/react-scroll-area';
import { cn } from '@/lib/utils';

export function ScrollArea({
  className,
  children,
  viewportClassName,
}: {
  className?: string;
  children: React.ReactNode;
  viewportClassName?: string;
}) {
  return (
    <RadixScrollArea.Root className={cn('relative overflow-hidden', className)}>
      <RadixScrollArea.Viewport className={cn('h-full w-full rounded-[inherit]', viewportClassName)}>
        {children}
      </RadixScrollArea.Viewport>
      <RadixScrollArea.Scrollbar
        orientation="vertical"
        className="flex w-2.5 touch-none bg-transparent p-0.5"
      >
        <RadixScrollArea.Thumb className="flex-1 rounded-full bg-white/20" />
      </RadixScrollArea.Scrollbar>
      <RadixScrollArea.Scrollbar
        orientation="horizontal"
        className="flex h-2.5 touch-none bg-transparent p-0.5"
      >
        <RadixScrollArea.Thumb className="flex-1 rounded-full bg-white/20" />
      </RadixScrollArea.Scrollbar>
      <RadixScrollArea.Corner className="bg-transparent" />
    </RadixScrollArea.Root>
  );
}
