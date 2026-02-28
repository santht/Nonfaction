import { cn } from '@/lib/utils';

type BadgeVariant = 'default' | 'blue' | 'red' | 'green' | 'yellow' | 'outline';

interface BadgeProps {
  variant?: BadgeVariant;
  className?: string;
  children: React.ReactNode;
}

const variantStyles: Record<BadgeVariant, string> = {
  default: 'border-white/10 bg-white/10 text-gray-200',
  blue: 'border-blue-500/30 bg-blue-500/15 text-blue-300',
  red: 'border-red-500/30 bg-red-500/15 text-red-300',
  green: 'border-green-500/30 bg-green-500/15 text-green-300',
  yellow: 'border-yellow-500/30 bg-yellow-500/15 text-yellow-300',
  outline: 'border-white/20 bg-transparent text-gray-300',
};

export function Badge({ variant = 'default', className, children }: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 rounded-md border px-2 py-0.5 text-xs font-medium',
        variantStyles[variant],
        className
      )}
    >
      {children}
    </span>
  );
}
