import { cn } from '@/lib/utils';

interface TableProps {
  className?: string;
  children: React.ReactNode;
}

export function Table({ className, children }: TableProps) {
  return (
    <div className={cn('w-full overflow-x-auto', className)}>
      <table className="w-full border-collapse text-sm">{children}</table>
    </div>
  );
}

export function TableHead({ className, children }: TableProps) {
  return <thead className={cn('', className)}>{children}</thead>;
}

export function TableBody({ className, children }: TableProps) {
  return <tbody className={cn('', className)}>{children}</tbody>;
}

export function TableRow({
  className,
  children,
  flagged,
}: TableProps & { flagged?: boolean }) {
  return (
    <tr
      className={cn(
        'border-b border-white/5 transition-colors duration-150',
        flagged
          ? 'bg-red-500/5 hover:bg-red-500/10'
          : 'hover:bg-white/3',
        className
      )}
    >
      {children}
    </tr>
  );
}

export function TableHeader({
  className,
  children,
  onClick,
  sortable,
}: {
  className?: string;
  children: React.ReactNode;
  onClick?: () => void;
  sortable?: boolean;
}) {
  return (
    <th
      onClick={onClick}
      className={cn(
        'px-4 py-3 text-left text-xs font-semibold text-gray-500 uppercase tracking-wider border-b border-white/8',
        sortable && 'cursor-pointer hover:text-gray-300 select-none',
        className
      )}
    >
      {children}
    </th>
  );
}

export function TableCell({
  className,
  children,
}: {
  className?: string;
  children: React.ReactNode;
}) {
  return (
    <td className={cn('px-4 py-3 text-gray-300 align-top', className)}>
      {children}
    </td>
  );
}
