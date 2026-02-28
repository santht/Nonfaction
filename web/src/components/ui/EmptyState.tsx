import Link from 'next/link';
import { type LucideIcon } from 'lucide-react';
import { Button } from '@/components/ui/Button';

interface EmptyStateProps {
  icon: LucideIcon;
  title: string;
  description: string;
  ctaLabel?: string;
  ctaHref?: string;
}

export function EmptyState({
  icon: Icon,
  title,
  description,
  ctaLabel,
  ctaHref,
}: EmptyStateProps) {
  return (
    <div className="rounded-xl border border-dashed border-white/15 bg-white/3 px-6 py-12 text-center">
      <div className="mx-auto mb-3 flex h-10 w-10 items-center justify-center rounded-full bg-white/10">
        <Icon className="h-5 w-5 text-gray-300" />
      </div>
      <h3 className="text-lg font-semibold text-white">{title}</h3>
      <p className="mx-auto mt-2 max-w-lg text-sm text-gray-400">{description}</p>
      {ctaLabel && ctaHref ? (
        <Link href={ctaHref} className="mt-5 inline-block">
          <Button size="sm">{ctaLabel}</Button>
        </Link>
      ) : null}
    </div>
  );
}
