'use client';

import { useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import { ChevronDown, ExternalLink, RefreshCcw } from 'lucide-react';
import { getConductRows, type ConductRow } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/Card';
import { Select } from '@/components/ui/Select';
import { Skeleton } from '@/components/ui/Skeleton';
import { SourceNote } from '@/components/ui/SourceNote';

export default function ConductPage() {
  const [rows, setRows] = useState<ConductRow[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [expanded, setExpanded] = useState<string | null>(null);
  const [official, setOfficial] = useState('all');

  async function loadRows() {
    setLoading(true);
    setError(null);
    try {
      const data = await getConductRows();
      setRows(data);
    } catch {
      setError('Unable to load conduct records.');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadRows();
  }, []);

  const officials = useMemo(() => ['all', ...new Set(rows.map((row) => row.official))], [rows]);

  const filtered = useMemo(
    () => rows.filter((row) => (official === 'all' ? true : row.official === official)),
    [rows, official]
  );

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <div className="mb-8 flex flex-col gap-4 sm:flex-row sm:items-end sm:justify-between">
        <div>
          <h1 className="text-3xl font-semibold text-white">Conduct Comparison</h1>
          <p className="mt-1 max-w-3xl text-sm text-gray-400">
            Documented public actions compared with equivalent private-sector conduct contexts.
          </p>
        </div>
        <div className="w-full max-w-xs">
          <label className="mb-1 block text-xs text-gray-400">Filter by official</label>
          <Select value={official} onChange={(e) => setOfficial(e.target.value)}>
            {officials.map((item) => (
              <option key={item} value={item} className="bg-[#0a0f1c]">
                {item === 'all' ? 'All officials' : item}
              </option>
            ))}
          </Select>
        </div>
      </div>

      {error ? (
        <Card className="mb-4 border-red-500/35 bg-red-500/10">
          <CardContent className="flex items-center justify-between gap-3 p-4">
            <p className="text-sm text-red-200">{error}</p>
            <Button variant="outline" size="sm" onClick={loadRows}>
              <RefreshCcw className="h-3.5 w-3.5" /> Retry
            </Button>
          </CardContent>
        </Card>
      ) : null}

      {loading ? (
        <div className="space-y-3">
          {Array.from({ length: 4 }).map((_, idx) => (
            <Skeleton key={idx} className="h-20" />
          ))}
        </div>
      ) : (
        <div className="space-y-3">
          {filtered.map((row) => {
            const isOpen = expanded === row.id;
            return (
              <Card key={row.id}>
                <button
                  type="button"
                  onClick={() => setExpanded((curr) => (curr === row.id ? null : row.id))}
                  className="w-full text-left"
                >
                  <CardHeader>
                    <div className="flex items-start justify-between gap-3">
                      <div>
                        <div className="mb-1 flex flex-wrap items-center gap-2">
                          <Badge variant="red">Flagged comparison</Badge>
                          <p className="text-xs text-gray-500">{row.date}</p>
                        </div>
                        <CardTitle className="text-base">{row.officialAction}</CardTitle>
                        <p className="mt-1 text-sm text-gray-400">Official: {row.official}</p>
                      </div>
                      <ChevronDown className={`h-4 w-4 text-gray-400 transition-transform ${isOpen ? 'rotate-180' : ''}`} />
                    </div>
                  </CardHeader>
                </button>
                {isOpen ? (
                  <CardContent className="space-y-3 pt-0">
                    <div className="rounded-xl border border-white/10 bg-white/3 p-3">
                      <p className="text-xs text-gray-500">Equivalent private conduct</p>
                      <p className="text-sm text-gray-200">{row.equivalentPrivateConduct}</p>
                    </div>
                    <div className="rounded-xl border border-white/10 bg-white/3 p-3">
                      <p className="text-xs text-gray-500">Observed consequence</p>
                      <p className="text-sm text-gray-200">{row.consequence}</p>
                    </div>
                    <div className="flex flex-wrap items-center justify-between gap-2 text-sm">
                      <Link href={`/entity/${row.officialId}`} className="text-blue-300 hover:text-blue-200">
                        Open entity profile
                      </Link>
                      <a
                        href={row.source.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="inline-flex items-center gap-1 text-blue-300 hover:text-blue-200"
                      >
                        <ExternalLink className="h-3.5 w-3.5" />
                        {row.source.publisher}
                      </a>
                    </div>
                  </CardContent>
                ) : null}
              </Card>
            );
          })}
        </div>
      )}

      <SourceNote text="Source attribution: official filings, vote records, and legal/public disclosures." />
    </div>
  );
}
