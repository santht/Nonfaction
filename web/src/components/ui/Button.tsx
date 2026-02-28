import { cn } from '@/lib/utils';
import { type ButtonHTMLAttributes } from 'react';

type Variant = 'primary' | 'secondary' | 'ghost' | 'danger' | 'outline';
type Size = 'sm' | 'md' | 'lg';

interface ButtonProps extends ButtonHTMLAttributes<HTMLButtonElement> {
  variant?: Variant;
  size?: Size;
}

const variantStyles: Record<Variant, string> = {
  primary:
    'bg-blue-500 hover:bg-blue-400 text-white shadow-lg shadow-blue-500/25 focus-visible:ring-blue-500',
  secondary:
    'bg-white/10 hover:bg-white/15 text-white border border-white/10 focus-visible:ring-white/30',
  ghost:
    'text-gray-300 hover:text-white hover:bg-white/10 focus-visible:ring-white/20',
  danger:
    'bg-red-500 hover:bg-red-400 text-white shadow-lg shadow-red-500/25 focus-visible:ring-red-500',
  outline:
    'text-gray-200 border border-white/15 hover:bg-white/10 hover:border-white/25 focus-visible:ring-white/20',
};

const sizeStyles: Record<Size, string> = {
  sm: 'h-8 px-3 text-xs rounded-lg',
  md: 'h-10 px-4 text-sm rounded-xl',
  lg: 'h-12 px-6 text-base rounded-xl',
};

export function Button({
  variant = 'primary',
  size = 'md',
  className,
  type = 'button',
  children,
  ...props
}: ButtonProps) {
  return (
    <button
      type={type}
      className={cn(
        'inline-flex items-center justify-center gap-2 font-medium',
        'transition-all duration-200',
        'focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-offset-2 focus-visible:ring-offset-[#0A0F1C]',
        'disabled:cursor-not-allowed disabled:opacity-50',
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
