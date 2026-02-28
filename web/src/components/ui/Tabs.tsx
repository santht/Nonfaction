'use client';

import * as RadixTabs from '@radix-ui/react-tabs';
import { cn } from '@/lib/utils';

export const Tabs = RadixTabs.Root;

export function TabsList({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <RadixTabs.List
      className={cn(
        'flex gap-1 border-b border-white/8 pb-0',
        className
      )}
    >
      {children}
    </RadixTabs.List>
  );
}

export function TabsTrigger({
  value,
  className,
  children,
}: {
  value: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <RadixTabs.Trigger
      value={value}
      className={cn(
        'px-4 py-2.5 text-sm font-medium text-gray-500 rounded-t-lg',
        'hover:text-gray-300 transition-colors duration-150',
        'data-[state=active]:text-white data-[state=active]:border-b-2 data-[state=active]:border-blue-500',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-blue-500/50',
        '-mb-px',
        className
      )}
    >
      {children}
    </RadixTabs.Trigger>
  );
}

export function TabsContent({
  value,
  className,
  children,
}: {
  value: string;
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <RadixTabs.Content
      value={value}
      className={cn('pt-6 focus-visible:outline-none', className)}
    >
      {children}
    </RadixTabs.Content>
  );
}
