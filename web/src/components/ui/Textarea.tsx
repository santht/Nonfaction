import { cn } from '@/lib/utils';
import { forwardRef, type TextareaHTMLAttributes } from 'react';

interface TextareaProps extends TextareaHTMLAttributes<HTMLTextAreaElement> {
  error?: boolean;
}

export const Textarea = forwardRef<HTMLTextAreaElement, TextareaProps>(function Textarea(
  { className, error = false, ...props },
  ref
) {
  return (
    <textarea
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
