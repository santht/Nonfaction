import { cn } from '@/lib/utils';
import { forwardRef, type InputHTMLAttributes } from 'react';

interface InputProps extends InputHTMLAttributes<HTMLInputElement> {
  error?: boolean;
}

export const Input = forwardRef<HTMLInputElement, InputProps>(function Input(
  { className, error = false, ...props },
  ref
) {
  return (
    <input
      ref={ref}
      className={cn(
        'w-full rounded-xl border bg-white/6 px-4 py-2.5 text-sm text-white placeholder:text-gray-500',
        'transition-all duration-200 focus:outline-none focus:ring-2',
        error
          ? 'border-red-500/40 bg-red-500/10 focus:ring-red-500/40'
          : 'border-white/12 focus:border-blue-500/50 focus:ring-blue-500/40',
        className
      )}
      {...props}
    />
  );
});
