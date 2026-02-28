import Link from 'next/link';
import { ArrowRight, CheckCircle2, ShieldCheck, Radar, Database, Sparkles } from 'lucide-react';
import { getRecentActivity, getStats } from '@/lib/api';
import { Button } from '@/components/ui/Button';
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';
import { Input } from '@/components/ui/Input';
import { Stat } from '@/components/ui/Stat';
import { SourceNote } from '@/components/ui/SourceNote';

const HOW_IT_WORKS = [
  {
    title: 'Ingest',
    description: 'Structured pull from filings, votes, dockets, and disclosure records.',
    icon: Database,
  },
  {
    title: 'Verify',
    description: 'Each claim candidate requires source-linked references before publication.',
    icon: ShieldCheck,
  },
  {
    title: 'Correlate',
    description: 'Timing models surface connections between money movement and official actions.',
    icon: Radar,
  },
];

const TESTIMONIALS = [
  {
    quote: 'The source traceability makes this usable for newsroom workflows.',
    author: 'Investigative Editor (Partner Beta)',
  },
  {
    quote: 'The no-editorial policy is exactly what we need for clean civic data.',
    author: 'Civic Research Lab',
  },
  {
    quote: 'Correlation timelines are clearer than any spreadsheet pipeline we built internally.',
    author: 'Open Governance Analyst',
  },
];

export default async function HomePage() {
  const [stats, recentActivity] = await Promise.all([getStats(), getRecentActivity()]);

  return (
    <div>
      <section className="relative overflow-hidden border-b border-white/10">
        <div className="absolute inset-0 bg-[radial-gradient(circle_at_top,rgba(59,130,246,0.24),transparent_55%)]" />
        <div className="relative mx-auto max-w-7xl px-4 pb-20 pt-20 sm:px-6">
          <Badge variant="blue" className="mb-5">
            <Sparkles className="h-3 w-3" /> Public-record accountability intelligence
          </Badge>
          <h1 className="max-w-4xl text-4xl font-semibold leading-tight text-white sm:text-6xl">
            Premium political accountability,
            <span className="block bg-gradient-to-r from-blue-300 to-blue-500 bg-clip-text text-transparent">
              built on citations not narratives.
            </span>
          </h1>
          <p className="mt-5 max-w-2xl text-base text-gray-300 sm:text-lg">
            Nonfaction tracks money, influence, and policy with source-level evidence. Every displayed signal points back to a public record.
          </p>

          <div className="mt-8 flex flex-wrap gap-3">
            <Link href="/search">
              <Button>
                Start exploring
                <ArrowRight className="h-4 w-4" />
              </Button>
            </Link>
            <Link href="/methodology">
              <Button variant="secondary">Read methodology</Button>
            </Link>
          </div>

          <div className="mt-10 grid grid-cols-1 gap-4 sm:grid-cols-2 xl:grid-cols-4">
            <Stat label="Tracked entities" value={stats.entities.toLocaleString()} change="+5.4% this month" trend="up" />
            <Stat label="Linked connections" value={stats.connections.toLocaleString()} change="+1,012 this week" trend="up" />
            <Stat label="Source records" value={stats.sources.toLocaleString()} change="99.8% verified" trend="neutral" />
            <Stat label="Flagged patterns" value={stats.flagged.toLocaleString()} change="Needs review" trend="down" />
          </div>
        </div>
      </section>

      <section className="mx-auto max-w-7xl px-4 py-16 sm:px-6">
        <div className="mb-8 flex items-end justify-between gap-4">
          <div>
            <h2 className="text-2xl font-semibold text-white">How it works</h2>
            <p className="text-sm text-gray-400">End-to-end accountability workflow with reproducible evidence chains.</p>
          </div>
          <Link href="/sources" className="text-sm text-blue-300 hover:text-blue-200">
            View source catalog
          </Link>
        </div>
        <div className="grid gap-4 md:grid-cols-3">
          {HOW_IT_WORKS.map((step) => (
            <Card key={step.title} hover>
              <CardHeader>
                <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-xl bg-blue-500/15">
                  <step.icon className="h-5 w-5 text-blue-300" />
                </div>
                <CardTitle>{step.title}</CardTitle>
                <CardDescription>{step.description}</CardDescription>
              </CardHeader>
              <CardContent className="pt-2">
                <div className="rounded-lg border border-white/10 bg-white/3 px-3 py-2 text-xs text-gray-400">
                  System state: audited source-links required.
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      <section className="mx-auto grid max-w-7xl gap-6 px-4 pb-16 sm:px-6 lg:grid-cols-[1.2fr_0.8fr]">
        <Card>
          <CardHeader>
            <CardTitle>Recent activity feed</CardTitle>
            <CardDescription>Newest validations, source updates, and alerting signals.</CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            {recentActivity.map((item) => (
              <div key={item.id} className="rounded-xl border border-white/10 bg-white/3 p-4">
                <div className="flex items-center justify-between gap-3">
                  <p className="text-sm font-medium text-white">{item.title}</p>
                  <Badge variant={item.severity === 'high' ? 'red' : item.severity === 'medium' ? 'yellow' : 'green'}>
                    {item.severity}
                  </Badge>
                </div>
                <p className="mt-1 text-xs text-gray-400">{item.time} · Source: {item.source}</p>
              </div>
            ))}
            <SourceNote text="Data attribution: FEC, Congress.gov, Senate LDA, CourtListener, OpenSecrets." />
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>Data source coverage</CardTitle>
            <CardDescription>Tiered ingest roadmap from day-one to month-one source sets.</CardDescription>
          </CardHeader>
          <CardContent>
            <div className="space-y-3 text-sm text-gray-300">
              <p className="rounded-xl border border-white/10 bg-white/3 p-3">Tier 1 Day One: Federal filings, roll calls, and disclosure baselines.</p>
              <p className="rounded-xl border border-white/10 bg-white/3 p-3">Tier 2 Week One: State ethics, procurement, and judiciary expansions.</p>
              <p className="rounded-xl border border-white/10 bg-white/3 p-3">Tier 3 Month One: Municipal, contractor, and watchdog aggregation.</p>
            </div>
            <div className="mt-4">
              <Link href="/sources" className="text-sm text-blue-300 hover:text-blue-200">
                Browse all 108+ sources →
              </Link>
            </div>
          </CardContent>
        </Card>
      </section>

      <section className="border-y border-white/10 bg-white/3">
        <div className="mx-auto max-w-7xl px-4 py-14 sm:px-6">
          <h2 className="text-2xl font-semibold text-white">Trust signals</h2>
          <div className="mt-5 grid gap-4 md:grid-cols-3">
            {TESTIMONIALS.map((item) => (
              <Card key={item.author}>
                <CardContent>
                  <p className="text-sm text-gray-200">“{item.quote}”</p>
                  <p className="mt-3 text-xs text-gray-500">{item.author}</p>
                </CardContent>
              </Card>
            ))}
          </div>
        </div>
      </section>

      <section className="mx-auto max-w-7xl px-4 py-16 sm:px-6">
        <div className="rounded-2xl border border-blue-500/30 bg-gradient-to-r from-blue-500/20 to-blue-900/20 p-6 sm:p-8">
          <h2 className="text-2xl font-semibold text-white">Stay informed</h2>
          <p className="mt-2 max-w-2xl text-sm text-blue-100/90">
            Receive weekly source additions, new story package releases, and accountability milestones.
          </p>
          <form className="mt-5 flex max-w-xl flex-col gap-2 sm:flex-row" action="#" method="post">
            <Input type="email" required placeholder="name@organization.org" aria-label="Newsletter email" />
            <Button type="submit">Join newsletter</Button>
          </form>
          <p className="mt-2 inline-flex items-center gap-1 text-xs text-blue-100/75">
            <CheckCircle2 className="h-3.5 w-3.5" /> No user PII stored client-side.
          </p>
        </div>
      </section>
    </div>
  );
}
