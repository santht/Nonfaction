'use client';

import { Suspense, useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import { useRouter, useSearchParams } from 'next/navigation';
import { AlertTriangle, CalendarRange, DollarSign, Filter, RefreshCcw } from 'lucide-react';
import { searchEntities, type Entity, type SearchFilters } from '@/lib/api';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardContent } from '@/components/ui/Card';
import { Input } from '@/components/ui/Input';
import { Select } from '@/components/ui/Select';
import { Skeleton } from '@/components/ui/Skeleton';
import { EmptyState } from '@/components/ui/EmptyState';
import { SourceNote } from '@/components/ui/SourceNote';

const entityTypes: Entity['type'][] = ['politician', 'corporation', 'lobbyist', 'nonprofit', 'donor'];
const states = ['ALL', 'TX', 'CA', 'FL', 'NM', 'OH', 'IL'];

function SearchView() {
  const params = useSearchParams();
  const router = useRouter();

  const [query, setQuery] = useState(params.get('q') ?? '');
  const [results, setResults] = useState<Entity[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const [filters, setFilters] = useState<SearchFilters>({
    dateFrom: '',
    dateTo: '',
    minAmount: undefined,
    maxAmount: undefined,
    state: '',
    entityTypes: [],
    flaggedOnly: false,
  });

  const activeFilterCount = useMemo(() => {
    let count = 0;
    if (filters.entityTypes?.length) count += 1;
    if (filters.dateFrom || filters.dateTo) count += 1;
    if (filters.minAmount || filters.maxAmount) count += 1;
    if (filters.state) count += 1;
    if (filters.flaggedOnly) count += 1;
    return count;
  }, [filters]);

  async function runSearch(nextQuery = query, nextFilters = filters) {
    try {
      setError(null);
      setLoading(true);
      const sanitized = nextQuery.replace(/[<>]/g, '').trim();
      const data = await searchEntities(sanitized, nextFilters);
      setResults(data);
    } catch {
      setError('Unable to fetch results. Please retry.');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    const q = params.get('q') ?? '';
    setQuery(q);
    runSearch(q, filters);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [params]);

  function submitSearch() {
    router.push(`/search?q=${encodeURIComponent(query.trim())}`);
  }

  function toggleType(type: Entity['type']) {
    const nextTypes = filters.entityTypes?.includes(type)
      ? filters.entityTypes.filter((value) => value !== type)
      : [...(filters.entityTypes ?? []), type];

    const next = { ...filters, entityTypes: nextTypes };
    setFilters(next);
    runSearch(query, next);
  }

  function resetFilters() {
    const reset: SearchFilters = {
      dateFrom: '',
      dateTo: '',
      minAmount: undefined,
      maxAmount: undefined,
      state: '',
      entityTypes: [],
      flaggedOnly: false,
    };
    setFilters(reset);
    runSearch(query, reset);
  }

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <div className="rounded-2xl border border-white/12 bg-white/4 p-4 sm:p-5">
        <div className="flex flex-col gap-3 lg:flex-row">
          <Input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search officials, PACs, corporations, lobbyists..."
            className="text-base"
          />
          <Button onClick={submitSearch}>Search database</Button>
        </div>

        <div className="mt-4 grid gap-3 md:grid-cols-2 xl:grid-cols-4">
          <div>
            <label className="mb-1 block text-xs text-gray-400">
              <CalendarRange className="mr-1 inline h-3.5 w-3.5" /> Date from
            </label>
            <Input
              type="date"
              value={filters.dateFrom ?? ''}
              onChange={(e) => {
                const next = { ...filters, dateFrom: e.target.value };
                setFilters(next);
                runSearch(query, next);
              }}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-gray-400">Date to</label>
            <Input
              type="date"
              value={filters.dateTo ?? ''}
              onChange={(e) => {
                const next = { ...filters, dateTo: e.target.value };
                setFilters(next);
                runSearch(query, next);
              }}
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-gray-400">
              <DollarSign className="mr-1 inline h-3.5 w-3.5" /> Min amount (USD)
            </label>
            <Input
              type="number"
              min={0}
              value={filters.minAmount ?? ''}
              onChange={(e) => {
                const value = e.target.value ? Number(e.target.value) : undefined;
                const next = { ...filters, minAmount: value };
                setFilters(next);
                runSearch(query, next);
              }}
              placeholder="100000"
            />
          </div>
          <div>
            <label className="mb-1 block text-xs text-gray-400">State selector</label>
            <Select
              value={filters.state || 'ALL'}
              onChange={(e) => {
                const value = e.target.value === 'ALL' ? '' : e.target.value;
                const next = { ...filters, state: value };
                setFilters(next);
                runSearch(query, next);
              }}
            >
              {states.map((state) => (
                <option key={state} value={state} className="bg-[#0a0f1c]">
                  {state === 'ALL' ? 'All states' : state}
                </option>
              ))}
            </Select>
          </div>
        </div>

        <div className="mt-4 flex flex-wrap items-center gap-2">
          <div className="inline-flex items-center gap-1 rounded-lg border border-white/10 bg-white/4 px-2.5 py-1 text-xs text-gray-400">
            <Filter className="h-3.5 w-3.5" /> Filters
          </div>
          {entityTypes.map((type) => (
            <button
              key={type}
              onClick={() => toggleType(type)}
              className={`rounded-lg border px-2.5 py-1 text-xs transition-colors ${
                filters.entityTypes?.includes(type)
                  ? 'border-blue-500/40 bg-blue-500/15 text-blue-200'
                  : 'border-white/12 bg-white/4 text-gray-300 hover:text-white'
              }`}
            >
              {type}
            </button>
          ))}
          <button
            onClick={() => {
              const next = { ...filters, flaggedOnly: !filters.flaggedOnly };
              setFilters(next);
              runSearch(query, next);
            }}
            className={`rounded-lg border px-2.5 py-1 text-xs transition-colors ${
              filters.flaggedOnly
                ? 'border-red-500/35 bg-red-500/15 text-red-200'
                : 'border-white/12 bg-white/4 text-gray-300 hover:text-white'
            }`}
          >
            Flagged only
          </button>
          {activeFilterCount > 0 ? (
            <Button variant="ghost" size="sm" onClick={resetFilters}>
              Clear {activeFilterCount} filters
            </Button>
          ) : null}
        </div>
      </div>

      <div className="mt-6 flex items-center justify-between">
        <p className="text-sm text-gray-400">
          {loading ? 'Loading results...' : `${results.length} result${results.length === 1 ? '' : 's'}`}
        </p>
        {error ? (
          <Button variant="outline" size="sm" onClick={() => runSearch()}>
            <RefreshCcw className="h-3.5 w-3.5" /> Retry
          </Button>
        ) : null}
      </div>

      {error ? (
        <div className="mt-4 rounded-xl border border-red-500/30 bg-red-500/10 p-4 text-sm text-red-200">{error}</div>
      ) : null}

      {loading ? (
        <div className="mt-5 grid gap-3">
          {Array.from({ length: 5 }).map((_, idx) => (
            <Skeleton key={idx} className="h-28 w-full" />
          ))}
        </div>
      ) : results.length === 0 ? (
        <div className="mt-6">
          <EmptyState
            icon={AlertTriangle}
            title="No entities matched"
            description="Adjust filters or broaden your date/amount range to see matching entities."
          />
        </div>
      ) : (
        <div className="mt-5 space-y-3">
          {results.map((entity) => (
            <Link key={entity.id} href={`/entity/${entity.id}`}>
              <Card hover>
                <CardContent className="p-5">
                  <div className="flex flex-col justify-between gap-4 sm:flex-row sm:items-start">
                    <div>
                      <div className="flex flex-wrap items-center gap-2">
                        <h3 className="text-base font-semibold text-white">{entity.name}</h3>
                        <Badge variant="blue" className="capitalize">
                          {entity.type}
                        </Badge>
                        {entity.flagged ? <Badge variant="red">Flagged</Badge> : null}
                      </div>
                      <p className="mt-1 text-sm text-gray-400">
                        {entity.role} {entity.state ? `· ${entity.state}` : ''} {entity.party ? `· ${entity.party}` : ''}
                      </p>
                      <div className="mt-3 grid grid-cols-2 gap-2 text-xs text-gray-400 sm:grid-cols-4">
                        <span>Connections: {entity.connectionCount}</span>
                        <span>Sources: {entity.sourceCount}</span>
                        <span>Updated: {entity.lastUpdated}</span>
                        <span>Preview score: {(entity.sourceCount / Math.max(1, entity.connectionCount)).toFixed(2)}</span>
                      </div>
                    </div>
                    <div className="rounded-xl border border-white/10 bg-white/4 px-3 py-2 text-xs text-gray-300">
                      Open profile →
                    </div>
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      )}

      <SourceNote text="Source attribution: FEC, Congress.gov, Senate LDA, OpenSecrets and other public filings." />
    </div>
  );
}

export default function SearchPage() {
  return (
    <Suspense
      fallback={
        <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
          <Skeleton className="h-32 w-full" />
        </div>
      }
    >
      <SearchView />
    </Suspense>
  );
}
