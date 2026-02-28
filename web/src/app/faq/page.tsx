'use client';

import { useState } from 'react';
import { ChevronDown, Mail } from 'lucide-react';
import { Badge } from '@/components/ui/Badge';
import { cn } from '@/lib/utils';

const categories = [
  {
    id: 'general',
    label: 'General',
    badge: 'blue' as const,
    questions: [
      {
        q: 'What is Nonfaction?',
        a: 'Nonfaction is a civic infrastructure platform that traces relationships between money, influence, and policy using verifiable public records. Every data point maps to a primary source — a government filing, a vote record, or an official disclosure. We present facts and let you draw your own conclusions.',
      },
      {
        q: 'Is Nonfaction biased toward a particular political party?',
        a: 'No. Nonfaction applies identical methodology across all elected officials and institutions regardless of party affiliation. The timing windows, source tiers, and verification rules are fixed and publicly documented. If a pattern appears in the data, it appears because the public record shows it — not because of editorial selection.',
      },
      {
        q: 'Who runs Nonfaction?',
        a: 'Nonfaction is an independent civic project. It is not funded by political parties, lobbying groups, media organizations, or government agencies. Full contributor and funding disclosures will be published as the project matures, held to the same transparency standards we apply to the institutions we document.',
      },
      {
        q: 'How is Nonfaction funded?',
        a: 'The project operates on minimal infrastructure costs and volunteer labor. We do not run advertising, sell data, or accept funding from entities with interests in the political outcomes we document. Funding disclosures are maintained in the open-source repository.',
      },
      {
        q: 'Is this a news organization?',
        a: 'No. Nonfaction is a data infrastructure project, not a journalism outlet. We do not write articles, editorials, or opinion pieces. We publish structured records. Journalists, researchers, and citizens are free to use the data under our CC-BY-SA license.',
      },
      {
        q: 'Does Nonfaction cover international politics?',
        a: 'Currently, the primary focus is on United States federal, state, and local government. International expansion is on the roadmap but not currently resourced. Community contributions covering international jurisdictions are welcome through the standard submission process.',
      },
    ],
  },
  {
    id: 'data',
    label: 'Data & Sources',
    badge: 'green' as const,
    questions: [
      {
        q: 'How accurate is Nonfaction data?',
        a: 'Every published record requires at least one linked primary source. High-impact records — particularly those involving criminal proceedings or major financial relationships — undergo additional human review before publication. We display confidence indicators where data quality is partial or where sources have known limitations.',
      },
      {
        q: 'What sources does Nonfaction use?',
        a: 'We use a three-tier source hierarchy: Tier 1 includes direct government APIs such as FEC, Congress.gov, PACER, and USASpending. Tier 2 includes state-level ethics commission filings and public databases. Tier 3 covers municipal records and verified community submissions. Full source documentation is available on the Methodology page.',
      },
      {
        q: 'How often is the data updated?',
        a: 'Tier 1 government API sources are refreshed daily. Tier 2 state sources update weekly. Tier 3 local sources update on variable schedules based on source availability. Community submissions are reviewed and published on a rolling basis. Each record displays its last-verified timestamp.',
      },
      {
        q: 'I found an error. How do I report it?',
        a: 'Use the contact form or email santht@proton.me with "Data Correction" in the subject line. Include the specific record identifier, the claimed error, and a link to a primary source that supports the correction. All correction requests are reviewed and adjudicated publicly in our GitHub issue tracker.',
      },
      {
        q: 'Does Nonfaction make allegations against public officials?',
        a: 'No. Nonfaction presents records, timelines, and temporal correlations as they appear in public documentation. We explicitly distinguish between correlation and causation. Timing analysis flags proximity between events — it does not assert wrongdoing. Any language that implies an allegation is a data error we want corrected.',
      },
      {
        q: 'Can I download the full dataset?',
        a: 'Yes. Bulk data exports are available under the CC-BY-SA 4.0 license. Access the API documentation page for endpoint details and rate limits. Full database dumps are published periodically to the GitHub repository for researchers who need offline access.',
      },
    ],
  },
  {
    id: 'contributing',
    label: 'Contributing',
    badge: 'blue' as const,
    questions: [
      {
        q: 'How can I submit a record?',
        a: 'Use the Submit page to provide sourced records. Every submission requires: a direct link to the primary source document, specific identifiers (filing ID, case number, bill number, or official reference), and the relevant public officials or entities involved. Anonymous submissions are not accepted.',
      },
      {
        q: 'Can I contribute to the codebase?',
        a: 'Yes. The entire platform is open source under GPL v3. Visit github.com/santht/Nonfaction to find the code, open issues, contribution guidelines, and active development discussions. All code contributions are reviewed and merged under the same transparency standards as data contributions.',
      },
      {
        q: 'What kinds of records are most needed?',
        a: 'Tier 3 local and municipal records are the area with the most gaps. School board votes, county commissioner decisions, local lobbying registrations, and municipal procurement contracts are all in scope and underrepresented in the current database. State-level campaign finance records for smaller races are also highly valued.',
      },
      {
        q: 'Is there a review process for submissions?',
        a: 'Yes. All submissions go through automated validation (source URL reachability, required field checks, schema compliance) followed by human review for high-impact records. The review status of your submission is trackable through the submission interface. We aim to process submissions within 72 hours.',
      },
      {
        q: 'What happens to rejected submissions?',
        a: 'Rejected submissions receive a specific reason for rejection with guidance on what additional evidence or clarification would allow reconsideration. We do not permanently blacklist submitters. The goal is to help contributors meet the evidentiary bar, not to exclude them.',
      },
    ],
  },
  {
    id: 'privacy',
    label: 'Privacy & Legal',
    badge: 'yellow' as const,
    questions: [
      {
        q: 'Does Nonfaction track visitors?',
        a: 'We collect minimal operational logs (IP addresses, request timestamps, error rates) for security and reliability purposes. We do not run advertising trackers, fingerprinting scripts, or behavioral analytics. We do not sell visitor data to any third party.',
      },
      {
        q: 'Is publishing this information about politicians legal?',
        a: 'Yes. Nonfaction publishes exclusively from public records — information that government agencies are legally required to make available. Campaign finance disclosures, vote records, court filings, and lobbying registrations are public by law. Aggregating and republishing public information for civic purposes is protected activity.',
      },
      {
        q: 'What if a public official requests removal of their information?',
        a: 'Public officials\' official conduct in their public roles is a matter of public record and is not subject to removal requests. We evaluate requests involving private individuals who appear in public records on a case-by-case basis, particularly where sensitive information about non-public-figure third parties is involved.',
      },
      {
        q: 'Does Nonfaction publish private individuals\' information?',
        a: 'We focus on public officials, registered lobbyists, and entities that have voluntarily entered the public sphere through participation in political or governmental processes. We do not publish private citizen information from public records absent a compelling public interest reason.',
      },
      {
        q: 'Is this legal advice?',
        a: 'No. Nonfaction provides public data aggregation and analytical tools. Nothing on this platform constitutes legal, financial, or political advice. For legal questions related to specific public records or your rights under public disclosure laws, consult a qualified attorney.',
      },
    ],
  },
  {
    id: 'technical',
    label: 'Technical',
    badge: 'blue' as const,
    questions: [
      {
        q: 'Is there an API?',
        a: 'Yes. The Nonfaction API provides programmatic access to officials, entities, events, relationships, and timing analyses. See the API Docs page for endpoint reference, authentication details, and rate limits. The API is free for research and journalism use within documented rate limits.',
      },
      {
        q: 'What is the Merkle DAG audit trail?',
        a: 'Every mutation to the Nonfaction database is recorded in a Merkle Directed Acyclic Graph — a cryptographic data structure where each node references its predecessors by hash. This makes it mathematically impossible to alter historical records without invalidating all subsequent nodes, creating a tamper-evident audit trail.',
      },
      {
        q: 'What is content-addressable storage?',
        a: 'Every source document we archive is stored using its cryptographic hash as its identifier. The content determines the address. This means a document cannot be silently altered — any change produces a different hash and therefore a different identifier, immediately detectable.',
      },
      {
        q: 'What technology stack does Nonfaction use?',
        a: 'The web interface runs on Next.js with TypeScript. Backend services are written in Rust for performance-critical ingestion and Rust/Python for scraping. Data storage uses PostgreSQL with content-addressed archive. Infrastructure is on commodity cloud compute. Full technical details are in the open-source repository.',
      },
      {
        q: 'How does compile-time source enforcement work?',
        a: 'The Rust type system is used to make source chain completeness a compile-time requirement rather than a runtime check. Record types that represent publishable data carry type-level proofs that verified source references are present. Code that tries to publish a record without valid provenance simply will not compile.',
      },
    ],
  },
];

function FAQItem({ q, a }: { q: string; a: string }) {
  const [open, setOpen] = useState(false);
  return (
    <div className="rounded-xl border border-white/10 bg-white/4 transition-all duration-200 hover:border-white/15">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="flex w-full items-center justify-between gap-4 px-5 py-4 text-left"
        aria-expanded={open}
      >
        <span className="text-sm font-medium text-white">{q}</span>
        <ChevronDown
          className={cn(
            'h-4 w-4 shrink-0 text-gray-400 transition-transform duration-200',
            open && 'rotate-180'
          )}
        />
      </button>
      {open && (
        <div className="border-t border-white/8 px-5 pb-5 pt-4 text-sm leading-relaxed text-gray-400">
          {a}
        </div>
      )}
    </div>
  );
}

export default function FaqPage() {
  const [activeCategory, setActiveCategory] = useState('general');

  const current = categories.find((c) => c.id === activeCategory) ?? categories[0];

  return (
    <div className="mx-auto max-w-4xl px-4 py-16 sm:px-6">
      {/* Header */}
      <Badge variant="blue" className="mb-6">FAQ</Badge>
      <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl">
        Frequently asked{' '}
        <span className="bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent">
          questions
        </span>
      </h1>
      <p className="mt-6 max-w-2xl text-lg leading-relaxed text-gray-400">
        Everything you need to know about how Nonfaction works, where data comes from,
        and how to get involved.
      </p>

      {/* Category tabs */}
      <div className="mt-10 flex flex-wrap gap-2">
        {categories.map((cat) => (
          <button
            key={cat.id}
            type="button"
            onClick={() => setActiveCategory(cat.id)}
            className={cn(
              'rounded-xl border px-4 py-2 text-sm font-medium transition-all duration-200',
              activeCategory === cat.id
                ? 'border-blue-500/50 bg-blue-500/15 text-blue-300'
                : 'border-white/10 bg-white/4 text-gray-400 hover:border-white/20 hover:text-white'
            )}
          >
            {cat.label}
          </button>
        ))}
      </div>

      {/* Questions */}
      <div className="mt-8 space-y-3">
        <div className="mb-4 flex items-center gap-2">
          <Badge variant={current.badge}>{current.label}</Badge>
          <span className="text-xs text-gray-500">{current.questions.length} questions</span>
        </div>
        {current.questions.map((item) => (
          <FAQItem key={item.q} q={item.q} a={item.a} />
        ))}
      </div>

      {/* Contact CTA */}
      <div className="mt-16 rounded-2xl border border-blue-500/20 bg-blue-500/6 p-8 text-center">
        <h3 className="text-lg font-semibold text-white">Still have questions?</h3>
        <p className="mt-2 text-sm text-gray-400">
          If you did not find what you were looking for, reach out directly. We respond to all
          substantive inquiries about data accuracy, methodology, and collaboration.
        </p>
        <a
          href="mailto:santht@proton.me"
          className="mt-6 inline-flex items-center gap-2 rounded-xl bg-blue-500 px-5 py-2.5 text-sm font-medium text-white shadow-lg shadow-blue-500/25 transition-all duration-200 hover:bg-blue-400"
        >
          <Mail className="h-4 w-4" />
          santht@proton.me
        </a>
      </div>
    </div>
  );
}
