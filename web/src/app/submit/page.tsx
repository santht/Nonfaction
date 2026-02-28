'use client';

import { useMemo, useState } from 'react';
import { AlertCircle, CheckCircle2, Mail } from 'lucide-react';
import { submitConnection, type SubmissionData } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { ProgressBar } from '@/components/ui/ProgressBar';
import { Select } from '@/components/ui/Select';
import { Textarea } from '@/components/ui/Textarea';
import { SourceNote } from '@/components/ui/SourceNote';

const tiers = [
  { name: 'Bronze', requirement: '1-10 validated submissions', color: 'yellow' as const },
  { name: 'Silver', requirement: '11-50 validated submissions', color: 'blue' as const },
  { name: 'Gold', requirement: '51-150 validated submissions', color: 'green' as const },
  { name: 'Platinum', requirement: '151+ validated submissions', color: 'red' as const },
];

const connectionTypes = [
  'Financial — PAC / Campaign Donation',
  'Financial — Direct Payment',
  'Meeting / Correspondence',
  'Vote / Legislative Action',
  'Appointment / Personnel',
  'Regulatory Action',
  'Contract / Award',
  'Other',
];

function sanitize(input: string) {
  return input.replace(/[<>]/g, '').trim();
}

export default function SubmitPage() {
  const [step, setStep] = useState(1);
  const [form, setForm] = useState<SubmissionData>({
    entityName: '',
    connectionType: '',
    description: '',
    sourceUrl: '',
    referenceDetail: '',
    submitterNote: '',
  });
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<{ success: boolean; id?: string; error?: string } | null>(null);

  const progress = useMemo(() => Math.round((step / 3) * 100), [step]);

  function updateField<K extends keyof SubmissionData>(key: K, value: SubmissionData[K]) {
    setForm((prev) => ({ ...prev, [key]: value }));
  }

  function isStepValid() {
    if (step === 1) return sanitize(form.entityName).length > 0 && sanitize(form.connectionType).length > 0;
    if (step === 2) return sanitize(form.description).length >= 20;
    return sanitize(form.sourceUrl).length > 0 && sanitize(form.referenceDetail).length > 8;
  }

  async function onSubmit() {
    setSubmitting(true);
    setResult(null);

    try {
      const cleanData: SubmissionData = {
        entityName: sanitize(form.entityName),
        connectionType: sanitize(form.connectionType),
        description: sanitize(form.description),
        sourceUrl: sanitize(form.sourceUrl),
        referenceDetail: sanitize(form.referenceDetail),
        submitterNote: sanitize(form.submitterNote || ''),
      };

      if (!/^https?:\/\//.test(cleanData.sourceUrl)) {
        setResult({ success: false, error: 'Source URL must start with http:// or https://.' });
      } else {
        const response = await submitConnection(cleanData);
        setResult(response);
        if (response.success) {
          setStep(1);
          setForm({
            entityName: '',
            connectionType: '',
            description: '',
            sourceUrl: '',
            referenceDetail: '',
            submitterNote: '',
          });
        }
      }
    } catch {
      setResult({ success: false, error: 'Submission failed. Retry in a moment.' });
    } finally {
      setSubmitting(false);
    }
  }

  return (
    <div className="mx-auto max-w-4xl px-4 py-10 sm:px-6">
      <div className="mb-8">
        <h1 className="text-3xl font-semibold text-white">Submit a Connection</h1>
        <p className="mt-1 text-sm text-gray-400">
          Contributor submissions must be source-based, factual, and verifiable. Contact for review support: santht@proton.me.
        </p>
      </div>

      <Card className="mb-6">
        <CardContent className="p-5">
          <div className="mb-3 flex items-center justify-between text-sm">
            <span className="text-gray-300">Submission wizard</span>
            <span className="text-gray-400">Step {step} of 3</span>
          </div>
          <ProgressBar value={progress} />
        </CardContent>
      </Card>

      <div className="mb-6 grid gap-3 sm:grid-cols-2 lg:grid-cols-4">
        {tiers.map((tier) => (
          <Card key={tier.name}>
            <CardContent className="p-4">
              <Badge variant={tier.color}>{tier.name}</Badge>
              <p className="mt-2 text-xs text-gray-400">{tier.requirement}</p>
            </CardContent>
          </Card>
        ))}
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="text-base">Contributor guidelines</CardTitle>
        </CardHeader>
        <CardContent className="space-y-2 text-sm text-gray-300">
          <p>1. Use primary records when possible (filings, dockets, vote logs).</p>
          <p>2. Include identifiers (filing ID, case number, bill number, docket link).</p>
          <p>3. Describe facts only, no commentary or speculative claims.</p>
          <p>4. Do not submit private or personally sensitive non-public data.</p>
        </CardContent>
      </Card>

      <Card className="mt-6">
        <CardContent className="space-y-4 p-5">
          {step === 1 ? (
            <>
              <Input
                value={form.entityName}
                onChange={(e) => updateField('entityName', e.target.value)}
                placeholder="Entity name"
              />
              <Select
                value={form.connectionType}
                onChange={(e) => updateField('connectionType', e.target.value)}
              >
                <option value="" className="bg-[#0a0f1c]">Select connection type</option>
                {connectionTypes.map((item) => (
                  <option key={item} value={item} className="bg-[#0a0f1c]">
                    {item}
                  </option>
                ))}
              </Select>
            </>
          ) : null}

          {step === 2 ? (
            <Textarea
              rows={6}
              value={form.description}
              onChange={(e) => updateField('description', e.target.value)}
              placeholder="Describe the connection factually with amounts, dates, and actions."
            />
          ) : null}

          {step === 3 ? (
            <>
              <Input
                type="url"
                value={form.sourceUrl}
                onChange={(e) => updateField('sourceUrl', e.target.value)}
                placeholder="https://source-url"
              />
              <Input
                value={form.referenceDetail}
                onChange={(e) => updateField('referenceDetail', e.target.value)}
                placeholder="Reference detail (filing id, case number, etc.)"
              />
              <Textarea
                rows={3}
                value={form.submitterNote}
                onChange={(e) => updateField('submitterNote', e.target.value)}
                placeholder="Optional reviewer note"
              />
            </>
          ) : null}

          <div className="flex flex-wrap items-center gap-2">
            <Button variant="ghost" disabled={step === 1} onClick={() => setStep((curr) => Math.max(1, curr - 1))}>
              Back
            </Button>
            {step < 3 ? (
              <Button disabled={!isStepValid()} onClick={() => setStep((curr) => Math.min(3, curr + 1))}>
                Continue
              </Button>
            ) : (
              <Button disabled={!isStepValid() || submitting} onClick={onSubmit}>
                {submitting ? 'Submitting...' : 'Submit for review'}
              </Button>
            )}
          </div>

          {result?.success ? (
            <div className="rounded-xl border border-green-500/30 bg-green-500/10 p-3 text-sm text-green-200">
              <CheckCircle2 className="mr-2 inline h-4 w-4" />
              Submission accepted. Reference ID: {result.id}
            </div>
          ) : null}

          {result && !result.success ? (
            <div className="rounded-xl border border-red-500/30 bg-red-500/10 p-3 text-sm text-red-200">
              <AlertCircle className="mr-2 inline h-4 w-4" />
              {result.error}
            </div>
          ) : null}
        </CardContent>
      </Card>

      <SourceNote text="Source attribution required: accepted references include FEC, Congress.gov, Senate LDA, court records, and major publication archives." />

      <a href="mailto:santht@proton.me" className="mt-5 inline-flex items-center gap-2 text-sm text-blue-300 hover:text-blue-200">
        <Mail className="h-4 w-4" /> Need help with submission quality checks? santht@proton.me
      </a>
    </div>
  );
}
