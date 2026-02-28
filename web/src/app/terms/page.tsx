import { Mail } from 'lucide-react';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent } from '@/components/ui/Card';

export const metadata = {
  title: 'Terms of Service — Nonfaction',
  description: 'Terms governing use of the Nonfaction platform, data, and API.',
};

export default function TermsPage() {
  return (
    <div className="mx-auto max-w-4xl px-4 py-16 sm:px-6">
      {/* Header */}
      <Badge variant="blue" className="mb-6">Legal</Badge>
      <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl">
        Terms of Service
      </h1>
      <p className="mt-4 text-sm text-gray-500">
        Effective date: February 2026 — These terms apply to all use of the Nonfaction platform,
        API, and data.
      </p>
      <p className="mt-6 max-w-2xl text-base leading-relaxed text-gray-400">
        Please read these terms carefully. By using Nonfaction, you agree to be bound by them.
        If you disagree with any part, you may not use the platform.
      </p>

      <div className="mt-12 space-y-10">

        {/* Acceptance */}
        <section>
          <h2 className="text-xl font-semibold text-white">1. Acceptance of Terms</h2>
          <Card className="mt-4">
            <CardContent className="p-5">
              <p className="text-sm leading-relaxed text-gray-400">
                By accessing or using the Nonfaction platform, website, or API (collectively, the
                "Service"), you agree to be bound by these Terms of Service and our Privacy Policy.
                If you are using the Service on behalf of an organization, you represent that you
                have authority to bind that organization to these terms. These terms may be updated
                periodically — continued use of the Service after changes constitutes acceptance
                of the revised terms.
              </p>
            </CardContent>
          </Card>
        </section>

        {/* Use of Service */}
        <section>
          <h2 className="text-xl font-semibold text-white">2. Use of Service</h2>
          <div className="mt-4 space-y-3">
            <Card>
              <CardContent className="p-5">
                <p className="text-sm font-semibold text-white">Permitted uses</p>
                <p className="mt-2 text-sm leading-relaxed text-gray-400">
                  The Service is provided for informational, research, journalistic, and civic
                  education purposes. You may access, search, and export data for these purposes
                  subject to applicable license terms and rate limits.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <p className="text-sm font-semibold text-white">Prohibited uses</p>
                <ul className="mt-2 space-y-1.5 text-sm text-gray-400">
                  {[
                    'Uploading unlawful content, defamatory material, or non-public personal data without legal basis',
                    'Attempting to circumvent rate limits, access controls, or security measures',
                    'Using the platform or data to harass, threaten, or dox any individual',
                    'Uploading malicious code, scripts, or payloads of any kind',
                    'Presenting Nonfaction data as your own without appropriate attribution',
                    'Using the Service in ways that violate applicable law or regulation',
                    'Automated scraping that exceeds documented rate limits without prior approval',
                  ].map((item) => (
                    <li key={item} className="flex items-start gap-2">
                      <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-red-500" />
                      {item}
                    </li>
                  ))}
                </ul>
              </CardContent>
            </Card>
          </div>
        </section>

        {/* Data Accuracy */}
        <section>
          <h2 className="text-xl font-semibold text-white">3. Data Accuracy and Corrections</h2>
          <Card className="mt-4">
            <CardContent className="p-5">
              <p className="text-sm leading-relaxed text-gray-400">
                Nonfaction makes every reasonable effort to ensure data accuracy but cannot
                guarantee the completeness or currency of all records. The Service is provided as
                an aggregation of public records — errors may exist due to source data quality,
                processing errors, or outdated information. If you identify an error, please
                report it. We investigate and correct substantiated errors promptly. Published
                corrections include a correction notice and timestamp in the affected record.
              </p>
            </CardContent>
          </Card>
        </section>

        {/* Contributing */}
        <section>
          <h2 className="text-xl font-semibold text-white">4. Contributing Data</h2>
          <Card className="mt-4">
            <CardContent className="p-5 space-y-3">
              <p className="text-sm leading-relaxed text-gray-400">
                By submitting a record to Nonfaction, you represent and warrant that:
              </p>
              <ul className="space-y-1.5 text-sm text-gray-400">
                {[
                  'The submission is factually accurate to the best of your knowledge',
                  'The submission is supported by a verifiable primary source',
                  'The content is legally shareable and does not infringe third-party rights',
                  'You are not submitting material you know to be false or misleading',
                  'The submission does not contain non-public personal data of private individuals',
                ].map((item) => (
                  <li key={item} className="flex items-start gap-2">
                    <span className="mt-1.5 h-1.5 w-1.5 shrink-0 rounded-full bg-blue-500" />
                    {item}
                  </li>
                ))}
              </ul>
              <p className="text-sm leading-relaxed text-gray-400">
                By submitting, you grant Nonfaction a perpetual, irrevocable, worldwide license
                to use, publish, and distribute the submitted data under the CC-BY-SA 4.0 license.
                False submissions made in bad faith may result in permanent suspension of
                submission privileges.
              </p>
            </CardContent>
          </Card>
        </section>

        {/* Intellectual Property */}
        <section>
          <h2 className="text-xl font-semibold text-white">5. Intellectual Property</h2>
          <div className="mt-4 grid gap-3 sm:grid-cols-2">
            <Card>
              <CardContent className="p-5">
                <Badge variant="blue" className="mb-3">Source code</Badge>
                <p className="text-sm font-semibold text-white">GNU GPL v3</p>
                <p className="mt-2 text-sm leading-relaxed text-gray-400">
                  All Nonfaction platform source code is licensed under the GNU General Public
                  License v3. You are free to use, study, modify, and distribute the code under
                  the GPL v3 terms. Derivative works must remain open source.
                </p>
              </CardContent>
            </Card>
            <Card>
              <CardContent className="p-5">
                <Badge variant="green" className="mb-3">Data</Badge>
                <p className="text-sm font-semibold text-white">CC-BY-SA 4.0</p>
                <p className="mt-2 text-sm leading-relaxed text-gray-400">
                  Nonfaction&apos;s curated datasets are published under the Creative Commons
                  Attribution-ShareAlike 4.0 International license. You may freely use and
                  redistribute the data with attribution. Derivative datasets must use the
                  same license.
                </p>
              </CardContent>
            </Card>
          </div>
          <p className="mt-4 text-sm text-gray-500">
            Third-party source material (government documents, court filings, etc.) remains
            the property of the originating institutions and is governed by applicable
            public records law.
          </p>
        </section>

        {/* Disclaimers */}
        <section>
          <h2 className="text-xl font-semibold text-white">6. Disclaimers and Limitation of Liability</h2>
          <Card className="mt-4">
            <CardContent className="p-5 space-y-3">
              <p className="text-sm leading-relaxed text-gray-400">
                <span className="font-semibold text-white">As-is basis:</span> The Service is
                provided "as is" and "as available" without warranties of any kind, express or
                implied, including warranties of merchantability, fitness for a particular
                purpose, or accuracy.
              </p>
              <p className="text-sm leading-relaxed text-gray-400">
                <span className="font-semibold text-white">Not legal or professional advice:</span> Nothing
                on Nonfaction constitutes legal, financial, political, or professional advice.
                Data is provided for informational purposes only.
              </p>
              <p className="text-sm leading-relaxed text-gray-400">
                <span className="font-semibold text-white">Limitation of liability:</span> To the
                maximum extent permitted by applicable law, Nonfaction and its contributors shall
                not be liable for any indirect, incidental, special, or consequential damages
                arising from your use of the Service.
              </p>
              <p className="text-sm leading-relaxed text-gray-400">
                <span className="font-semibold text-white">Correlation ≠ causation:</span> Timing
                analysis and relationship data presented on this platform are descriptive
                analytical outputs. They do not constitute allegations of wrongdoing, corruption,
                or illegal activity.
              </p>
            </CardContent>
          </Card>
        </section>

        {/* Governing Law */}
        <section>
          <h2 className="text-xl font-semibold text-white">7. Governing Law</h2>
          <Card className="mt-4">
            <CardContent className="p-5">
              <p className="text-sm leading-relaxed text-gray-400">
                These terms are governed by applicable law. Disputes arising from use of the
                Service will be resolved through good-faith negotiation where possible. If you
                have a dispute, please contact us at santht@proton.me before pursuing formal
                legal action.
              </p>
            </CardContent>
          </Card>
        </section>

      </div>

      {/* Contact */}
      <section className="mt-16">
        <div className="rounded-2xl border border-blue-500/20 bg-blue-500/6 p-8">
          <h3 className="text-lg font-semibold text-white">Questions about these terms</h3>
          <p className="mt-2 text-sm text-gray-400">
            If you have questions about these terms or need clarification on what is permitted,
            reach out directly. We will respond to all substantive legal inquiries.
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
