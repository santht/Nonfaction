import { cn } from '@/lib/utils';

type BadgeVariant = 'default' | 'blue' | 'red' | 'green' | 'yellow' | 'outline';

interface BadgeProps {
  variant?: BadgeVariant;
  className?: string;
  children: React.ReactNode;
}

const variantStyles: Record<BadgeVariant, string> = {
  default: 'bg-white/10 text-gray-300 border-white/10',
  blue: 'bg-blue-500/15 text-blue-400 border-blue-500/20',
  red: 'bg-red-500/15 text-red-400 border-red-500/20',
  green: 'bg-green-500/15 text-green-400 border-green-500/20',
  yellow: 'bg-yellow-500/15 text-yellow-400 border-yellow-500/20',
  outline: 'bg-transparent text-gray-400 border-white/15',
};

export function Badge({
  variant = 'default',
  className,
  children,
}: BadgeProps) {
  return (
    <span
      className={cn(
        'inline-flex items-center gap-1 px-2 py-0.5 rounded-md text-xs font-medium border',
        variantStyles[variant],
        className
      )}
    >
      {children}
    </span>
  );
}
