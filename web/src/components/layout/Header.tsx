'use client';

import Link from 'next/link';
import { usePathname } from 'next/navigation';
import { useMemo, useState } from 'react';
import * as DropdownMenu from '@radix-ui/react-dropdown-menu';
import { Menu, X, Database, Github, ChevronDown, Heart } from 'lucide-react';
import { Button } from '@/components/ui/Button';
import { cn } from '@/lib/utils';

interface NavGroup {
  label: string;
  items: { href: string; label: string; description: string }[];
}

const NAV_GROUPS: NavGroup[] = [
  {
    label: 'Database',
    items: [
      { href: '/search', label: 'Search', description: 'Explore entities and records.' },
      { href: '/officials', label: 'Officials', description: 'Browse tracked public officials.' },
      { href: '/timing', label: 'Timing', description: 'Analyze temporal correlations.' },
      { href: '/conduct', label: 'Conduct', description: 'Compare actions and outcomes.' },
      { href: '/graph', label: 'Graph', description: 'Inspect relationship networks.' },
    ],
  },
  {
    label: 'Community',
    items: [
      { href: '/submit', label: 'Submit', description: 'Contribute sourced records.' },
      { href: '/leaderboard', label: 'Leaderboard', description: 'Top contributors and trust tiers.' },
      { href: '/watchlist', label: 'Watchlist', description: 'Follow entities and receive alerts.' },
    ],
  },
  {
    label: 'About',
    items: [
      { href: '/about', label: 'Mission', description: 'Why Nonfaction exists.' },
      { href: '/methodology', label: 'Methodology', description: 'Verification and publication standards.' },
      { href: '/sources', label: 'Sources', description: 'Public records source registry.' },
      { href: '/faq', label: 'FAQ', description: 'Operational and legal questions.' },
      { href: '/api-docs', label: 'API', description: 'REST endpoint documentation.' },
    ],
  },
];

const PRIMARY_LINKS = [
  { href: '/updates', label: 'Updates' },
  { href: '/story-packages', label: 'Story Packages' },
];

export function Header() {
  const pathname = usePathname();
  const [mobileOpen, setMobileOpen] = useState(false);

  const githubStars = useMemo(() => '2.1k', []);

  return (
    <header className="sticky top-0 z-50 border-b border-white/10 bg-[#0A0F1C]/85 backdrop-blur-xl">
      <div className="mx-auto flex h-16 max-w-7xl items-center gap-3 px-4 sm:px-6">
        <Link href="/" className="group flex shrink-0 items-center gap-2">
          <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-blue-500 shadow-lg shadow-blue-500/30 transition-all group-hover:scale-[1.03]">
            <Database className="h-4 w-4 text-white" />
          </div>
          <span className="text-sm font-semibold tracking-tight text-white sm:text-base">Nonfaction</span>
        </Link>

        <div className="hidden items-center gap-1 lg:flex">
          {NAV_GROUPS.map((group) => (
            <DropdownMenu.Root key={group.label}>
              <DropdownMenu.Trigger asChild>
                <button
                  className={cn(
                    'inline-flex items-center gap-1 rounded-lg px-3 py-2 text-sm text-gray-300 transition-colors hover:bg-white/8 hover:text-white',
                    group.items.some((item) => pathname.startsWith(item.href)) && 'bg-white/10 text-white'
                  )}
                >
                  {group.label}
                  <ChevronDown className="h-3.5 w-3.5" />
                </button>
              </DropdownMenu.Trigger>
              <DropdownMenu.Portal>
                <DropdownMenu.Content
                  sideOffset={10}
                  align="start"
                  className="z-50 w-[360px] rounded-2xl border border-white/15 bg-[#0D162D] p-2 shadow-2xl"
                >
                  {group.items.map((item) => (
                    <DropdownMenu.Item key={item.href} asChild>
                      <Link
                        href={item.href}
                        className="block rounded-xl px-3 py-2.5 text-sm outline-none transition-colors hover:bg-white/10"
                      >
                        <p className="font-medium text-white">{item.label}</p>
                        <p className="text-xs text-gray-400">{item.description}</p>
                      </Link>
                    </DropdownMenu.Item>
                  ))}
                </DropdownMenu.Content>
              </DropdownMenu.Portal>
            </DropdownMenu.Root>
          ))}

          {PRIMARY_LINKS.map((link) => (
            <Link
              key={link.href}
              href={link.href}
              className={cn(
                'rounded-lg px-3 py-2 text-sm text-gray-300 transition-colors hover:bg-white/8 hover:text-white',
                pathname === link.href && 'bg-white/10 text-white'
              )}
            >
              {link.label}
            </Link>
          ))}
        </div>

        <div className="ml-auto hidden items-center gap-2 lg:flex">
          <a
            href="https://github.com/santht/Nonfaction"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-2 rounded-lg border border-white/15 bg-white/5 px-3 py-2 text-xs text-gray-200 transition-colors hover:border-white/25 hover:bg-white/10"
          >
            <Github className="h-3.5 w-3.5" />
            <span>GitHub</span>
            <span className="rounded bg-green-500/20 px-1.5 py-0.5 text-[10px] text-green-300">{githubStars}</span>
          </a>
          <Link href="/donate">
            <Button size="sm" className="h-9">
              <Heart className="h-3.5 w-3.5" />
              Donate
            </Button>
          </Link>
        </div>

        <button
          type="button"
          className="ml-auto rounded-lg p-2 text-gray-300 hover:bg-white/10 hover:text-white lg:hidden"
          onClick={() => setMobileOpen((v) => !v)}
          aria-expanded={mobileOpen}
          aria-label="Toggle menu"
        >
          {mobileOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
        </button>
      </div>

      {mobileOpen ? (
        <div className="border-t border-white/10 px-4 pb-4 pt-3 lg:hidden">
          <div className="space-y-2">
            {NAV_GROUPS.map((group) => (
              <div key={group.label} className="rounded-xl border border-white/10 bg-white/4 p-2">
                <p className="px-2 py-1 text-xs uppercase tracking-wide text-gray-500">{group.label}</p>
                {group.items.map((item) => (
                  <Link
                    key={item.href}
                    href={item.href}
                    onClick={() => setMobileOpen(false)}
                    className={cn(
                      'block rounded-lg px-2 py-2 text-sm text-gray-300 hover:bg-white/8 hover:text-white',
                      pathname === item.href && 'bg-white/10 text-white'
                    )}
                  >
                    {item.label}
                  </Link>
                ))}
              </div>
            ))}

            <div className="flex items-center gap-2 pt-1">
              <a
                href="https://github.com/santht/Nonfaction"
                target="_blank"
                rel="noopener noreferrer"
                className="inline-flex flex-1 items-center justify-center gap-2 rounded-xl border border-white/15 px-3 py-2 text-sm text-gray-200"
              >
                <Github className="h-4 w-4" /> GitHub
              </a>
              <Link href="/donate" onClick={() => setMobileOpen(false)} className="flex-1">
                <Button className="w-full" size="sm">
                  Donate
                </Button>
              </Link>
            </div>
          </div>
        </div>
      ) : null}
    </header>
  );
}
