'use client';

import { useState } from 'react';
import { Github, Mail, MapPin, Clock, Send, CheckCircle2 } from 'lucide-react';
import { Badge } from '@/components/ui/Badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Button } from '@/components/ui/Button';
import { Input } from '@/components/ui/Input';
import { Textarea } from '@/components/ui/Textarea';

const subjects = [
  'Data correction request',
  'Methodology question',
  'Collaboration inquiry',
  'Press / media inquiry',
  'Bug report',
  'API access request',
  'General question',
  'Other',
];

export default function ContactPage() {
  const [form, setForm] = useState({ name: '', email: '', subject: '', message: '' });
  const [sent, setSent] = useState(false);
  const [sending, setSending] = useState(false);

  function handleChange(e: React.ChangeEvent<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>) {
    setForm((f) => ({ ...f, [e.target.name]: e.target.value }));
  }

  function handleSubmit(e: React.FormEvent) {
    e.preventDefault();
    setSending(true);
    // Simulate async send
    setTimeout(() => {
      setSending(false);
      setSent(true);
    }, 1200);
  }

  return (
    <div className="mx-auto max-w-4xl px-4 py-16 sm:px-6">
      {/* Header */}
      <Badge variant="blue" className="mb-6">Contact</Badge>
      <h1 className="text-4xl font-bold tracking-tight text-white sm:text-5xl">
        Get in{' '}
        <span className="bg-gradient-to-r from-blue-400 to-blue-600 bg-clip-text text-transparent">
          touch
        </span>
      </h1>
      <p className="mt-6 max-w-2xl text-lg leading-relaxed text-gray-400">
        Questions about data accuracy, methodology, API access, collaboration, or press inquiries
        — we read everything and respond to all substantive messages.
      </p>

      <div className="mt-12 grid gap-8 lg:grid-cols-5">
        {/* Contact form */}
        <div className="lg:col-span-3">
          <Card>
            <CardHeader>
              <CardTitle>Send a message</CardTitle>
            </CardHeader>
            <CardContent>
              {sent ? (
                <div className="flex flex-col items-center py-8 text-center">
                  <div className="flex h-14 w-14 items-center justify-center rounded-full bg-green-500/15 ring-1 ring-green-500/30">
                    <CheckCircle2 className="h-7 w-7 text-green-400" />
                  </div>
                  <h3 className="mt-4 text-lg font-semibold text-white">Message received</h3>
                  <p className="mt-2 text-sm text-gray-400">
                    Thank you for reaching out. We typically respond within 48–72 hours for
                    substantive inquiries.
                  </p>
                  <button
                    type="button"
                    onClick={() => { setSent(false); setForm({ name: '', email: '', subject: '', message: '' }); }}
                    className="mt-6 text-sm text-blue-400 hover:text-blue-300 transition-colors"
                  >
                    Send another message
                  </button>
                </div>
              ) : (
                <form onSubmit={handleSubmit} className="space-y-4">
                  <div className="grid gap-4 sm:grid-cols-2">
                    <div>
                      <label className="mb-1.5 block text-xs font-medium text-gray-400">
                        Name
                      </label>
                      <Input
                        name="name"
                        value={form.name}
                        onChange={handleChange}
                        placeholder="Your name"
                        required
                      />
                    </div>
                    <div>
                      <label className="mb-1.5 block text-xs font-medium text-gray-400">
                        Email
                      </label>
                      <Input
                        type="email"
                        name="email"
                        value={form.email}
                        onChange={handleChange}
                        placeholder="your@email.com"
                        required
                      />
                    </div>
                  </div>
                  <div>
                    <label className="mb-1.5 block text-xs font-medium text-gray-400">
                      Subject
                    </label>
                    <select
                      name="subject"
                      value={form.subject}
                      onChange={handleChange}
                      required
                      className="w-full rounded-xl border border-white/12 bg-white/6 px-4 py-2.5 text-sm text-white transition-all duration-200 focus:border-blue-500/50 focus:outline-none focus:ring-2 focus:ring-blue-500/40"
                    >
                      <option value="" disabled className="bg-[#0A0F1C] text-gray-400">
                        Select a subject
                      </option>
                      {subjects.map((s) => (
                        <option key={s} value={s} className="bg-[#0A0F1C] text-white">
                          {s}
                        </option>
                      ))}
                    </select>
                  </div>
                  <div>
                    <label className="mb-1.5 block text-xs font-medium text-gray-400">
                      Message
                    </label>
                    <Textarea
                      name="message"
                      value={form.message}
                      onChange={handleChange}
                      placeholder="Describe your question or request in detail. For data corrections, include the record identifier and source URL."
                      rows={6}
                      required
                    />
                  </div>
                  <Button
                    type="submit"
                    disabled={sending}
                    size="lg"
                    className="w-full"
                  >
                    {sending ? (
                      <>
                        <span className="h-4 w-4 animate-spin rounded-full border-2 border-white/30 border-t-white" />
                        Sending…
                      </>
                    ) : (
                      <>
                        <Send className="h-4 w-4" />
                        Send message
                      </>
                    )}
                  </Button>
                  <p className="text-center text-xs text-gray-500">
                    Or email directly:{' '}
                    <a href="mailto:santht@proton.me" className="text-blue-400 hover:text-blue-300">
                      santht@proton.me
                    </a>
                  </p>
                </form>
              )}
            </CardContent>
          </Card>
        </div>

        {/* Sidebar */}
        <div className="space-y-4 lg:col-span-2">
          {/* Email */}
          <Card hover>
            <CardContent className="p-5">
              <div className="flex items-start gap-3">
                <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-blue-500/15 ring-1 ring-blue-500/30">
                  <Mail className="h-4.5 w-4.5 text-blue-400" />
                </div>
                <div>
                  <p className="text-xs font-semibold uppercase tracking-widest text-gray-500">Email</p>
                  <a
                    href="mailto:santht@proton.me"
                    className="mt-0.5 block text-sm font-medium text-blue-400 hover:text-blue-300 transition-colors"
                  >
                    santht@proton.me
                  </a>
                  <p className="mt-1 text-xs text-gray-500">
                    Encrypted with ProtonMail. PGP key available on request.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* GitHub */}
          <Card hover>
            <CardContent className="p-5">
              <div className="flex items-start gap-3">
                <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-white/8 ring-1 ring-white/12">
                  <Github className="h-4.5 w-4.5 text-white" />
                </div>
                <div>
                  <p className="text-xs font-semibold uppercase tracking-widest text-gray-500">GitHub</p>
                  <a
                    href="https://github.com/santht/Nonfaction"
                    target="_blank"
                    rel="noopener noreferrer"
                    className="mt-0.5 block text-sm font-medium text-blue-400 hover:text-blue-300 transition-colors"
                  >
                    github.com/santht/Nonfaction
                  </a>
                  <p className="mt-1 text-xs text-gray-500">
                    Open issues, PRs, and discussions welcome.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Location */}
          <Card>
            <CardContent className="p-5">
              <div className="flex items-start gap-3">
                <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-white/8 ring-1 ring-white/12">
                  <MapPin className="h-4.5 w-4.5 text-gray-400" />
                </div>
                <div>
                  <p className="text-xs font-semibold uppercase tracking-widest text-gray-500">Location</p>
                  <p className="mt-0.5 text-sm font-medium text-white">Remote / Open Source</p>
                  <p className="mt-1 text-xs text-gray-500">
                    Nonfaction is a distributed project. Contributors work remotely across timezones.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Response time */}
          <Card>
            <CardContent className="p-5">
              <div className="flex items-start gap-3">
                <div className="flex h-9 w-9 shrink-0 items-center justify-center rounded-lg bg-white/8 ring-1 ring-white/12">
                  <Clock className="h-4.5 w-4.5 text-gray-400" />
                </div>
                <div>
                  <p className="text-xs font-semibold uppercase tracking-widest text-gray-500">
                    Response time
                  </p>
                  <p className="mt-0.5 text-sm font-medium text-white">48–72 hours</p>
                  <p className="mt-1 text-xs text-gray-500">
                    For data corrections and substantive methodology questions. General inquiries
                    may take longer.
                  </p>
                </div>
              </div>
            </CardContent>
          </Card>

          {/* What to include */}
          <div className="rounded-xl border border-white/8 bg-white/3 p-4">
            <p className="text-xs font-semibold text-white">For data corrections, include:</p>
            <ul className="mt-2 space-y-1 text-xs text-gray-400">
              {[
                'Record identifier or URL',
                'Specific field with the error',
                'Correct value and primary source URL',
                'Filing ID or case number if applicable',
              ].map((item) => (
                <li key={item} className="flex items-start gap-2">
                  <span className="mt-1 h-1 w-1 shrink-0 rounded-full bg-blue-500" />
                  {item}
                </li>
              ))}
            </ul>
          </div>
        </div>
      </div>
    </div>
  );
}
