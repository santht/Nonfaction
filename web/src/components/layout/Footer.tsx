import Link from 'next/link';
import { Database, Github } from 'lucide-react';

export function Footer() {
  return (
    <footer className="border-t border-white/6 bg-[#0A0F1C] mt-20">
      <div className="max-w-7xl mx-auto px-4 sm:px-6 py-12">
        <div className="grid grid-cols-1 md:grid-cols-3 gap-10">
          {/* Brand */}
          <div>
            <div className="flex items-center gap-2 mb-4">
              <div className="w-7 h-7 bg-blue-500 rounded-lg flex items-center justify-center shadow-lg shadow-blue-500/30">
                <Database className="w-4 h-4 text-white" />
              </div>
              <span className="font-semibold text-white">Nonfaction</span>
            </div>
            <p className="text-sm text-gray-500 leading-relaxed max-w-xs">
              Every connection traced to its source. No claims. Only citations.
              A political accountability database built on public records.
            </p>
          </div>

          {/* Navigation */}
          <div>
            <h4 className="text-sm font-semibold text-gray-300 mb-4">
              Database
            </h4>
            <ul className="space-y-2.5">
              {[
                { href: '/search', label: 'Explore Entities' },
                { href: '/timing', label: 'Timing Correlations' },
                { href: '/conduct', label: 'Conduct Comparison' },
                { href: '/submit', label: 'Submit a Connection' },
              ].map((link) => (
                <li key={link.href}>
                  <Link
                    href={link.href}
                    className="text-sm text-gray-500 hover:text-white transition-colors"
                  >
                    {link.label}
                  </Link>
                </li>
              ))}
            </ul>
          </div>

          {/* Principles */}
          <div>
            <h4 className="text-sm font-semibold text-gray-300 mb-4">
              Principles
            </h4>
            <ul className="space-y-2.5 text-sm text-gray-500">
              <li>Every fact carries a source</li>
              <li>No anonymous claims accepted</li>
              <li>Public records only</li>
              <li>No editorial opinion</li>
            </ul>
          </div>
        </div>

        <div className="mt-10 pt-6 border-t border-white/6 flex flex-col sm:flex-row items-center justify-between gap-4">
          <p className="text-xs text-gray-600">
            © {new Date().getFullYear()} Nonfaction. All data sourced from
            public records. No claims without citations.
          </p>
          <a
            href="https://github.com"
            target="_blank"
            rel="noopener noreferrer"
            className="flex items-center gap-2 text-xs text-gray-500 hover:text-white transition-colors"
          >
            <Github className="w-3.5 h-3.5" />
            Open Source
          </a>
        </div>
      </div>
    </footer>
  );
}
