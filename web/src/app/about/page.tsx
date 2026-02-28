import { Github, Mail, Shield, Eye, Code2, ArrowRight } from 'lucide-react';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardHeader, CardTitle, CardDescription } from '@/components/ui/Card';

export const metadata = {
  title: 'About — Nonfaction',
  description:
    'Political accountability through radical transparency. Learn about Nonfaction\'s mission, methodology, and open-source commitment.',
};

const pillars = [
  {
    icon: Shield,
    title: 'Source Attribution',
    badge: 'Core Principle' as const,
    description:
      'Every data point published on Nonfaction traces directly to a verifiable primary source — a government filing, an official vote record, a federal disclosure document. If we cannot link it, we do not publish it. Attribution is not a feature; it is the foundation.',
  },
  {
    icon: Eye,
    title: 'No Editorial Opinion',
    badge: 'Core Principle' as const,
    description:
      'We present records, timelines, and relationships exactly as they appear in public documentation. Nonfaction draws no conclusions, assigns no motives, and issues no endorsements. The data speaks. You decide what it means.',
  },
  {
    icon: Code2,
    title: 'Open Source',
    badge: 'Core Principle' as const,
    description:
      'Every algorithm that scores, ranks, or surfaces data on this platform is publicly auditable. Our scoring logic, ingestion pipelines, and verification workflows live in open repositories under GPL v3. Transparency about transparency.',
  },
];

const timeline = [
  { phase: '01', label: 'Ingestion', detail: 'Automated scrapers and API clients pull from federal, state, and municipal sources on scheduled intervals.' },
  { phase: '02', label: 'Normalization', detail: 'Raw records are parsed into canonical schemas — entities, events, relationships — with provenance hashes preserved.' },
  { phase: '03', label: 'Verification', detail: 'Automated field validation runs first. High-impact records undergo additional human review before publication.' },
  { phase: '04', label: 'Timing Analysis', detail: 'Event pairs are timestamped and scored for temporal proximity using documented, deterministic rules.' },
  { phase: '05', label: 'Archive', detail: 'Every source is stored in a content-addressable archive with Merkle DAG integrity proofs, tamper-evident by design.' },
  { phase: '06', label: 'Publication', detail: 'Verified records surface in the public database with full source chains, confidence indicators, and audit links.' },
];

export default function AboutPage() {
  return (
    <div className="mx-auto max-w-4xl px-4 py-16 sm:px-6">
      {/* Hero */}
      <div className="relative">
        <Badge variant="blue" className="mb-6">About Nonfaction</Badge>
        <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl lg:text-6xl">
          Political accountability through{' '}
          <span className="bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent">
            radical transparency
          </span>
        </h1>
        <p className="mt-6 max-w-2xl text-lg leading-relaxed text-gray-400">
          Nonfaction is a civic infrastructure project. We trace the relationships between money,
          influence, and policy — mapping every signal back to primary source evidence, without
          editorial commentary, without hidden logic, without partisan framing.
        </p>
        <p className="mt-4 max-w-2xl text-base leading-relaxed text-gray-500">
          We believe the most powerful accountability tool is a complete, accurate, and auditable
          record. Not narrative. Not opinion. Record.
        </p>
      </div>

      {/* 3 Pillars */}
      <section className="mt-20">
        <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">
          Three pillars
        </h2>
        <h3 className="mt-2 text-2xl font-semibold text-white">What we stand for</h3>
        <div className="mt-8 grid gap-5 sm:grid-cols-3">
          {pillars.map(({ icon: Icon, title, badge, description }) => (
            <Card key={title} hover className="flex flex-col">
              <CardHeader>
                <div className="mb-3 flex h-10 w-10 items-center justify-center rounded-xl bg-blue-500/15 ring-1 ring-blue-500/30">
                  <Icon className="h-5 w-5 text-blue-400" />
                </div>
                <CardTitle className="text-base">{title}</CardTitle>
                <div className="mt-1">
                  <Badge variant="blue">{badge}</Badge>
                </div>
              </CardHeader>
              <CardContent className="flex-1 pt-0">
                <p className="text-sm leading-relaxed text-gray-400">{description}</p>
              </CardContent>
            </Card>
          ))}
        </div>
      </section>

      {/* Methodology timeline */}
      <section className="mt-20">
        <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">
          How it works
        </h2>
        <h3 className="mt-2 text-2xl font-semibold text-white">Methodology overview</h3>
        <p className="mt-3 text-sm text-gray-400">
          Every record on Nonfaction passes through a six-stage pipeline before it surfaces publicly.
          Each stage is documented, versioned, and open for inspection.
        </p>
        <div className="mt-8 space-y-0">
          {timeline.map((step, i) => (
            <div key={step.phase} className="relative flex gap-6">
              {/* connector line */}
              {i < timeline.length - 1 && (
                <div className="absolute left-[19px] top-12 h-full w-px bg-white/8" />
              )}
              <div className="flex h-10 w-10 shrink-0 items-center justify-center rounded-full border border-blue-500/30 bg-blue-500/10 text-xs font-bold text-blue-400">
                {step.phase}
              </div>
              <div className="pb-8 pt-1.5">
                <p className="text-sm font-semibold text-white">{step.label}</p>
                <p className="mt-1 text-sm leading-relaxed text-gray-400">{step.detail}</p>
              </div>
            </div>
          ))}
        </div>
        <a
          href="/methodology"
          className="mt-2 inline-flex items-center gap-1.5 text-sm font-medium text-blue-400 hover:text-blue-300 transition-colors"
        >
          Full methodology documentation <ArrowRight className="h-3.5 w-3.5" />
        </a>
      </section>

      {/* Team */}
      <section className="mt-20">
        <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">Team</h2>
        <h3 className="mt-2 text-2xl font-semibold text-white">Who builds this</h3>
        <Card className="mt-6">
          <CardContent className="p-8">
            <div className="flex items-start gap-4">
              <div className="flex h-12 w-12 shrink-0 items-center justify-center rounded-full bg-white/8 ring-1 ring-white/12">
                <span className="text-lg font-bold text-white">N</span>
              </div>
              <div>
                <p className="font-semibold text-white">Core Contributor</p>
                <p className="mt-0.5 text-sm text-gray-400">
                  Nonfaction is an independent civic infrastructure project. Full contributor
                  profiles and advisory relationships will be published with complete disclosure
                  notes as the project matures. We are committed to the same transparency
                  standards we demand of the institutions we document.
                </p>
                <Badge variant="outline" className="mt-3">Profiles forthcoming</Badge>
              </div>
            </div>
          </CardContent>
        </Card>
      </section>

      {/* Open source */}
      <section className="mt-20">
        <h2 className="text-xs font-semibold uppercase tracking-widest text-blue-400">
          Open source
        </h2>
        <h3 className="mt-2 text-2xl font-semibold text-white">Our commitment to auditability</h3>
        <p className="mt-3 text-sm leading-relaxed text-gray-400">
          The entire Nonfaction platform — scrapers, ingestion pipelines, scoring algorithms, and
          this web application — is developed in the open under GPL v3. If you can use it to hold
          governments accountable, you can read, fork, and improve it.
        </p>
        <div className="mt-6 grid gap-4 sm:grid-cols-2">
          <Card hover>
            <CardContent className="p-5">
              <p className="text-xs font-semibold uppercase tracking-widest text-gray-500">License</p>
              <p className="mt-1 text-sm font-medium text-white">GPL v3 — Source code</p>
              <p className="mt-1 text-xs text-gray-400">
                All platform source code is free software. Forks must remain open.
              </p>
            </CardContent>
          </Card>
          <Card hover>
            <CardContent className="p-5">
              <p className="text-xs font-semibold uppercase tracking-widest text-gray-500">Data license</p>
              <p className="mt-1 text-sm font-medium text-white">CC-BY-SA 4.0 — Data</p>
              <p className="mt-1 text-xs text-gray-400">
                Curated datasets are freely shareable with attribution and share-alike.
              </p>
            </CardContent>
          </Card>
        </div>
        <a
          href="https://github.com/santht/Nonfaction"
          target="_blank"
          rel="noopener noreferrer"
          className="mt-6 inline-flex items-center gap-2 rounded-xl border border-white/12 bg-white/6 px-5 py-3 text-sm font-medium text-white transition-all duration-200 hover:bg-white/10 hover:border-white/20"
        >
          <Github className="h-4 w-4" />
          View on GitHub — github.com/santht/Nonfaction
        </a>
      </section>

      {/* Contact */}
      <section className="mt-20">
        <div className="rounded-2xl border border-blue-500/20 bg-blue-500/6 p-8">
          <h3 className="text-xl font-semibold text-white">Get in touch</h3>
          <p className="mt-2 text-sm text-gray-400">
            Questions about data accuracy, methodology, potential collaboration, or press
            inquiries — reach out directly.
          </p>
          <div className="mt-6 flex flex-col gap-3 sm:flex-row sm:items-center">
            <a
              href="mailto:santht@proton.me"
              className="inline-flex items-center gap-2 rounded-xl bg-blue-500 px-5 py-2.5 text-sm font-medium text-white shadow-lg shadow-blue-500/25 transition-all duration-200 hover:bg-blue-400"
            >
              <Mail className="h-4 w-4" />
              santht@proton.me
            </a>
            <a
              href="https://github.com/santht/Nonfaction"
              target="_blank"
              rel="noopener noreferrer"
              className="inline-flex items-center gap-2 rounded-xl border border-white/12 bg-white/6 px-5 py-2.5 text-sm font-medium text-white transition-all duration-200 hover:bg-white/10"
            >
              <Github className="h-4 w-4" />
              Open an issue on GitHub
            </a>
          </div>
        </div>
      </section>
    </div>
  );
}
