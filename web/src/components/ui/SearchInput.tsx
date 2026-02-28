'use client';

import { cn } from '@/lib/utils';
import { Search } from 'lucide-react';
import { type InputHTMLAttributes } from 'react';

interface SearchInputProps extends InputHTMLAttributes<HTMLInputElement> {
  onSearch?: (value: string) => void;
  containerClassName?: string;
  large?: boolean;
}

export function SearchInput({
  onSearch,
  containerClassName,
  large = false,
  className,
  ...props
}: SearchInputProps) {
  return (
    <div className={cn('relative', containerClassName)}>
      <Search
        className={cn(
          'absolute left-4 top-1/2 -translate-y-1/2 text-gray-500 pointer-events-none',
          large ? 'w-5 h-5' : 'w-4 h-4'
        )}
      />
      <input
        type="search"
        className={cn(
          'w-full bg-white/6 border border-white/10 rounded-xl text-white placeholder-gray-500',
          'focus:outline-none focus:ring-2 focus:ring-blue-500/50 focus:border-blue-500/50',
          'transition-all duration-200',
          large
            ? 'pl-12 pr-6 py-4 text-lg'
            : 'pl-10 pr-4 py-2.5 text-sm',
          className
        )}
        onKeyDown={(e) => {
          if (e.key === 'Enter' && onSearch) {
            onSearch((e.target as HTMLInputElement).value);
          }
        }}
        {...props}
      />
    </div>
  );
}
