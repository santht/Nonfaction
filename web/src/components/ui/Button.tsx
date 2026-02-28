import { cn } from '@/lib/utils';
import { type ButtonHTMLAttributes } from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger';
type Size = 'sm' | 'md' | 'lg';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantStyles: Record<Variant, string> = {
  primary:
    'bg-blue-500 hover:bg-blue-400 text-white shadow-lg shadow-blue-500/20 focus-visible:ring-blue-500',
  secondary:
    'bg-white/10 hover:bg-white/15 text-white border border-white/10 focus-visible:ring-white/20',
  ghost:
    'hover:bg-white/8 text-gray-400 hover:text-white focus-visible:ring-white/10',
  danger:
    'bg-red-500 hover:bg-red-400 text-white shadow-lg shadow-red-500/20 focus-visible:ring-red-500',
};

const sizeStyles: Record<Size, string> = {
  sm: 'px-3 py-1.5 text-xs',
  md: 'px-4 py-2 text-sm',
  lg: 'px-6 py-3 text-base',
};

export function Button({
  variant = 'primary',
  size = 'md',
  className,
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      className={cn(
        'inline-flex items-center justify-center gap-2 font-medium rounded-xl',
        'transition-all duration-200',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-[#0A0F1C]',
        'disabled:opacity-50 disabled:cursor-not-allowed',
        variantStyles[variant],
        sizeStyles[size],
        className
      )}
      {...props}
    >
      {children}
    </button>
  );
}
