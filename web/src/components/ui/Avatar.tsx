import { cn } from '@/lib/utils';

interface AvatarProps {
  name: string;
  src?: string;
  className?: string;
}

function initials(name: string) {
  const parts = name.trim().split(/\s+/);
  return parts.slice(0, 2).map((part) => part[0]?.toUpperCase() ?? '').join('');
}

export function Avatar({ name, src, className }: AvatarProps) {
  return (
    <div
      className={cn(
        'flex h-10 w-10 items-center justify-center overflow-hidden rounded-full border border-white/15 bg-white/8 text-xs font-semibold text-gray-200',
        className
      )}
      aria-label={name}
    >
      {src ? <img src={src} alt={name} className="h-full w-full object-cover" /> : initials(name)}
    </div>
  );
}
