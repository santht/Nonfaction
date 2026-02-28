'use client';

import { ChevronDown } from 'lucide-react';
import { cn } from '@/lib/utils';
import { useState } from 'react';

export interface AccordionItem {
  id: string;
  title: string;
  content: React.ReactNode;
}

export function Accordion({ items }: { items: AccordionItem[] }) {
  const [openItem, setOpenItem] = useState<string | null>(items[0]?.id ?? null);

  return (
    <div className="space-y-3">
      {items.map((item) => {
        const open = openItem === item.id;
        return (
          <div key={item.id} className="rounded-xl border border-white/10 bg-white/4">
            <button
              type="button"
              onClick={() => setOpenItem((curr) => (curr === item.id ? null : item.id))}
              className="flex w-full items-center justify-between gap-4 px-5 py-4 text-left"
            >
              <span className="text-sm font-medium text-white">{item.title}</span>
              <ChevronDown className={cn('h-4 w-4 text-gray-400 transition-transform', open && 'rotate-180')} />
            </button>
            {open ? <div className="border-t border-white/8 px-5 py-4 text-sm text-gray-300">{item.content}</div> : null}
          </div>
        );
      })}
    </div>
  );
}
