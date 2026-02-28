import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';
import { forwardRef, type SelectHTMLAttributes } from 'react';

interface SelectProps extends SelectHTMLAttributes<HTMLSelectElement> {
  error?: boolean;
}

export const Select = forwardRef<HTMLSelectElement, SelectProps>(function Select(
  { className, error = false, children, ...props },
  ref
) {
  return (
    <div className="relative">
      <select
        ref={ref}
        className={cn(
          'w-full appearance-none rounded-xl border bg-white/6 px-4 py-2.5 pr-9 text-sm text-white',
          'transition-all duration-200 focus:outline-none focus:ring-2',
          error
            ? 'border-red-500/40 bg-red-500/10 focus:ring-red-500/40'
            : 'border-white/12 focus:border-blue-500/50 focus:ring-blue-500/40',
          className
        )}
        {...props}
      >
        {children}
      </select>
      <ChevronDown className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-gray-400" />
    </div>
  );
});
