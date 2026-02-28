import { Mail } from 'lucide-react';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent } from '@/components/ui/Card';

export const metadata = {
  title: 'Privacy Policy — Nonfaction',
  description: 'How Nonfaction handles visitor data, operational logs, and your privacy rights.',
};

const sections = [
  {
    id: 'collect',
    title: 'Information We Collect',
    content: [
      {
        subtitle: 'Operational logs',
        text: 'Our servers automatically record basic technical information for security and reliability: IP addresses, request timestamps, HTTP methods, URLs accessed, response codes, and user-agent strings. These logs are retained for up to 90 days and are used exclusively for debugging, abuse prevention, and infrastructure monitoring.',
      },
      {
        subtitle: 'Submission data',
        text: 'When you submit a record through the Submit interface, we collect the content of your submission, the primary source URL you provide, and optionally your contact email if you choose to provide it for follow-up. Submission content is retained indefinitely as part of the data provenance chain.',
      },
      {
        subtitle: 'Contact inquiries',
        text: 'If you contact us via email or the contact form, we retain your message and contact information for correspondence purposes. We do not add you to any mailing list without explicit consent.',
      },
      {
        subtitle: 'What we do not collect',
        text: 'We do not run advertising trackers, behavioral fingerprinting scripts, or cross-site analytics. We do not collect payment information. We do not build visitor profiles. We do not use cookies for tracking — only session-level technical cookies where strictly necessary for form functionality.',
      },
    ],
  },
  {
    id: 'use',
    title: 'How We Use Information',
    content: [
      {
        subtitle: 'Infrastructure and security',
        text: 'Operational logs are used to identify and block abusive traffic, diagnose technical errors, and maintain platform reliability. They are not used for marketing, profiling, or any commercial purpose.',
      },
      {
        subtitle: 'Data verification',
        text: 'Submission content — including the contact email you optionally provide — may be used to follow up on data questions, request clarification, or notify you of the disposition of your submission.',
      },
      {
        subtitle: 'Research and improvement',
        text: 'Aggregated, anonymized usage patterns (page view counts, API call volumes, search term frequencies) may be used to prioritize development work and understand which features are most valuable to users.',
      },
      {
        subtitle: 'Legal compliance',
        text: 'We may retain or disclose information where required by applicable law, court order, or to protect the rights and safety of the project or its users.',
      },
    ],
  },
  {
    id: 'security',
    title: 'Data Security',
    content: [
      {
        subtitle: 'Technical measures',
        text: 'All data in transit is encrypted using TLS 1.3 or higher. Operational databases are encrypted at rest. Access to production systems is restricted to authorized contributors with hardware-key two-factor authentication.',
      },
      {
        subtitle: 'Data minimization',
        text: 'We collect only what is necessary for the stated purposes. Operational logs are automatically purged after 90 days. We do not retain data beyond its operational purpose.',
      },
      {
        subtitle: 'Breach notification',
        text: 'In the event of a security breach affecting personal data, we will notify affected individuals within 72 hours of becoming aware of the breach, consistent with applicable regulations.',
      },
    ],
  },
  {
    id: 'rights',
    title: 'Your Rights',
    content: [
      {
        subtitle: 'Access and portability',
        text: 'You may request a copy of any personal data we hold about you. Requests will be fulfilled within 30 days. Contact us at santht@proton.me with "Data Access Request" in the subject line.',
      },
      {
        subtitle: 'Correction',
        text: 'If you believe we hold inaccurate personal information about you, you may request correction. We will review and respond within 30 days.',
      },
      {
        subtitle: 'Deletion',
        text: 'You may request deletion of personal data we hold about you, subject to legal retention requirements and data provenance obligations for submitted records. We cannot delete submission records that form part of the public provenance chain for published data.',
      },
      {
        subtitle: 'Objection',
        text: 'You may object to specific processing of your data. We will review all objections and respond with our assessment and any action taken.',
      },
    ],
  },
];

export default function PrivacyPage() {
  return (
    <div className="mx-auto max-w-4xl px-4 py-16 sm:px-6">
      {/* Header */}
      <Badge variant="blue" className="mb-6">Legal</Badge>
      <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl">
        Privacy Policy
      </h1>
      <p className="mt-4 text-sm text-gray-500">
        Effective date: February 2026 — We may update this policy as the platform evolves.
        Updates are posted here with a new effective date.
      </p>
      <p className="mt-6 max-w-2xl text-base leading-relaxed text-gray-400">
        Nonfaction is a public records platform. We are committed to collecting the minimum data
        necessary, being transparent about how it is used, and never monetizing visitor
        information.
      </p>

      {/* Sections */}
      <div className="mt-12 space-y-10">
        {sections.map((section) => (
          <section key={section.id}>
            <h2 className="text-xl font-semibold text-white">{section.title}</h2>
            <div className="mt-4 space-y-4">
              {section.content.map((item) => (
                <Card key={item.subtitle}>
                  <CardContent className="p-5">
                    <p className="text-sm font-semibold text-white">{item.subtitle}</p>
                    <p className="mt-2 text-sm leading-relaxed text-gray-400">{item.text}</p>
                  </CardContent>
                </Card>
              ))}
            </div>
          </section>
        ))}
      </div>

      {/* Third parties */}
      <section className="mt-10">
        <h2 className="text-xl font-semibold text-white">Third-Party Services</h2>
        <Card className="mt-4">
          <CardContent className="p-5">
            <p className="text-sm leading-relaxed text-gray-400">
              Nonfaction does not sell, rent, or share personal data with advertising networks
              or data brokers. We may use limited third-party infrastructure services (hosting,
              CDN, DNS) that process technical data as data processors under appropriate
              contractual protections. We do not integrate social media tracking pixels or
              behavioral advertising networks of any kind.
            </p>
          </CardContent>
        </Card>
      </section>

      {/* Contact */}
      <section className="mt-16">
        <div className="rounded-2xl border border-blue-500/20 bg-blue-500/6 p-8">
          <h3 className="text-lg font-semibold text-white">Privacy questions</h3>
          <p className="mt-2 text-sm text-gray-400">
            For any privacy-related requests, concerns, or questions, contact us directly.
            Include a clear description of your request and we will respond within 30 days.
          </p>
          <a
            href="mailto:santht@proton.me"
            className="mt-6 inline-flex items-center gap-2 rounded-xl bg-blue-500 px-5 py-2.5 text-sm font-medium text-white shadow-lg shadow-blue-500/25 transition-all duration-200 hover:bg-blue-400"
          >
            <Mail className="h-4 w-4" />
            santht@proton.me
          </a>
        </div>
      </section>
    </div>
  );
}
