'use client';

import { useState, useEffect, useCallback } from 'react';
import Link from 'next/link';
import {
  AlertTriangle,
  ExternalLink,
  ArrowUpDown,
  ArrowUp,
  ArrowDown,
} from 'lucide-react';
import { getTimingCorrelations, type TimingCorrelation } from '@/lib/api';
import {
  Table,
  TableHead,
  TableBody,
  TableRow,
  TableHeader,
  TableCell,
} from '@/components/ui/Table';
import { Badge } from '@/components/ui/Badge';
import { Button } from '@/components/ui/Button';

type SortKey = 'official' | 'daysBetween' | 'correlationType' | 'flagged';
type SortDir = 'asc' | 'desc';

const TYPE_BADGE: Record<
  TimingCorrelation['correlationType'],
  { variant: 'blue' | 'default' | 'yellow' | 'green' | 'red'; label: string }
> = {
  vote: { variant: 'blue', label: 'Vote' },
  donation: { variant: 'yellow', label: 'Donation' },
  meeting: { variant: 'default', label: 'Meeting' },
  regulation: { variant: 'green', label: 'Regulation' },
  appointment: { variant: 'default', label: 'Appointment' },
};

export default function TimingPage() {
  const [rows, setRows] = useState<TimingCorrelation[]>([]);
  const [sorted, setSorted] = useState<TimingCorrelation[]>([]);
  const [sortKey, setSortKey] = useState<SortKey>('daysBetween');
  const [sortDir, setSortDir] = useState<SortDir>('asc');
  const [flaggedOnly, setFlaggedOnly] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    getTimingCorrelations().then((data) => {
      setRows(data);
      setLoading(false);
    });
  }, []);

  const applySort = useCallback(
    (data: TimingCorrelation[], key: SortKey, dir: SortDir) => {
      const sorted = [...data].sort((a, b) => {
        let va: string | number | boolean = a[key];
        let vb: string | number | boolean = b[key];
        if (typeof va === 'string') va = va.toLowerCase();
        if (typeof vb === 'string') vb = vb.toLowerCase();
        if (va < vb) return dir === 'asc' ? -1 : 1;
        if (va > vb) return dir === 'asc' ? 1 : -1;
        return 0;
      });
      return sorted;
    },
    []
  );

  useEffect(() => {
    const filtered = flaggedOnly ? rows.filter((r) => r.flagged) : rows;
    setSorted(applySort(filtered, sortKey, sortDir));
  }, [rows, sortKey, sortDir, flaggedOnly, applySort]);

  function handleSort(key: SortKey) {
    if (key === sortKey) {
      setSortDir((d) => (d === 'asc' ? 'desc' : 'asc'));
    } else {
      setSortKey(key);
      setSortDir('asc');
    }
  }

  function SortIcon({ col }: { col: SortKey }) {
    if (col !== sortKey) return <ArrowUpDown className="w-3 h-3 opacity-40" />;
    return sortDir === 'asc' ? (
      <ArrowUp className="w-3 h-3 text-blue-400" />
    ) : (
      <ArrowDown className="w-3 h-3 text-blue-400" />
    );
  }

  return (
    <div className="max-w-7xl mx-auto px-4 sm:px-6 py-10">
      {/* Header */}
      <div className="mb-8">
        <h1 className="text-3xl font-bold text-white mb-2">
          Timing Correlations
        </h1>
        <p className="text-gray-400 max-w-2xl">
          Days elapsed between financial events and official legislative actions.
          All events sourced from public records. Correlations are{' '}
          <span className="text-white font-medium">not</span> claims of
          causation.
        </p>
      </div>

      {/* Controls */}
      <div className="flex flex-wrap items-center gap-3 mb-6">
        <Button
          variant={flaggedOnly ? 'danger' : 'secondary'}
          size="sm"
          onClick={() => setFlaggedOnly(!flaggedOnly)}
        >
          <AlertTriangle className="w-3.5 h-3.5" />
          {flaggedOnly ? 'Showing flagged' : 'Show flagged only'}
        </Button>
        <span className="text-sm text-gray-500">
          {sorted.length} correlation{sorted.length !== 1 ? 's' : ''}
        </span>
      </div>

      {/* Legend */}
      <div className="flex items-center gap-3 mb-4 p-3 rounded-xl bg-red-500/5 border border-red-500/10">
        <div className="w-3 h-3 rounded-sm bg-red-500/20 border border-red-500/30 shrink-0" />
        <p className="text-xs text-red-300">
          Red rows indicate flagged correlations — events within a short window
          of each other with high financial magnitude. Sourced from FEC filings,
          LDA disclosures, and congressional records.
        </p>
      </div>

      {/* Table */}
      {loading ? (
        <div className="space-y-2">
          {[1, 2, 3, 4].map((i) => (
            <div
              key={i}
              className="h-16 rounded-xl bg-white/4 animate-pulse"
            />
          ))}
        </div>
      ) : (
        <div className="rounded-xl border border-white/8 overflow-hidden">
          <Table>
            <TableHead>
              <TableRow>
                <TableHeader
                  sortable
                  onClick={() => handleSort('official')}
                >
                  <span className="flex items-center gap-1.5">
                    Official <SortIcon col="official" />
                  </span>
                </TableHeader>
                <TableHeader>Event A</TableHeader>
                <TableHeader>Event B</TableHeader>
                <TableHeader
                  sortable
                  onClick={() => handleSort('daysBetween')}
                >
                  <span className="flex items-center gap-1.5">
                    Days <SortIcon col="daysBetween" />
                  </span>
                </TableHeader>
                <TableHeader
                  sortable
                  onClick={() => handleSort('correlationType')}
                >
                  <span className="flex items-center gap-1.5">
                    Type <SortIcon col="correlationType" />
                  </span>
                </TableHeader>
                <TableHeader
                  sortable
                  onClick={() => handleSort('flagged')}
                >
                  <span className="flex items-center gap-1.5">
                    Flagged <SortIcon col="flagged" />
                  </span>
                </TableHeader>
                <TableHeader>Sources</TableHeader>
              </TableRow>
            </TableHead>
            <TableBody>
              {sorted.map((row) => (
                <TableRow key={row.id} flagged={row.flagged}>
                  <TableCell>
                    <Link
                      href={`/entity/${row.officialId}`}
                      className="text-white font-medium hover:text-blue-400 transition-colors whitespace-nowrap"
                    >
                      {row.official}
                    </Link>
                  </TableCell>
                  <TableCell>
                    <div className="max-w-xs">
                      <p className="text-sm text-gray-300">{row.eventA}</p>
                      <p className="text-xs text-gray-600 mt-0.5">
                        {row.eventADate}
                      </p>
                    </div>
                  </TableCell>
                  <TableCell>
                    <div className="max-w-xs">
                      <p className="text-sm text-gray-300">{row.eventB}</p>
                      <p className="text-xs text-gray-600 mt-0.5">
                        {row.eventBDate}
                      </p>
                    </div>
                  </TableCell>
                  <TableCell>
                    <span
                      className={`text-lg font-bold ${
                        row.flagged ? 'text-red-400' : 'text-white'
                      }`}
                    >
                      {row.daysBetween}
                    </span>
                  </TableCell>
                  <TableCell>
                    <Badge
                      variant={TYPE_BADGE[row.correlationType].variant}
                    >
                      {TYPE_BADGE[row.correlationType].label}
                    </Badge>
                  </TableCell>
                  <TableCell>
                    {row.flagged ? (
                      <Badge variant="red">
                        <AlertTriangle className="w-2.5 h-2.5" />
                        Yes
                      </Badge>
                    ) : (
                      <span className="text-gray-600 text-xs">—</span>
                    )}
                  </TableCell>
                  <TableCell>
                    <div className="flex flex-col gap-1">
                      {row.sources.map((src) => (
                        <a
                          key={src.id}
                          href={src.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="inline-flex items-center gap-1 text-xs text-blue-400 hover:text-blue-300 whitespace-nowrap"
                        >
                          <ExternalLink className="w-2.5 h-2.5 shrink-0" />
                          {src.publisher}
                        </a>
                      ))}
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        </div>
      )}

      {/* Disclaimer */}
      <p className="mt-6 text-xs text-gray-600 text-center max-w-2xl mx-auto">
        All data sourced from Federal Election Commission filings, U.S. Senate
        Lobbying Disclosure Act database, Congress.gov vote records, and
        OpenSecrets. Temporal proximity is not evidence of causation.
      </p>
    </div>
  );
}
