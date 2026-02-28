import Link from 'next/link';
import { Database, Github, Mail } from 'lucide-react';
import { Input } from '@/components/ui/Input';
import { Button } from '@/components/ui/Button';

const databaseLinks = [
  { href: '/search', label: 'Search' },
  { href: '/officials', label: 'Officials' },
  { href: '/timing', label: 'Timing Correlations' },
  { href: '/conduct', label: 'Conduct Comparison' },
  { href: '/graph', label: 'Network Graph' },
];

const communityLinks = [
  { href: '/submit', label: 'Submit Connection' },
  { href: '/leaderboard', label: 'Leaderboard' },
  { href: '/watchlist', label: 'Watchlist' },
  { href: '/story-packages', label: 'Story Packages' },
  { href: '/updates', label: 'Updates' },
];

const legalLinks = [
  { href: '/privacy', label: 'Privacy Policy' },
  { href: '/terms', label: 'Terms of Service' },
  { href: '/contact', label: 'Contact' },
  { href: '/faq', label: 'FAQ' },
];

export function Footer() {
  return (
    <footer className="mt-16 border-t border-white/10 bg-[#090e1a]">
      <div className="mx-auto max-w-7xl px-4 py-12 sm:px-6">
        <div className="grid grid-cols-1 gap-10 md:grid-cols-2 xl:grid-cols-4">
          <div>
            <div className="mb-4 flex items-center gap-2">
              <div className="flex h-8 w-8 items-center justify-center rounded-xl bg-blue-500 shadow-lg shadow-blue-500/30">
                <Database className="h-4 w-4 text-white" />
              </div>
              <span className="text-base font-semibold text-white">Nonfaction</span>
            </div>
            <p className="max-w-sm text-sm text-gray-400">
              Every connection traced to source records. No editorial framing. Built with public records and open methods.
            </p>
            <a
              href="mailto:santht@proton.me"
              className="mt-4 inline-flex items-center gap-2 text-sm text-blue-300 transition-colors hover:text-blue-200"
            >
              <Mail className="h-4 w-4" />
              santht@proton.me
            </a>
            <a
              href="https://github.com/santht/Nonfaction"
              target="_blank"
              rel="noopener noreferrer"
              className="mt-3 block text-sm text-gray-300 transition-colors hover:text-white"
            >
              github.com/santht/Nonfaction
            </a>
          </div>

          <FooterCol title="Database" links={databaseLinks} />
          <FooterCol title="Community" links={communityLinks} />
          <FooterCol title="Legal" links={legalLinks} />
        </div>

        <div className="mt-10 rounded-2xl border border-white/10 bg-white/4 p-4 sm:p-5">
          <div className="flex flex-col gap-3 sm:flex-row sm:items-end sm:justify-between">
            <div>
              <p className="text-sm font-medium text-white">Newsletter</p>
              <p className="text-xs text-gray-400">Get source additions, major investigations, and release notes.</p>
            </div>
            <form className="flex w-full max-w-md gap-2" action="#" method="post">
              <Input
                type="email"
                name="newsletterEmail"
                placeholder="name@domain.com"
                required
                aria-label="Email address"
              />
              <Button type="submit" size="sm">Subscribe</Button>
            </form>
          </div>
        </div>

        <div className="mt-8 flex flex-col gap-3 border-t border-white/10 pt-6 text-xs text-gray-500 sm:flex-row sm:items-center sm:justify-between">
          <p>© {new Date().getFullYear()} Nonfaction. Built with public records.</p>
          <a
            href="https://github.com/santht/Nonfaction"
            target="_blank"
            rel="noopener noreferrer"
            className="inline-flex items-center gap-1 text-gray-400 transition-colors hover:text-white"
          >
            <Github className="h-3.5 w-3.5" />
            Open source repository
          </a>
        </div>
      </div>
    </footer>
  );
}

function FooterCol({
  title,
  links,
}: {
  title: string;
  links: { href: string; label: string }[];
}) {
  return (
    <div>
      <h4 className="mb-3 text-sm font-semibold text-gray-200">{title}</h4>
      <ul className="space-y-2">
        {links.map((link) => (
          <li key={link.href}>
            <Link href={link.href} className="text-sm text-gray-400 transition-colors hover:text-white">
              {link.label}
            </Link>
          </li>
        ))}
      </ul>
    </div>
  );
}
