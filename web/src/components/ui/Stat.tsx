import { ArrowDownRight, ArrowUpRight } from 'lucide-react';
import { cn } from '@/lib/utils';

interface StatProps {
  label: string;
  value: string;
  change?: string;
  trend?: 'up' | 'down' | 'neutral';
  className?: string;
}

export function Stat({ label, value, change, trend = 'neutral', className }: StatProps) {
  return (
    <div className={cn('rounded-xl border border-white/10 bg-white/4 p-4', className)}>
      <p className="text-xs uppercase tracking-wide text-gray-500">{label}</p>
      <p className="mt-2 text-2xl font-semibold text-white">{value}</p>
      {change ? (
        <p
          className={cn(
            'mt-1 inline-flex items-center gap-1 text-xs',
            trend === 'up' && 'text-green-400',
            trend === 'down' && 'text-red-400',
            trend === 'neutral' && 'text-gray-400'
          )}
        >
          {trend === 'up' ? <ArrowUpRight className="h-3 w-3" /> : null}
          {trend === 'down' ? <ArrowDownRight className="h-3 w-3" /> : null}
          {change}
        </p>
      ) : null}
    </div>
  );
}
