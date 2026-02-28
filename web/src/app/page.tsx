import Link from 'next/link';
import { ArrowRight, Shield, Clock, FileText, AlertTriangle } from 'lucide-react';
import { getStats } from '@/lib/api';
import { Card, CardContent } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';

const RECENT_CORRELATIONS = [
  {
    id: 't1',
    official: 'Sen. Richard Walsh',
    description: '$450K PAC donation → voted YES on $8.2B defense bill',
    days: 37,
    flagged: true,
    source: 'FEC Filing + Congress.gov',
  },
  {
    id: 't2',
    official: 'Rep. Diana Chen',
    description: 'Meeting with PharmaCorp CEO → co-sponsored drug price deregulation',
    days: 18,
    flagged: true,
    source: 'LDA Senate Disclosure',
  },
  {
    id: 't3',
    official: 'Gov. Patricia Monroe',
    description: "$1.2M donor bundling → donor's associate appointed State Treasurer",
    days: 106,
    flagged: true,
    source: 'OpenSecrets / Campaign Finance Disclosure',
  },
];

const FEATURES = [
  {
    icon: Shield,
    title: 'Source Attribution',
    description:
      'Every fact links to a primary source: FEC filings, congressional records, court documents, lobbying disclosures.',
  },
  {
    icon: Clock,
    title: 'Timing Correlations',
    description:
      'We track the days between financial events and legislative actions. Patterns surface. You decide.',
  },
  {
    icon: FileText,
    title: 'Conduct Comparison',
    description:
      'Official actions placed alongside equivalent private-sector conduct and their real-world consequences.',
  },
  {
    icon: AlertTriangle,
    title: 'Flagged Connections',
    description:
      'High-probability correlations flagged automatically based on timing, amount, and action type.',
  },
];

export default async function HomePage() {
  const stats = await getStats();

  return (
    <div>
      {/* Hero */}
      <section className="relative overflow-hidden">
        <div className="absolute inset-0 pointer-events-none">
          <div className="absolute top-0 left-1/2 -translate-x-1/2 w-[800px] h-[500px] bg-blue-500/8 rounded-full blur-3xl" />
        </div>

        <div className="max-w-4xl mx-auto px-4 sm:px-6 pt-24 pb-20 text-center relative">
          <div className="inline-flex items-center gap-2 px-3 py-1.5 rounded-full bg-white/6 border border-white/10 text-xs text-gray-400 mb-8">
            <span className="w-1.5 h-1.5 rounded-full bg-green-400 animate-pulse" />
            Built on public records — no claims without citations
          </div>

          <h1 className="text-5xl sm:text-6xl md:text-7xl font-bold text-white tracking-tight leading-tight mb-6">
            Every connection
            <br />
            <span className="text-transparent bg-clip-text bg-gradient-to-r from-blue-400 to-blue-600">
              traced to its source.
            </span>
          </h1>

          <p className="text-xl text-gray-400 max-w-2xl mx-auto mb-10 leading-relaxed">
            No claims. Only citations. A political accountability database
            documenting the connections between money, power, and legislation —
            sourced entirely from public records.
          </p>

          <form
            action="/search"
            method="get"
            className="relative max-w-2xl mx-auto mb-6"
          >
            <input
              type="search"
              name="q"
              placeholder="Search politicians, corporations, lobbyists, donors…"
              className="w-full pl-6 pr-36 py-5 text-lg bg-white/6 border border-white/12 rounded-2xl text-white placeholder-gray-600 focus:outline-none focus:ring-2 focus:ring-blue-500/40 focus:border-blue-500/40 transition-all shadow-2xl shadow-black/40"
            />
            <button
              type="submit"
              className="absolute right-2 top-1/2 -translate-y-1/2 px-5 py-2.5 bg-blue-500 hover:bg-blue-400 text-white text-sm font-medium rounded-xl transition-colors duration-200 flex items-center gap-2"
            >
              Search
              <ArrowRight className="w-4 h-4" />
            </button>
          </form>

          <div className="flex flex-wrap items-center justify-center gap-2 text-sm text-gray-600">
            <span>Try:</span>
            {['Senator Walsh', 'PharmaCorp', 'Defense PAC', 'Marcus Leland'].map(
              (term) => (
                <Link
                  key={term}
                  href={`/search?q=${encodeURIComponent(term)}`}
                  className="px-2.5 py-1 rounded-lg bg-white/5 border border-white/8 text-gray-400 hover:text-white hover:border-white/15 transition-all text-xs"
                >
                  {term}
                </Link>
              )
            )}
          </div>
        </div>
      </section>

      {/* Stats */}
      <section className="border-y border-white/6 bg-white/2">
        <div className="max-w-5xl mx-auto px-4 sm:px-6 py-12">
          <div className="grid grid-cols-2 md:grid-cols-4 gap-8 text-center">
            {[
              { label: 'Entities', value: stats.entities.toLocaleString(), color: 'text-white' },
              { label: 'Connections', value: stats.connections.toLocaleString(), color: 'text-white' },
              { label: 'Sources', value: stats.sources.toLocaleString(), color: 'text-white' },
              { label: 'Flagged', value: stats.flagged.toLocaleString(), color: 'text-red-400' },
            ].map((stat) => (
              <div key={stat.label}>
                <div className={`text-3xl font-bold ${stat.color} mb-1`}>
                  {stat.value}
                </div>
                <div className="text-sm text-gray-500">{stat.label}</div>
              </div>
            ))}
          </div>
        </div>
      </section>

      {/* Features */}
      <section className="max-w-5xl mx-auto px-4 sm:px-6 py-20">
        <div className="grid grid-cols-1 sm:grid-cols-2 gap-6">
          {FEATURES.map((feature) => (
            <Card key={feature.title} className="p-6">
              <div className="flex gap-4">
                <div className="shrink-0 w-10 h-10 bg-blue-500/15 rounded-xl flex items-center justify-center">
                  <feature.icon className="w-5 h-5 text-blue-400" />
                </div>
                <div>
                  <h3 className="font-semibold text-white mb-1">
                    {feature.title}
                  </h3>
                  <p className="text-sm text-gray-400 leading-relaxed">
                    {feature.description}
                  </p>
                </div>
              </div>
            </Card>
          ))}
        </div>
      </section>

      {/* Recent Correlations */}
      <section className="max-w-5xl mx-auto px-4 sm:px-6 pb-20">
        <div className="flex items-center justify-between mb-6">
          <div>
            <h2 className="text-2xl font-bold text-white">Recent Correlations</h2>
            <p className="text-sm text-gray-500 mt-1">
              Flagged connections between financial events and legislative actions
            </p>
          </div>
          <Link
            href="/timing"
            className="text-sm text-blue-400 hover:text-blue-300 flex items-center gap-1 transition-colors"
          >
            View all <ArrowRight className="w-3.5 h-3.5" />
          </Link>
        </div>

        <div className="space-y-3">
          {RECENT_CORRELATIONS.map((corr) => (
            <Link key={corr.id} href="/timing">
              <Card hover className="p-5">
                <CardContent className="p-0">
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-1.5">
                        <span className="text-sm font-semibold text-white">
                          {corr.official}
                        </span>
                        {corr.flagged && (
                          <Badge variant="red">
                            <AlertTriangle className="w-2.5 h-2.5" />
                            Flagged
                          </Badge>
                        )}
                      </div>
                      <p className="text-sm text-gray-400">{corr.description}</p>
                      <p className="text-xs text-gray-600 mt-1.5">
                        Source:{' '}
                        <span className="text-gray-500">{corr.source}</span>
                      </p>
                    </div>
                    <div className="text-right shrink-0">
                      <div className="text-2xl font-bold text-white">
                        {corr.days}
                      </div>
                      <div className="text-xs text-gray-500">days</div>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      </section>
    </div>
  );
}
