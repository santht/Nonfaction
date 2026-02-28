'use client';

import { useEffect, useMemo, useState } from 'react';
import Link from 'next/link';
import { LayoutGrid, List, RefreshCcw } from 'lucide-react';
import { getOfficials, type Official } from '@/lib/api';
import { Avatar } from '@/components/ui/Avatar';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';
import { Card, CardContent } from '@/components/ui/Card';
import { Select } from '@/components/ui/Select';
import { Skeleton } from '@/components/ui/Skeleton';
import { SourceNote } from '@/components/ui/SourceNote';

export default function OfficialsPage() {
  const [items, setItems] = useState<Official[]>([]);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  const [stateFilter, setStateFilter] = useState('ALL');
  const [partyFilter, setPartyFilter] = useState('ALL');
  const [chamberFilter, setChamberFilter] = useState('ALL');
  const [flaggedOnly, setFlaggedOnly] = useState(false);
  const [viewMode, setViewMode] = useState<'grid' | 'list'>('grid');

  async function loadOfficials() {
    setLoading(true);
    setError(null);
    try {
      const data = await getOfficials({
        state: stateFilter === 'ALL' ? undefined : stateFilter,
        party: partyFilter === 'ALL' ? undefined : (partyFilter as Official['party']),
        chamber: chamberFilter === 'ALL' ? undefined : (chamberFilter as Official['chamber']),
        flaggedOnly,
      });
      setItems(data);
    } catch {
      setError('Unable to load officials directory.');
    } finally {
      setLoading(false);
    }
  }

  useEffect(() => {
    loadOfficials();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [stateFilter, partyFilter, chamberFilter, flaggedOnly]);

  const knownStates = useMemo(() => ['ALL', 'TX', 'CA', 'FL', 'NM', 'OH', 'IL'], []);

  return (
    <div className="mx-auto max-w-7xl px-4 py-10 sm:px-6">
      <div className="mb-6 flex flex-col gap-4 md:flex-row md:items-end md:justify-between">
        <div>
          <h1 className="text-3xl font-semibold text-white">Officials Directory</h1>
          <p className="mt-1 text-sm text-gray-400">Filter by state, party, chamber, and flag status.</p>
        </div>
        <div className="inline-flex rounded-xl border border-white/10 bg-white/4 p-1">
          <button
            type="button"
            onClick={() => setViewMode('grid')}
            className={`rounded-lg px-2.5 py-1.5 text-sm ${viewMode === 'grid' ? 'bg-white/12 text-white' : 'text-gray-400'}`}
          >
            <LayoutGrid className="mr-1 inline h-4 w-4" /> Grid
          </button>
          <button
            type="button"
            onClick={() => setViewMode('list')}
            className={`rounded-lg px-2.5 py-1.5 text-sm ${viewMode === 'list' ? 'bg-white/12 text-white' : 'text-gray-400'}`}
          >
            <List className="mr-1 inline h-4 w-4" /> List
          </button>
        </div>
      </div>

      <div className="grid gap-3 md:grid-cols-4">
        <Select value={stateFilter} onChange={(e) => setStateFilter(e.target.value)}>
          {knownStates.map((state) => (
            <option key={state} value={state} className="bg-[#0a0f1c]">{state === 'ALL' ? 'All states' : state}</option>
          ))}
        </Select>
        <Select value={partyFilter} onChange={(e) => setPartyFilter(e.target.value)}>
          <option className="bg-[#0a0f1c]" value="ALL">All parties</option>
          <option className="bg-[#0a0f1c]" value="Democrat">Democrat</option>
          <option className="bg-[#0a0f1c]" value="Republican">Republican</option>
          <option className="bg-[#0a0f1c]" value="Independent">Independent</option>
        </Select>
        <Select value={chamberFilter} onChange={(e) => setChamberFilter(e.target.value)}>
          <option className="bg-[#0a0f1c]" value="ALL">All chambers</option>
          <option className="bg-[#0a0f1c]" value="Senate">Senate</option>
          <option className="bg-[#0a0f1c]" value="House">House</option>
          <option className="bg-[#0a0f1c]" value="Governor">Governor</option>
        </Select>
        <Button variant={flaggedOnly ? 'danger' : 'outline'} onClick={() => setFlaggedOnly((v) => !v)}>
          {flaggedOnly ? 'Flagged only' : 'All status'}
        </Button>
      </div>

      {error ? (
        <Card className="mt-4 border-red-500/35 bg-red-500/10">
          <CardContent className="flex items-center justify-between p-4 text-sm text-red-200">
            {error}
            <Button size="sm" variant="outline" onClick={loadOfficials}><RefreshCcw className="h-3.5 w-3.5" /> Retry</Button>
          </CardContent>
        </Card>
      ) : null}

      {loading ? (
        <div className="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {Array.from({ length: 6 }).map((_, idx) => <Skeleton key={idx} className="h-32" />)}
        </div>
      ) : viewMode === 'grid' ? (
        <div className="mt-5 grid gap-3 md:grid-cols-2 xl:grid-cols-3">
          {items.map((item) => (
            <Link key={item.id} href={`/officials/${item.id}`}>
              <Card hover>
                <CardContent className="p-4">
                  <div className="flex items-start gap-3">
                    <Avatar name={item.name} src={item.photoUrl} />
                    <div className="flex-1">
                      <div className="flex items-center justify-between gap-2">
                        <p className="text-sm font-medium text-white">{item.name}</p>
                        {item.flagged ? <Badge variant="red">Flagged</Badge> : <Badge variant="green">Clear</Badge>}
                      </div>
                      <p className="text-xs text-gray-400">{item.role}</p>
                      <p className="mt-2 text-xs text-gray-500">{item.party} · {item.state} · {item.chamber}</p>
                      <p className="mt-1 text-xs text-blue-300">Connections: {item.connectionCount}</p>
                    </div>
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      ) : (
        <div className="mt-5 space-y-2">
          {items.map((item) => (
            <Link key={item.id} href={`/officials/${item.id}`}>
              <Card hover>
                <CardContent className="flex items-center justify-between gap-3 p-4">
                  <div>
                    <p className="text-sm font-medium text-white">{item.name}</p>
                    <p className="text-xs text-gray-400">{item.role} · {item.party} · {item.state}</p>
                  </div>
                  <div className="flex items-center gap-2">
                    <Badge variant="outline">{item.connectionCount} links</Badge>
                    {item.flagged ? <Badge variant="red">Flagged</Badge> : null}
                  </div>
                </CardContent>
              </Card>
            </Link>
          ))}
        </div>
      )}

      <SourceNote text="Source attribution: directory records are derived from public office metadata and source-linked entity graphs." />
    </div>
  );
}
