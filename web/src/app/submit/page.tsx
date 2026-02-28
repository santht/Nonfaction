'use client';

import { useState } from 'react';
import { CheckCircle, AlertCircle, ExternalLink, Info } from 'lucide-react';
import { submitConnection, type SubmissionData } from '@/lib/api';
import { Button } from '@/components/ui/Button';
import { Card, CardContent } from '@/components/ui/Card';
import { Badge } from '@/components/ui/Badge';

const CONNECTION_TYPES = [
  'Financial — PAC / Campaign Donation',
  'Financial — Direct Payment',
  'Meeting / Correspondence',
  'Vote / Legislative Action',
  'Appointment / Personnel',
  'Regulatory Action',
  'Contract / Award',
  'Other',
];

interface FormState {
  entityName: string;
  connectionType: string;
  description: string;
  sourceUrl: string;
  referenceDetail: string;
  submitterNote: string;
}

interface FormErrors {
  entityName?: string;
  connectionType?: string;
  description?: string;
  sourceUrl?: string;
  referenceDetail?: string;
}

export default function SubmitPage() {
  const [form, setForm] = useState<FormState>({
    entityName: '',
    connectionType: '',
    description: '',
    sourceUrl: '',
    referenceDetail: '',
    submitterNote: '',
  });
  const [errors, setErrors] = useState<FormErrors>({});
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<{
    success: boolean;
    id?: string;
    error?: string;
  } | null>(null);

  function validate(): FormErrors {
    const e: FormErrors = {};
    if (!form.entityName.trim()) e.entityName = 'Entity name is required';
    if (!form.connectionType) e.connectionType = 'Connection type is required';
    if (!form.description.trim() || form.description.trim().length < 20)
      e.description = 'Please provide at least 20 characters describing the connection';
    if (!form.sourceUrl.trim()) {
      e.sourceUrl = 'A source URL is required — no anonymous submissions';
    } else {
      try {
        new URL(form.sourceUrl);
      } catch {
        e.sourceUrl = 'Please enter a valid URL (e.g. https://…)';
      }
    }
    if (!form.referenceDetail.trim() || form.referenceDetail.trim().length < 10)
      e.referenceDetail = 'Reference detail is required (e.g. filing ID, case number, article title)';
    return e;
  }

  function handleChange(
    e: React.ChangeEvent<
      HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement
    >
  ) {
    const { name, value } = e.target;
    setForm((f) => ({ ...f, [name]: value }));
    if (errors[name as keyof FormErrors]) {
      setErrors((err) => ({ ...err, [name]: undefined }));
    }
  }

  async function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    const errs = validate();
    if (Object.keys(errs).length > 0) {
      setErrors(errs);
      return;
    }

    setSubmitting(true);
    setResult(null);
    const data: SubmissionData = {
      entityName: form.entityName,
      connectionType: form.connectionType,
      description: form.description,
      sourceUrl: form.sourceUrl,
      referenceDetail: form.referenceDetail,
      submitterNote: form.submitterNote || undefined,
    };
    const res = await submitConnection(data);
    setResult(res);
    setSubmitting(false);

    if (res.success) {
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

  const inputClass =
    'w-full px-4 py-2.5 bg-white/6 border border-white/10 rounded-xl text-white placeholder-gray-600 text-sm focus:outline-none focus:ring-2 focus:ring-blue-500/40 focus:border-blue-500/40 transition-all';
  const errorInputClass =
    'w-full px-4 py-2.5 bg-red-500/5 border border-red-500/30 rounded-xl text-white placeholder-gray-600 text-sm focus:outline-none focus:ring-2 focus:ring-red-500/30 transition-all';
  const labelClass = 'block text-sm font-medium text-gray-300 mb-1.5';
  const errorClass = 'mt-1.5 flex items-center gap-1.5 text-xs text-red-400';

  return (
    <div className="max-w-2xl mx-auto px-4 sm:px-6 py-10">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-white mb-2">
          Submit a Connection
        </h1>
        <p className="text-gray-400">
          Help expand the database. Every submission requires a verifiable
          public source. No anonymous claims accepted.
        </p>
      </div>

      {/* Principles */}
      <div className="flex items-start gap-3 mb-8 p-4 rounded-xl bg-blue-500/5 border border-blue-500/10">
        <Info className="w-4 h-4 text-blue-400 shrink-0 mt-0.5" />
        <div className="text-sm text-blue-300 space-y-1">
          <p className="font-medium text-blue-200">Submission requirements:</p>
          <ul className="text-xs space-y-1 text-blue-300/80">
            <li>• Source URL must point to a public record, news article, or government database</li>
            <li>• Reference detail must include a specific identifier (filing ID, case number, etc.)</li>
            <li>• No editorializing — describe facts only</li>
            <li>• Submissions are reviewed before publishing</li>
          </ul>
        </div>
      </div>

      {/* Success */}
      {result?.success && (
        <div className="mb-6 flex items-start gap-3 p-4 rounded-xl bg-green-500/10 border border-green-500/20">
          <CheckCircle className="w-5 h-5 text-green-400 shrink-0 mt-0.5" />
          <div>
            <p className="text-sm font-semibold text-green-300">
              Submission received
            </p>
            <p className="text-xs text-green-400/80 mt-0.5">
              Reference ID: <code className="font-mono">{result.id}</code>.
              Your submission will be reviewed against the provided source
              before being added to the database.
            </p>
          </div>
        </div>
      )}

      {/* Error */}
      {result && !result.success && (
        <div className="mb-6 flex items-start gap-3 p-4 rounded-xl bg-red-500/10 border border-red-500/20">
          <AlertCircle className="w-5 h-5 text-red-400 shrink-0 mt-0.5" />
          <p className="text-sm text-red-300">{result.error}</p>
        </div>
      )}

      {/* Form */}
      <Card>
        <CardContent className="p-6">
          <form onSubmit={handleSubmit} noValidate className="space-y-6">
            {/* Entity name */}
            <div>
              <label className={labelClass} htmlFor="entityName">
                Entity Name{' '}
                <span className="text-red-400">*</span>
              </label>
              <input
                id="entityName"
                name="entityName"
                type="text"
                placeholder="e.g. Senator Jane Smith, PharmaCorp Industries"
                value={form.entityName}
                onChange={handleChange}
                className={errors.entityName ? errorInputClass : inputClass}
              />
              {errors.entityName && (
                <p className={errorClass}>
                  <AlertCircle className="w-3 h-3" />
                  {errors.entityName}
                </p>
              )}
            </div>

            {/* Connection type */}
            <div>
              <label className={labelClass} htmlFor="connectionType">
                Connection Type{' '}
                <span className="text-red-400">*</span>
              </label>
              <select
                id="connectionType"
                name="connectionType"
                value={form.connectionType}
                onChange={handleChange}
                className={`${errors.connectionType ? errorInputClass : inputClass} appearance-none`}
              >
                <option value="" className="bg-[#111827]">
                  Select a type…
                </option>
                {CONNECTION_TYPES.map((t) => (
                  <option key={t} value={t} className="bg-[#111827]">
                    {t}
                  </option>
                ))}
              </select>
              {errors.connectionType && (
                <p className={errorClass}>
                  <AlertCircle className="w-3 h-3" />
                  {errors.connectionType}
                </p>
              )}
            </div>

            {/* Description */}
            <div>
              <label className={labelClass} htmlFor="description">
                Description{' '}
                <span className="text-red-400">*</span>
              </label>
              <textarea
                id="description"
                name="description"
                rows={4}
                placeholder="Describe the connection factually. Do not editorialize. Include specific amounts, dates, and actions where known."
                value={form.description}
                onChange={handleChange}
                className={`${errors.description ? errorInputClass : inputClass} resize-none`}
              />
              <div className="flex items-center justify-between mt-1">
                {errors.description ? (
                  <p className={errorClass}>
                    <AlertCircle className="w-3 h-3" />
                    {errors.description}
                  </p>
                ) : (
                  <span />
                )}
                <span className="text-xs text-gray-600">
                  {form.description.length} chars
                </span>
              </div>
            </div>

            {/* Source URL */}
            <div>
              <label className={labelClass} htmlFor="sourceUrl">
                Source URL{' '}
                <span className="text-red-400">*</span>
                <span className="text-gray-600 font-normal ml-2 text-xs">
                  (required — no anonymous submissions)
                </span>
              </label>
              <div className="relative">
                <input
                  id="sourceUrl"
                  name="sourceUrl"
                  type="url"
                  placeholder="https://www.fec.gov/… or https://lda.senate.gov/…"
                  value={form.sourceUrl}
                  onChange={handleChange}
                  className={`${errors.sourceUrl ? errorInputClass : inputClass} pr-10`}
                />
                {form.sourceUrl && !errors.sourceUrl && (
                  <a
                    href={form.sourceUrl}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="absolute right-3 top-1/2 -translate-y-1/2 text-gray-500 hover:text-blue-400"
                  >
                    <ExternalLink className="w-3.5 h-3.5" />
                  </a>
                )}
              </div>
              {errors.sourceUrl ? (
                <p className={errorClass}>
                  <AlertCircle className="w-3 h-3" />
                  {errors.sourceUrl}
                </p>
              ) : (
                <p className="mt-1 text-xs text-gray-600">
                  Accepted: FEC.gov, Congress.gov, lda.senate.gov, courtlistener.com,
                  OpenSecrets, government .gov domains, major news organizations
                </p>
              )}
            </div>

            {/* Reference detail */}
            <div>
              <label className={labelClass} htmlFor="referenceDetail">
                Reference Detail{' '}
                <span className="text-red-400">*</span>
              </label>
              <input
                id="referenceDetail"
                name="referenceDetail"
                type="text"
                placeholder="e.g. FEC Filing ID C00123456, Case No. 24-cv-0091, Bill HR-2024-0189"
                value={form.referenceDetail}
                onChange={handleChange}
                className={errors.referenceDetail ? errorInputClass : inputClass}
              />
              {errors.referenceDetail && (
                <p className={errorClass}>
                  <AlertCircle className="w-3 h-3" />
                  {errors.referenceDetail}
                </p>
              )}
            </div>

            {/* Optional note */}
            <div>
              <label className={labelClass} htmlFor="submitterNote">
                Additional Context{' '}
                <Badge variant="outline" className="ml-1">
                  Optional
                </Badge>
              </label>
              <textarea
                id="submitterNote"
                name="submitterNote"
                rows={3}
                placeholder="Any additional context for reviewers (not published)"
                value={form.submitterNote}
                onChange={handleChange}
                className={`${inputClass} resize-none`}
              />
            </div>

            <Button
              type="submit"
              variant="primary"
              size="lg"
              className="w-full"
              disabled={submitting}
            >
              {submitting ? 'Submitting…' : 'Submit for Review'}
            </Button>
          </form>
        </CardContent>
      </Card>

      <p className="mt-4 text-xs text-gray-600 text-center">
        Submissions are verified against primary sources before publication.
        Fabricated submissions are rejected and logged.
      </p>
    </div>
  );
}
