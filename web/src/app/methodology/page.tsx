import { Database, GitBranch, Clock, FileSearch, ShieldCheck, Link2 } from 'lucide-react';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';

export const metadata = {
  title: 'Methodology — Nonfaction',
  description:
    'How Nonfaction collects, verifies, and publishes political accountability data — source tiers, timing analysis, and tamper-proof archiving.',
};

const sourceTiers = [
  {
    tier: 'Tier 1',
    label: 'Government APIs',
    badge: 'Highest trust' as const,
    badgeVariant: 'blue' as const,
    sources: [
      'FEC electronic filings API',
      'Congress.gov vote records',
      'PACER federal court filings',
      'USASpending.gov contracts database',
      'SEC EDGAR disclosures',
      'OpenSecrets lobbying registrations',
    ],
    description:
      'Direct machine-readable feeds from official government systems. These sources are considered authoritative with minimal transformation required. Hash integrity is verified on every fetch cycle.',
  },
  {
    tier: 'Tier 2',
    label: 'Public Databases',
    badge: 'High trust' as const,
    badgeVariant: 'green' as const,
    sources: [
      'State ethics commission filings',
      'Lobbyist registration databases',
      'State campaign finance records',
      'Court PACER state-level equivalents',
      'Public procurement portals',
      'Inspector General reports',
    ],
    description:
      'Curated public databases maintained by state agencies and established civic organizations. Records undergo normalization and deduplication before integration.',
  },
  {
    tier: 'Tier 3',
    label: 'State & Local Sources',
    badge: 'Verified' as const,
    badgeVariant: 'yellow' as const,
    sources: [
      'Municipal council voting records',
      'County-level property and tax data',
      'Local lobbying registrations',
      'School board and special district filings',
      'Verified crowdsourced submissions with source links',
      'Journalist-archived public documents',
    ],
    description:
      'Long-tail accountability data covering the sub-federal layer where much governance actually happens. Higher verification overhead — every record requires a direct source URL or filing reference.',
  },
];

const timingRules = [
  {
    label: 'Donation → Vote',
    window: '< 90 days',
    color: 'red' as const,
    description:
      'Campaign contributions received within 90 days of a directly related legislative vote are flagged for proximity analysis. The timing window is documented in academic literature on campaign finance influence.',
  },
  {
    label: 'Lobbying → Vote',
    window: '< 180 days',
    color: 'yellow' as const,
    description:
      'Registered lobbying activity on a specific bill or policy area, within 180 days of a relevant vote, is surfaced as a timing correlation. The expanded window reflects the lobbying disclosure lag.',
  },
  {
    label: 'Indictment → Pardon',
    window: 'Always flagged',
    color: 'red' as const,
    description:
      'Any presidential or gubernatorial pardon or commutation granted to an individual with an active federal or state indictment is flagged regardless of timing. This category has no time window — the relationship is unconditional.',
  },
  {
    label: 'Regulatory Action → Donation',
    window: '< 60 days',
    color: 'yellow' as const,
    description:
      'Donations received within 60 days following a favorable regulatory decision affecting the donor\'s industry are flagged for reverse-proximity analysis.',
  },
];

export default function MethodologyPage() {
  return (
    <div className="mx-auto max-w-4xl px-4 py-16 sm:px-6">
      {/* Hero */}
      <Badge variant="blue" className="mb-6">Methodology</Badge>
      <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl">
        How we collect, verify, and{' '}
        <span className="bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent">
          publish data
        </span>
      </h1>
      <p className="mt-6 max-w-2xl text-lg leading-relaxed text-gray-400">
        Every record on Nonfaction is traceable to a primary source. This document describes
        exactly how that data moves from raw government filings to the public interface — with no
        gaps, no black boxes, and no editorial intervention.
      </p>

      {/* Data Collection */}
      <section className="mt-20">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-500/15 ring-1 ring-blue-500/30">
            <Database className="h-4 w-4 text-blue-400" />
          </div>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">Collection</h2>
        </div>
        <h3 className="mt-3 text-2xl font-semibold text-white">How data is collected</h3>
        <p className="mt-3 text-sm leading-relaxed text-gray-400">
          Data enters the Nonfaction pipeline through three complementary mechanisms, each with
          different trust requirements and update frequencies.
        </p>
        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          {[
            {
              title: 'Automated Scrapers',
              detail: 'Scheduled cron-based scrapers in Rust and Python pull structured data from government portals, parsing HTML, XML, and JSON into canonical records with provenance hashes.',
            },
            {
              title: 'Official APIs',
              detail: 'Where government agencies provide machine-readable APIs (FEC, Congress.gov, USASpending), we use authenticated API clients with rate limiting, retry logic, and change detection.',
            },
            {
              title: 'Crowdsourced Submissions',
              detail: 'Community submissions via the Submit interface are accepted only with a verifiable primary source link. No anonymous evidence. Every submission undergoes human review before publication.',
            },
          ].map((item) => (
            <Card key={item.title} hover>
              <CardHeader>
                <CardTitle className="text-sm">{item.title}</CardTitle>
              </CardHeader>
              <CardContent className="pt-0">
                <p className="text-xs leading-relaxed text-gray-400">{item.detail}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      {/* Source Tiers */}
      <section className="mt-20">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-500/15 ring-1 ring-blue-500/30">
            <GitBranch className="h-4 w-4 text-blue-400" />
          </div>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">Source tiers</h2>
        </div>
        <h3 className="mt-3 text-2xl font-semibold text-white">Hierarchy of source trust</h3>
        <p className="mt-3 text-sm leading-relaxed text-gray-400">
          Not all public sources are equal in reliability and latency. Nonfaction uses a
          three-tier classification to communicate confidence and update cadence.
        </p>
        <div className="mt-6 space-y-4">
          {sourceTiers.map((tier) => (
            <Card key={tier.tier} hover>
              <CardContent className="p-6">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-xs font-bold text-gray-500">{tier.tier}</span>
                      <Badge variant={tier.badgeVariant}>{tier.badge}</Badge>
                    </div>
                    <h4 className="mt-1 text-base font-semibold text-white">{tier.label}</h4>
                  </div>
                </div>
                <p className="mt-3 text-sm leading-relaxed text-gray-400">{tier.description}</p>
                <div className="mt-4 flex flex-wrap gap-2">
                  {tier.sources.map((s) => (
                    <span
                      key={s}
                      className="rounded-md border border-white/8 bg-white/4 px-2 py-0.5 text-xs text-gray-400"
                    >
                      {s}
                    </span>
                  ))}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      {/* Verification Pipeline */}
      <section className="mt-20">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-500/15 ring-1 ring-blue-500/30">
            <ShieldCheck className="h-4 w-4 text-blue-400" />
          </div>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">Verification</h2>
        </div>
        <h3 className="mt-3 text-2xl font-semibold text-white">Two-stage verification pipeline</h3>
        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Automated checks</CardTitle>
              <CardDescription>Runs on every ingested record</CardDescription>
            </CardHeader>
            <CardContent className="pt-0">
              <ul className="space-y-2 text-sm text-gray-400">
                {[
                  'Source URL reachability and hash consistency',
                  'Required field completeness validation',
                  'Entity name normalization and deduplication',
                  'Date/timestamp format and range validation',
                  'Cross-reference against existing records for conflicts',
                  'Schema compliance against canonical record types',
                ].map((item) => (
                  <li key={item} className="flex items-start gap-2">
                    <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-500" />
                    {item}
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>
          <Card>
            <CardHeader>
              <CardTitle className="text-base">Human review</CardTitle>
              <CardDescription>Applied to high-impact records</CardDescription>
            </CardHeader>
            <CardContent className="pt-0">
              <ul className="space-y-2 text-sm text-gray-400">
                {[
                  'Manual source verification against original document',
                  'Contextual accuracy review against public record',
                  'Entity disambiguation for common name collisions',
                  'Legal review for records involving active litigation',
                  'Sensitivity review for records involving minors',
                  'Community correction processing and adjudication',
                ].map((item) => (
                  <li key={item} className="flex items-start gap-2">
                    <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-green-500" />
                    {item}
                  </li>
                ))}
              </ul>
            </CardContent>
          </Card>
        </div>
      </section>

      {/* Timing Analysis */}
      <section className="mt-20">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-500/15 ring-1 ring-blue-500/30">
            <Clock className="h-4 w-4 text-blue-400" />
          </div>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">Timing analysis</h2>
        </div>
        <h3 className="mt-3 text-2xl font-semibold text-white">Temporal proximity rules</h3>
        <p className="mt-3 text-sm leading-relaxed text-gray-400">
          Timing analysis surfaces meaningful temporal proximity between events — not causation,
          not conclusions. The rules below are deterministic, documented, and version-controlled.
          All timing windows are based on publicly documented standards in campaign finance and
          lobbying disclosure literature.
        </p>
        <div className="mt-6 space-y-4">
          {timingRules.map((rule) => (
            <Card key={rule.label} hover>
              <CardContent className="p-5">
                <div className="flex flex-wrap items-center justify-between gap-3">
                  <h4 className="font-semibold text-white">{rule.label}</h4>
                  <Badge variant={rule.color}>Window: {rule.window}</Badge>
                </div>
                <p className="mt-2 text-sm leading-relaxed text-gray-400">{rule.description}</p>
              </CardContent>
            </Card>
          ))}
        </div>
        <div className="mt-4 rounded-xl border border-yellow-500/20 bg-yellow-500/6 p-4">
          <p className="text-sm text-yellow-300">
            <span className="font-semibold">Important:</span> Timing analysis surfaces correlation
            only. Nonfaction makes no claim of causation. Scores are descriptive analytical tools,
            not allegations.
          </p>
        </div>
      </section>

      {/* No-editorial policy */}
      <section className="mt-20">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-500/15 ring-1 ring-blue-500/30">
            <FileSearch className="h-4 w-4 text-blue-400" />
          </div>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">No-editorial policy</h2>
        </div>
        <h3 className="mt-3 text-2xl font-semibold text-white">What we never do</h3>
        <Card className="mt-6">
          <CardContent className="p-6">
            <div className="grid gap-4 sm:grid-cols-2">
              {[
                { label: 'No narrative framing', detail: 'Records are presented as structured data, not as stories with protagonists and antagonists.' },
                { label: 'No partisan signals', detail: 'No language, imagery, or ordering that implies political endorsement or opposition.' },
                { label: 'No anonymous sources', detail: 'Every surfaced record traces to a named, verifiable public document. Unnamed allegations are not published.' },
                { label: 'No hidden algorithms', detail: 'Every scoring function, ranking rule, and timing window is documented in public code under GPL v3.' },
              ].map((item) => (
                <div key={item.label} className="rounded-xl border border-white/8 bg-white/3 p-4">
                  <p className="text-sm font-semibold text-white">{item.label}</p>
                  <p className="mt-1 text-xs leading-relaxed text-gray-400">{item.detail}</p>
                </div>
              ))}
            </div>
          </CardContent>
        </Card>
      </section>

      {/* Archive & Integrity */}
      <section className="mt-20">
        <div className="flex items-center gap-3">
          <div className="flex h-8 w-8 items-center justify-center rounded-lg bg-blue-500/15 ring-1 ring-blue-500/30">
            <Link2 className="h-4 w-4 text-blue-400" />
          </div>
          <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">Archive & integrity</h2>
        </div>
        <h3 className="mt-3 text-2xl font-semibold text-white">Tamper-proof by design</h3>
        <div className="mt-6 grid gap-4 sm:grid-cols-3">
          {[
            {
              title: 'Content-Addressable Archive',
              detail: 'Every source document is stored by its cryptographic hash. The content cannot be altered without changing the identifier — making silent tampering impossible.',
            },
            {
              title: 'Merkle DAG Audit Trail',
              detail: 'The sequence of all data mutations is linked in a Merkle Directed Acyclic Graph. Any alteration to historical records invalidates all subsequent nodes.',
            },
            {
              title: 'Compile-Time Source Enforcement',
              detail: 'The type system enforces source chain completeness at build time. Records missing verified provenance cannot compile into a publishable state.',
            },
          ].map((item) => (
            <Card key={item.title} hover>
              <CardHeader>
                <CardTitle className="text-sm">{item.title}</CardTitle>
              </CardHeader>
              <CardContent className="pt-0">
                <p className="text-xs leading-relaxed text-gray-400">{item.detail}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>
    </div>
  );
}
